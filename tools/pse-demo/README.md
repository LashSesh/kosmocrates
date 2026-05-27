# pse-demo

30-second showcase: streaming workload, crystal formation rate, gate diagnostics

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

`pse-demo` — 30-second runnable showcase of the Kosmocrates.

Drives a synthetic structured stream (a damped oscillator with a
mid-stream regime shift) through PSE's full pipeline using a sliding
window. Reports throughput, crystal formation rate, **bottleneck gate**
(which Kairos threshold blocks the most ticks), and the SHA-256 of the
first crystal — so a fresh user can see the engine produce real
verifiable artifacts in under a minute.

Usage:

```bash
cargo run --release -p pse-demo
RUST_LOG=pse_core=debug cargo run --release -p pse-demo  # gate diagnostics
```

## Run

```bash
# From the workspace root:
cargo run --release -p pse-demo
# or, after `cargo install --path .`:
pse-demo
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
