# pse-store

Persistence layer for crystals, traces, and metrics

`pse-store` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Structured persistence layer for PSE (C17).

Supports two backends:
- **SQLite** (default `sqlite` feature): file-backed store via `IslandStore`
- **Memory-only** (`memory-only` feature): in-memory HashMap store via `MemoryStore`

Both backends implement the `CrystalStore` trait for crystal persistence.

## Add to your project

```toml
[dependencies]
pse-store = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-store --open`
(once published, also available on [docs.rs](https://docs.rs/pse-store)).

## License

MIT — see [`LICENSE`](../../LICENSE).
