//! `cr.shell@2`: the control room's shell, a full-stack cell. Its backend answers `Layout`: who the page is for and
//! the tabs of the slots this viewer's chain may open (only those; a slot the viewer may not have is not named). Its
//! view `ControlRoom` places the slot cells. Major 1 (`Session`, the first edition's panel list) is frozen and served.
#![forbid(unsafe_code)]

pub mod versions {
    pub mod v1;
}

ce_wire::api! {
    api CrShell = "cr.shell", version 2;
    types {
        /// Health contract: these three fields, in this order, in every cell.
        pub struct Health { pub ok: bool, pub version: String, pub uptime_s: u64 }
        pub struct Tab { pub slot: String, pub label: String }
        /// person, role and until_ms come from the verified leaf of the grant, never from the page.
        pub struct LayoutView { pub title: String, pub person: String, pub role: String, pub until_ms: u64, pub tabs: Vec<Tab> }
        pub struct LayoutReq { pub route: Option<String> }
    }
    ops {
        1 => HealthOp(()) -> Health, needs "cr.shell:health";
        2 => Layout(LayoutReq) -> LayoutView, needs "cr.shell:open";
    }
}

ui_part::views! {
    api CrShell;
    view ControlRoom {
        props { route: Option<String> }
        needs "cr.shell:open";
        events {}
        data { layout: Layout }
        streams {}
        slots { keys: "cr.keys@2", devices: "cr.devices@2", audit: "cr.audit@2" }
        speaks [2];
    }
    parts {}
}
