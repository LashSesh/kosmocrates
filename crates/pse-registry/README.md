# pse-registry

Content-addressed catalog for operators, profiles, and obligations

`pse-registry` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Digest-bound registry infrastructure for PSE (C12).

Provides content-addressed catalogs for operators, profiles, obligations,
and macros, with integrity verification via SHA-256 digests.

## Add to your project

```toml
[dependencies]
pse-registry = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-registry --open`
(once published, also available on [docs.rs](https://docs.rs/pse-registry)).

## License

MIT — see [`LICENSE`](../../LICENSE).
