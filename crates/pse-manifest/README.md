# pse-manifest

Manifest construction and verification for PSE

`pse-manifest` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Execution manifest for PSE (C13).

Produces a content-addressed meta-artifact that binds an entire run,
linking crystals, traces, registries, and evidence into a single verifiable envelope.

## Add to your project

```toml
[dependencies]
pse-manifest = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-manifest --open`
(once published, also available on [docs.rs](https://docs.rs/pse-manifest)).

## License

MIT — see [`LICENSE`](../../LICENSE).
