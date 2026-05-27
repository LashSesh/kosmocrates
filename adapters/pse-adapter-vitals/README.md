# pse-adapter-vitals

PSE domain adapter for heartbeat and vital signs monitoring

A domain adapter for the [Kosmocrates](https://github.com/lashsesh/pse)
workspace. Implements the `ObservationAdapter` trait (defined in
[`pse-graph`](../../crates/pse-graph)) so the core engine can ingest
the corresponding data stream through the standard `macro_step` loop.

## What it does

PSE domain adapter for heartbeat and vital signs monitoring.

Generates synthetic ECG-like signals to detect cardiac rhythm anomalies.

**DISCLAIMER: FOR DEMONSTRATION PURPOSES ONLY. NOT CLINICALLY VALIDATED.
NOT A MEDICAL DEVICE. DO NOT USE FOR DIAGNOSTIC OR TREATMENT DECISIONS.**

## Add to your project

```toml
[dependencies]
pse-adapter-vitals = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-adapter-vitals --open`.
For the adapter contract, see [`crates/pse-graph`](../../crates/pse-graph).

## License

MIT — see [`LICENSE`](../../LICENSE).
