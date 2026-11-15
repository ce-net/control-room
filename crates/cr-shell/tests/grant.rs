//! The shell's own grant names exactly the one right it calls a source with: minting through wire.ws-door.
#[test]
fn the_cell_grant_is_narrow() {
    assert_eq!(cr_shell::GRANT, ["wire.ws-door:mint", "delegate:1"]);
}
