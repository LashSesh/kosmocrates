# pse-nxalien-core

Canonicalization, hashing, project scanner, and gate evaluation for nxalien

`pse-nxalien-core` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Core logic for the nxalien agent-context exoskeleton.

Provides canonicalization/hashing (`canon`), project scanning (`scanner`),
and 8-gate evaluation (`gate`).

## Add to your project

```toml
[dependencies]
pse-nxalien-core = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-nxalien-core --open`
(once published, also available on [docs.rs](https://docs.rs/pse-nxalien-core)).

## License

MIT — see [`LICENSE`](../../LICENSE).
