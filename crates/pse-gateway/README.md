# pse-gateway

HTTP API exposing universal PSE endpoints

`pse-gateway` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

PSE Gateway — REST API for universal PSE endpoints.

Exposes: /health, /crystals, /observe, /navigate, /constitution, /benchmarks, /accumulation.

## Add to your project

```toml
[dependencies]
pse-gateway = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-gateway --open`
(once published, also available on [docs.rs](https://docs.rs/pse-gateway)).

## License

MIT — see [`LICENSE`](../../LICENSE).
