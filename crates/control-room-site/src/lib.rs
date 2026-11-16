//! The control room's site, declared once (what `site!` will generate): route `/control-room/*` on apps.ce-net.com,
//! `cr.shell@2`'s `ControlRoom` with the slice 1 slots. The cells are pinned by commit in the workspace Cargo.toml.
#![forbid(unsafe_code)]

use ui_manifest::{Place, Site};

pub const ROUTE: &str = "/control-room/*";

/// The slots in tab order and their labels.
pub const TABS: &[(&str, &str)] = &[("keys", "Keys"), ("devices", "Devices"), ("audit", "Audit"), ("quota", "Quota")];

pub fn site() -> Site {
    let leaf = |v: ui_part::ViewShape| Place { view: v, slots: vec![] };
    Site {
        name: "control-room",
        route: ROUTE,
        root: Place {
            view: cr_shell_api::VIEWS[0],
            slots: vec![
                ("keys", leaf(cr_keys_api::VIEWS[0])),
                ("devices", leaf(cr_devices_api::VIEWS[0])),
                ("audit", leaf(cr_audit_api::VIEWS[0])),
                ("quota", leaf(sub_gateway_api::VIEWS[0])),
            ],
        },
    }
}

/// Every view of the site, root first.
pub fn views() -> Vec<ui_part::ViewShape> {
    let s = site();
    let mut v = vec![s.root.view];
    v.extend(s.root.slots.iter().map(|(_, p)| p.view));
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use control_room_policy::{covers, ROOT};

    #[test]
    fn the_tree_matches_every_slot_type() {
        ui_manifest::check(&site()).unwrap();
        for (slot, _) in TABS {
            assert!(site().root.slots.iter().any(|(s, _)| s == slot));
        }
    }

    #[test]
    fn root_holds_every_right_a_view_or_part_needs() {
        let a: Vec<String> = ROOT.rights.iter().map(|r| r.to_string()).collect();
        for v in views() {
            assert!(covers(&a, v.needs), "no role holds {}", v.needs);
        }
        for p in cr_keys_api::PARTS.iter().chain(cr_devices_api::PARTS).chain(cr_audit_api::PARTS).chain(sub_gateway_api::PARTS) {
            assert!(covers(&a, p.right), "no role holds {}", p.right);
        }
    }
}
