# pse-swarm

Multi-agent coordination, consensus modes, PMHD-backed verification for PSE

`pse-swarm` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Multi-agent swarm coordinator for PSE.

Spawns N autonomous agents with distinct deterministic seeds, runs them
through configurable rounds, and applies a consensus policy (optionally
backed by PMHD drills) to converge on a collective result.

## Add to your project

```toml
[dependencies]
pse-swarm = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-swarm --open`
(once published, also available on [docs.rs](https://docs.rs/pse-swarm)).

## License

MIT — see [`LICENSE`](../../LICENSE).
