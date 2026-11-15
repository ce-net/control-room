//! The one function of `cr.shell`: the control room's layout for a viewer, and (in `main`) the site's gateway face.
//!
//! `Layout` names who the grant is for (person, role, end: abilities this node signed into the chain at sign-in,
//! read from the verified leaf) and the tabs of the slots the chain may open, in the site's order. A slot whose view
//! the chain may not have is not named, so a narrower viewer never learns it exists. Major 1's `Session` is in [`v1`].
#![forbid(unsafe_code)]

pub mod door_api;
pub mod v1;

use cr_shell_api::*;

pub const TITLE: &str = "Control room";

/// The built site this binary carries (ui/dist at build time).
pub mod site_files {
    include!(concat!(env!("OUT_DIR"), "/site.rs"));
}

/// Where the site is served from: `$CR_SHELL_WEB` when set, else the carried site written once to
/// `~/.local/share/cr-shell/site/<digest of the index>/` (read only to the gateway from then on).
pub fn site_dir() -> Result<std::path::PathBuf, String> {
    if let Ok(d) = std::env::var("CR_SHELL_WEB") {
        if !d.is_empty() {
            return Ok(d.into());
        }
    }
    let index = site_files::SITE.iter().find(|(n, _)| *n == "bundles.json").map(|(_, b)| *b).ok_or("this build carries no site (ui/dist was empty)")?;
    let tag: String = index.iter().fold(0xcbf29ce484222325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100000001b3)).to_le_bytes().iter().map(|b| format!("{b:02x}")).collect();
    let dir = std::path::PathBuf::from(format!("{}/.local/share/cr-shell/site/{tag}", std::env::var("HOME").unwrap_or_default()));
    if !dir.join("index.html").exists() {
        for (name, bytes) in site_files::SITE {
            let p = dir.join(name);
            std::fs::create_dir_all(p.parent().unwrap_or(&dir)).map_err(|e| e.to_string())?;
            std::fs::write(&p, bytes).map_err(|e| format!("{}: {e}", p.display()))?;
        }
    }
    Ok(dir)
}

/// `cr.shell:<k>=<v>` from the caller's verified leaf, "" when absent.
pub fn leaf(c: &ce_wire::Caller, k: &str) -> String {
    let head = format!("{}cr.shell:{k}=", ce_wire::ABILITY_RIGHT);
    c.rights.iter().find_map(|r| r.strip_prefix(head.as_str())).unwrap_or("").to_string()
}

pub fn layout(c: &ce_wire::Caller) -> LayoutView {
    let site = control_room_site::site();
    let tabs = control_room_site::TABS
        .iter()
        .filter(|(slot, _)| site.root.slots.iter().any(|(s, p)| s == slot && ui_chain::holds(c, p.view.needs)))
        .map(|(slot, label)| Tab { slot: slot.to_string(), label: label.to_string() })
        .collect();
    LayoutView { title: TITLE.into(), person: leaf(c, "person"), role: leaf(c, "role"), until_ms: leaf(c, "until").parse().unwrap_or(0), tabs }
}

/// The people file: `$CR_SHELL_PEOPLE` (the JSON itself, or a path to it), else `~/.config/control-room/people.json`,
/// else Leif alone as root (the policy's owner).
pub fn people_text() -> String {
    if let Ok(v) = std::env::var("CR_SHELL_PEOPLE") {
        if v.trim_start().starts_with('{') {
            return v;
        }
        if let Ok(t) = std::fs::read_to_string(&v) {
            return t;
        }
    }
    let own = format!("{}/.config/control-room/people.json", std::env::var("HOME").unwrap_or_default());
    std::fs::read_to_string(own).unwrap_or_else(|_| r#"{"leif": {"role": "root"}}"#.into())
}

/// The gateway's viewer: a person's role and grant abilities from the policy and the people file.
pub fn viewer(person: &str) -> Result<site_gateway::Viewer, String> {
    let people = control_room_policy::people(&people_text())?;
    let (role, abilities) = control_room_policy::abilities(person, &people, ui_chain::now_ms())?;
    Ok(site_gateway::Viewer { role, abilities })
}
