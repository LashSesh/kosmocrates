# pse

Kosmocrates / PSE — Kosmocrates crystallization layer: observation → fail-closed gate → SemanticCrystal

`pse` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

# PSE — Kosmocrates

A universal computation engine that processes information through
topology, physics, and geometry rather than through symbols or statistics.

## Quick Start

```rust,no_run
use pse::prelude::*;

// Implement ObservationAdapter for your domain, then:
// let config = Config::default();
// let mut state = GlobalState::new(&config);
// let adapter = PassthroughAdapter::new("my-domain");
// let result = macro_step(&mut state, &observations, &config, &adapter);
```

## Add to your project

```toml
[dependencies]
pse = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse --open`
(once published, also available on [docs.rs](https://docs.rs/pse)).

## License

MIT — see [`LICENSE`](../../LICENSE).
