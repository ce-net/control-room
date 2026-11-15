# Control room: design (2026-11-15, second edition)

Leif 2026-11-15: "deploy a multi cellular system using our new frontend multi cellular frontend + baceknd and auth
paradigm system to manage my keys and cells and allow me to renew the keys and monitor all audit trails and traces
and everything and all of my devices in one place. like the previous app but more powerful, 100% type sfe and
writtine with the new design."

Control room is a live system: seven panels and a shell, eight full-stack cells, composed on one page. It owns no
secret and no source of truth. It reads the cells that already hold the truth and asks them, through
passkey-approved typed ops, to change things.

This edition replaces the first one (`repo:control-room@54f3988`), which was written before the paradigm document
existed. It follows `repo:ce/docs/FULLSTACK-CELLS.md` (`repo:ce@04b8d44`, fix `6022520`), called FS below, section
by section. Section 12 says what happens to the code written under the first edition.

## 1. Rules

The seven rules of FS section 1 hold unchanged. What they mean here:

1. Each panel is one full-stack cell: one repo, one contract crate `<name>-api`, one backend program, one `ui/`
   half. The face is the same cell's `ui { }` block, not a second cell.
2. Every panel has its own running backend, the shell included (`cr.shell` answers its own layout).
3. Declared once, in a Rust contract. TS types, clients, view bindings, the route, the manifest tree, the grants
   and the deployment plan are generated. `decl.lint` fails the build on a second declaration.
4. The server decides what the viewer gets. A panel the viewer may not have is not in the manifest, its bundle is
   refused, and its name never reaches the browser. No view hides anything by itself.
5. A view places another cell's view only through a typed slot pinned by ce-ref in `cell.lock`.
6. Majors N and N-1 are served together during every change.
7. Machines change only through typed cell ops. Secrets stay in `cell:keys`: no control-room cell holds
   `keys:read`, no control-room type has a field that can carry a value, and a renewal is an op `keys` performs
   itself after it has verified Leif's passkey signature over exactly that change.

## 2. What exists and is reused

