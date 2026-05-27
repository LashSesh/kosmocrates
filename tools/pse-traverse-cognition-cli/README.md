# pse-traverse-cognition-cli

CLI for PSE-TRAVERSE-COGNITION-01 — panoptic phase cognition kernel (inspect / observe / state5 / memory-query / lattice / puzzle / panorama / calibrate / trigger / bundle / replay / verify)

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

`pse-traverse-cognition` — CLI for the v0.1 panoptic phase cognition
kernel (PSE-TRAVERSE-COGNITION-01, §18).

Subcommands (per spec):

```text
inspect       <input.json>
observe       <input.json>   --rd <rd.json> --out <cognition.json>
state5        <input.json>   --rd <rd.json> --out <state5.json>
memory-query  <input.json>   --rd <rd.json> --out <hits.json>
lattice       <input.json>   --rd <rd.json> --out <lattice.json>
puzzle        <input.json>   --rd <rd.json> --out <puzzle.json>
panorama      <input.json>   --rd <rd.json> --out <panorama.json>
calibrate     <bench.json>   --rd <rd.json> --out <cal.json>
trigger       <input.json>   --rd <rd.json> --out <trigger.json>
bundle        <run.json>                    --out <bundle.json>
replay        <run.json>     [--rd <rd.json>]
verify        <report.json>
```

Manual flag parsing — keeps the binary dependency-free.

## Run

```bash
# From the workspace root:
cargo run --release -p pse-traverse-cognition-cli
# or, after `cargo install --path .`:
pse-traverse-cognition
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
