# pse-adapter-entsoe

PSE domain adapter for ENTSO-E European energy grid data

A domain adapter for the [Kosmocrates](https://github.com/lashsesh/pse)
workspace. Implements the `ObservationAdapter` trait (defined in
[`pse-graph`](../../crates/pse-graph)) so the core engine can ingest
the corresponding data stream through the standard `macro_step` loop.

## What it does

PSE domain adapter for ENTSO-E European energy grid data.

Reads ENTSO-E Transparency Platform CSV data and ingests generation,
load, and cross-border flow data as PSE observations.

# Example

```rust
use pse_adapter_entsoe::{GridAdapter, embedded_grid_data};
use pse_types::Config;
use pse_core::{GlobalState, macro_step};

let config = Config::default();
let mut state = GlobalState::new(&config);
let adapter = GridAdapter::new("DE_LU");
let observations = embedded_grid_data();

for obs in &observations {
    let batch = vec![serde_json::to_vec(obs).unwrap()];
    let _ = macro_step(&mut state, &batch, &config, &adapter);
}
```

## Add to your project

```toml
[dependencies]
pse-adapter-entsoe = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-adapter-entsoe --open`.
For the adapter contract, see [`crates/pse-graph`](../../crates/pse-graph).

## License

MIT — see [`LICENSE`](../../LICENSE).
