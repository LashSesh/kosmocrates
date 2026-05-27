# pse-adapter-weather

PSE domain adapter for Open-Meteo weather data

A domain adapter for the [Kosmocrates](https://github.com/lashsesh/pse)
workspace. Implements the `ObservationAdapter` trait (defined in
[`pse-graph`](../../crates/pse-graph)) so the core engine can ingest
the corresponding data stream through the standard `macro_step` loop.

## What it does

PSE domain adapter for Open-Meteo weather data.

Ingests hourly weather observations from Open-Meteo's free API and
crystallizes meteorological patterns through the PSE pipeline.

# Example

```rust,no_run
use pse_adapter_weather::{WeatherAdapter, embedded_weather_data};
use pse_types::Config;
use pse_core::{GlobalState, macro_step};

let config = Config::default();
let mut state = GlobalState::new(&config);
let adapter = WeatherAdapter::new("berlin");
let readings = embedded_weather_data();

for batch in readings.chunks(10) {
    let obs: Vec<Vec<u8>> = batch.iter()
        .filter_map(|r| serde_json::to_vec(r).ok())
        .collect();
    let _ = macro_step(&mut state, &obs, &config, &adapter);
}
```

## Add to your project

```toml
[dependencies]
pse-adapter-weather = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-adapter-weather --open`.
For the adapter contract, see [`crates/pse-graph`](../../crates/pse-graph).

## License

MIT — see [`LICENSE`](../../LICENSE).
