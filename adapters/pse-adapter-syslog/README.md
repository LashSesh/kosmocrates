# pse-adapter-syslog

PSE domain adapter for syslog anomaly detection

A domain adapter for the [Kosmocrates](https://github.com/lashsesh/pse)
workspace. Implements the `ObservationAdapter` trait (defined in
[`pse-graph`](../../crates/pse-graph)) so the core engine can ingest
the corresponding data stream through the standard `macro_step` loop.

## What it does

PSE domain adapter for syslog anomaly detection.

Processes server log entries to detect security and performance anomalies
including DDoS onset, brute force attempts, and service degradation.

## Add to your project

```toml
[dependencies]
pse-adapter-syslog = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-adapter-syslog --open`.
For the adapter contract, see [`crates/pse-graph`](../../crates/pse-graph).

## License

MIT — see [`LICENSE`](../../LICENSE).
