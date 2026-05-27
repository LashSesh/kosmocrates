# pse-evidence

Evidence chain construction, SHA-256 linking, crystal archival for PSE

`pse-evidence` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Crystal archival and evidence-chain verification.

Builds semantic crystals with hash-chained evidence entries and provides
replay-based verification of content addresses, operator drift, and consensus.

## Add to your project

```toml
[dependencies]
pse-evidence = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-evidence --open`
(once published, also available on [docs.rs](https://docs.rs/pse-evidence)).

## License

MIT — see [`LICENSE`](../../LICENSE).
