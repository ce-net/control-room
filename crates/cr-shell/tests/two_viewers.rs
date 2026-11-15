//! Two viewers, one URL, as `#[cell_test(sim, as = [operator, root])]` would run it: the same policy, the same site,
//! the same gateway code; root gets the shell and three slot cells, the operator the shell, devices and audit. What a
//! viewer may not have never reaches their browser: not in the manifest, its bundle answers 403 Refused, its tab is
//! not named in Layout.
use ce_wire::Caller;
use std::collections::BTreeMap;
use std::io::{Read, Write};

const PEOPLE: &str = r#"{"leif": {"role": "root"}, "ops": {"role": "operator", "until": "2099-12-31", "nodes": ["ops-1"], "apps": ["orch"]}}"#;

fn abilities(person: &str) -> Vec<String> {
    control_room_policy::abilities(person, &control_room_policy::people(PEOPLE).unwrap(), 1_794_700_000_000).unwrap().1
}

/// What ce-gate hands a cell for a grant with these abilities: the op rights of `A` it covers, then every ability.
fn caller<A: ce_wire::Api>(a: &[String]) -> Caller {
    let mut rights: Vec<String> = A::RIGHTS.iter().filter(|r| control_room_policy::covers(a, r)).map(|r| r.to_string()).collect();
    rights.extend(a.iter().map(|x| format!("ability:{x}")));
    rights.push("grant:00112233445566ff".into());
    Caller { name: "cd".repeat(32), rights }
}

#[test]
fn layout_names_only_the_tabs_a_viewer_may_open() {
    let l = cr_shell::layout(&caller::<cr_shell_api::CrShell>(&abilities("leif")));
    assert_eq!((l.person.as_str(), l.role.as_str()), ("leif", "root"));
    assert_eq!(l.tabs.iter().map(|t| t.slot.as_str()).collect::<Vec<_>>(), ["keys", "devices", "audit"]);
    let l = cr_shell::layout(&caller::<cr_shell_api::CrShell>(&abilities("ops")));
    assert_eq!(l.tabs.iter().map(|t| t.slot.as_str()).collect::<Vec<_>>(), ["devices", "audit"]);
    assert!(!format!("{l:?}").to_lowercase().contains("key"));
}

fn bundles() -> ui_manifest::Bundles {
    let mut m = BTreeMap::new();
    for (k, h) in [("cr.shell@2/ControlRoom", "aa"), ("cr.keys@2/KeyList", "bb"), ("cr.devices@2/DeviceList", "cc"), ("cr.audit@2/AuditTrail", "dd")] {
        m.insert(k.to_string(), ui_manifest::Bundle { file: format!("_c/{}.js", h.repeat(32)), sha256: h.repeat(32) });
    }
    ui_manifest::Bundles(m)
}

const KEY: [u8; 32] = [7u8; 32];

fn get(port: u16, path: &str, person: &str) -> (u16, Vec<u8>) {
    use hmac::{Hmac, Mac};
    let at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let mut m = Hmac::<sha2::Sha256>::new_from_slice(&KEY).unwrap();
    m.update(format!("ce-person\n{person}\n{at}").as_bytes());
    let sig: String = m.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect();
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "GET {path} HTTP/1.1\r\nHost: apps.ce-net.com\r\nX-Ce-Person: {person}\r\nX-Ce-Person-At: {at}\r\nX-Ce-Person-Sig: {sig}\r\n\r\n").unwrap();
    let mut b = Vec::new();
    s.read_to_end(&mut b).unwrap();
    let code = String::from_utf8_lossy(&b[9..12]).parse().unwrap();
    let body = b.windows(4).position(|w| w == b"\r\n\r\n").map(|i| b[i + 4..].to_vec()).unwrap_or_default();
    (code, body)
}

struct NoMint;
impl site_gateway::Mint for NoMint {
    fn mint(&self, _: &str, _: &[String], _: u32) -> Result<(String, u64), String> {
        Err("no door in this test".into())
    }
}

#[test]
fn the_operator_receives_no_byte_of_the_keys_cell() {
    let dir = std::env::temp_dir().join(format!("cr-shell-test-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("_c")).unwrap();
    let b = bundles();
    std::fs::write(dir.join("bundles.json"), serde_json::to_vec(&b).unwrap()).unwrap();
    for x in b.0.values() {
        std::fs::write(dir.join(&x.file), format!("export default {{}} // {}", x.file)).unwrap();
    }
    std::fs::write(dir.join("index.html"), "<!doctype html><title>Control room</title>").unwrap();
    std::fs::write(dir.join("person-key"), KEY.iter().map(|b| format!("{b:02x}")).collect::<String>()).unwrap();
    let port = 18000 + (std::process::id() % 2000) as u16;
    let people = control_room_policy::people(PEOPLE).unwrap();
    let web = site_gateway::Web {
        dir: dir.clone(),
        wire: "127.0.0.1:1".into(),
        mint: std::sync::Arc::new(NoMint),
        person_key: dir.join("person-key"),
        viewer: std::sync::Arc::new(move |p: &str| {
            let (role, abilities) = control_room_policy::abilities(p, &people, 1_794_700_000_000)?;
            Ok(site_gateway::Viewer { role, abilities })
        }),
        site: std::sync::Arc::new(control_room_site::site()),
        ttl_s: 60,
        page: "https://apps.ce-net.com/control-room/".into(),
        door: "wss://apps.ce-net.com/control-room/wire".into(),
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.spawn(async move { site_gateway::serve(&format!("127.0.0.1:{port}"), web).await });
    std::thread::sleep(std::time::Duration::from_millis(300));
    let keys_file = format!("/_c/{}.js", "bb".repeat(32));

    let (code, root) = get(port, "/manifest.json", "leif");
    assert_eq!(code, 200);
    let root = String::from_utf8(root).unwrap();
    assert!(root.contains("cr.keys") && root.contains("KeyList"), "{root}");

    let mut received = Vec::new();
    for path in ["/", "/manifest.json", "/bundles.json", &keys_file] {
        let (code, body) = get(port, path, "ops");
        if path == keys_file.as_str() {
            assert_eq!((code, body.as_slice()), (403, b"Refused".as_slice()), "ui.serve Fetch of the keys bundle");
        }
        if path == "/bundles.json" {
            assert_eq!(code, 404, "the full index is never served");
        }
        received.extend(body);
    }
    let text = String::from_utf8_lossy(&received);
    for leak in ["cr.keys", "KeyList", &"bb".repeat(32), "keys"] {
        assert!(!text.contains(leak), "the operator received {leak:?}: {text}");
    }
    assert!(text.contains("DeviceList") && text.contains("AuditTrail"));
    assert_eq!(get(port, &format!("/_c/{}.js", "cc".repeat(32)), "ops").0, 200);
    assert_eq!(get(port, "/manifest.json", "stranger").0, 403);
    assert_eq!(get(port, &format!("/_c/{}.js", "ee".repeat(32)), "leif").0, 404);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn major_1_is_frozen_byte_for_byte() {
    use ce_wire::Api;
    assert_eq!(<cr_shell_api::versions::v1::CrShell as Api>::SCHEMA, 0x78b3c4fd73f72fce, "the schema the first edition's page speaks");
    assert_eq!(<cr_shell_api::CrShell as Api>::VERSION, 2);
}
