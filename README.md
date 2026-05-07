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
| **PSE Traversal Agent v0.1** (post-symbolic agent layer) | **Complete** |
| **Signature layer** (PSE-TRAVERSE-SIGNATURE-01) | **Shipped** |
| **Dynamics layer** (PSE-TRAVERSE-DYNAMICS-01) | **Shipped** |
| Calibration on real production data | **Open frontier** |

Verified throughput, single-thread, release build, Xeon @ 2.10 GHz:

| Bench | Value |
|---|---|
| `B01a` observe-only ingest | up to **2.07 M obs/sec** |
| `B01b` full pipeline (gate path) | up to **659 K obs/sec** |
| `B15` `macro_step` end-to-end | **43–110 µs** |
| `B05` determinism check | **PASS** (bit-identical replay) |
| Workspace test suite | **627 / 627** passing |

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

## PSE Traversal Agent v0.1

PSE is the **Commit-/Evidence-/Falsifier-Kern**. The new
[`pse-traverse`](crates/pse-traverse) crate ships the **agent layer**
on top: a deterministic controller that turns a *structured problem
space* into a *fail-closed traversal of degrees of freedom*, then
binds successful candidates to PSE crystals via the bridge — never
fabricating crystals on its own.

> *Ein post-symbolischer Traversierungsagent ist ein Agent, der nicht
> nur Antworten produziert, sondern Problemräume topologisch erschließt,
> Lösungspfade kontrolliert kollabiert und jeden stabilen Erkenntnis-
> oder Implementierungsschritt überprüfbar historisiert.*

The pipeline:

```text
ProblemSpec  →  FieldCube         (dimensions, constraints, couplings, paths,
                                   carriers, evidence, topology summary)
             →  DoFGraph           (degree-of-freedom graph)
             →  PathExcision[]     (formal options without admissible paths)
             →  CollapsePlan       (deterministic step ordering: detect → hard
                                   → soft → resolve → verify → commit)
             →  StructuralOperator (Laplacian/matrix profile from DoFGraph)
             →  Signature          (sorted spectral values, content-addressed)
             →  SignatureDiagnostics (gap / degeneracy / rigidity /
                                   asymmetry / fragmentation ∈ [0,1], RegimeHint)
             →  SignatureGateOutcome (advisory or fail-closed gate check)
             →  Candidate          (solver-emitted, with assignments + payloads)
             →  GateReport         (Dual-Fabric: primal + mirror + MCI;
                                   + SignatureGate diagnostic channel)
             →  CommitOutcome      (Crystal | NoCrystal | EvidenceOnly | GateFailed)
```

The **signature layer** exposes a blueprint search surface for
multi-cycle traversal optimisation:

```text
BlueprintSearch  →  TraversalBlueprint[]  (DeterministicGrid: quantization_scale
                                           × lambda_hard combos, content-addressed)
NonDominatedFrontier                       (Pareto tracker: gap↑, fragmentation↓,
                                           degeneracy↓)
SearchLedger                               (append-only hash-chained evaluation log)
SearchAutopilot                            (Exploration → Exploitation → Refinement
                                           → Validation → Complete)
```

The optional **dynamics layer** (PSE-TRAVERSE-DYNAMICS-01) stabilises problem
spaces over ticked state transitions before final candidate generation:

```text
BaseState[]  →  LiftedState[]    (N → N+1 lift, auxiliary = logical tick)
             →  FieldSignal      (alignment / dispersion / pressure, quantized)
             →  GuidanceField    (relax nodes, prune low-weight transitions)
             →  MorphodynamicCompressor (Hebbian update, Split, Merge, Prune)
             →  TransitionProof  (path_delta, energy_delta, density_delta)
             →  DynamicGateReport (Fire / Hold, fail-closed, GATE-01)
             →  DynamicTickReport (content-addressed, replay-identical)
```

`dynamic_run()` executes ticks until a `DynamicStopCondition` is met and
embeds the `DynamicRunReport` as an optional field in `TraversalRunReport`.
`DynamicPolicy` supports `Explore / Exploit / Homeostasis` with deterministic
density-based adaption (POLICY-01). No SemanticCrystals are produced by the
dynamics layer — PSE-bridge remains the sole commit path.

Spec compliance highlights:

* **Determinism.** Every keyed collection is `BTreeMap`, every list
  sorted, every output round-tripped through JCS (RFC 8785). Two runs
  on the same `ProblemSpec` produce **byte-identical** reports
  (verified by `tests/replay_byte_identity.rs`).
* **Fail-closed.** A gate failure NEVER produces a commit. It produces
  a `GateReport` with a `FailurePolicy` (Refine / Excise / Boundary /
  Abort). The PSE bridge is the *only* path to a `SemanticCrystal`.
* **PSE-anchored.** `PseMacroStepCommitter` calls `pse_core::macro_step`
  and inspects `state.last_gate`. If PSE rejects, the traversal report
  carries a `NoCrystal { reason, gate_snapshot_json }` — never a
  fabricated crystal.
* **Replayable.** `TraversalRunReport` embeds the full `ProblemSpec` so
  `pse-traverse-cli replay` can re-derive cube/graph/plan/excisions
  without external context and assert canonical-byte identity.

