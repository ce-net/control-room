//! The one function of `cr.shell`: a signed-in person gets a browser grant attenuated to their control-room role, and
//! the page learns from `Session` which panels that grant can use.
//!
//! Who gets which role is `people.json` (`{"<person>": "<role>"}`); what a role may do is its ce-iam policy file
//! (`roles/<role>.json`, `ce-iam-policy-v1`, the same files `ce-iam role put` takes). A person with no role gets no
//! grant. The grant also carries `cr.shell:person=`, `cr.shell:role=` and `cr.shell:until=` abilities, signed into the
//! chain by this node, so `Session` reads them from the verified leaf and never from the page.
#![forbid(unsafe_code)]

pub mod door_api;
pub mod web;

use cr_shell_api::*;
use std::collections::BTreeMap;
use std::path::Path;

/// Grants last 8 hours; the page signs in again after that.
pub const TTL_S: u32 = 8 * 3600;

/// Every panel of slice 1: name, its backend cell, and the right its ops need.
pub const PANELS: &[(&str, &str, &str)] = &[
    ("keys", "cr.keys", "cr.keys:read"),
    ("devices", "cr.devices", "cr.devices:read"),
    ("audit", "cr.audit", "cr.audit:read"),
];

/// Rights a panel uses beyond its read right (shown so a viewer knows what their panel can do).
pub const EXTRA: &[(&str, &str)] = &[("keys", "cr.keys:holders"), ("audit", "cr.audit:verify")];

#[derive(Debug, Clone, PartialEq)]
pub struct Role {
    pub name: String,
    pub actions: Vec<String>,
}

/// A role from its ce-iam policy text: every Allow statement's actions, in order. Anything else is refused.
pub fn parse_role(name: &str, text: &str) -> Result<Role, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("{name}: {e}"))?;
    if v["version"] != "ce-iam-policy-v1" {
        return Err(format!("{name}: not a ce-iam-policy-v1 document"));
    }
    let mut actions = Vec::new();
    for s in v["statements"].as_array().ok_or_else(|| format!("{name}: no statements"))? {
        if s["effect"] != "Allow" {
            return Err(format!("{name}: only Allow statements"));
        }
        for a in s["actions"].as_array().ok_or_else(|| format!("{name}: statement without actions"))? {
            let a = a.as_str().ok_or_else(|| format!("{name}: action is not text"))?;
            if !valid_ability(a) || !a.starts_with("cr.") {
                return Err(format!("{name}: {a:?} is not a control-room ability"));
            }
            actions.push(a.to_string());
        }
    }
    Ok(Role { name: name.into(), actions })
}

pub fn valid_ability(a: &str) -> bool {
    !a.is_empty() && a.len() <= 128 && a.contains(':') && a.chars().all(|c| c.is_ascii_alphanumeric() || "-_.@:=*".contains(c))
}

pub fn valid_person(p: &str) -> bool {
    !p.is_empty() && p.len() <= 64 && p.chars().all(|c| c.is_ascii_alphanumeric() || "-_.@".contains(c))
}

/// `people.json`: person -> role name.
pub fn parse_people(text: &str) -> Result<BTreeMap<String, String>, String> {
    serde_json::from_str(text).map_err(|e| format!("people.json: {e}"))
}

pub struct Config {
    pub people: BTreeMap<String, String>,
    pub roles: BTreeMap<String, Role>,
}

impl Config {
    /// `people.json` and every `<role>.json` in `dir`.
    pub fn load(dir: &Path, people: &Path) -> Result<Config, String> {
        let people = parse_people(&std::fs::read_to_string(people).map_err(|e| format!("{}: {e}", people.display()))?)?;
        let mut roles = BTreeMap::new();
        for e in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if let Some(name) = n.strip_suffix(".json").filter(|r| r.starts_with("control-room.")) {
                let r = parse_role(name, &std::fs::read_to_string(e.path()).map_err(|e| e.to_string())?)?;
                roles.insert(name.to_string(), r);
            }
        }
        Ok(Config { people, roles })
    }

    /// The abilities a person's browser grant carries, or why there is none.
    pub fn abilities(&self, person: &str, now_ms: u64) -> Result<(String, Vec<String>), CrError> {
        if !valid_person(person) {
            return Err(CrError::Invalid { why: format!("{person:?} is not a signed-in person") });
        }
        let role = self.people.get(person).ok_or_else(|| CrError::Refused { needs: "a control-room role for this person".into() })?;
        let r = self.roles.get(role).ok_or_else(|| CrError::Source { cell: "ce-iam".into(), why: format!("role {role} is not in the catalog") })?;
        let mut a = vec![
            format!("cr.shell:person={person}"),
            format!("cr.shell:role={role}"),
            format!("cr.shell:until={}", now_ms + TTL_S as u64 * 1000),
        ];
        a.extend(r.actions.iter().cloned());
        Ok((role.clone(), a))
    }
}

/// `Session` for a caller: who, which role, and which panels the verified chain can use.
pub fn session(c: &ce_wire::Caller) -> Result<SessionInfo, CrError> {
    let leaf = |k: &str| {
        let head = format!("{}cr.shell:{k}=", ce_wire::ABILITY_RIGHT);
        c.rights.iter().find_map(|r| r.strip_prefix(head.as_str())).unwrap_or("").to_string()
    };
    let person = leaf("person");
    if person.is_empty() {
        return Err(CrError::Refused { needs: "a grant minted by cr.shell after sign-in".into() });
    }
    let mut panels = Vec::new();
    let mut refused = Vec::new();
    for (p, api, right) in PANELS {
        if cr_view::holds(c, right) {
            let mut rights = vec![right.to_string()];
            rights.extend(EXTRA.iter().filter(|(q, x)| q == p && cr_view::holds(c, x)).map(|(_, x)| x.to_string()));
            let cell = api.to_string();
            let scoped = c.rights.iter().any(|r| r.strip_prefix(ce_wire::ABILITY_RIGHT).is_some_and(|a| a.starts_with(&format!("{cell}:scope:"))));
            panels.push(PanelGrant { panel: p.to_string(), api: cell, rights, scoped });
        } else {
            refused.push(PanelRefused { panel: p.to_string(), needs: right.to_string() });
        }
    }
    Ok(SessionInfo {
        person,
        role: leaf("role"),
        chain_id: cr_view::chain_id(c),
        expires_at_ms: leaf("until").parse().unwrap_or(0),
        panels,
        refused,
    })
}

pub trait Mint {
    /// A link `<page>#g=...` for a fresh browser key holding `abilities` for `ttl_s`, and its end (unix seconds).
    fn mint(&self, page: &str, abilities: &[String], ttl_s: u32) -> Result<(String, u64), String>;
}

/// The fragment of a minted link with the door the page must use.
pub fn fragment(link: &str, door: &str) -> Result<String, CrError> {
    let g = link.split_once('#').map(|(_, f)| f).filter(|f| f.starts_with("g="))
        .ok_or_else(|| CrError::Source { cell: "wire.ws-door".into(), why: "mint gave no grant".into() })?;
    Ok(format!("{g}&w={door}"))
}
