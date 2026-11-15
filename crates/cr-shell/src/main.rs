//! `cr-shell serve`: the cr.shell cell behind this machine's door (ce-cap, default deny), majors 2 and 1, and the
//! site's gateway face on loopback for the sign-in gate (repo:ui-cells crates/site-gateway): `$CR_SHELL_HTTP`, default
//! 127.0.0.1:8834; `$CR_SHELL_WEB`, the built site, default the one this binary carries; `$CR_SHELL_WIRE`, wire.ws-door,
//! default 127.0.0.1:7792; `$CR_SHELL_PEOPLE`, the people file (control-room-policy).
//! `cr-shell mint <person> [<page>]`: print a link for that person's grant, minted by wire.ws-door (operators only: it
//! needs the cell's own key, whose grant holds wire.ws-door:mint).
//! The cell's own key and grant ([`cr_shell::GRANT`], `<ce data dir>/cell-keys/cr.shell/`): never the node's key.
use ce_wire::Server;
use cr_shell_api::*;
use site_gateway::Mint;
use cr_shell::door_api::wire_ws_door::{self, MintReq, WireWsDoor};
use std::sync::Arc;
use std::time::Instant;

/// Mint through wire.ws-door's typed op with the cell's own proof (its grant holds wire.ws-door:mint).
struct Door;
impl Mint for Door {
    fn mint(&self, page: &str, abilities: &[String], ttl_s: u32) -> Result<(String, u64), String> {
        let token = ui_chain::source_token::<WireWsDoor>(&["wire.ws-door:mint"], 120, None)?;
        let c = ce_wire::blocking::Client::<WireWsDoor>::by_name(token).map_err(|e| format!("wire.ws-door: {e:?}"))?;
        let m = c
            .call::<wire_ws_door::Mint>(&MintReq { page: page.into(), abilities: abilities.to_vec(), ttl_s })
            .map_err(|e| format!("wire.ws-door: {e:?}"))?
            .map_err(|e| format!("wire.ws-door: {e:?}"))?;
        Ok((m.link, m.not_after))
    }
}

fn env_or(k: &str, d: String) -> String {
    std::env::var(k).ok().filter(|v| !v.is_empty()).unwrap_or(d)
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

async fn serve() -> Result<(), String> {
    // this cell's own key and narrow grant (never the node's key); the gate roots callers' chains at the grant's root
    let me = ui_chain::init(cr_shell::CELL)?.root();
    let started = Instant::now();
    control_room_policy::people(&cr_shell::people_text())?;
    let web = site_gateway::Web {
        dir: cr_shell::site_dir()?,
        wire: env_or("CR_SHELL_WIRE", "127.0.0.1:7792".into()),
        mint: Arc::new(Door),
        person_key: env_or("CE_GATE_PERSON_KEY", format!("{}/.config/work-show/person-key", home())).into(),
        viewer: Arc::new(cr_shell::viewer),
        site: Arc::new(control_room_site::site()),
        ttl_s: control_room_policy::TTL_S,
        page: "https://apps.ce-net.com/control-room/".into(),
        door: "wss://apps.ce-net.com/control-room/wire".into(),
    };
    let http = env_or("CR_SHELL_HTTP", "127.0.0.1:8834".into());
    tokio::spawn(async move {
        if let Err(e) = site_gateway::serve(&http, web).await {
            eprintln!("cr-shell http: {e}");
            std::process::exit(1)
        }
    });
    let gate = ce_gate::Gate::<CrShell>::new(me, ce_gate::mesh_roots()).into_fn();
    let v2 = Server::<CrShell>::new(gate)
        .on::<HealthOp, _, _>(move |_: (), _| async move { Ok(Health { ok: true, version: env!("CARGO_PKG_VERSION").into(), uptime_s: started.elapsed().as_secs() }) })
        .on::<Layout, _, _>(|_: LayoutReq, who| async move { Ok(cr_shell::layout(&who)) })
        .audit(ce_gate::audit::file())
        .serve_door();
    use cr_shell_api::versions::v1 as old;
    let gate1 = ce_gate::Gate::<old::CrShell>::new(me, ce_gate::mesh_roots()).into_fn();
    let v1 = Server::<old::CrShell>::new(gate1)
        .on::<old::HealthOp, _, _>(move |_: (), _| async move { Ok(old::Health { ok: true, version: env!("CARGO_PKG_VERSION").into(), uptime_s: started.elapsed().as_secs() }) })
        .on::<old::Session, _, _>(|_: (), who| async move { Ok(cr_shell::v1::session(&who)) })
        .audit(ce_gate::audit::file())
        .serve_door();
    let (a, b) = tokio::join!(v2, v1);
    a.map_err(|e| e.to_string())?;
    b.map_err(|e| e.to_string())
}

fn mint(person: &str, page: &str) -> Result<(), String> {
    ui_chain::init(cr_shell::CELL)?;
    let v = cr_shell::viewer(person)?;
    let door = format!("{}wire", page.replace("https://", "wss://").replace("http://", "ws://"));
    let (link, not_after) = Door.mint(page, &v.abilities, 900)?;
    let f = site_gateway::fragment(&link, &door)?;
    eprintln!("cr-shell: {person} as {}, 15 min, until {not_after}", v.role);
    println!("{page}#{f}");
    Ok(())
}

/// The nonce of the grant in a minted link (the leaf of its chain), for `ce revoke <nonce>` on the node that minted it.
fn nonce(link: &str) -> Result<(), String> {
    let g = link.split_once("#g=").map(|(_, f)| f).ok_or("no #g= in the link")?;
    let chain = g.split('&').next().and_then(|g| g.split_once('.')).map(|(_, c)| c).ok_or("no chain in the grant")?;
    let bytes = b64u(chain).ok_or("the chain is not base64url")?;
    let caps = ce_gate::ce_cap::decode_chain_bytes(&bytes).map_err(|e| format!("{e:?}"))?;
    let leaf = caps.last().ok_or("an empty chain")?;
    println!("{}", leaf.cap.nonce);
    Ok(())
}

fn b64u(s: &str) -> Option<Vec<u8>> {
    let v = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            _ => return None,
        } as u32)
    };
    let s = s.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for ch in s.chunks(4) {
        let mut n = 0u32;
        for (i, c) in ch.iter().enumerate() {
            n |= v(*c)? << (18 - 6 * i);
        }
        let bytes = n.to_be_bytes();
        out.extend_from_slice(&bytes[1..ch.len()]);
    }
    Some(out)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = match a.get(1).map(String::as_str) {
        Some("serve") => tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string()).and_then(|rt| rt.block_on(serve())),
        Some("mint") if a.len() >= 3 => mint(&a[2], a.get(3).map(String::as_str).unwrap_or("http://127.0.0.1:8834/")),
        Some("nonce") if a.len() >= 3 => nonce(&a[2]),
        _ => Err("usage: cr-shell serve | mint <person> [<page>] | nonce <link>".into()),
    };
    if let Err(e) = r {
        eprintln!("cr-shell: {e}");
        std::process::exit(1);
    }
}