Run it:

```bash
# Inspect the structured problem space
cargo run --release -p pse-traverse-cli -- inspect \
    --problem crates/pse-traverse/examples/problem_minimal.json

# Generate a deterministic CollapsePlan
cargo run --release -p pse-traverse-cli -- plan \
    --problem crates/pse-traverse/examples/problem_minimal.json \
    --out target/traverse/plan.json

# Plan + full signature layer (operator → signature → diagnostics → gate)
cargo run --release -p pse-traverse-cli -- plan \
    --problem crates/pse-traverse/examples/problem_minimal.json \
    --signature

# Full run including a PSE-bridge commit attempt per required dimension
cargo run --release -p pse-traverse-cli -- run \
    --problem crates/pse-traverse/examples/problem_minimal.json \
    --out target/traverse/run.json

# Run with SignatureGate as a diagnostic channel on every GateReport
cargo run --release -p pse-traverse-cli -- run \
    --problem crates/pse-traverse/examples/problem_minimal.json \
    --signature-gate --out target/traverse/run_sig.json

# Verify byte-identical replay
cargo run --release -p pse-traverse-cli -- replay \
    --run target/traverse/run.json

# Generate traversal blueprints (DeterministicGrid search)
cargo run --release -p pse-traverse-cli -- search \
    --problem crates/pse-traverse/examples/problem_minimal.json \
    --n 8 --out target/traverse/blueprints.json
```

The MVP solver in `run` is a one-value-per-dimension template — by
design. Real solvers (template / LLM / tool / human) plug in via the
`Candidate`-producing surface; the gating, fail-closed conversion and
PSE binding are the same regardless.

See `pse_traversal_agent_spec_v0_1_REUPLOAD.pdf` and
`pse_traverse_signature_spec.pdf` (PSE-TRAVERSE-SIGNATURE-01) for the
specs this layer realises, and
`topologisches_traversierungsframework_v3.pdf` for the underlying
topological framework.

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
  pse-metatron    Periodic Table of Graphs (Metatron Scan, n ≤ 8)
  pse-traverse    PSE Traversal Agent v0.1 + Signature Layer + Dynamics Layer
                  (PSE-TRAVERSE-SIGNATURE-01, PSE-TRAVERSE-DYNAMICS-01):
                  ProblemSpec → FieldCube → DoFGraph → CollapsePlan →
                  StructuralOperator → Signature → SignatureDiagnostics →
                  SignatureGate → [optional: BaseState→LiftedState→FieldSignal→
                  GuidanceField→Compressor→TransitionProof→DynamicGate] →
                  Candidate → GateReport → PSE-bridge (fail-closed);
                  BlueprintSearch, SearchLedger, SearchAutopilot,
                  dynamic_tick / dynamic_run for multi-cycle stabilisation

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
  pse-bench-gt        Ground-truth precision/recall (PSE vs STL-z-score vs IsoForest)
  pse-bench-bbo       TRITON spiral vs Random vs Halton on BBO test functions
  pse-audit           Determinism / replay auditor
  pse-demo            30-second runnable showcase + gate diagnostics
  pse-traverse-cli    Traversal Agent CLI: inspect / plan [--signature] /
                      run [--signature-gate] / replay / search /
                      dynamics (init | tick | run | replay | inspect)
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
* **Signature layer (PSE-TRAVERSE-SIGNATURE-01)** — full spectral
  operator/diagnostics/gate/blueprint-search layer on top of the Traversal Agent:
  `StructuralOperator` (Laplacian matrix profile from `DoFGraph`) →
  `Signature` (sorted fixed-point spectral values, Jacobi eigensolver
  for n ≤ 8, `MatrixProfileApprox` for larger graphs) →
  `SignatureDiagnostics` (five scores in [0,1] + advisory `RegimeHint`) →
  `SignatureGate` (configurable thresholds, fail-closed or diagnostic-only).
  Blueprint search: `BlueprintSearch` with `DeterministicGrid` policy
  (quantization_scale × lambda_hard grid), `NonDominatedFrontier`
  (Pareto tracker), `SearchLedger` (hash-chained evaluation log),
  `SearchAutopilot` (5-phase state machine). CLI extended with
  `--signature`, `--signature-gate`, and `search` subcommand.

* **Dynamics layer (PSE-TRAVERSE-DYNAMICS-01)** — optional morphodynamic
  tick engine extracted from neutral DioniceOS mechanics:
  `CanonicalNumber` (scale-9 banker's-rounding fixed-point), `Hash256`/`StableId`
  content addresses, `BaseState`/`LiftedState` (N→N+1 lift/projection),
  `FieldAbsorber` (alignment/dispersion/pressure), `GuidanceField` (relax +
  gradient), `MorphodynamicCompressor` (Hebbian/Split/Merge/Prune),
  `TransitionProof` (path_delta/energy_delta), `DynamicGate` (fail-closed
  Fire/Hold), `dynamic_tick` (total — valid on empty input, TICK-01),
  `dynamic_run` (tick loop with configurable stop conditions).
  `DynamicRunReport` embedded optionally in `TraversalRunReport`.
  CLI: `dynamics init|tick|run|replay|inspect`. 87 pse-traverse tests pass.

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
