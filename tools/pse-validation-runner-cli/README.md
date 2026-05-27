# pse-validation-runner-cli

CLI for PSE-VALIDATION-RUNNER-01 — one-button validation singularity (binary: pse-validate)

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

pse-validate — One-Button Validation Singularity CLI (PSE-VALIDATION-RUNNER-01).

Primary command:
  pse-validate run --profile full --out validation_runs/latest

Subcommands: init inspect run resume replay score report bundle verify-bundle

## Run

```bash
# From the workspace root:
cargo run --release -p pse-validation-runner-cli
# or, after `cargo install --path .`:
pse-validate
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
