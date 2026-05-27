# pse-nxalien-cube

Hypercube-HDAG embedding and compiler for nxalien

`pse-nxalien-cube` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Hypercube-HDAG embedding and compiler for nxalien.

Builds a directed acyclic graph over C^8 from a set of rules, unknowns, and
project metadata.  Cycle-forming edges are silently dropped (recorded as
advisory notes).

## Add to your project

```toml
[dependencies]
pse-nxalien-cube = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-nxalien-cube --open`
(once published, also available on [docs.rs](https://docs.rs/pse-nxalien-cube)).

## License

MIT — see [`LICENSE`](../../LICENSE).
