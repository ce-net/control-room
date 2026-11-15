//! Two viewers, one shell: Leif's role and the operator's role get different panels from the same Session op, and
//! the abilities their grants carry come only from the ce-iam role files in roles/.
use ce_wire::Caller;
use cr_shell::*;
use std::path::Path;

fn cfg() -> Config {
    let d = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../roles");
    let mut c = Config::load(&d, &d.join("people.json")).unwrap();
    c.people.insert("ops".into(), "control-room.operator".into());
    c
}

/// What the gate hands a cell for a grant with these abilities: the op rights it covers, then every ability.
fn caller_for(abilities: &[String], op_rights: &[&str]) -> Caller {
    let mut rights: Vec<String> = op_rights.iter().filter(|r| abilities.iter().any(|a| a == *r)).map(|r| r.to_string()).collect();
    rights.extend(abilities.iter().map(|a| format!("ability:{a}")));
    rights.push("grant:00112233445566ff".into());
    Caller { name: "cd".repeat(32), rights }
}

#[test]
fn leif_sees_three_panels_and_the_operator_two() {
    let c = cfg();
    let (role, a) = c.abilities("leif", 1_000).unwrap();
    assert_eq!(role, "control-room.root");
    let s = session(&caller_for(&a, &["cr.shell:session"])).unwrap();
    assert_eq!(s.person, "leif");
    assert_eq!(s.panels.iter().map(|p| p.panel.as_str()).collect::<Vec<_>>(), ["keys", "devices", "audit"]);
    assert!(s.panels.iter().all(|p| !p.scoped));
    assert!(s.refused.is_empty());
    assert_eq!(s.expires_at_ms, 1_000 + TTL_S as u64 * 1000);
    assert_eq!(s.chain_id, "00112233445566ff");

    let (role, a) = c.abilities("ops", 1_000).unwrap();
    assert_eq!(role, "control-room.operator");
    let s = session(&caller_for(&a, &["cr.shell:session"])).unwrap();
    assert_eq!(s.panels.iter().map(|p| p.panel.as_str()).collect::<Vec<_>>(), ["devices", "audit"]);
    assert!(s.panels.iter().all(|p| p.scoped));
    assert_eq!(s.refused.len(), 1);
    assert_eq!(s.refused[0].panel, "keys");
    assert_eq!(s.refused[0].needs, "cr.keys:read");
    assert!(!a.iter().any(|x| x.starts_with("cr.keys")), "operator grant carries no keys right: {a:?}");
}

#[test]
fn nobody_without_a_role_gets_a_grant() {
    let c = cfg();
    assert!(matches!(c.abilities("stranger", 0), Err(cr_shell_api::CrError::Refused { .. })));
    assert!(matches!(c.abilities("bad name", 0), Err(cr_shell_api::CrError::Invalid { .. })));
}

#[test]
fn a_grant_not_minted_by_the_shell_has_no_session() {
    let a = vec!["cr.shell:session".to_string()];
    assert!(session(&caller_for(&a, &["cr.shell:session"])).is_err());
}

#[test]
fn no_role_holds_a_right_outside_the_control_room() {
    let c = cfg();
    for r in c.roles.values() {
        assert!(r.actions.iter().all(|a| a.starts_with("cr.")), "{}: {:?}", r.name, r.actions);
        assert!(!r.actions.iter().any(|a| a.contains("keys:read") && !a.starts_with("cr.keys:")), "{}", r.name);
    }
    assert!(parse_role("x", r#"{"version":"ce-iam-policy-v1","statements":[{"effect":"Allow","actions":["keys:read"],"resource":"any"}]}"#).is_err());
}
