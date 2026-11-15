//! The control room's policy, declared once. Two roles:
//!
//! - `root` (Leif): every control-room right, every field, every row.
//! - `operator` (a named person, until a date): the shell, devices and the audit trail, read only; rows only of the
//!   nodes and apps their entry names (`where` caveats signed into their grant); no `tailscale`, `passkeys` or
//!   `detail` field; no right of keys, renewals or approvals, so their browser receives no byte naming those cells.
//!
//! Who holds which is the people file (`~/.config/control-room/people.json`, never in this repo):
//! `{"leif": {"role": "root"}, "<person>": {"role": "operator", "until": "2026-12-31", "nodes": ["<node>"], "apps": ["orch"]}}`.
//! A person with no entry, or past `until`, gets no grant.
#![forbid(unsafe_code)]

use serde::Deserialize;
use std::collections::BTreeMap;

pub struct Role {
    pub name: &'static str,
    pub rights: &'static [&'static str],
}

pub const ROOT: Role = Role {
    name: "root",
    rights: &[
        "cr.shell:health", "cr.shell:open", "cr.shell:session",
        "cr.keys:health", "cr.keys:read", "cr.keys:holders", "cr.keys:history",
        "cr.devices:health", "cr.devices:read", "cr.devices:net", "cr.devices:passkeys",
        "cr.audit:health", "cr.audit:read", "cr.audit:detail", "cr.audit:verify",
    ],
};

pub const OPERATOR: Role = Role {
    name: "operator",
    rights: &["cr.shell:health", "cr.shell:open", "cr.shell:session", "cr.devices:health", "cr.devices:read", "cr.audit:health", "cr.audit:read"],
};

pub const ROLES: &[&Role] = &[&ROOT, &OPERATOR];

/// A grant lasts 8 hours; the page signs in again after that.
pub const TTL_S: u32 = 8 * 3600;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Person {
    pub role: String,
    /// last day (YYYY-MM-DD, UTC) the person holds the role; none = no end
    #[serde(default)]
    pub until: Option<String>,
    /// operator only: the nodes and apps their rows are caveated to
    #[serde(default)]
    pub nodes: Vec<String>,
    #[serde(default)]
    pub apps: Vec<String>,
}

pub fn people(text: &str) -> Result<BTreeMap<String, Person>, String> {
    serde_json::from_str(text).map_err(|e| format!("people.json: {e}"))
}

pub fn valid_person(p: &str) -> bool {
    !p.is_empty() && p.len() <= 64 && p.chars().all(|c| c.is_ascii_alphanumeric() || "-_.@".contains(c))
}

fn valid_value(v: &str) -> bool {
    !v.is_empty() && v.len() <= 64 && v.chars().all(|c| c.is_ascii_alphanumeric() || "-_.@".contains(c))
}

