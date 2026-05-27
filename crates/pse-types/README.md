# pse-types

Core shared types for PSE: crystals, observations, temporal primitives

`pse-types` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Canonical data model for PSE (Kosmocrates).

Defines the shared types, temporal primitives, 5D state representations,
and content-addressed hashing used by all other PSE crates.

## Add to your project

```toml
[dependencies]
pse-types = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-types --open`
(once published, also available on [docs.rs](https://docs.rs/pse-types)).

## License

MIT — see [`LICENSE`](../../LICENSE).
