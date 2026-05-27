# pse-replay

Deterministic replay engine for PSE

`pse-replay` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Deterministic replay engine for PSE.

Provides replay verification by re-executing observation sequences
and comparing crystal outputs against recorded manifests.

## Add to your project

```toml
[dependencies]
pse-replay = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-replay --open`
(once published, also available on [docs.rs](https://docs.rs/pse-replay)).

## License

MIT — see [`LICENSE`](../../LICENSE).
