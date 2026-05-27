# pse-adapter-binance

PSE domain adapter for Binance cryptocurrency market data

A domain adapter for the [Kosmocrates](https://github.com/lashsesh/pse)
workspace. Implements the `ObservationAdapter` trait (defined in
[`pse-graph`](../../crates/pse-graph)) so the core engine can ingest
the corresponding data stream through the standard `macro_step` loop.

## What it does

PSE domain adapter for Binance cryptocurrency market data.

Connects to Binance public REST API (no API key needed) to fetch
kline (candlestick) data and ingest OHLCV as PSE observations.

# Example

```rust,no_run
use pse_adapter_binance::{BinanceAdapter, embedded_btc_klines};
use pse_types::Config;
use pse_core::{GlobalState, macro_step};

let config = Config::default();
let mut state = GlobalState::new(&config);
let adapter = BinanceAdapter::new("BTCUSDT");
let klines = embedded_btc_klines();

for tick in &klines {
    let batch = vec![serde_json::to_vec(tick).unwrap()];
    let _ = macro_step(&mut state, &batch, &config, &adapter);
}
```

## Add to your project

```toml
[dependencies]
pse-adapter-binance = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-adapter-binance --open`.
For the adapter contract, see [`crates/pse-graph`](../../crates/pse-graph).

## License

MIT — see [`LICENSE`](../../LICENSE).
