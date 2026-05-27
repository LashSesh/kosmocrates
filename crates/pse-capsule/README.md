# pse-capsule

AES-256-GCM encryption, HKDF key derivation, policy-gated encapsulation for PSE

`pse-capsule` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Operator lock protocol for PSE (C14).

Evidence-bound secret encapsulation using AES-256-GCM, with policy-gated
seal/open operations, expiration, and use-count limits.

## Add to your project

```toml
[dependencies]
pse-capsule = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-capsule --open`
(once published, also available on [docs.rs](https://docs.rs/pse-capsule)).

## License

MIT — see [`LICENSE`](../../LICENSE).
