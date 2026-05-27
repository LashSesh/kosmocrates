# pse-traverse-topology-cli

CLI for the PSE-TRAVERSE-TPT-MTL-04 topology layer

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

`pse-traverse-topology` — CLI for the TPT-MTL-04 topology layer.

Subcommands (TPT-MTL §14 Reference CLI):

```text
inspect       <state.json>                              → JSON digest summary
window        --state <state.json> --rd <rd.json> --out <window.json>
axis          --window <window.json> --out <axis_report.json>
triangulate   --window <window.json> --out <mesh_holo.json>
lift          --window <window.json> --out <micro_fibers.json>
weight        --mesh <mesh.json> --fibers <fibers.json> --out <weighted.json>
refine        --mesh <mesh.json> --depth <N> --out <refined.json>
carrier       --mesh <mesh.json> --window <window.json> --out <carrier.json>
reinterpret   --mesh <mesh.json> --carrier <carrier.json> --out <reinterp.json>
gate          --window <w.json> --mesh <m.json> --fibers <f.json>
              --reinterpretation <r.json> --out <gate.json>
bundle        --gate <gate.json> --out <bundle.json>
replay        --bundle <bundle.json> --expect same-digest
verify        --bundle <bundle.json>
run           --state <state.json> --rd <rd.json> --out <bundle.json>   (full pipeline)
```

## Run

```bash
# From the workspace root:
cargo run --release -p pse-traverse-topology-cli
# or, after `cargo install --path .`:
pse-traverse-topology
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
