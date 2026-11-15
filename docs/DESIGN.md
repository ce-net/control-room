# Control room: design (2026-11-15)

Leif 2026-11-15: "deploy a multi cellular system using our new frontend multi cellular frontend + baceknd and auth
paradigm system to manage my keys and cells and allow me to renew the keys and monitor all audit trails and traces
and everything and all of my devices in one place. like the previous app but more powerful, 100% type sfe and
writtine with the new design."

Control room is a live system at `apps.ce-net.com/control-room`: seven panels, each a UI cell with its own backend
cell, composed by one shell. It owns no secrets and almost no state. It reads the cells that already hold the truth
and asks them, through passkey-approved typed ops, to change things.

Status of the paradigm reference: `repo:ce/docs/FULLSTACK-CELLS.md` was not on disk when this was written
(checked at `repo:ce@1d770e9`). This design is built on what that paradigm is made of and which does exist:
`repo:multicell-native/docs/CELL-CONTRACT.md` (a face talks only to typed ops), `repo:wire-ws-door` (browser speaks
ce-wire with a short grant), `ce-wire-gen --lang ts-web`, `repo:rust-stack/docs/CAPABILITY-CHAIN.md` and
`repo:ce/docs/LIVE-SYSTEMS-2026-11-15.md`. Section 12 lists what to re-check against FULLSTACK-CELLS.md when it lands.

## 1. Rules this design holds

1. One function per cell, typed contract in `<cell>-api` (`ce_wire::api!`), Rust backend, generated TypeScript.
2. A UI cell never uses fetch, raw HTTP or a hand-written type for a cell answer. Its only way out is its
   generated client.
3. Secrets never leave `cell:keys`. No control-room cell holds `keys:read`. The UI shows metadata only: name,
   holders, expiry, rotation state, version, short hash, last check.
4. Every change (renew, rotate, stop, move, grant) is a typed op the owning cell performs after it has verified
   Leif's P-256 passkey signature over exactly that change. Control room proposes; it cannot perform.
5. What a viewer sees is decided by the backend cell from the caller's capability chain. The UI hides nothing on its
   own; a UI that is tampered with gets the same rows.
6. Other machines are reached only through typed cell ops. No host shell anywhere in the path.
7. Reuse first. New code is the seven thin view cells, the shell, the UI cells and three additive ops (section 5).

## 2. What exists and is reused as is

| Need | Cell / crate | Ops used (right) |
|---|---|---|
| Key metadata | `keys@1` `repo:keys-cell` | `List` (`keys:list`), `Meta` (`keys:meta`), `Audit` (`keys:audit`), `Boxes` (`keys:list`) |
| What renews itself | `keys autopilot` and its one "needs you" card, `repo:keys-cell/docs/keys-autopilot.service` | read through the new `keys` op in section 5 |
| Store grant renewal | `state.grant-renew@1` | `Renew{dry_run}` (`state.grant-renew:renew`) gives `RenewReport{due, renewed, skipped, failed}` |
| Grants held, revoked | `access.grant@1` | `Grants`, `Revoked` (`access.grant:read`), `Check` (`:check`) |
| Policy, who may hold what | `repo:ce-iam`, cap v3 + CAL, `cap.mint/admit/revoke/ledger` | chain verify, attenuation, `Via` conditions |
| Passkey approval | `root.approve@1` | `Propose`, `Pending`, `Get`, `Sign`, `Cancel`, `Devices` |
| Message approvals | `feed.approve@1` | `List`, `Count` (`feed.approve:read`) |
| Needs-you items | `needs.list@1` | `List` (`needs.list:read`) |
| Devices | `fleet.inventory@1` | `State`, `Node{node, section}`, `Items` (`fleet.inventory:read`) |
| Device health | `node.doctor` | its report, read through `fleet.inventory` `Node{section:"doctor"}` |
| Phones, passkeys | `root.approve` `Devices`, `account.passkey` `List` | enrolled approval devices and sign-in passkeys |
| Running cells | `orch@1` | `Ps` (`orch:read`), `Up`, `Down`, `Restart` (`orch:manage`), `Logs` |
| Replicas, failover | `repo:ce/crates/ce-replica`, per-cell `CellInstances`/`CellDeploy` (ops 901, 903) | placement and instance list |
| Versions, pinned builds | `artifact.index`, `cell.build` | `List`, `Resolve` |
| Audit | `audit.log@1` | `Query`, `Trail`, `Verify` (`audit:read`, `audit:trail`) hash-chained rows, signed head |
| Receipts offline | `repo:ce-net-capability-verify` | verify a chain or a receipt file without the mesh |
| Traces | `ui.trace@1` | `Query(Ask)`, `Summary` (`ui.trace:read`); `Span{call_id, parent, api, op, node, ms, outcome}` |
| Sign-in, session | `account.signin`, `account.passkey` on `apps.ce-net.com` | passkey sign-in, gate-signed person |
| Browser to cell | `wire.ws-door` | `Mint` gives the page a 60 s to 24 h grant; the page speaks ce-wire on `/wire` |
| Typed rows | `ce-db-rs` `#[derive(Record)]`, `ce-db-ts` | control room's own small state |
| Typed refs, macros | `repo:ce-ref` (`ce-ref-id`), `cell-macros` (`ce_ref!`, `use_cell!`) | every link in a row is a typed `Ref<Kind>` |
| Codegen | `ce-wire-gen` (`ts-web`, `json`, `check`, `classify`) | clients, drift gate |
| Look | `repo:celldocs-ui/src/components.ts`, `tokens.css`, the ce design standard | components and tokens, light theme default |
| Previous app | `repo:keys-cell/crates/keys-ui`, `repo:needs-list` | its key list and needs card are the starting point of the keys and renewals panels |

