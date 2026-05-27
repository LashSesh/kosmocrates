# pse-memory

Persistent pattern memory — topological similarity index for cross-session learning

`pse-memory` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

# pse-memory — Persistent Pattern Memory

Loads crystals from prior sessions, builds a topological similarity index,
and provides fast lookup for incoming patterns against known crystals.
This is what makes PSE learn across sessions.

The memory index uses spectral/topological features extracted from each
crystal's `TopologySignature` and computes cosine similarity weighted
with resonance and confidence proximity.

## Add to your project

```toml
[dependencies]
pse-memory = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-memory --open`
(once published, also available on [docs.rs](https://docs.rs/pse-memory)).

## License

MIT — see [`LICENSE`](../../LICENSE).
