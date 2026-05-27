# pse-bench-gt

Ground-truth precision/recall benchmark suite for the Kosmocrates

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

Ground-truth precision/recall benchmark suite for the Kosmocrates.

This crate provides the *measurement apparatus* for Research Strand A.
It is independent of any domain adapter or detector: a detector produces
a stream of [`Detection`]s, a scenario carries a stream of
[`GroundTruthEvent`]s, and [`score_detections`] turns the pair into a
[`Metrics`] record with precision, recall, F1 and AUPRC.

The scorer is the pure-math core of the suite. Domain scenarios,
PSE/baseline runners, and CLI bench binaries are added in later
increments and consume this module unchanged.

## Run

```bash
# From the workspace root:
cargo run --release -p pse-bench-gt
# or, after `cargo install --path .`:
bench_gt
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
