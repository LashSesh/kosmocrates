# Doors — the self-describing docking surface

The design law: **every function gets its own addressed door; a chat
window is one door among many, never the door.** The docking surface of
the whole stack is meant to spread — vertically, organ by organ — until a
surface skin (CLI, browser, GUI) is nothing but a projection of the
catalog underneath.

That requires the operator's standing question — *what, exactly, could I
operate right now?* — to have a machine answer that cannot rot. A
hand-written inventory drifts the day it is written. So the system speaks
for itself.

## The vocabulary

`kosmo_core::doors`: a [`Door`] is one operator-facing entry point — its
surface (CLI flag of a binary, HTTP route of a server), its name and
aliases, one summary sentence, its inputs (companion flags / request
fields, with value shapes and required-ness), its **write power as data**
(`read-only` / `writes-workspace` / `appends-store` / `governance-act`)
and its needs (provider, cargo, network, workspace, store, file). Doors
are content-addressed; a [`DoorCatalog`] is the deterministic, deduped,
content-addressed inventory of one surface — and catalogs from several
surfaces merge.

## Surfaces describe themselves

- `kosmo-run --doors` (add `--json` for the machine shape): the binary's
  doors, spoken from inside the binary.
- `GET /api/doors` on kosmo-server: the server's routes, spoken by the
  server (and rendered by the browser panel's *Doors* card).
- `GET /doors` on pse-server: the cognition server's routes — keyless
  like every self-description (`/doors` is a probe path beside `/health`
  and `/ready`; all other routes honor `PSE_SERVER_TOKEN`).

A surface only ever catalogs **its own** doors. No binary claims truth
about another binary's flags — that would be the hand-written inventory
in disguise.

## Federation: the ecosystem inventory

Catalogs travel as content-addressed JSON artifacts and merge:

```sh
curl -s localhost:7777/api/doors > kosmo-server.json
curl -s localhost:8765/doors     > pse-server.json
kosmo-run --doors-merge kosmo-server.json,pse-server.json
```

`--doors-merge` unites the binary's own catalog with every named file
into one deterministic, deduplicated, content-addressed inventory — the
whole stack's docking surface in one artifact. Trust is mathematical and
fail-closed: every door's identity and every catalog's identity must
**recompute** from the visible content (`DoorCatalog::verify`); a
tampered file is refused by name. No surface needs to reach another at
runtime — federation is artifact-shaped, not network-shaped.

## The pin: the description cannot drift

Each self-describing surface carries a test that scans its **own parser
or router source** and asserts set-equality with its catalog:

- kosmo-run: every `"--flag"` match arm in `parse_args` must be spoken by
  the catalog (as a door, alias or input), and everything the catalog
  speaks must be parsed. A new flag without a description fails the
  build; a described flag without a mechanism fails the build.
- kosmo-server: every `.route(...)` registration must appear in the
  catalog with the right method, and vice versa.

The catalog is deterministic (same binary ⇒ byte-identical answer,
pinned e2e), needs no workspace, no key, no network.

## The self-renewing harvest

`scripts/harvest-doors.sh` turns federation from a manual act into a
standing one. Every door-speaking binary emits its own catalog without
being started or curled — CLIs via a `doors` subcommand, the rest
(including both servers) via a `--doors` flag that prints the catalog and
exits before doing anything else. The script collects all of them and
runs `kosmo-run --doors-merge`, which verifies each by content address
and writes one federated inventory whose catalog id is deterministic
given the surfaces present.

`.github/workflows/doors.yml` runs that harvest nightly (and on any push
that could move the surface), publishing the inventory as a build
artifact and a job-summary table — the operator's standing answer to
"what, exactly, could I operate right now?", kept fresh without anyone
hand-writing it. The harvest **fails closed**: if a surface stops
answering or a catalog no longer verifies, the job goes red.

## Coverage today

Eighteen surfaces self-describe into one inventory of **189 doors**,
with the whole stack's write power summarized in one place (read-only,
appends-store, writes-workspace, governance-act). The operator CLIs and
both servers, the substrate organ doors, the cognition layer
(pse, pse-metatron, the five traverse CLIs, pse-eval-matrix, pse-validate,
phase-matrix, nxalien) and the benchmark all speak.

Honest residue, named rather than faked: the vendored `mef` CLI (under
`vendors/infinityledger`, not modified so it survives re-vendoring), the
WASM and Python **bindings** (library surfaces, not operator CLIs), and
the eight PSE extraction operators (library-captive; a door needs
graph-region input design). These join when their shape genuinely fits a
door, not before.

## Where this goes

A GUI built from here is generated *from* the inventory — the skin
collapses out of the docking surface instead of being invented in front
of it. The catalog is the contract; the harvest keeps it true.
