# pse-eval-matrix-cli

CLI for PSE-EVAL-MATRIX-01 — init / validate / plan / run / replay / score / ablate / compare / report (binary: pse-eval-matrix)

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

`pse-eval-matrix` — CLI for the PSE-EVAL-MATRIX-01 empirical
benchmark matrix (§15).

Subcommands:

```text
init     --template <preset>         --out <eval_spec.json>
validate --spec <eval_spec.json>
plan     --spec <eval_spec.json>     --out <eval_plan.json>
run      --spec <eval_spec.json>     --plan <eval_plan.json> --out <runs_dir/>
replay   --ledger <runs/ledger.json> --reports <runs/reports.json>
score    --spec <spec.json> --ledger <ledger.json> --reports <reports.json> --out <summary.json>
ablate   --spec <eval_spec.json>     --base <variant_id>      --out <ablations.json>
compare  --summary <summary.json>    --baseline <variant_id>
report   --summary <summary.json>    --format <md|json>       [--out <path>]
```

Manual flag parsing — keeps the binary dependency-free.

## Run

```bash
# From the workspace root:
cargo run --release -p pse-eval-matrix-cli
# or, after `cargo install --path .`:
pse-eval-matrix
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
