# pse-metatron-cli

CLI for PSE-METATRON-MONOLITH-01 — Holistic Eigenmode Closure Layer (binary: pse-metatron)

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

pse-metatron — Holistic Eigenmode Closure Layer CLI (PSE-METATRON-MONOLITH-01).

Subcommands:
  inspect        Print available artifacts in a validation run directory
  project-local  Collect local Monolith projections from artifacts
  isomorphism    Evaluate isomorphic projection reports
  spectral-gap   Evaluate spectral gap stitch report
  close          Run the full Metatron closure chain
  replay         Verify byte-identity of a closure report
  verify         Verify a holistic_eigenmode_state.json

## Run

```bash
# From the workspace root:
cargo run --release -p pse-metatron-cli
# or, after `cargo install --path .`:
pse-metatron
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
