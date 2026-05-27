# pse-adapter-modelmon

PSE adapter for ML model monitoring and drift detection

A domain adapter for the [Kosmocrates](https://github.com/lashsesh/pse)
workspace. Implements the `ObservationAdapter` trait (defined in
[`pse-graph`](../../crates/pse-graph)) so the core engine can ingest
the corresponding data stream through the standard `macro_step` loop.

## What it does

PSE adapter for ML model monitoring and drift detection.

Monitors inference events to detect input drift, confidence degradation,
latency anomalies, and accuracy drops.

## Add to your project

```toml
[dependencies]
pse-adapter-modelmon = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-adapter-modelmon --open`.
For the adapter contract, see [`crates/pse-graph`](../../crates/pse-graph).

## License

MIT — see [`LICENSE`](../../LICENSE).