## 3. Shape

```
browser (apps.ce-net.com/control-room, signed in with a passkey)
  shell UI ── cr.shell ───────────── session grant (wire.ws-door Mint), panel list for this viewer
   ├ Keys panel ───── cr.keys ─────── keys, access.grant
   ├ Cells panel ──── cr.cells ────── orch, ce-replica instance ops, artifact.index
   ├ Devices panel ── cr.devices ──── fleet.inventory, node.doctor, root.approve Devices, account.passkey
   ├ Audit panel ──── cr.audit ────── audit.log (every node), keys Audit, cap.ledger
   ├ Traces panel ─── cr.traces ───── ui.trace, audit.log (join on call id and chain id)
   ├ Renewals panel ─ cr.renewals ─── keys autopilot, state.grant-renew, access.grant, needs.list
   └ Approvals panel  cr.approvals ── root.approve, feed.approve
```

A panel is one repo and one unit of reuse: `crates/cr-<p>-api` (contract), `crates/cr-<p>` (backend cell),
`ui/` (the UI cell), `sdk/ts-web/` (generated client), `cell.toml`. Each is made with `cell-forge new-cell`.
`repo:control-room` holds only the shell (`cr.shell` + shell UI), `ceapp.toml`, the face entry and this document.
Another app can mount the Keys panel without taking anything else.

### 3.1 Backend cells are views, not stores

Each `cr.<p>` cell is stateless: it holds read grants on its sources, joins their answers into rows shaped for one
panel, filters by the caller's chain, and returns. It caches for a few seconds at most. Stop one and nothing is lost;
that is why they can run on every node (section 9). The exceptions are `cr.approvals` (bundles, section 6) and
`cr.shell` (viewer preferences), which keep typed records in `state.store`.

### 3.2 The envelope every view op returns

```rust
pub struct View<T> {
    pub rows: Vec<T>,
    pub as_of_ms: u64,
    pub sources: Vec<SourceState>,   // per source cell: Live | Stale{age_s} | Down{why} | Refused
    pub narrowed: bool,              // true when this viewer's chain removed rows or fields
    pub can_ask: Vec<String>,        // rights this viewer may request to see more
    pub next: Option<String>,        // cursor
}
```

`narrowed` is a flag, never a count: a narrow viewer does not learn how much is hidden. `sources` makes a dead node
visible as "debian: no answer for 3 d" instead of an empty table.

## 4. Contracts, rights, records

All ops are read-only unless marked. Every row carries `refs: Vec<Ref>` (typed ce-refs) so any row links to its
cell, node, key, receipt or trace.

### cr.keys@1

