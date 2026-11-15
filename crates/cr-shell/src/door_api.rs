//! wire.ws-door's contract, copied verbatim from repo:wire-ws-door/crates/wire-ws-door-api (same token text, same
//! schema hash; tests/door_api.rs pins it), so this crate builds against one ce-wire. Major 2: Mint bound to the
//! caller's grant (`wire.ws-door:mint-for:..`), refusals as `NotAllowed`.
/// repo:wire-ws-door/crates/wire-ws-door-api
pub mod wire_ws_door {
ce_wire::api! {
    api WireWsDoor = "wire.ws-door", version 2;
    types {
        /// Health contract: these three fields, in this order, in every cell.
        pub struct Health { pub ok: bool, pub version: String, pub uptime_s: u64 }
        /// `page`: the app's URL the grant is appended to (`<page>#g=...`). `abilities`: what the browser may do,
        /// e.g. ["doc.workspace:read"]. `ttl_s`: 60 to 86400.
        pub struct MintReq { pub page: String, pub abilities: Vec<String>, pub ttl_s: u32 }
        /// `link`: page + fragment; the fragment is the secret and a browser never sends it to a server.
        pub struct Minted { pub link: String, pub not_after: u64, pub browser_key: String }
        /// `NotAllowed`: an asked ability outside what the caller's own grant lets it mint (its
        /// `wire.ws-door:mint-for:<pattern>` abilities), or one no caller but this node may ever mint (keys,
        /// cell.install, this door, any own-key-only cell, a wildcard, `delegate:`).
        pub enum MintError { Invalid(String), Failed(String), NotAllowed { ability: String, why: String } }
    }
    ops {
        1 => HealthOp(()) -> Health, needs "wire.ws-door:health";
        2 => Mint(MintReq) -> Result<Minted, MintError>, needs "wire.ws-door:mint";
    }
}
}
