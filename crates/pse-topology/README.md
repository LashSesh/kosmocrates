# pse-topology

Laplacian computation, spectral decomposition, Fiedler vector, Betti numbers for PSE

`pse-topology` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Topological-spectral orbit core for PSE (C16).

Spectral graph analysis, CTQW propagation, Kuramoto synchronization,
DTL predicates, fixpoint detection, and observation deduplication.

## Add to your project

```toml
[dependencies]
pse-topology = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-topology --open`
(once published, also available on [docs.rs](https://docs.rs/pse-topology)).

## License

MIT — see [`LICENSE`](../../LICENSE).