```rust
ce_wire::api! {
    api CrKeys = "cr.keys", version 1;
    types {
        pub enum RotationState { Fresh, DueSoon { days: u32 }, Overdue { days: u32 }, Rotating, Failed(String), Manual }
        pub struct Holder { pub lane: String, pub home: String, pub not_after: i64 }
        pub struct KeyRow {
            pub name: String, pub service: String, pub purpose: String, pub owner: String,
            pub version: i64, pub sha256_8: String, pub written_at: i64, pub expires_at: Option<i64>,
            pub rotation: RotationState, pub self_renewing: bool,
            pub holders: Vec<Holder>, pub last_opened: i64, pub last_check: String, pub last_check_at: i64,
            pub refs: Vec<Ref>,
        }
        pub struct ShareRow { pub key: String, pub to: String, pub due_at: i64, pub state: String, pub refs: Vec<Ref> }
        pub struct KeysAsk { pub q: String, pub due_within_days: Option<u32>, pub cursor: Option<String> }
        pub enum CrError { Refused { needs: String }, Source { cell: String, why: String }, Invalid(String) }
    }
    ops {
        1 => HealthOp(()) -> Health, needs "cr.keys:health";
        2 => List(KeysAsk) -> Result<View<KeyRow>, CrError>, needs "cr.keys:read";
        3 => One(String) -> Result<KeyRow, CrError>, needs "cr.keys:read";
        4 => Shares(KeysAsk) -> Result<View<ShareRow>, CrError>, needs "cr.keys:read";
        5 => History(String) -> Result<View<KeyEvent>, CrError>, needs "cr.keys:history";
    }
}
```

`KeyRow` has no field that can carry a value; the type is the guarantee. `cr.keys` holds `keys:list`, `keys:meta`,
`keys:audit` and nothing else, and its test asserts that its grant has no `keys:read`.
Row scope: `cr.keys:read` may be attenuated to `service=<glob>` or `owner=<lane>`; holders are a field that needs
`cr.keys:holders`, else it comes back empty with `narrowed = true`.

### cr.cells@1

`CellRow { api, version, build: Ref<Pinned>, placement: Vec<String>, instances: Vec<Instance{ node, id, state, since_ms, health, leader }>, wanted_replicas, live_replicas, drift: Option<String>, protected: bool, refs }`

| Op | Right |
|---|---|
| `List(CellsAsk{q, node, unhealthy_only})`, `One(api)` | `cr.cells:read` (scope `node=`, `api=`) |
| `Logs(LogsAsk)` stream | `cr.cells:logs` |
| `Plan(Change) -> Planned{steps, refuses}` | `cr.cells:plan` |

`Change` is `Start{api,node} | Stop{api,node} | Restart{api,node} | Move{api,from,to} | Deploy{api,node,version}`.
`Plan` is a dry run that returns the exact `orch` and instance ops, in order, and anything that refuses. Performing
it goes through an approval bundle (section 6); `orch` runs it. `Move` is always `Up` on the target, health and
replica caught up, then `Down` on the source. `protected` cells (`state.store`, `keys`, the door, `root.approve`)
refuse `Stop` and `Move` in `Plan` with the reason; a node's `state.store` is never stopped because its gate reads it.

### cr.devices@1

`DeviceRow { id, name, kind: Server | Desktop | Laptop | Phone | Tablet, online, last_seen_s, tailscale: Option<Tailnet{ ip, online, exit_node, key_expires_at }>, disk: Vec<Disk{ mount, used_pct, free_gb }>, load: Option<f32>, doctor: Doctor{ ok, failing: Vec<String>, at }, cells: u32, passkeys: Vec<PasskeyRef{ label, enrolled_at, last_used_at }>, refs }`

