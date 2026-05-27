# pse-adapter-airquality

PSE domain adapter for OpenAQ air quality data

A domain adapter for the [Kosmocrates](https://github.com/lashsesh/pse)
workspace. Implements the `ObservationAdapter` trait (defined in
[`pse-graph`](../../crates/pse-graph)) so the core engine can ingest
the corresponding data stream through the standard `macro_step` loop.

## What it does

PSE domain adapter for OpenAQ air quality data.

Ingests air quality readings from OpenAQ monitoring stations and feeds
them through the PSE pipeline as observations. Includes embedded
synthetic data for five German monitoring stations covering 48 hours
with a realistic industrial spike event.

# Example

```rust,no_run
use pse_adapter_airquality::{AirQualityAdapter, embedded_airquality_data};
use pse_types::Config;
use pse_core::{GlobalState, macro_step};

let config = Config::default();
let mut state = GlobalState::new(&config);
let adapter = AirQualityAdapter::new(1001);
let readings = embedded_airquality_data();

for reading in &readings {
    let batch = vec![serde_json::to_vec(reading).unwrap()];
    let _ = macro_step(&mut state, &batch, &config, &adapter);
}
```

## Add to your project

```toml
[dependencies]
pse-adapter-airquality = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-adapter-airquality --open`.
For the adapter contract, see [`crates/pse-graph`](../../crates/pse-graph).

## License

MIT — see [`LICENSE`](../../LICENSE).
