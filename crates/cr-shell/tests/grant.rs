//! The shell's own grant names exactly the one right it calls a source with (minting through wire.ws-door) and the
//! control room's own rights it may mint (and sub.gateway's board for the quota slot); never keys, cell.install, a
//! wildcard, or sub.gateway's ask or set.
#[test]
fn the_cell_grant_is_narrow() {
    let g = cr_shell::GRANT;
    assert_eq!(g[0], "wire.ws-door:mint");
    for a in g.iter().filter_map(|a| a.strip_prefix("wire.ws-door:mint-for:")) {
        let api = a.split(':').next().unwrap();
        // the control room's own cells as bare apis; sub.gateway only as its two read-only rights
        assert!(["cr.shell", "cr.keys", "cr.devices", "cr.audit"].contains(&a) || ["sub.gateway:health", "sub.gateway:board"].contains(&a), "{a}");
    }
    assert_eq!(g.iter().filter(|a| a.starts_with("wire.ws-door:mint-for:")).count(), 6);
    assert!(!g.iter().any(|a| *a == "*" || a.starts_with("keys") || a.starts_with("cell.install")));
    assert!(!cr_shell::door_abilities().contains(&"delegate:1"));
}
