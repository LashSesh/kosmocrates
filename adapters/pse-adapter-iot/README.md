# pse-adapter-iot

PSE domain adapter for IoT predictive maintenance sensor data

A domain adapter for the [Kosmocrates](https://github.com/lashsesh/pse)
workspace. Implements the `ObservationAdapter` trait (defined in
[`pse-graph`](../../crates/pse-graph)) so the core engine can ingest
the corresponding data stream through the standard `macro_step` loop.

## What it does

PSE domain adapter for IoT predictive maintenance sensor data.

Generates and processes industrial sensor readings (vibration, temperature,
pressure, current, RPM, oil viscosity) to detect equipment degradation patterns.

## Add to your project

```toml
[dependencies]
pse-adapter-iot = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-adapter-iot --open`.
For the adapter contract, see [`crates/pse-graph`](../../crates/pse-graph).

## License

MIT — see [`LICENSE`](../../LICENSE).
