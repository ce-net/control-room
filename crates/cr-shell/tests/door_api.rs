//! The copied wire.ws-door contract hashes to what the live door runs (`ce cell ops wire.ws-door`, 2026-11-15, major 2).
use ce_wire::Api;
#[test]
fn door_schema_matches_the_live_cell() {
    assert_eq!(<cr_shell::door_api::wire_ws_door::WireWsDoor as Api>::SCHEMA, 0xbf52da0b93eb061b);
}
