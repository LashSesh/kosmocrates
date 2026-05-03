# PSE — Post-Symbolic Engine

A streaming computation engine that detects structural events in observation
streams and emits **content-addressed, falsifiable, deterministic crystal
artifacts** — without symbols, statistics, or trained models.

PSE is single-binary, single-thread-fast, audit-grade by construction.

---

## What it does

You feed PSE a stream of observations. PSE projects each observation onto
a topological substrate (a 5D-embedded graph), tests whether the current
configuration resonates with a carrier helix-pair (Mandorla coherence κ),
gates the test through eight conjunctive metrics (Kairos), runs a dual
cascade-consensus, optionally falsifies against surrogate streams, and
— when a configuration survives all of that — emits a **Crystal**: a
content-addressed (SHA-256 / JCS) record of *which graph region produced
the resonance, under which constraints, with which provenance*.

Crystals are byte-identical across replays, verifiable against the
EU AI Act compliance proof in [`docs/COMPLIANCE.md`](docs/COMPLIANCE.md),
accumulate across sessions via the pattern-memory index, and compose
through an algebra of operators (`compose / dual / bridge / query /
interpolate`).

For *what kind of computation this is* and *why it's a category of its
own*, see **[docs/POST_SYMBOLIC.md](docs/POST_SYMBOLIC.md)**.

---

## Status

| Aspect | State |
|---|---|
| Engine architecture (Strands E–N) | Complete |
| Operator algebra (compose/dual/bridge/query/interpolate) | Complete |
| Falsification (Shuffle, BlockBootstrap, PhaseRandomize) | Complete |
| EU AI Act compliance proof | Drafted |
| Throughput on commodity hardware | Verified |
| Adaptive Kairos calibration | Opt-in, working |
| Diagnostic surface (`state.last_gate`, `pse-demo`) | Live |
| Calibration on real production data | **Open frontier** |

Verified throughput, single-thread, release build, Xeon @ 2.10 GHz:

| Bench | Value |
|---|---|
| `B01a` observe-only ingest | up to **2.07 M obs/sec** |
| `B01b` full pipeline (gate path) | up to **659 K obs/sec** |
| `B15` `macro_step` end-to-end | **43–110 µs** |
| `B05` determinism check | **PASS** (bit-identical replay) |
| Workspace test suite | **467 / 467** passing |

The original i3 dual-core baseline of 655 K obs/sec is exceeded on observe
and matched on the full pipeline. See `cargo run --release --example
bench_full -p pse-core`.

The engine produces **0 crystals on default thresholds** for unconfigured
synthetic workloads — by design. Crystal formation requires either real
data the metrics were calibrated for, or the opt-in adaptive calibrator
(see `pse-demo`). Calibration on a real domain is the work that turns
this from "well-built engine" into "deployed product".

---

## Quick start

```bash
# Build the workspace
cargo build --release

# Run the 30-second demo (synthetic stream, full diagnostics)
cargo run --release -p pse-demo

# Run the full benchmark suite
cargo run --release --example bench_full -p pse-core

# Run the ground-truth benchmark (PSE vs STL-z-score vs Isolation Forest)
cargo run --release -p pse-bench-gt --bin bench_gt

# Inspect kairos rejections in real time
RUST_LOG=pse_core=debug cargo run --release -p pse-demo
```

Embed PSE in your own program:

```rust
use pse_core::{macro_step, GlobalState};
use pse_core::adaptive::AdaptiveCalibrator;
use pse_graph::PassthroughAdapter;
use pse_types::Config;

let config = Config::default();
let mut state = GlobalState::new(&config);

// Optional: self-calibrating Kairos thresholds.
state.adaptive = Some(AdaptiveCalibrator::new(0.05, 200, 100));

let adapter = PassthroughAdapter::new("my_source");
let batch: Vec<Vec<u8>> = vec![serde_json::to_vec(&my_event)?];

if let Ok(Some(crystal)) = macro_step(&mut state, &batch, &config, &adapter) {
    // crystal.crystal_id is the SHA-256 content address
    // crystal.region is the set of graph vertices that produced the resonance
    // crystal.commit_proof carries the falsification p-value (if enabled)
    println!("crystal: {}", hex::encode(crystal.crystal_id));
}

// state.last_gate carries the full GateSnapshot (all 8 metrics) for every tick,
// pass or fail — read it to diagnose why a tick did or didn't crystallize.
```

---

## Architecture

The workspace ships **24 crates**, **10 domain adapters**, **4 tool
binaries**:

