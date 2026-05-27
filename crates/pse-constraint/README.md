# pse-constraint

Morphogenic graph transformations, DoF analysis, constraint propagation for PSE

`pse-constraint` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Morphogenic graph transformations (Layer L4).

Applies structural mutations (node split, merge, edge retype, subgraph
replication) and intrinsic time-step evolution to the persistent graph.

## Add to your project

```toml
[dependencies]
pse-constraint = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-constraint --open`
(once published, also available on [docs.rs](https://docs.rs/pse-constraint)).

## License

MIT — see [`LICENSE`](../../LICENSE).
