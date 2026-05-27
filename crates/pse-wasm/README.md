# pse-wasm

WebAssembly build of PSE — runs in any browser

`pse-wasm` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

# pse-wasm — WebAssembly build of PSE

Wraps the PSE engine with a JSON-in/JSON-out interface for browser use.
All processing happens locally in the browser — no data leaves the machine.

## Add to your project

```toml
[dependencies]
pse-wasm = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-wasm --open`
(once published, also available on [docs.rs](https://docs.rs/pse-wasm)).

## License

MIT — see [`LICENSE`](../../LICENSE).
