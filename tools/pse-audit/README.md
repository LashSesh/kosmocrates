# pse-audit

PSE audit pipeline — evidence chain verification and compliance reporting

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

PSE audit pipeline — evidence chain verification and compliance reporting.

Reads crystal archives and produces audit-ready documentation including
evidence trail verification, integrity checks, and EU AI Act compliance mapping.

## Run

```bash
# From the workspace root:
cargo run --release -p pse-audit
# or, after `cargo install --path .`:
pse-audit
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
