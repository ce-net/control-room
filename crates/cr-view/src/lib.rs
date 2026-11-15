//! cr-view: what every control-room view cell shares. A view cell is stateless: it reads its sources with its own
//! narrow grants, joins rows for one panel, filters them by the caller's chain and returns. This crate holds the
//! pieces of that path that are the same in every cell: rights and scopes read from the caller's verified chain,
//! the grant a cell presents to a source, and call ids.
//!
//! Scopes. A right such as `cr.devices:read` may come with scope abilities `cr.devices:scope:node=<glob>` in the same
//! chain. When any scope ability for a key is present, only rows whose value matches one of them are returned and the
//! view says `narrowed`. Scope abilities are read literally from the verified leaf, so a wildcard right never invents
//! one, and a page cannot add one: the chain is signed by the node that minted it.
#![forbid(unsafe_code)]

use ce_wire::{Api, Caller};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn now_s() -> i64 {
    (now_ms() / 1000) as i64
}

/// Does the caller's chain hold `ability` (an op right the gate matched, or a verified leaf ability covering it)?
pub fn holds(c: &Caller, ability: &str) -> bool {
    c.has(ability) || c.grants(ability)
}

/// The values of every `<cell>:scope:<key>=<value>` ability in the caller's verified leaf, in order.
/// Empty: the caller is not scoped on `key`.
pub fn scopes(c: &Caller, cell: &str, key: &str) -> Vec<String> {
    let head = format!("{}{cell}:scope:{key}=", ce_wire::ABILITY_RIGHT);
    c.rights.iter().filter_map(|r| r.strip_prefix(head.as_str())).filter(|v| !v.is_empty()).map(String::from).collect()
}

/// `pattern` matches `value`: equal, `*`, or a trailing `*` prefix match.
pub fn glob(pattern: &str, value: &str) -> bool {
    pattern == "*" || pattern == value || pattern.strip_suffix('*').is_some_and(|p| value.starts_with(p))
}

/// A row with `value` for this scope key is visible: unscoped, or one scope matches.
pub fn in_scope(scopes: &[String], value: &str) -> bool {
    scopes.is_empty() || scopes.iter().any(|s| glob(s, value))
}

/// The id of the chain the caller came in with (the gate's `grant:<16 hex>`), "" when the gate gave none.
pub fn chain_id(c: &Caller) -> String {
    c.rights.iter().find_map(|r| r.strip_prefix(ce_wire::GRANT_RIGHT)).unwrap_or("").to_string()
}

/// 16 random bytes as 32 hex: a call id or span id in `ui.trace`'s shape.
pub fn new_id() -> String {
    let mut b = [0u8; 16];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| std::io::Read::read_exact(&mut f, &mut b)).is_err() {
        let t = now_ms().to_le_bytes();
        b[..8].copy_from_slice(&t);
        b[8..].copy_from_slice(&(std::process::id() as u64).to_le_bytes());
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The grant this cell presents to source `A`: a chain this node signs for itself with exactly `abilities` (never
/// `<api>:*`), for `secs`, optionally confined to `path_prefix`. What a view cell may read is this list, in its code.
pub fn source_token<A: Api>(abilities: &[&str], secs: u64, path_prefix: Option<&str>) -> Result<Vec<u8>, String> {
    let me = ce_gate::node_identity().map_err(|e| format!("this machine has no ce identity: {e}"))?;
    let chain = ce_gate::self_chain_in(&me, abilities.iter().map(|a| a.to_string()).collect(), secs, path_prefix.map(String::from));
    Ok(ce_gate::proof::<A>(&me, &chain))
}

/// This machine's short host name (the node name the rows use for the local node).
pub fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()).ok().filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok()).unwrap_or_else(|| "localhost".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caller(rights: &[&str]) -> Caller {
        Caller { name: "ab".repeat(32), rights: rights.iter().map(|r| r.to_string()).collect() }
    }

    #[test]
    fn scopes_are_read_from_leaf_abilities_only() {
        let c = caller(&["cr.devices:read", "ability:cr.devices:read", "ability:cr.devices:scope:node=node-1", "cr.devices:scope:node=x"]);
        assert_eq!(scopes(&c, "cr.devices", "node"), vec!["node-1".to_string()]);
        assert!(scopes(&c, "cr.devices", "app").is_empty());
    }

    #[test]
    fn a_wildcard_right_is_unscoped_but_never_a_scope() {
        let c = caller(&["ability:*"]);
        assert!(holds(&c, "cr.keys:holders"));
        assert!(scopes(&c, "cr.keys", "service").is_empty());
    }

    #[test]
    fn glob_and_scope() {
        assert!(glob("stripe*", "stripe-live"));
        assert!(!glob("stripe*", "github"));
        assert!(in_scope(&[], "anything"));
        assert!(!in_scope(&["a".into()], "b"));
    }

    #[test]
    fn chain_id_from_gate_right() {
        assert_eq!(chain_id(&caller(&["grant:0123456789abcdef"])), "0123456789abcdef");
        assert_eq!(chain_id(&caller(&[])), "");
    }

    #[test]
    fn ids_are_32_hex() {
        let a = new_id();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, new_id());
    }
}