```
crates/
  pse-types       Schema (Observation, Crystal, GateSnapshot, …)
  pse-graph       Persistent graph + Observation → vertex projection
  pse-extract     Constraint program / inverse-weave / DoF analysis
  pse-cascade     Mandorla, helix-pair, cascade operators (DK/SW/PI/WT)
  pse-evidence    Crystal construction, evidence chain, content address
  pse-replay      Deterministic replay & verification
  pse-constraint  Intrinsic step, morphogenic update
  pse-registry    Crystal registry / lookup
  pse-manifest    Run descriptors / provenance
  pse-capsule     AES-256-GCM sealed transport (counter-reuse detector)
  pse-scheduler   Tick orchestration
  pse-topology    Laplacian, Fiedler, Betti, spectral gap
  pse-store       Persistent crystal store
  pse-scale       Multi-scale state (Micro/Meso/Macro)
  pse-pmhd        Path-Minimum Hierarchical Decomposition
  pse-navigator   TRITON spiral / SimplexMesh / singularity scan
  pse-swarm       Multi-agent crystal propagation
  pse-memory      Pattern-memory index (cross-session)
  pse-net         Network transport (feature-gated)
  pse-wasm        WebAssembly bindings
  pse-gateway     HTTP gateway
  pse-cli         CLI front-end
  pse             Meta-crate
  pse-core        Engine orchestrator (`macro_step`), DomainAdapter trait,
                  AdaptiveCalibrator, operator algebra, falsifier

adapters/
  pse-adapter-binance     Crypto markets (Binance OHLCV)
  pse-adapter-vitals      Medical (ECG / vital signs)
  pse-adapter-seismo      Seismology (USGS earthquakes)
  pse-adapter-weather     Weather (Open-Meteo)
  pse-adapter-airquality  OpenAQ
  pse-adapter-entsoe      Energy grid (ENTSO-E)
  pse-adapter-iot         Predictive maintenance
  pse-adapter-syslog      Syslog / security ops
  pse-adapter-tabular     CSV / tabular
  pse-adapter-modelmon    ML model monitoring

tools/
  pse-bench-gt   Ground-truth precision/recall (PSE vs STL-z-score vs IsoForest)
  pse-bench-bbo  TRITON spiral vs Random vs Halton on BBO test functions
  pse-audit      Determinism / replay auditor
  pse-demo       30-second runnable showcase + gate diagnostics
```

---

## What's new since the last README

The previous README described the engine before Strands E through P. The
short version of what changed:

* **E** — Engine made *real*. Data stream gets its own helix; Mandorla
  becomes actual standing-wave interference of carrier and data; 5D
  state grounded in graph topology; cascade operators became real
  resonance tests; CrystalAdapter; resonance fingerprint query;
  resonance-landscape-aware TRITON.
* **F** — Sliding-window ground-truth bench scenarios (seismo, vitals
  AFib, Binance regime shift) with EventScopedAdapter.
* **G** — TRITON BBO benchmark vs random and Halton.
* **H** — EU AI Act compliance proof sketch.
* **I** — Semantic phase hint on `Observation` (avalanche-hash fallback
  preserved).
* **J** — Adaptive carrier tracker (per-tick re-selection).
* **K** — `CapsuleSealer` with AES-GCM counter-reuse detector.
* **L** — Full TRITON navigator in BBO benchmark.
* **M** — Operator algebra: `compose / dual / bridge / query / interpolate`.
* **N** — Generative interpolation primitive.
* **P.1** — Hot-path `eprintln` → `tracing` (+15 % throughput).
* **P.2** — Visible engine: `state.last_gate` exposes the full
  `GateSnapshot` every tick. Bug fix: `q` (coherence) is now the
  *fraction* of intrinsic carrier coherence preserved by the data,
  not the absolute κ (whose ≈ 0.54 ceiling made the 0.5 threshold
  structurally unreachable). New `pse-demo` binary.
* **P.3** — `AdaptiveCalibrator`: rolling-history quantile thresholds.
  Engine self-calibrates per workload. Opt-in; default path unchanged.
  d-metric extended to include p90 + vertex-set churn for
  windowed-streaming workloads.

The 8-fold Kairos AND, falsifier gating, content-address scheme, and
EU-AI-Act compliance proof are unchanged across all of the above —
calibration moves; the *contract* doesn't.

---

## Where to go next

* **Reading**: [`docs/POST_SYMBOLIC.md`](docs/POST_SYMBOLIC.md) — what
  this paradigm *is*, why it's a category, what it can and cannot do.
* **Reading**: [`docs/COMPLIANCE.md`](docs/COMPLIANCE.md) — EU AI Act
  formal compliance proof sketch.
* **Running**: `cargo run --release -p pse-demo` — see the engine work
  end-to-end, with gate diagnostics, in under a minute.
* **Calibrating**: pick a domain you care about (or use one of the ten
  shipped adapters), feed real observations through, watch `state.last_gate`,
  tune. The diagnostic surface is the lever.
* **Extending**: implement `ObservationAdapter` for your data source;
  optionally implement `DomainAdapter` for domain-specific vocabulary.
  The trait surface is two methods.

---

## Author

Sebastian Klemm

## License

MIT
