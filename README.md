# control-room

One place for the keys, devices and audit trail of a ce-net mesh, as full-stack cells on one page: a shell
(`cr.shell`, this repo) and one cell per panel (repo:cr-keys, repo:cr-devices, repo:cr-audit). Each cell has a typed
Rust contract with its view declared in it, a Rust backend, and a TypeScript view generated against that contract.
The server decides what a viewer gets: a panel they may not open is not in their manifest, its bundle is refused, a
row outside their caveats is never read, a field they may not have is absent on the wire. No cell holds a secret: key
rows are metadata only.

- Design: [docs/DESIGN.md](docs/DESIGN.md). Slice 1 as built, with its proofs: [docs/SLICE-1.md](docs/SLICE-1.md).
- Live system: `https://apps.ce-net.com/control-room/` behind the ce-net sign-in.
- Sim view with invented data only: the [sim page](https://ce-net.github.io/control-room/).

| Path | What |
|---|---|
| `crates/cr-shell-api` | `cr.shell@2` contract (`Layout`) and its `ControlRoom` view (slots keys, devices, audit); major 1 frozen |
| `crates/cr-shell` | the shell's backend and the site's gateway face (repo:ui-cells `site-gateway`), the built site inside |
| `crates/control-room-policy` | the `root` and `operator` roles, declared once; who holds which is a people file outside the repo |
| `crates/control-room-site` | the route and slot tree, from the cells' declared views; the cells pinned by commit |
| `ui/src` | `ControlRoom.ts` (the shell's view) and `host.ts` (the page) |
| `ui/tools/build.mjs` | codegen for every cell, one strict `tsc` over every view, one bundle per view named by its sha256 |
| `ui/sim` | the sim page: the same views, invented rows |

Build: `cargo test` (Rust), `cd ui && npm ci && node tools/build.mjs` (needs `ce-wire-gen` and repo:ui-cells beside
this repo). Apache-2.0.