| Op | Right |
|---|---|
| `List(DevicesAsk)`, `One(id)` | `cr.devices:read` (scope `node=`) |
| `Doctor(id) -> DoctorReport` (asks the node's `node.doctor` now) | `cr.devices:doctor` |

Servers and desktops (ops-1, leif-desk, the Mac, debian) come from `fleet.inventory` `State` and `Node`. The Mac
answers only through its own typed cells. Phones come from `root.approve` `Devices` and `account.passkey` `List`: a
phone is a device because it holds an approval key, and the row shows when it last signed. A node that does not
answer stays in the list as offline with its last row and age.

### cr.audit@1

`AuditRow { seq, at_ms, node, writer, actor, app, op, target, ok, detail, chain_id: Option<String>, call_id: Option<String>, prev_hash, hash, kind: Call | Grant | Revoke | Approval | KeyEvent, refs }`

| Op | Right |
|---|---|
| `Search(AuditAsk{q, actor, app, op, target_prefix, kind, since_ms, until_ms, only_refused, cursor})` | `cr.audit:read` (scope `app=`, `actor=`) |
| `Chain(VerifyAsk{node}) -> ChainState{rows, head_seq, anchored_seq, first_bad, why}` | `cr.audit:verify` |
| `Receipt(RowId) -> Receipt{row, proof: Vec<Hash>, head_sig}` | `cr.audit:read` |
| `Export(AuditAsk) -> stream AuditRow` | `cr.audit:export` |

It fans `Query`/`Trail` out to `audit.log` on every node, merges by time, and adds `keys` `Audit` rows and
`cap.ledger` rows as the same type. `prev_hash`/`hash` are shown, and the UI cell re-computes the link for the rows
on screen so a broken chain is visible in the browser, not only reported by the server. `Receipt` can be checked
offline with `repo:ce-net-capability-verify`.

### cr.traces@1

`TraceRow { call_id, root_api, root_op, started_ms, ms, outcome, nodes: Vec<String>, cells: u32, chain_id: Option<String> }`
`SpanRow { call_id, parent, api, op, node, start_ms, ms, outcome, chain_id, chain_depth, right_used, audit_seq: Option<u64> }`

| Op | Right |
|---|---|
| `Recent(TraceAsk{since_ms, api, op, min_ms, only_failed, chain_id})`, `Summary(TraceAsk)` | `cr.traces:read` (scope `api=`) |
| `Tree(call_id) -> Vec<SpanRow>` (the whole call, across cells and nodes) | `cr.traces:read` |
| `ByChain(chain_id) -> View<TraceRow>` (everything done under one capability chain) | `cr.traces:read` |

Source is `ui.trace` `Query`/`Summary`. The capability chain id (cap hash of the leaf, `repo:rust-stack/docs/CAP-HASH.md`)
is joined from the audit row written by the ledger stage for the same call. A span, its audit row and its chain
therefore open from each other in one click.

### cr.renewals@1

`RenewalRow { what: Key | Share | StoreGrant | AccessGrant | Cert | TailscaleKey | Passkey, name, holder, expires_at, mode: SelfRenewing { next_run_at, last: Outcome } | NeedsLeif { why, propose: Change }, refs }`

| Op | Right |
|---|---|
| `List(RenewAsk{within_days, mode})` | `cr.renewals:read` |
| `Runs(RenewAsk) -> View<AutopilotRun>` | `cr.renewals:read` |

Two lists by construction: "renews itself" (with the next run and the last result) and "needs you" (with the
reason and a ready `Change` to approve). Sources: the keys autopilot state and its needs-you card, `state.grant-renew`
`Renew{dry_run: true}` per node, `access.grant` `Grants`, tailscale key expiry from `cr.devices`.

### cr.approvals@1 (the only cell that writes)

```rust
pub enum Change {
    KeyRenew { name: String }, KeyRotate { name: String }, ShareSend { key: String, to: String },
    StoreGrantRenew { node: String, label: String }, Grant { to: String, abilities: Vec<String>, not_after: i64 },
    Revoke { chain_id: String }, Cell(cr_cells_api::Change),
}
pub struct Item { pub id: String, pub change: Change, pub performer: String, pub effect: String, pub state: ItemState }
pub struct Bundle { pub id: String, pub items: Vec<Item>, pub by: String, pub until: u64, pub to_sign: String, pub state: BundleState, pub refs: Vec<Ref> }
```

| Op | Right |
|---|---|
| `Inbox(()) -> View<Bundle>` (bundles, `root.approve` `Pending`, `feed.approve` `List` as one list) | `cr.approvals:read` |
| `Propose(Vec<Change>) -> Bundle` | `cr.approvals:propose` |
| `SignStart(bundle_id) -> GetOptions`, `SignFinish(Assertion) -> Bundle` | `cr.approvals:sign` (only Leif's chain) |
| `Cancel(bundle_id)` | `cr.approvals:cancel` |
| `Watch(bundle_id) -> stream ItemState` | `cr.approvals:read` |

### cr.shell@1

`Session(()) -> SessionInfo{person, chain_id, expires_at, panels: Vec<PanelGrant{panel, rights}>}` (`cr.shell:session`),
`Prefs`/`SetPrefs` (`cr.shell:prefs`). `panels` is computed from the chain, so the shell mounts only panels the
viewer can use.

### Records (ce-db, `state.store`)

| Record | Cell | Fields |
|---|---|---|
| `cr.bundle` | `cr.approvals` | id, items, by, until, to_sign, sig, device, state, created_ms |
| `cr.bundle_item` | `cr.approvals` | bundle, index, change, performer, state, receipt_seq |
| `cr.spent` | each performing cell | bundle, index, at_ms (an approved item runs once) |
| `cr.prefs` | `cr.shell` | person, layout, theme (`light` default), saved filters |

No record holds a secret, a token or a grant body; a grant is referred to by its chain id.

## 5. Three additive ops in existing cells

Each is an added op, which `ce-wire-gen classify` reports as additive: old callers keep working.

1. `keys`: `Renew(RenewReq{name, mode: Renew | Rotate, proof: ApprovalProof}) -> Result<KeyInfo, KeysError>`, needs
   `keys:renew`. keysd verifies the proof, runs the same code the autopilot runs for that key, writes the new
   version sealed, and answers with `KeyInfo` (metadata). The new value never appears in a request or an answer.
2. `keys`: `Autopilot(()) -> AutopilotState{rules, last_runs, needs_you}`, needs `keys:list`. The autopilot's own
   state as a typed answer, so `cr.renewals` does not read a file.
3. `orch`: `Apply(ApplyReq{change, proof: ApprovalProof}) -> Result<Done, OrchError>`, needs `orch:apply`. The
   approved form of `Up`/`Down`/`Restart`; `orch:manage` stays with operators and is not given to control room.

`ApprovalProof` and `verify_item(proof, enrolled_devices, performer, now)` live in a small crate `approval-proof`
lifted from `root.approve`'s existing P-256 verification, used by `keys`, `orch`, `state.grant-renew` and
`access.grant`.

## 6. Approvals: one popup for a bundle

1. Any panel builds `Change`s (the Renewals panel offers "approve all 6 due"). `cr.approvals` `Propose` fills in for
   each item the performer cell and a plain-words `effect` taken from that cell's dry run, and computes
   `to_sign = "ce.control-room.approve v1\n" || sha256(canonical{items, by, until, nonce})`.
2. The Approvals panel shows every item with its effect. One button, one WebAuthn prompt: `SignStart` returns options
   whose challenge is `to_sign`; Leif's passkey (P-256) signs; `SignFinish` checks the assertion against the devices
   enrolled in `root.approve` and stores it.
3. `cr.approvals` then calls each performer with `ApprovalProof{bundle (canonical), index, assertion, device}`.
4. The performer trusts nothing about `cr.approvals`. It checks: the signature is by an enrolled device of the mesh
   root person; its own item is in the signed bundle at `index`; `until` has not passed; `(bundle, index)` is not in
   `cr.spent`. Then it performs, writes `cr.spent`, and its ledger stage writes the audit row with the bundle id.
5. `Watch` streams item states to the panel: `Approved`, `Running`, `Done{receipt}`, `Failed{why}`. A failed item
   does not undo the others; it returns to the inbox as a new proposal.

So a stolen session, a compromised `cr.approvals` or a changed UI cannot renew, stop or grant anything: the
signature covers the exact changes and the performing cell verifies it itself.

## 7. Auth and what a viewer sees

1. Sign-in is the existing passkey sign-in on `apps.ce-net.com` (`account.signin`, gate-signed person).
2. `cr.shell` asks `ce-iam` for that person's control-room rights and has `wire.ws-door` `Mint` a grant for the
   browser: a fresh ed25519 key and a chain attenuated to exactly those rights with their scopes, 8 h. The grant
   reaches the page in the URL fragment and stays in memory.
3. Every panel client signs its Hello with that key. The call runs the chain stages of
   `repo:rust-stack/docs/CAPABILITY-CHAIN.md`: auth, cap, limit, route, call, ledger. Stage 2 refuses an op the
   chain lacks; inside the op, the view cell filters rows and fields by the chain's scopes (`covers`/`check_scope`,
   the same functions `audit-log-api` uses).
4. A view cell reads its sources with its own narrow read grants, and returns only what the caller's chain covers.
   It holds no write rights, so it cannot be used as a deputy for a change.

Two viewers, same URL, same bundle:

| | Leif (root person) | Operator (narrow) |
|---|---|---|
| Chain | all `cr.*` rights, `cr.approvals:sign` | `cr.devices:read node=ops-1`, `cr.cells:read node=ops-1`, `cr.audit:read app=orch`, `cr.traces:read api=orch` |
| Shell | seven panels | Devices, Cells, Audit, Traces |
| Devices | every node and phone | ops-1 only, `narrowed = true` |
| Keys, Renewals, Approvals | full | not mounted; a direct call answers `Refused{needs}` |
| A refused panel shows | | what is missing and a button that files the request into Leif's approvals inbox |

Acceptance test `tests/two_viewers.rs` signs both in against in-process cells and asserts the row sets, then calls
every op of every cell with the operator chain and asserts each answer is either scoped or refused.

## 8. UI cells, codegen, bundling

**A UI cell** is a TypeScript package in its panel's repo:

```ts
// cr-keys/ui/panel.ts
import { CrKeysClient, type KeyRow } from '../sdk/ts-web/CrKeys';      // generated, never edited
export const KeysPanel = definePanel<{ dueWithinDays?: number; onOpen?: (k: KeyRow) => void }, CrKeysClient>({
  api: CrKeysClient,                       // the shell connects it with the session grant
  rights: ['cr.keys:read'],                // must equal the rights of the ops used; checked at build
  view: (props, cell) => /* components from the shared kit; live(cell.List(...)) re-renders on change */,
});
```

Panels compose like components: typed props in, typed events out, and a panel can mount another panel. The Renewals
panel mounts `<KeysPanel dueWithinDays={30} onOpen={...}/>`; clicking an audit row's `call_id` opens
`<TracesPanel tree={callId}/>`; clicking a span's chain opens `<AuditPanel chainId={...}/>`. Cross-panel links are
typed `Ref<Kind>` values routed by the shell, so a link to a kind no mounted panel handles does not compile.
Every panel renders the five states of the cell contract: loading, empty in one sentence, refused with the ask
button, no answer, signed out.

**Type safety, end to end.** Rust struct in `api!` -> `ce-wire-gen --lang ts-web` -> client and types -> panel props.
`tsc` with `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`; lint forbids `any`, `as` casts and
`fetch`/`WebSocket` outside the generated client. Rust enums arrive as discriminated unions and every `switch`
is exhaustive (`never` check), so a new `RotationState` breaks the build until the panel draws it. `Result<T, E>`
arrives as a typed result, not an exception. Rust: `#![forbid(unsafe_code)]`, clippy deny warnings, no `serde_json::Value`
in a contract.

**Codegen gates** (per panel repo, CI): `ce-wire-gen --lang ts-web` and `--lang json` regenerate `sdk/` and `wire/`
and fail on drift; `ce-wire-gen check --base origin/main` fails a breaking change; the `rights` list of each panel
is compared with the `needs` of the ops it imports; source cells are bound with
`use_cell!("keys" => "<pinned contract ref>")` so a source contract change is a compile error in the view cell.

**Bundling.** Each UI cell builds to one content-hashed ES module (`keys.<hash>.js`) plus the shared kit and tokens
as their own chunks; no panel bundles another panel. The shell is a small module that, from `Session().panels`,
imports only the modules of the panels this viewer has: an operator's browser never downloads the Keys panel code.
Each module is published to `artifact.index` with its build statement and resolved by hash; the page pins hashes
with subresource integrity. One strict CSP: scripts from self by hash, connect only to `/wire`. The same panel
modules load in the desktop window through a `[[face]]` entry in `multicell-native` (`serve = "origin"`).

**Look.** The ce design standard, light theme by default, dark by preference; shared components and tokens, no
per-panel CSS framework. Dense tables, one accent, expiry shown as a date and as days left, state always as text plus
colour. Works at phone width because approvals are signed on the phone.

## 9. Deployment

| Cell | Runs on | Replicas | State | Notes |
|---|---|---|---|---|
| `cr.shell` | ops-1, leif-desk | 2, active-active | `cr.prefs` | next to `account.signin` and `wire.ws-door` |
| `cr.keys` | ops-1, debian when back | 2 | none | on the nodes that run `keys`, so metadata reads stay local |
| `cr.cells` | ops-1, leif-desk | 2 | none | talks to `orch` on every node |
| `cr.devices` | ops-1, leif-desk | 2 | none | |
| `cr.audit` | ops-1, leif-desk | 2 | none | fans out to each node's `audit.log` |
| `cr.traces` | ops-1 | 1, restart on leif-desk | none | next to `ui.trace` |
| `cr.renewals` | ops-1, leif-desk | 2 | none | |
| `cr.approvals` | ops-1, leif-desk | 2, leader by `ce-replica` | `cr.bundle*` | one leader performs a bundle; items are idempotent through `cr.spent` |
| UI modules | `artifact.index`, served on `apps.ce-net.com/control-room` | every door node | static | |

Each cell has a `cell.toml` (`api`, `depends_on`, `[launch] health`, `[placement] nodes`), is built once by
`cell.build`, pinned in `artifact.index`, and started by `orch` `Up`; callers find a replica with `by_name`. The
Mac and phones run no control-room cell: they are viewers and signers. Control room shows itself: its eight cells
are rows in the Cells panel, its calls are in Audit and Traces. If ops-1 is down, leif-desk serves everything but
`cr.traces` until it restarts there, and each panel names the source that is missing.

## 10. Slice 1 (a few hours): read-only control room with two viewers, live

Goal: Leif opens the URL, signs in with his passkey, and sees his keys with expiry, his devices and the audit trail;
a second sign-in with a narrow chain sees less; proof by screenshot of both.

1. `cr.shell` + shell UI: `Session`, grant mint through `wire.ws-door`, lazy panel mount, light theme, five states.
2. `cr.keys` + Keys panel: `List`, `One` from `keys` `List`/`Meta`; `RotationState` and days left; holders gated by
   `cr.keys:holders`. Test: grant has no `keys:read`; `KeyRow` JSON of a seeded store contains no value bytes.
3. `cr.devices` + Devices panel: `List`, `One` from `fleet.inventory` (online, age, disk, tailscale, doctor
   summary); offline nodes stay listed.
4. `cr.audit` + Audit panel: `Search` over `audit.log` on ops-1 and leif-desk, `Chain` from `Verify`, hash link
   re-checked in the browser, search by actor, app, op, target.
5. Shared: `cr-view` crate (`View<T>`, `SourceState`, `CrError`, scope filter), `definePanel` and the kit import,
   codegen and drift gates in CI.
6. `tests/two_viewers.rs` and a second `ce-iam` role `control-room.operator`.
7. Deploy: build with `cell.build`, `orch` `Up` on ops-1 and leif-desk, face entry and route on `apps.ce-net.com`.
8. Proof: screenshots as Leif and as operator, phone width included, and the test output.

Not in slice 1: any write. The Keys panel shows "Renew" disabled with "arrives with approvals".

## 11. Later slices

| Slice | Adds |
|---|---|
| 2 | `approval-proof` crate, `cr.approvals` + panel, `keys` `Renew` and `Autopilot` ops, renew and rotate one key from the phone; `cr.renewals` + panel with "approve all due" |
| 3 | `cr.cells` + panel, `orch` `Apply`, start/stop/restart/move with plan preview and protected cells |
| 4 | `cr.traces` + panel, chain id join, audit-trace-chain cross links, `ByChain` |
| 5 | Phones and passkeys in Devices, grant and revoke from the inbox, access requests from a refused panel, export, desktop face |

## 12. Open points to settle while building

1. Re-read this against `repo:ce/docs/FULLSTACK-CELLS.md` when it exists: the name and shape of the panel
   definition (`definePanel` here), where a UI cell lives relative to its backend, and how a UI cell is registered.
   The contracts, rights and approval flow do not depend on it.
2. `ui.trace` `Span` has no chain id field today. Slice 4 either joins through the audit row by `call_id` (no
   contract change, needs the ledger stage to record `call_id` and chain id on every cell) or adds
   `chain_id: Option<String>` to `Span` (additive).
3. `root.approve` signs one request per signature. The bundle in section 6 keeps its device enrolment and key
   format and adds a bundle-level `to_sign`; decide whether that lives in `cr.approvals` or becomes
   `root.approve` `ProposeBundle`.
4. Which `fleet.inventory` `Node` sections carry tailscale and doctor on each node; missing ones are added in the
   node's inventory writer, not scraped by control room.
5. `debian` is offline now: `cr.keys` runs on ops-1 alone until it returns; the Devices panel shows it as offline.
