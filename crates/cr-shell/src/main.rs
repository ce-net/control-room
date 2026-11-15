//! `cr-shell serve`: the cr.shell cell behind this machine's door (ce-cap, default deny) and its loopback HTTP face for
//! the sign-in gate (`$CR_SHELL_HTTP`, default 127.0.0.1:8834; `$CR_SHELL_WEB`, the built shell, default
//! ~/dev/control-room/ui/dist; `$CR_SHELL_WIRE`, wire.ws-door, default 127.0.0.1:7792; `$CR_SHELL_ROLES`, the role
//! files, default ~/dev/control-room/roles; `$CR_SHELL_PEOPLE`, default ~/.config/control-room/people.json, else the
//! repo's roles/people.json).
//! `cr-shell mint <person> [<page>]`: print a link for that person's role grant, minted by this node (operators only:
//! it needs this node's own key, which holds wire.ws-door:mint).
use ce_wire::Server;
use cr_shell::{Config, Mint};
use cr_shell_api::*;
use cr_shell::door_api::wire_ws_door::{self, MintReq, WireWsDoor};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Mint through wire.ws-door's typed op with this node's own proof (the node holds wire.ws-door:mint).
struct Door;
impl Mint for Door {
    fn mint(&self, page: &str, abilities: &[String], ttl_s: u32) -> Result<(String, u64), String> {
        let token = cr_view::source_token::<WireWsDoor>(&["wire.ws-door:mint"], 120, None)?;
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

fn config() -> Result<Config, String> {
    let roles = PathBuf::from(env_or("CR_SHELL_ROLES", format!("{}/dev/control-room/roles", home())));
    let own = PathBuf::from(format!("{}/.config/control-room/people.json", home()));
    let people = std::env::var("CR_SHELL_PEOPLE").ok().map(PathBuf::from).unwrap_or(if own.exists() { own } else { roles.join("people.json") });
    Config::load(&roles, &people)
}

async fn serve() -> Result<(), String> {
    let me = ce_gate::node_identity().map_err(|e| e.to_string())?;
    let started = Instant::now();
    config()?;
    let web = cr_shell::web::Web {
        dir: PathBuf::from(env_or("CR_SHELL_WEB", format!("{}/dev/control-room/ui/dist", home()))),
        wire: env_or("CR_SHELL_WIRE", "127.0.0.1:7792".into()),
        mint: Arc::new(Door),
        person_key: env_or("CE_GATE_PERSON_KEY", format!("{}/.config/work-show/person-key", home())).into(),
        config: Arc::new(config),
        page: "https://apps.ce-net.com/control-room/".into(),
        door: "wss://apps.ce-net.com/control-room/wire".into(),
    };
    let http = env_or("CR_SHELL_HTTP", "127.0.0.1:8834".into());
    tokio::spawn(async move {
        if let Err(e) = cr_shell::web::serve(&http, web).await {
            eprintln!("cr-shell http: {e}");
            std::process::exit(1)
        }
    });
    let gate = ce_gate::Gate::<CrShell>::new(me.node_id(), ce_gate::mesh_roots()).into_fn();
    Server::<CrShell>::new(gate)
        .on::<HealthOp, _, _>(move |_: (), _| async move { Ok(Health { ok: true, version: env!("CARGO_PKG_VERSION").into(), uptime_s: started.elapsed().as_secs() }) })
        .on::<Session, _, _>(|_: (), who| async move { Ok(cr_shell::session(&who)) })
        .audit(ce_gate::audit::file())
        .serve_door()
        .await
        .map_err(|e| e.to_string())
}

fn mint(person: &str, page: &str) -> Result<(), String> {
    let c = config()?;
    let (role, a) = c.abilities(person, cr_view::now_ms()).map_err(|e| format!("{e:?}"))?;
    let door = format!("{}wire", page.replace("https://", "wss://").replace("http://", "ws://"));
    let (link, not_after) = Door.mint(page, &a, 900)?;
    let f = cr_shell::fragment(&link, &door).map_err(|e| format!("{e:?}"))?;
    eprintln!("cr-shell: {person} as {role}, 15 min, until {not_after}");
    println!("{page}#{f}");
    Ok(())
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = match a.get(1).map(String::as_str) {
        Some("serve") => tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string()).and_then(|rt| rt.block_on(serve())),
        Some("mint") if a.len() >= 3 => mint(&a[2], a.get(3).map(String::as_str).unwrap_or("http://127.0.0.1:8834/")),
        _ => Err("usage: cr-shell serve | mint <person> [<page>]".into()),
    };
    if let Err(e) = r {
        eprintln!("cr-shell: {e}");
        std::process::exit(1);
    }
}