| Need | Cell / crate | Ops used (right) |
|---|---|---|
| Key metadata | `keys@1` `repo:keys-cell` | `List`, `Boxes` (`keys:list`), `Meta` (`keys:meta`), `Audit` (`keys:audit`) |
| What renews itself | keys autopilot and its needs-you card, `repo:keys-cell` | through the added op of section 7 |
| Store grant renewal | `state.grant-renew@1` | `Renew{dry_run}` gives `RenewReport{due, renewed, skipped, failed}` |
| Grants held, revoked | `access.grant@1`, `cap.ledger` | `Grants`, `Revoked` (`access.grant:read`) |
| Policy | cap v3, CAL, `repo:ce-iam`, `cap.request`, `cap.mint`, `cap.revoke` | chain verify, `narrow`, caveats |
| One-popup approval | `grant.bundle@1` `repo:grant-bundle`, card served by `feed.approve` | `Propose`, `Get`, `List`, `Verify` |
| Signature check | `approval.check`, `root.approve@1` (Leif's P-256 WebAuthn device key) | `Pending`, `Devices`; `RootKey` items |
| Needs-you items | `needs.list@1`, `feed.approve@1` | `List` (`needs.list:read`, `feed.approve:read`) |
| Devices | `fleet.inventory@1`, `node.doctor` | `State`, `Node{node, section}`, `Items` (`fleet.inventory:read`) |
| Phones, passkeys | `root.approve` `Devices`, `account.passkey` `List` | approval devices and sign-in passkeys |
| Running cells | `orch@1`, `cell.registry`, `deploy.version`, `repo:ce/crates/ce-replica` | `Ps`, `Logs` (`orch:read`); `Up`, `Down`, `Restart` (`orch:manage`) |
| Builds | `cell.build`, `artifact.index` | `Publish`, `List`, `Resolve` |
| Audit | `audit.log@1` | `Query`, `Verify` (`audit:read`), `Trail` (`audit:trail`): hash-linked rows, signed head |
| Receipts offline | `repo:ce-net-capability-verify` | check a chain or a receipt file without the mesh |
| Traces | `ui.trace@1` | `Query`, `Summary` (`ui.trace:read`) |
| Sign-in | `account.signin`, `account.session`, `account.passkey` on apps.ce-net.com | cookie to person |
| Person to browser grant | `person.grant` `Mint` through `wire.ws-door` | chain in the URL fragment, one socket on `/wire` |
| Records | `ce-db-rs` `#[derive(Record)]`, `state.store@2`, `ce-replica` | projections and their `Watch` |
| Contract, codegen | `ce_wire::api!`, `ce-wire-gen` (`freeze`, `adapt`, `json`), `ce-gate`, `cell-check conform` | all contracts |
| Previous app | `repo:keys-cell/crates/keys-ui`, `repo:needs-list` | key list and needs card are the base of two panels |

From FS, used in their fixed shapes and not on disk on 2026-11-15 (checked in `~/dev`): `ui { }` in `ce-wire`,
`ce-wire-gen --lang ts-ui`, `ui-part`, `ui-rt`, `ui-host`, `ui-kit`, `ui-chain`, `layer-view`, `cell-site`
(`site!`), `ui.manifest`, `ui.bundle`, `ui.serve`, `site.gateway`, `decl.lint`, `contract.check`, `cap.explain`,
`cap.inspect`, `ui.inspect`, `deploy.ship Plan/Apply` over `net!`. Control room is their first consumer; slice 1
(section 13) builds or takes the FS slice 1 subset and nothing wider.

## 3. Shape

```
browser ── GET ce-net.com/x/control ──▶ site.gateway (ops-1, leif-desk)
   cookie -> account.session -> person.grant Mint -> chain C
   ui.manifest Resolve{route, C} -> only the cells C may receive; ui.serve gives only those bundles
cr.shell ─┬ slot keys ───── cr.keys ────── keys, access.grant
          ├ slot cells ──── cr.cells ───── cell.registry, orch, deploy.version, artifact.index, ce-replica
          ├ slot devices ── cr.devices ─── fleet.inventory, node.doctor, root.approve Devices, account.passkey
          ├ slot audit ──── cr.audit ───── audit.log on every node, keys Audit, cap.ledger
          ├ slot traces ─── cr.traces ──── ui.trace, audit.log (join on call id)
          ├ slot renewals ─ cr.renewals ── keys autopilot, state.grant-renew, access.grant; slot keys: cr.keys
          ├ slot approvals  cr.approvals ─ grant.bundle, root.approve, feed.approve, needs.list
          └ slot inspect ── cap.inspect ── cap.explain (Leif only, later slice)
```

The route and the tree are declared once, in `site!` (`cell-site`):

```rust
// repo:control-room/crates/control-room-site/src/lib.rs   moves into repo:ce-net-site when that repo exists
site! {
    pub CONTROL_ROOM; host "ce-net.com"; policy control_room_policy::POLICY; kit repo:ui-kit@<pinned>;
    route "/x/control/*" => cr.shell@2 { slots {
        keys: cr.keys@2;  cells: cr.cells@1;  devices: cr.devices@2;  audit: cr.audit@2;  traces: cr.traces@1;
        renewals: cr.renewals@1 { slots { keys: cr.keys@2; } };
        approvals: cr.approvals@1;  inspect: cap.inspect@1;
    } };
}
```

`/x/control` is where the gateway answers today (FS section 10, step a). The final route is `ce-net.com/control`,
and the same shell fills a slot of `site.admin`. `apps.ce-net.com/control-room` (the first edition's face) keeps
serving until the new route has passed section 13's proof on real use, then answers 301.

Repos, one per cell: `repo:control-room` (`cr.shell`, the site crate, `control-room-policy`, this document),
`repo:cr-keys`, `repo:cr-cells`, `repo:cr-devices`, `repo:cr-audit`, `repo:cr-traces`, `repo:cr-renewals`,
`repo:cr-approvals`. Each: `crates/<name>-api`, `crates/<name>`, `ui/src/<View>.ts` (the only hand-written
frontend file), `ui/gen/` (generated), `versions/`, `cell.toml`, `cell.lock`. Any other site can place the keys
cell in a slot without taking anything else.

### 3.1 Backends are projections, not stores of truth

A `cr.<p>` backend holds narrow read grants on its sources, joins their answers into rows for one panel, and keeps
those rows as a typed record collection in `state.store` (section 6). That collection is what `Watch` streams and
what `layer-view` filters per viewer. It is a cache: drop it and it is rebuilt from the sources. A backend holds no
write right that works without Leif's signature (section 7).

Every contract has `Sources(()) -> Vec<Source{cell, node, state: Live | Stale{age_s} | Down{why} | Refused{need},
ms}>`, so a dead node reads "debian: no answer for 3 d" and not an empty table. `Source` rows are filtered by the
same row caveats as the data.

## 4. Contracts

Shared types (`Health`, `Source`, `SourceState`, `Ref`, `KeyName`, `NodeName`, `CallId`, `ChainId`) are declared
once in `repo:control-room/crates/cr-types-api` and imported; `ce-wire-gen` emits them as one shared module.
Contracts that existed under the first edition change shape (rows as a stream, `Part` fields, no envelope), which
is a breaking change, so they are major 2 with `versions/v1.rs` frozen; new cells start at major 1.

### cr.keys@2 (in full; the others follow the same form)

```rust
// repo:cr-keys/crates/cr-keys-api/src/lib.rs
ce_wire::api! {
    pub CrKeys = "cr.keys" @ 2;
    types {
        pub enum RotationState open { Fresh, DueSoon { days: u32 }, Overdue { days: u32 }, Rotating,
                                      Failed { why: String }, Manual }
        pub struct Holder  { pub name: String, pub home: NodeName, pub lanes: Vec<String>, pub not_after: i64 }
        pub struct KeyRow  { pub name: KeyName, pub service: String, pub purpose: String, pub owner: String,
                             pub version: i64, pub sha256_8: String, pub written_at: i64,
                             pub expires_at: Option<i64>, pub rotation: RotationState, pub self_renewing: bool,
                             pub last_check: String, pub last_check_at: i64,
                             pub holders: Part<Vec<Holder>, "cr.keys:holders">,     // field-level right
                             pub last_opened: Part<i64, "cr.keys:history">,
                             pub refs: Vec<Ref> }
        pub struct ShareRow { pub key: KeyName, pub to: String, pub due_at: i64, pub state: String }
        pub struct KeysQuery { pub q: Option<String>, pub due_within_days: Option<u32> }
    }
    ops {
        1 => HealthOp(()) -> Health                        needs "cr.keys:health";
        2 => List(KeysQuery) -> Vec<KeyRow>                needs "cr.keys:read";
        3 => Watch(KeysQuery) -> stream Delta<KeyRow>      needs "cr.keys:read";
        4 => Shares(KeysQuery) -> Vec<ShareRow>            needs "cr.keys:read";
        5 => History(KeyName) -> Vec<KeyEvent>             needs "cr.keys:history";
        6 => Sources(()) -> Vec<Source>                    needs "cr.keys:read";
    }
    ui {
        view KeyList {
            props  { q: Option<String>, due_within_days: Option<u32> }
            needs  "cr.keys:read";                   // without it: no manifest entry, no bundle, slot empty
            events { opened(KeyName); renew(KeyName); rotate(KeyName); }
            data   { rows: Watch(KeysQuery), shares: Shares(KeysQuery), sources: Sources(()) }
            speaks [2];
        }
    }
}
```

(Header and op punctuation are whatever `ce-wire` parses at the pinned rev; the content is as written.)
`KeyRow` has no field that can carry a value: the type is the guarantee. `cr.keys` holds `keys:list`, `keys:meta`,
`keys:audit` on `keys` and its test asserts the grant has no `keys:read`. The view has no renew op: it emits
`renew(KeyName)`, the shell passes it to the approvals slot as a typed prop, and the change travels section 7.

```ts
// repo:cr-keys/ui/src/KeyList.ts      imports only ui/gen and ui-kit
import { KeyList } from "../gen/cr_keys";
export default KeyList.render(({ data, emit }) =>
  table({}, each(data.rows, k => k.name, k =>
    row({ onclick: () => emit.opened(k.name) }, k.name, k.service, expiry(k.expires_at), rotation(k.rotation),
        show(k.holders, hs => holders(hs)),                        // absent on the wire for a narrow viewer
        button({ onclick: () => emit.renew(k.name) }, "Renew")))));
```

### The other contracts

| Cell | Row types (`Part` fields named) | Ops (right) | View: props, events, slots |
|---|---|---|---|
| `cr.cells@1` | `CellRow{api, major, serves, placement, instances[{node, state, since_ms, healthy, leader}], wanted, live, drift, protected, build: Part<Sha256, "cr.cells:builds">}` | `List`, `Watch`, `Sources` (`cr.cells:read`); `Logs` stream (`cr.cells:logs`); `Plan(CellChange) -> Planned{plan: Sha256, steps, refuses}` (`cr.cells:plan`) | `CellBoard`: `{node?, unhealthy_only?}`; `opened(CellName)`, `change(CellChange)` |
| `cr.devices@2` | `DeviceRow{id, name, kind: Server \| Desktop \| Laptop \| Phone \| Tablet, online, last_seen_s, disk[{mount, used_pct, free_gb}], load, doctor{ok, failing[], at}, cells, tailscale: Part<Tailnet{ip, online, key_expires_at}, "cr.devices:net">, passkeys: Part<Vec<PasskeyRef>, "cr.devices:passkeys">}` | `List`, `Watch`, `Sources` (`cr.devices:read`); `Doctor(id) -> DoctorReport`, asked of that node's `node.doctor` now (`cr.devices:doctor`) | `DeviceList`: `{kind?}`; `opened(NodeName)` |
| `cr.audit@2` | `AuditRow{seq, at_ms, node, actor, app, op, target, ok, kind: Call \| Grant \| Revoke \| Approval \| KeyEvent, call_id, chain_id, prev_hash, hash, detail: Part<String, "cr.audit:detail">}` | `Search(AuditQuery)`, `Watch`, `Sources` (`cr.audit:read`); `Chain(node) -> ChainState{head_seq, anchored_seq, first_bad}` (`cr.audit:verify`); `Receipt(RowId) -> Receipt{row, proof, head_sig}` (`cr.audit:read`); `Export` stream (`cr.audit:export`) | `AuditTrail`: `{q?, chain?, call?}`; `trace(CallId)`, `chain(ChainId)` |
| `cr.traces@1` | `TraceRow{call_id, root_api, root_op, started_ms, ms, outcome, nodes, cells}`, `SpanRow{call_id, parent, api, op, node, start_ms, ms, outcome, audit_seq, chain: Part<ChainHop{chain_id, depth, right_used}, "cr.traces:chains">}` | `Recent(TraceQuery)`, `Watch`, `Tree(CallId)`, `ByChain(ChainId)`, `Summary`, `Sources` (`cr.traces:read`) | `TraceList`: `{tree?, chain?}`; `audit(CallId)`, `chain(ChainId)` |
| `cr.renewals@1` | `RenewalRow{what: Key \| Share \| StoreGrant \| AccessGrant \| Cert \| TailscaleKey \| Passkey, name, holder, expires_at, mode: SelfRenewing{next_run_at, last} \| NeedsLeif{why, propose: Change}}` | `List`, `Watch`, `Runs`, `Sources` (`cr.renewals:read`) | `Renewals`: `{within_days?}`; `approve(Vec<Change>)`; slot `keys: cell:cr.keys@2 optional` |
| `cr.approvals@1` | `InboxRow{id, kind: Bundle \| RootChange \| Message \| Need, title, items[{n, change: Change, owner, effect, state}], until, card: Ref}` | `List`, `Watch` (`cr.approvals:read`); `Propose(Vec<Change>) -> InboxRow`, `Cancel(id)` (`cr.approvals:propose`) | `Inbox`: `{draft?: Vec<Change>}`; `done(BundleId)` |
| `cr.shell@2` | `Layout{title, tabs[{slot, label}]}`: only slots the manifest kept | `Layout(route)` (`cr.shell:open`) | `ControlRoom`: the eight slots of section 3, all optional |

`Change` is declared once, in `cr-approvals-api`: `KeyRenew{name} | KeyRotate{name} | KeySeal{name} |
ShareSend{key, to} | StoreGrantRenew{node, label} | Grant{to, rights, caveats, not_after} | Revoke{grant} |
Cell(CellChange)`, with `CellChange` from `cr-cells-api`: `Start{api, node} | Stop{api, node} | Restart{api, node}
| Move{api, from, to} | Deploy{api, node, build}`. Both enums are `open`.

Composition is typed like props in React, checked by `tsc` from the imported contracts: the shell routes
`AuditTrail.trace(CallId)` into `TraceList{tree}`, `TraceList.chain(ChainId)` into `AuditTrail{chain}`,
`KeyList.renew` and `Renewals.approve` into `Inbox{draft}`. Renewals places `KeyList{due_within_days: 30}` in its
own slot. A slot whose cell the viewer may not have is empty, and the parent can observe only that.

`cr.audit` fans out to `audit.log` on every node and adds `keys Audit` and `cap.ledger` rows as the same type.
The view recomputes `hash` from `prev_hash` and the row for the rows on screen, so a broken link shows in the
browser and not only in the server's report; `Receipt` verifies offline with `repo:ce-net-capability-verify`.
`cr.cells Plan` is a dry run. `Move` is always up on the target, healthy and caught up, then down on the source.
Protected cells (`state.store`, `keys`, the door, `root.approve`, `feed.approve`, `grant.bundle`, the last
gateway) refuse `Stop` and `Move` in `Plan` with the reason: a node's `state.store` is never stopped, because its
gate reads it.

## 5. Rights: from the grant to the pixel

Policy is declared once, in `cap!` / CAL in `repo:control-room/crates/control-room-policy`. It generates the
grants, the `person.grant` table and the rights column of this document.

| Role | Holds | Row caveats |
|---|---|---|
| `root` (`person:leif`) | every `cr.*` right, `ui.manifest:inspect`, `cap.explain:read` | none |
| `operator` (a named person, until a date) | `cr.shell:open`, `cr.devices:read`, `cr.cells:read`, `cr.audit:read`, `cr.traces:read` | `node == "ops-1"`, `app == "orch"`, `api == "orch"` |
| visitor (the public table) | nothing of control room | the route answers the sign-in page |

The four server checks of FS 4.2, one declaration each:

| Level | Declared | Checked by | A viewer without it |
|---|---|---|---|
| cell | `view .. needs` | `ui.manifest`, `ui.serve` | no manifest entry, no bundle, slot empty |
| op | `ops .. needs` | `ce-gate` at the backend's door | `Refused{need}` |
| field | `Part<T, R>` | generated serialiser | field absent on the wire |
| row | `#[record(caveat = [node, service, owner, app, api])]`, CAL caveat in the chain | `layer-view`: the caveat becomes the `state.store` query and filters `Watch` deltas | row never read |

Two viewers, one URL:

| | Leif | Operator |
|---|---|---|
| Manifest | `cr.shell` and eight slot cells | `cr.shell`, `cr.devices`, `cr.cells`, `cr.audit`, `cr.traces` |
| Bytes received | all bundles | no byte naming keys, renewals or approvals; `ui.serve Fetch` of their hashes answers `Refused` |
| Devices | every node and phone, tailscale, passkeys | ops-1 only, no `tailscale`, no `passkeys` field |
| Audit | everything, with `detail` | rows of `orch`, no `detail` field |
| Direct call of `CrKeys::List` | rows | `Refused` |
| Right ends (`cap.revoke`) | | stream closes with `Revoked`, the panel leaves the open page |

A right is asked with `cap.request`, shows in `needs.list`, is approved in the inbox (section 7), minted by
`cap.mint`, recorded in `access.grant` and `cap.ledger`. No control-room cell mints a right. A backend narrows
the chain it was handed before it calls a source (`narrow`), so it can do no more for a viewer than the viewer
may do. Why something is absent is answered by `cap.explain Why` in the `cap.inspect` slot (Leif only), with
`as:` to look at the page as the operator. `ui-host` gives a view no way to read the viewer's rights, and
`decl.lint` refuses a role compare, a `display:none` driven by a right, or a call outside `ui/gen` in `ui/src`.
Until `layer-auth` puts cap v3 into `ce-gate`, the caveat check is the shared crate `ui-chain`.

## 6. Records

| Record (`cell`, `collection`, version) | Owner | Kind | Notes |
|---|---|---|---|
| `cr.keys/key_row@1`, `cr.keys/share_row@1` | `cr.keys` | projection | the contract's row types themselves derive `Record`; no second declaration |
| `cr.cells/cell_row@1`, `cr.devices/device_row@1`, `cr.audit/audit_row@1`, `cr.traces/trace_row@1`, `span_row@1`, `cr.renewals/renewal_row@1` | each cell | projection | rebuilt from sources; `Part` fields stored whole, withheld per caller by the serialiser |
| `cr.approvals/run@1` `{bundle, n, state, receipt_seq, at_ms}` | `cr.approvals` | single writer | behind the `ce-replica` leader lease, never merged |
| `<owner>/spent@1` `{bundle, n, at_ms}` | the performing cell (`keys`, `orch`, ..) | single writer | an approved item runs once |
| `control-room.ui-state` `{layout, theme, filters}` | `control-room.ui-state` (the `<app>.ui-state` pattern) | `per_person` | theme `light` by default |

Bundles themselves live in `grant.bundle`. No record holds a secret, a token or a grant body: a grant is referred
to by its chain id. Projection rows merge by HLC (default merge); a record change follows FS 6.2 (`#[migrate]`,
`schema.migrate` in five phases) and is never an edit in place.

## 7. Approvals: one popup, and who actually performs

1. A panel emits `Change`s ("approve all 6 due"). `cr.approvals Propose` turns each into one `grant.bundle` item:
   the owner cell, the exact scope (`keys:renew` where `name == "<key>"`), an end at most ten minutes out, and a
   plain-words line taken from the owner's dry run. Kinds map to what exists: `Seal` for a secret Leif pastes
   (the value goes from the card straight into `keys` and never reaches control room), `RootKey` for root and
   device-key changes (redeemed by `root.approve`), `CapRequest` and `IamGrant` for rights.
2. The inbox view shows every item with its effect and links to the one card, `apps.ce-net.com/grant?b=<id>`,
   served by `feed.approve`: a tick per item, one passkey signature (P-256) over the item texts.
3. `cr.approvals` watches the bundle. For each approved item, under its leader lease, it calls the owner:
   `keys Renew{name, mode, item: {bundle, n}}`, `orch Apply{change, item}`, `state.grant-renew Renew`, `cap.mint`.
4. The owner trusts nothing about `cr.approvals`. It calls `grant.bundle Verify{bundle, n}` itself (the WebAuthn
   signature is re-checked by `approval.check` on every call), compares the signed text with the change it was
   asked to do, checks the end and its own `spent` record, performs, writes `spent`, and its ledger stage writes
   the audit row with the bundle id.
5. `Watch` streams item states to the inbox: `Waiting`, `Approved`, `Running`, `Done{receipt}`, `Failed{why}`. A
   failed item does not undo the others; it returns as a new proposal.

The rights `cr.approvals` holds on owners (`keys:renew`, `orch:apply`) carry the caveat `require
approved(grant.bundle item)`, the same construction as FS 4.1's `deploy.ship:apply .. require cosign`. So a stolen
session, a compromised `cr.approvals` or a changed bundle cannot renew, stop or grant anything.

Added ops in existing cells, all additive within their major:

| Cell | Op | Right |
|---|---|---|
| `keys` | `Renew(RenewReq{name, mode: Renew \| Rotate, item}) -> Result<KeyInfo, KeysError>`: runs the code the autopilot runs for that key, seals the new version, answers metadata | `keys:renew` |
| `keys` | `Autopilot(()) -> AutopilotState{rules, last_runs, needs_you}`: the autopilot's state as a typed answer | `keys:list` |
| `orch` | `Apply(ApplyReq{change, item}) -> Result<Done, OrchError>`: the approved form of `Up`/`Down`/`Restart`; `orch:manage` is not given to control room | `orch:apply` |

When `deploy.ship Plan/Apply` exists, `Deploy` and `Move` become a plan hash, and the signed item names that hash.

## 8. Codegen and type safety

`ce-wire-gen --lang ts-ui` writes `ui/gen/` per cell (types, client, `defineView` binding) and is never edited;
`--lang json` writes the descriptor; the same contracts become MCP tools at the site's MCP door for whatever the
caller's chain may call. `tsc --noEmit --strict`, no `any`, no `as`. Rust: `#![forbid(unsafe_code)]`, clippy with
warnings denied, no `serde_json::Value` in a contract. Each row below is a `break-proof` test:

| Change | Fails in |
|---|---|
| rename `KeyRow.rotation` | `tsc` of `repo:cr-keys/ui/src/KeyList.ts` |
| read `k.holders` without `show`/`match` | `tsc` (`Part<T>` has no direct value) |
| add a `RotationState` variant and not draw it | `tsc` (exhaustive `match`, `never`) |
| shell passes a `CallId` where `TraceList` wants a `ChainId`, or fills `keys` with another cell | `tsc` of `repo:control-room/ui/src/ControlRoom.ts` |
| view `KeyList` needs less than `Watch` needs | `ce-wire-gen` |
| a `cr.*` right no role in the policy holds | `cargo build` of the site crate (`cell-cap`) |
| contract changed without a major or a frozen `versions/v<N>.rs` | `contract.check` in `cell.build` |
| `Health` declared in two contracts, a second `tokens.css`, a right string outside a contract | `decl.lint` |
| a source contract (`keys@1`) changes under a pinned hash in `cell.lock` | `cargo build` of the view cell |

## 9. Bundles and runtime

- `ui.bundle` (`cell.build` target `ui-web`): codegen, `tsc`, esbuild to one ES module per view, `artifact.index
  Publish`. Eight view bundles; `ui-rt`, `ui-kit` and `cr-types` are external and published once. Budget: a view
  at most 30 kB gzip, `ui-rt` at most 6 kB.
- A bundle's name is its sha256 (`/_c/<sha256>.js`, immutable). Every control-room view has `needs`, so `ui.serve`
  gives a bundle only to a chain that holds the view's right and none is cached at the public edge.
- The manifest is signed by the gateway's node key and carries `integrity` per module; the import map repeats the
  hashes; CSP is `script-src 'self'`, no inline, no eval. `ui-host` refuses a module whose digest differs and
  reports it to `audit.log`.
- One socket to `wire.ws-door` for the page. Each view calls only its own backend over it with the same chain.
  `data { rows: Watch(q) }` is one subscription: a snapshot at an HLC, then `Delta{put, del, hlc}`, filtered per
  row and per `Part` on the server, applied once per animation frame, resumed from the last HLC after a
  reconnect, paused off screen.
- A panel fails alone: its backend down or refusing shows that panel's own error state with its `Sources`; the
  shell and the siblings keep running. `ui.manifest Watch` mounts a panel when a grant lands and unmounts it on a
  revoke, without reload.
- Look: `repo:ui-kit` (one `tokens.css`, one component set), light theme by default, dark by preference. Dense
  tables, one accent, expiry as a date and as days left, state as text plus colour. Works at phone width,
  because approvals are signed on the phone. Desktop and phone native faces take the same contracts through
  `ce-wire-gen --lang rust-ui` later.

## 10. Versions

- Identity is `cell:<api>@<major>`. Within a major only additive change. `cr.keys`, `cr.devices`, `cr.audit` and
  `cr.shell` go to major 2 now: `ce-wire-gen freeze` keeps `versions/v1.rs`, `adapt` writes the adapter, each
  backend serves 1 and 2 while the first edition's face is still up. Major 1 is retired by its own `Retire` plan
  change after that face is gone and `Callers` has been zero for 14 days.
- Hello carries the schema hash and `serves: [major]`; the generated client picks the highest major in both
  `speaks` and `serves`. `ui.manifest` picks, per cell, the newest bundle whose `speaks` meets what the replicas
  taking traffic serve. No intersection is a plan error and cannot reach a page.
- Rollout order (FS 6.4): backends first, one replica at a time, followers before the leader; bundles second, to
  1, 10, 50, 100 percent of new page loads; open pages keep their bundle and still have a backend.
- Source cells are pinned in each view cell's `cell.lock` (`cell:keys@1` plus contract hash). `contract.check
  Mesh` names every control-room cell a source change would break before that change can deploy.

## 11. Deployment

Placement is declared in `net!` beside `site!`; this table is its content.

| Cell | Runs on | Replicas | Notes |
|---|---|---|---|
| `site.gateway`, `ui.manifest`, `ui.serve` | ops-1, leif-desk | 2, active-active | the edge; beside `account.session`, `person.grant`, `wire.ws-door` |
| `cr.shell` | ops-1, leif-desk | 2, active-active | stateless |
| `cr.keys` | ops-1; debian when it is back | 1, then 2 | on the nodes that run `keys`, so metadata reads stay local |
| `cr.cells`, `cr.devices`, `cr.audit`, `cr.renewals` | ops-1, leif-desk | 2, active-active | each fans out to its sources on every node; projection in the node's `state.store` |
| `cr.traces` | ops-1 | 1, restarts on leif-desk | next to `ui.trace` |
| `cr.approvals` | ops-1, leif-desk | 2, leader by `ce-replica` | one leader runs an approved item; `spent` makes a repeat harmless |
| `control-room.ui-state` | ops-1, leif-desk | 2 | per-person rows |
| bundles | `artifact.index`, served by `ui.serve` | every edge node | by hash |

The Mac, debian while it is offline, and the phones run no control-room cell. The Mac is read only through its
own typed cells and `fleet.inventory`; a node that does not answer stays listed as offline with its last row and
age. Phones are viewers and signers and appear as devices because they hold an approval key. Control room shows
itself: its cells are rows in Cells, its calls are in Audit and Traces. With ops-1 down, leif-desk serves
everything but Keys and Traces, and each of those panels names the source that is missing.

```
ce site plan  ce-net.com --only cr.shell cr.keys cr.devices cr.audit   # every change, the op, the right it needs
ce site apply ce-net.com --plan <sha256>     # asks for ApprovedSigned when a change needs it, streams events
ce site back  ce-net.com                     # previous converged state
```

`Plan` runs `decl.lint`, `contract.check Mesh`, the negotiation check and the placement checks before it lists a
change; an empty plan after `Apply` is the definition of converged. Until `deploy.ship Plan/Apply` over `net!`
exists (FS slice 3), the same changes are made by the typed ops a plan would list, in this order: `cell.build`,
`artifact.index Publish`, `cell.install`, `orch Up`, then `deploy.version` switches the route. No host shell
anywhere in the path.

Dev: `cargo cell dev cr.shell cr.keys cr.devices --as operator` beside `--as root` shows the same page under two
chains on a `sim-mesh`; `#[cell_test(sim, as = [operator, root])]` runs one test body per role.

## 12. What happens to the first edition's code

`repo:control-room@e0f4a75` and the repos `cr-keys`, `cr-devices`, `cr-audit`, `cr-sources` were built on the
first edition and are installed on ops-1. Nothing is thrown away; each piece moves to its fixed place.

| Exists | Becomes |
|---|---|
| `crates/cr-keys-api`, `cr-devices-api`, `cr-audit-api` in `repo:control-room` | move into their cells' repos; gain the `ui { }` block and `Part` fields as major 2; v1 frozen |
| `Health`, `SourceState`, `Source`, `CrError` repeated in each contract | declared once in `cr-types-api`; `CrError::Refused` is the gate's `Refused{need}` |
| envelope `KeysView{rows, narrowed, ..}` | rows as `List` and `Watch`; `sources` is its own op; `narrowed` and `can_ask` are removed (a view is not told what it lacks; `cap.explain` answers) |
| `cr-view` (`scopes`, `holds`, scope abilities `cr.devices:scope:node=..`) | the row filter of `layer-view` and the chain reader of `ui-chain`; the scope-ability form is read as the v1 spelling of a caveat until cap v3 is in `ce-gate` |
| `cr.shell Session{panels}` and `ui/dist/panels.json` | `ui.manifest Resolve`; the shell keeps only `Layout` |
| `cr-shell/src/web.rs` (page, hashed modules, `/wire`) | the minimum of `site.gateway` and `ui.serve` |
| `ui/kit` (`definePanel`, `dom.ts`, `tokens.css`, fonts) | `defineView` in `ui-rt`/`ui-host`; tokens and components become the first content of `repo:ui-kit` |
| `ui/panels/<p>/panel.ts` | `ui/src/<View>.ts` in each cell's repo, against `ui/gen` |
| `ui/tools/lint.mjs` | the `ui/src` rules of `decl.lint` |
| `roles/*.json` | generated from `control-room-policy` |
| `repo:cr-sources` (source contracts copied, hashes pinned) | `cell.lock` pins on the sources' own `-api` crates; its `ui.trace` span helper stays |

## 13. Slice 1 (a few hours): read-only, two viewers, live at `ce-net.com/x/control`

Goal: Leif signs in with his passkey and sees his keys with expiry and holders, every device, and the audit trail,
live. The operator role opens the same URL and receives less, proved on the wire.

Build, in order:

1. Shared, the FS slice 1 subset, taken if another lane has landed it and otherwise built at the minimum in its
   own repo: `ui { }` in `ce-wire`; `ce-wire-gen --lang ts-ui`; `ui-part`; `ui-rt` and `ui-host` (mount, slot,
   signal, one socket), lifted from `ui/kit`; `ui.manifest` (`Resolve` only), lifted from `Session{panels}`;
   `ui.serve`; the minimum of `site.gateway`, lifted from `cr-shell/src/web.rs`; `ui-chain` and the equality and
   set caveats of `layer-view`, lifted from `cr-view`.
2. `cr-types-api`; `control-room-policy` with `root` and `operator`; the site crate of section 3 with the four
   slice 1 slots.
3. `cr.shell@2`: `Layout`, view `ControlRoom` with optional slots, light theme.
4. `cr.keys@2`: contract of section 4, `List`/`Watch`/`Sources` from `keys List`/`Meta`, projection `key_row`,
   view `KeyList`. "Renew" is drawn disabled with "arrives with approvals".
5. `cr.devices@2`: `List`/`Watch`/`Sources` from `fleet.inventory`, `Part` fields `tailscale` and `passkeys`,
   offline nodes kept, view `DeviceList`.
6. `cr.audit@2`: `Search`/`Watch`/`Chain` over `audit.log` on ops-1 and leif-desk, hash link rechecked in the
   view, `Part` field `detail`, view `AuditTrail`. Last, and cut first if time runs out.
7. v1 of the three contracts frozen and still served.
8. Deploy by typed ops (section 11) to ops-1 and leif-desk; the Pages Function forwards `/x/*`.

Proof, as `#[cell_test(sim, as = [operator, root])]` and again live, with screenshots at desktop and phone width:

1. Root: the manifest has four cells; keys show expiry, rotation state and holders; a device going offline
   changes its row without reload.
2. Operator: the manifest has `cr.shell`, `cr.devices`, `cr.audit`; devices shows ops-1 only.
3. Not sent: every byte the operator's browser received (HTML, manifest, modules, frames) is searched for
   `cr.keys`, its bundle hash and every key name: none. `ui.serve Fetch` of that hash and `CrKeys::List` with the
   operator chain both answer `Refused`.
4. Field level: the operator's device rows have no `tailscale` field on the wire; a root chain without
   `cr.keys:holders` gets key rows with no `holders`.
5. No secret: the JSON of every `KeyRow` from a seeded store contains no value bytes; `cr.keys`'s grant has no
   `keys:read`.
6. Revoke: `cap.revoke` on the operator grant with the page open; the streams close and the panels leave.
7. Types: `break-proof` renames `KeyRow.rotation`; the build fails in `ui/src/KeyList.ts`.
8. Integrity: one byte of a bundle changed on disk; `ui-host` refuses it.

Not in slice 1: any write, prerender, `decl.lint`, `contract.check`, `deploy.ship Plan/Apply`, `cap.inspect`.

## 14. Later slices

| Slice | Adds |
|---|---|
| 2 | `cr.approvals` and `cr.renewals` with their views; `keys Renew` and `Autopilot`; renew and rotate one key from the phone through one bundle; "approve all due" |
| 3 | `cr.cells` and its view; `orch Apply`; start, stop, restart and move with the plan shown first and protected cells refused |
| 4 | `cr.traces` and its view; chain id on spans; audit, trace and chain open from each other; `ByChain` |
| 5 | `decl.lint`, `contract.check`, `Retire` of major 1, `ce site plan/apply`, `cap.inspect` slot, the 301 from the old face |
| 6 | phones and passkeys in Devices, grant and revoke from the inbox, export, native faces through `rust-ui`, the route moves to `ce-net.com/control` inside `repo:ce-net-site` |

## 15. Open points

1. The session cookie is set on apps.ce-net.com and the gateway answers on ce-net.com: either the cookie's domain
   becomes `ce-net.com` or the gateway receives the person from `account.signin` by redirect. Decided with the
   owner of `account.session`, before slice 1 goes live.
2. `grant.bundle` items are grants. A one-shot change is written here as a grant of one narrowed right that ends
   in ten minutes; whether `grant.bundle` should gain an explicit `Change` kind is its owner's call.
3. Whether a contract row type can derive `Record` directly (section 6) depends on `ce-db-derive` accepting
   `Part` fields; if not, the record is generated from the contract type, still one declaration.
4. `ui.trace Span` has no chain id today: slice 4 adds `chain_id: Option<..>` (additive) or joins through the
   audit row by `call_id`.
5. `state.store Watch` under a per-row chain filter with resume by HLC is not measured (FS section 13).
6. Which `fleet.inventory Node` sections carry tailscale and doctor on each node; a missing one is added in that
   node's inventory writer, never scraped by control room.
7. debian is offline: `cr.keys` runs on ops-1 alone until it returns, and Devices shows it as offline.
