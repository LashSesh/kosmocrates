# pse-adapter-seismo

PSE domain adapter for USGS earthquake seismology data

A domain adapter for the [Kosmocrates](https://github.com/lashsesh/pse)
workspace. Implements the `ObservationAdapter` trait (defined in
[`pse-graph`](../../crates/pse-graph)) so the core engine can ingest
the corresponding data stream through the standard `macro_step` loop.

## What it does

PSE domain adapter for USGS earthquake seismology data.

Connects to the USGS Earthquake Hazards Program GeoJSON API to fetch
real-time and historical seismic event data and ingest it as PSE
observations.

# Example

```rust,no_run
use pse_adapter_seismo::{SeismoAdapter, embedded_seismo_data};
use pse_types::Config;
use pse_core::{GlobalState, macro_step};

let config = Config::default();
let mut state = GlobalState::new(&config);
let adapter = SeismoAdapter::new("pacific_rim");
let events = embedded_seismo_data();

for event in &events {
    let batch = vec![serde_json::to_vec(event).unwrap()];
    let _ = macro_step(&mut state, &batch, &config, &adapter);
}
```

## Add to your project

```toml
[dependencies]
pse-adapter-seismo = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-adapter-seismo --open`.
For the adapter contract, see [`crates/pse-graph`](../../crates/pse-graph).

## License

MIT — see [`LICENSE`](../../LICENSE).