/// Days since 1970-01-01 of a YYYY-MM-DD date (proleptic Gregorian), or None.
pub fn day_of(date: &str) -> Option<i64> {
    let mut it = date.split('-').map(|x| x.parse::<i64>().ok());
    let (y, m, d) = (it.next()??, it.next()??, it.next()??);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

/// The role and the abilities a person's browser grant carries, or why there is none (an `Err` starting "refused").
pub fn abilities(person: &str, people: &BTreeMap<String, Person>, now_ms: u64) -> Result<(String, Vec<String>), String> {
    if !valid_person(person) {
        return Err(format!("refused: {person:?} is not a signed-in person"));
    }
    let p = people.get(person).ok_or_else(|| "refused: no control-room role for this person".to_string())?;
    if let Some(u) = &p.until {
        let last = day_of(u).ok_or_else(|| format!("people.json: {person}: until {u:?} is not a date"))?;
        if (now_ms / 86_400_000) as i64 > last {
            return Err(format!("refused: {person}'s role ended on {u}"));
        }
    }
    let role = ROLES.iter().find(|r| r.name == p.role).ok_or_else(|| format!("people.json: {person}: no role {:?}", p.role))?;
    let mut a = vec![
        format!("cr.shell:person={person}"),
        format!("cr.shell:role={}", role.name),
        format!("cr.shell:until={}", now_ms + TTL_S as u64 * 1000),
    ];
    a.extend(role.rights.iter().map(|r| r.to_string()));
    if role.name == OPERATOR.name {
        // an operator always comes with caveats: no node or app named means no rows, never every row
        let nodes: Vec<&String> = p.nodes.iter().filter(|v| valid_value(v)).collect();
        let apps: Vec<&String> = p.apps.iter().filter(|v| valid_value(v)).collect();
        a.push(format!("cr.devices:where:node={}", if nodes.is_empty() { "-".to_string() } else { nodes.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("|") }));
        a.push(format!("cr.audit:where:app={}", if apps.is_empty() { "-".to_string() } else { apps.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("|") }));
        if !nodes.is_empty() {
            a.push(format!("cr.audit:where:node={}", nodes.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("|")));
        }
    }
    Ok((role.name.to_string(), a))
}

/// An ability covers a right: equal, `*`, or `<domain>:*` (ce-cap's rule).
pub fn covers(abilities: &[String], right: &str) -> bool {
    abilities.iter().any(|a| a == right || a == "*" || a.strip_suffix(":*").is_some_and(|d| right.split(':').next() == Some(d)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ppl() -> BTreeMap<String, Person> {
        people(r#"{"leif": {"role": "root"}, "ops": {"role": "operator", "until": "2026-12-31", "nodes": ["ops-1"], "apps": ["orch"]}, "gone": {"role": "operator", "until": "2026-01-01"}}"#).unwrap()
    }
    const NOV_15_2026: u64 = 1_794_700_000_000;

    #[test]
    fn root_and_operator() {
        let (r, a) = abilities("leif", &ppl(), NOV_15_2026).unwrap();
        assert_eq!(r, "root");
        assert!(covers(&a, "cr.keys:read") && covers(&a, "cr.keys:holders"));
        assert!(!a.iter().any(|x| x.contains(":where:")));
        let (r, a) = abilities("ops", &ppl(), NOV_15_2026).unwrap();
        assert_eq!(r, "operator");
        assert!(!a.iter().any(|x| x.starts_with("cr.keys") || x.starts_with("cr.renewals") || x.starts_with("cr.approvals")));
        assert!(!covers(&a, "cr.devices:net") && !covers(&a, "cr.audit:detail"));
        assert!(a.contains(&"cr.devices:where:node=ops-1".to_string()) && a.contains(&"cr.audit:where:app=orch".to_string()));
    }

    #[test]
    fn nobody_without_a_role_or_past_their_date() {
        assert!(abilities("stranger", &ppl(), NOV_15_2026).unwrap_err().starts_with("refused"));
        assert!(abilities("gone", &ppl(), NOV_15_2026).unwrap_err().starts_with("refused"));
        assert!(abilities("bad name", &ppl(), NOV_15_2026).unwrap_err().starts_with("refused"));
        assert_eq!(day_of("1970-01-01"), Some(0));
        assert_eq!(day_of("2026-11-15"), Some(20772));
    }

    #[test]
    fn an_operator_without_scope_gets_no_rows_not_every_row() {
        let p = people(r#"{"x": {"role": "operator"}}"#).unwrap();
        let (_, a) = abilities("x", &p, NOV_15_2026).unwrap();
        assert!(a.contains(&"cr.devices:where:node=-".to_string()));
    }

    #[test]
    fn no_role_holds_a_right_outside_the_control_room() {
        for r in ROLES {
            assert!(r.rights.iter().all(|a| a.starts_with("cr.")), "{}", r.name);
            assert!(!r.rights.iter().any(|a| a.ends_with('*')), "{}", r.name);
        }
    }
}
