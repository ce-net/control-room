//! Major 1 (frozen contract `cr_shell_api::versions::v1`): the first edition's `Session`, unchanged. It names refused
//! panels, which major 2 never does; it is served only while the first edition's page is.
use cr_shell_api::versions::v1::*;

/// Every panel of slice 1: name, its backend cell, and the right its ops need.
pub const PANELS: &[(&str, &str, &str)] = &[
    ("keys", "cr.keys", "cr.keys:read"),
    ("devices", "cr.devices", "cr.devices:read"),
    ("audit", "cr.audit", "cr.audit:read"),
];

/// Rights a panel uses beyond its read right (shown so a viewer knows what their panel can do).
pub const EXTRA: &[(&str, &str)] = &[("keys", "cr.keys:holders"), ("audit", "cr.audit:verify")];

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
        if ui_chain::holds(c, right) {
            let mut rights = vec![right.to_string()];
            rights.extend(EXTRA.iter().filter(|(q, x)| q == p && ui_chain::holds(c, x)).map(|(_, x)| x.to_string()));
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
        chain_id: ui_chain::chain_id(c),
        expires_at_ms: leaf("until").parse().unwrap_or(0),
        panels,
        refused,
    })
}

