# pse-adapter-tabular

PSE adapter for CSV/tabular data quality analysis

A domain adapter for the [Kosmocrates](https://github.com/lashsesh/pse)
workspace. Implements the `ObservationAdapter` trait (defined in
[`pse-graph`](../../crates/pse-graph)) so the core engine can ingest
the corresponding data stream through the standard `macro_step` loop.

## What it does

PSE adapter for CSV/tabular data quality analysis.

Takes any CSV file and runs it through PSE for data quality assessment,
detecting outliers, missing value clusters, distribution shifts, and correlation breaks.

## Add to your project

```toml
[dependencies]
pse-adapter-tabular = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-adapter-tabular --open`.
For the adapter contract, see [`crates/pse-graph`](../../crates/pse-graph).

## License

MIT — see [`LICENSE`](../../LICENSE).
