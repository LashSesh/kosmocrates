# Kosmocrates — Node.js binding

The Node.js binding is a thin CommonJS wrapper around the same WASM
build that powers the browser binding. It exposes the deterministic
PSE crystallization engine to any Node ≥ 18 process — no native build
toolchain required at install time.

> **Package:** [`@kosmocrates/pse-wasm-node`](https://www.npmjs.com/package/@kosmocrates/pse-wasm-node)
> **Source:** [`crates/pse-wasm/`](../../crates/pse-wasm/) (compiled with
> `wasm-pack --target nodejs`)
> **Sibling:** [`@kosmocrates/pse-wasm-web`](https://www.npmjs.com/package/@kosmocrates/pse-wasm-web)
> for browser / bundler consumption.

## Install

Once the first npm release lands:

```bash
npm install @kosmocrates/pse-wasm-node
```

Until then, build locally from the workspace root:

```bash
# Requires: Rust stable, wasm-pack (curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh)
wasm-pack build crates/pse-wasm --target nodejs --release \
  --out-dir ../../bindings/node/pkg --scope kosmocrates

# Use from a Node script
cd bindings/node
node examples/hello.mjs
```

## Hello world

```javascript
import { PseWasm } from '@kosmocrates/pse-wasm-node';

const pse = new PseWasm();
pse.run(100);                                // run 100 deterministic ticks
console.log(pse.status());                   // { observations, crystals, hit_rate, ... }
console.log(JSON.parse(pse.crystals())[0]);  // first crystal as a JS object
```

See [`examples/hello.mjs`](examples/hello.mjs) for a complete
copy-pasteable script (CSV ingest → run → crystal inspection).

## API

The full WASM API surface is the `PseWasm` class. Every method returns
a JSON string — parse it with `JSON.parse` on the Node side. Methods:

| Method | Returns | Description |
|---|---|---|
| `new PseWasm()` | `PseWasm` | Create a new deterministic engine instance |
| `ingest_csv(csv: string)` | JSON | Ingest a CSV blob; returns `{ rows_ingested, columns, entities, column_stats }` |
| `run(ticks: number)` | JSON | Run N ticks; returns `{ ticks_run, new_crystals, memory_hits, time_ms }` |
| `crystals()` | JSON | All crystallised topologies as a JSON array |
| `quality_report(csv: string)` | JSON | Anomaly / drift / column-stats report on a CSV blob |
| `status()` | JSON | `{ observations, crystals, memory_size, hit_rate }` |
| `accumulation_curve()` | JSON | Time-series of `{ tick, total_crystals, memory_hits }` |
| `reset_observations()` | `void` | Clear the active graph; keep pattern memory |
| `reset_all()` | `void` | Full reset including pattern memory |

The canonical reference for what each field means is the Rust
crate documentation — `cargo doc -p pse-wasm --open`.

## Determinism

The binding inherits the engine's content-addressing and replay
contracts intact. Two `PseWasm` instances fed the same sequence of
inputs produce the same crystal IDs, hit rates, and accumulation
curves. This is verified at the Rust layer by
[`tests/replay_byte_identity.rs`](../../tests/integration/) and holds
through the WASM compilation path.

## Why WASM and not native N-API?

The decision is documented in [`bindings/README.md`](../README.md):
the WASM binding covers every deterministic guarantee the engine
provides, ships a single artifact per platform, and avoids a separate
native build toolchain in `npm install`. If you have a workload where
the WASM ↔ JS marshalling overhead is the bottleneck, please open an
issue with a profile — we will reconsider.
