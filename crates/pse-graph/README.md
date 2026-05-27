# pse-graph

Observation graph, entity tracking, and persistent hash-DAG for PSE

`pse-graph` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Observation graph, entity tracking, and persistent hash-DAG for PSE.

Combines the observation ingestion layer (L0) with the persistent
graph storage layer (L1).

## Add to your project

```toml
[dependencies]
pse-graph = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-graph --open`
(once published, also available on [docs.rs](https://docs.rs/pse-graph)).

## License

MIT — see [`LICENSE`](../../LICENSE).
