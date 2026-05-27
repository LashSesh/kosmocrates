# pse-traverse-cli

CLI for the PSE Traversal Agent — inspect / plan / run / replay

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

`pse-traverse-cli` — minimal CLI for the PSE Traversal Agent.

Subcommands:

```text
inspect  --problem PATH                       → JSON dump of FieldCube + DoFGraph + excisions
plan     --problem PATH [--out FILE]          → CollapsePlan as canonical JSON
         [--signature]                        → also derive operator + signature + diagnostics
run      --problem PATH [--out FILE]          → TraversalRunReport (incl. PSE bridge attempt)
         [--signature-gate]                   → attach SignatureGate diagnostic channel
replay   --run FILE                           → re-derive plan + assert byte-identity
search   --problem PATH [--n NUM] [--out FILE]
                                              → generate traversal blueprints
```

Manual flag parsing — no `clap` dependency is added at this stage.

## Run

```bash
# From the workspace root:
cargo run --release -p pse-traverse-cli
# or, after `cargo install --path .`:
pse-traverse-cli
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
