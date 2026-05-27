# pse-scheduler

Tick-based scheduling and macro-step orchestration for PSE

`pse-scheduler` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Spiral scheduler for PSE (C15).

Provides adaptive tick granularity that adjusts sub-step counts based on
system pressure metrics (drift, flux, synchronization).

## Add to your project

```toml
[dependencies]
pse-scheduler = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-scheduler --open`
(once published, also available on [docs.rs](https://docs.rs/pse-scheduler)).

## License

MIT — see [`LICENSE`](../../LICENSE).
