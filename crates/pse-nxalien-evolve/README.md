# pse-nxalien-evolve

Attractor-constrained rule evolution: closes the PSE→nxalien feedback loop

`pse-nxalien-evolve` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Attractor-constrained rule evolution for nxalien.

Closes the PSE → nxalien feedback loop:

  nxalien bundle → PSE Observation → PersistentGraph → attractor centroid
        ↓                                                      ↓
  next compile ← validated rule proposals ← EpistemicSignal ←┘

Rules evolve toward the free-energy minimum of the knowledge field —
not arbitrarily, but constrained by PSE's own attractor dynamics.
An EvolutionGuard prevents unbounded drift by requiring attractor
alignment above a configurable threshold.

## Add to your project

```toml
[dependencies]
pse-nxalien-evolve = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-nxalien-evolve --open`
(once published, also available on [docs.rs](https://docs.rs/pse-nxalien-evolve)).

## License

MIT — see [`LICENSE`](../../LICENSE).
