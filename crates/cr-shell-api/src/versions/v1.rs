//! FROZEN: cr.shell major 1, the first edition's contract, byte for byte. Served beside major 2. Never edited.
//!
//! `cr.shell`: the control room's shell cell. `Session` tells a signed-in page who it is and which panels its chain
//! can use, so the shell loads only those panels' code. Types only; the TypeScript client is in sdk/ts-web
//! (ce-wire-gen). Op 1 is the health contract every cell exports.

ce_wire::api! {
    api CrShell = "cr.shell", version 1;
    types {
        /// Health contract: these three fields, in this order, in every cell.
        pub struct Health { pub ok: bool, pub version: String, pub uptime_s: u64 }
        /// One panel this viewer can open, with the rights of its chain that the panel's ops need.
        pub struct PanelGrant { pub panel: String, pub api: String, pub rights: Vec<String>, pub scoped: bool }
        /// A panel the viewer cannot open, and the right that would open it.
        pub struct PanelRefused { pub panel: String, pub needs: String }
        /// person: who the sign-in named when the grant was minted. role: the ce-iam role the grant came from.
        /// chain_id: the grant's id as every cell's audit row names it. expires_at_ms: when the grant ends.
        pub struct SessionInfo {
            pub person: String, pub role: String, pub chain_id: String, pub expires_at_ms: u64,
            pub panels: Vec<PanelGrant>, pub refused: Vec<PanelRefused>
        }
        pub enum CrError { Refused { needs: String }, Source { cell: String, why: String }, Invalid { why: String }, NotFound { what: String } }
    }
    ops {
        1 => HealthOp(()) -> Health, needs "cr.shell:health";
        2 => Session(()) -> Result<SessionInfo, CrError>, needs "cr.shell:session";
    }
}
