# pse-nxalien-pse

ObservationAdapter and PSE bridge handoff candidates for nxalien

`pse-nxalien-pse` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

PSE-bridge integration for nxalien.

Implements `pse_graph::ObservationAdapter` so that a compiled
NxAlienBundle enters the PSE stack as a first-class Observation —
the same pathway used by every other PSE adapter (binance, weather,
seismo, …).

Invariant I-BRIDGE-001: this crate never constructs SemanticCrystal
directly.  It produces Observations; crystal formation is PSE's job.

## Add to your project

```toml
[dependencies]
pse-nxalien-pse = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-nxalien-pse --open`
(once published, also available on [docs.rs](https://docs.rs/pse-nxalien-pse)).

## License

MIT — see [`LICENSE`](../../LICENSE).
