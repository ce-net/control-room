# Control room: slice 1 as built (2026-11-15)

Slice 1 of [DESIGN.md](DESIGN.md) section 13: read only, two viewers, three panels and the shell, each a full-stack
cell. This page says what was built, where, how it was proved, and what is not done.

## Cells and repos

| Cell | Repo | Contract | View | Majors served |
|---|---|---|---|---|
| `cr.shell` | repo:control-room | `crates/cr-shell-api` (`Layout`) | `ui/src/ControlRoom.ts` | 2, and 1 (`Session`) frozen |
| `cr.keys` | repo:cr-keys | `crates/cr-keys-api` (`List`, `Watch`, `Sources`) | `ui/src/KeyList.ts` | 2, and 1 frozen |
| `cr.devices` | repo:cr-devices | `crates/cr-devices-api` (`List`, `Watch`, `Sources`) | `ui/src/DeviceList.ts` | 2, and 1 frozen |
| `cr.audit` | repo:cr-audit | `crates/cr-audit-api` (`Search`, `Watch`, `Chain`, `Sources`) | `ui/src/AuditTrail.ts` | 2, and 1 frozen |

Shared minimum, in repo:ui-cells: `ui-part` (the `views!` block: props, needs, events, data, streams, slots,
speaks, and `parts`), `ui-chain` (chain reader and the `where` row caveats of `layer-view`), `ui-manifest`
(`Resolve` per viewer, ui.serve's rule), `site-gateway` (sign-in person, grant, manifest, bundles by sha256, one
socket), `ui-gen` (`views!` to typed TypeScript bindings), `ui-rt`, `ui-host` and the kit.

Each backend is a projection of its sources, rebuilt every 10 to 15 s and kept in memory; `Watch` sends a snapshot,
then deltas, and ends after 30 s so the page opens a new connection (a new Hello: an ended or revoked grant is refused
within 30 s). Rights are checked on the server at four levels: cell (manifest and ui.serve), op (ce-gate), field
(`Parts::withhold`, so a field is `None` on the wire) and row (`where` caveats signed into the grant), and source lines
follow the same node and app caveats.

## Policy

`crates/control-room-policy`: `root` holds every control-room right; `operator` holds the shell, devices and the audit
trail, with `cr.devices:where:node=..` and `cr.audit:where:app=..` caveats taken from the person's entry in the people
file, until a date. The people file lives on the node (`~/.config/control-room/people.json`), never in this repo.

## Deploy

On ops-1 by typed ops: `cell.install Upgrade {api, source: Repo}` for each of the four cells, from the pinned commits;
each registered both majors (two schemas per cell). The built site (one module per view, named by its sha256) is carried
inside the `cr-shell` binary. The gateway face answers on loopback `:8834` for the apps.ce-net.com sign-in gate.

## Proofs

Rust tests (`cargo test` in each repo; one body per viewer chain, root and operator): ui-cells 20, cr-keys 11,
cr-devices 6, cr-audit 8, control-room 10 (55, all passing). Among them: the operator's manifest has the shell, devices and audit and
no byte of the keys cell, the keys bundle answers `403 Refused` to them and `bundles.json` is never served
(`cr-shell/tests/two_viewers.rs`); no value bytes in any `KeyRow` and the cell's grant has no `keys:read`
(`cr-keys/.../no_values.rs`); `Part` fields absent on the wire for a narrower chain; a device going offline is one
delta and keeps its last-seen time; each major 1 schema is the first edition's, byte for byte; a view whose `needs`
differs from its data op's right, or that names an op the api lacks, does not compile (`ui-part` doctests).

Live, on the running cells, with headless Chromium (`ui/tools/prove.mjs`, the gate's signed person headers):

1. Root: the manifest lists four cells; keys show expiry, rotation state and holders; devices and audit rows update
   without a reload.
2. Operator: tabs devices and audit only; devices shows ops-1 only, with no tailnet or passkey field.
3. Not sent: every byte the operator's browser received (page, manifest, modules, socket frames) searched for
   `cr.keys`, `KeyList`, the keys bundle's sha256, key names and other nodes' names: none found. `ui.serve` Fetch of the
   keys bundle as the operator: `403 Refused`; `cr.keys List` with the operator's abilities: refused by the gate.
4. Field level: `cr.keys List` with `cr.keys:read` alone returns rows with `holders: null`; with `cr.keys:holders`, the
   holders.
5. A device that stopped answering showed offline in the open page 8 s later, without a reload.
6. Revoke: `ce revoke <nonce of the operator's grant>` with the page open; 16 s later the page answered "closed for
   you: capability revoked".
7. Types: `KeyRow.rotation` renamed in the contract; the site build failed in `cr-keys/ui/src/KeyList.ts` (and in the
   sim) until it was renamed back.
8. Integrity: the devices module delivered with one byte changed; `ui-host` refused it ("integrity: .. not run").

A trace: each `cr.audit` round is one call id in ui.trace, a root span with child spans into `audit.log` on every node
and into `keys`, so one call shows across three cells and five nodes.

## Not done in slice 1

- The gate route `apps.ce-net.com/control-room/` -> `127.0.0.1:8834` is not yet in the deployed gate: gate code moves
  only by its own deploy procedure, which its owner runs.
- leif-desk: its `cell.install` is major 2 (installs only from its own cells index or an allowlisted signed artifact)
  and it runs no `wire.ws-door`; the cells run on ops-1 only.
- `ui { }` inside `ce_wire::api!` itself (here `views!` beside it), shared types in one `cr-types-api` (ce-wire-gen reads
  only the types inside an api block), a signed manifest, ui.manifest and ui.serve as their own cells, projections
  persisted in `state.store`, the hash link rechecked in the browser (audit.log rows carry no hashes on the wire),
  `decl.lint` (its `ui/src` rules are `ui/tools/lint.mjs`), `site!`/`net!` macros, `ce site plan/apply`.
- Renewal through a passkey-approved bundle (slice 2): `keys` has no `Renew` op yet; the keys server holds sealed
  envelopes only and sealing is a client verb, so the op belongs to the keys cell and its owner.
