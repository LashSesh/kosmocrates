# pse-extract

Pattern extraction, invariance detection, and constraint discovery for PSE

`pse-extract` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Constraint extraction from observations (Layer L2).

Reads the persistent graph to discover constraint candidates and weave
them into constraint programs using configurable operator libraries.

## Add to your project

```toml
[dependencies]
pse-extract = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-extract --open`
(once published, also available on [docs.rs](https://docs.rs/pse-extract)).

## License

MIT — see [`LICENSE`](../../LICENSE).
