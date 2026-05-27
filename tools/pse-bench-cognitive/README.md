# pse-bench-cognitive

Cognitive-substrate benchmark — validates the PSE-TRAVERSE-COGNITION-01 pipeline on reasoning-trajectory scenarios

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

PSE Cognitive Substrate Benchmark

Validates PSE-TRAVERSE-COGNITION-01 on reasoning-trajectory scenarios.
Unlike bench-gt (which tests macro_step on sensor streams), this tool
feeds structured CognitionInput sequences directly to run_cognition()
and measures whether the pipeline correctly classifies cognitive states.

Scenarios:
  phase_transition  — exploratory → convergent trajectory
  deadlock          — contradictory constraints, should Hold indefinitely
  memory_recall     — state close to a stored memory triggers recall path
  singularity       — degenerate stability drives SingularityTrigger

## Run

```bash
# From the workspace root:
cargo run --release -p pse-bench-cognitive
# or, after `cargo install --path .`:
bench_cognitive
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
