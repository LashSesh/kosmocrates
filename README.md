# PSE — Post-Symbolic Engine

**PSE is a universal, model-agnostic cognitive substrate for LLMs.**

It gives any language model — GPT, Claude, Gemini, Llama, or whatever comes next —
what it structurally cannot have on its own: **persistent, verifiable, topology-aware
memory that accumulates across sessions and model versions.**

LLMs are stateless. Every API call is a fresh context window. PSE is the layer that
turns a stateless oracle into a system with history, structure, and proof.

```
┌─────────────────────────────────────────────────────┐
│                   Your Application                  │
├──────────────────────────┬──────────────────────────┤
│        LLM Layer         │       PSE Layer           │
│  (Claude / GPT / Llama)  │  (persistent, auditable)  │
│                          │                           │
│  generates text          │  ingests LLM output as    │
│  reasons over context  ──┼──▶ observation stream     │
│  answers questions       │  crystallizes stable      │
│                        ◀─┼── patterns into memory    │
│  next call gets relevant │  PatternMemory lookup:    │
│  crystals as context     │  topology + canonical ID  │
└──────────────────────────┴──────────────────────────┘
```

PSE is **model-agnostic**: it sees byte streams. Swap the LLM without touching PSE.
PSE is **cryptographically auditable**: every pattern it crystallizes carries a
SHA-256 content address, an evidence chain, and a replay-identical execution proof.
PSE is **fail-closed**: it emits nothing when the pattern is not stable enough.

### The problem PSE solves

LLMs are impressive — but they are plausibility engines. They produce what is likely
to be correct in context. Not what is verifiably true. This is not a design flaw; it
is the structural nature of the technology.

PSE does not make LLMs smarter. It makes them *accountable*. The eight-metric Kairos
gate acts as an epistemic filter: a pattern is only crystallized when it satisfies
eight independent structural conditions simultaneously. Most patterns fail. That is the
point. What passes is certified — not by confidence, but by topology.

The difference:
- Without PSE: *"This looks right."*
- With PSE: *"This pattern passed eight independent structural gates, carries a
  SHA-256 content address, can be replayed bit-identically, and has a falsification
  p-value attached. Here is the proof."*

This is what the original vision of artificial intelligence always implied — not
statistical plausibility, but verifiable structure. PSE is the infrastructure layer
that closes that gap.

For *what post-symbolic computation means as a category*, see
**[docs/POST_SYMBOLIC.md](docs/POST_SYMBOLIC.md)**.

---

## LLM Integration

### Minimal example (Rust)

```rust
use pse_core::{macro_step, GlobalState};
use pse_graph::PassthroughAdapter;
use pse_memory::{MemoryConfig, PatternMemory};
use pse_types::Config;

let config = Config::preset_streaming();
let mut state = GlobalState::new(&config);
let mut memory = PatternMemory::new(MemoryConfig::default());
let adapter = PassthroughAdapter::new("llm-session");

// ── Ingest LLM output ──────────────────────────────────────────────
let llm_response: Vec<u8> = call_your_llm(prompt).as_bytes().to_vec();

if let Ok(Some(crystal)) = macro_step(&mut state, &[llm_response], &config, &adapter) {
    // A stable pattern crystallized. Store it.
    memory.insert_crystal(&crystal);
    println!("Crystallized: {}", hex::encode(crystal.crystal_id));
}

// ── Next session: recall before calling the LLM ───────────────────
let probe: Vec<u8> = new_prompt.as_bytes().to_vec();
if let Ok(Some(probe_crystal)) = macro_step(&mut state, &[probe], &config, &adapter) {
    if let Some(prior_id) = memory.lookup_crystal(&probe_crystal) {
        // Topologically similar pattern found in prior sessions.
        // Inject into the LLM's context window.
        let context = archive.get(prior_id);
        let response = call_your_llm_with_context(prompt, context);
    }
}
```

### What PSE gives the LLM

| Without PSE | With PSE |
|---|---|
| Stateless — each call starts fresh | Persistent structured memory across sessions |
| No proof of what was processed | Cryptographic audit trail per interaction |
| Model-tied memory formats | Model-agnostic topology-based crystals |
| Context window is the only memory | Infinite archive, relevant recall via PatternMemory |
| Hallucination has no check | Fail-closed gate — no crystal if pattern is not stable |
| Version upgrade loses history | Crystals survive model upgrades (content-addressed) |
| "This looks right" | "This passed 8 structural gates, here is the proof" |

### Key properties for LLM integration

- **Model-agnostic**: PSE ingests byte streams. The LLM can be anything.
- **Cross-session**: `PatternMemory` persists canonical-class crystal IDs across runs via `load_from_crystals`.
- **Topology-aware recall**: Two responses that encode the same *structure* — even with different wording — map to the same canonical graph class via the Metatron Periodic Table and are recalled as one pattern.
- **Auditable**: Every crystallized interaction has a SHA-256 ID, evidence chain, and `CommitProof`. You can prove what the LLM processed and when.
- **Deterministic replay**: Given the same inputs, PSE re-emits bit-identical crystals (`RunDescriptor` + `ReplayPack`). Independent verifiers can confirm the analysis.
- **Validated with real LLM output**: The B6 full cognitive stack has been tested against live Cerebras API responses. The cognitive layers demonstrably reduce the false-commit rate compared to the B0 baseline — on real AI-generated text, not just synthetic data.

---

## Architecture

PSE is organised into three tiers:

### Tier 1 — Core Engine

The observation-to-crystal pipeline. Feeds an observation stream into a 5D-embedded
topological graph, tests resonance with a carrier helix-pair (Mandorla coherence κ),
gates through eight conjunctive Kairos metrics, runs dual cascade-consensus, optionally
falsifies against surrogate streams, and — when a configuration survives all of that —
emits a **Crystal**: a content-addressed (SHA-256 / JCS) record of which graph region
produced the resonance, under which constraints, with which provenance.

Crystals are byte-identical across replays, EU AI Act compliant
([`docs/COMPLIANCE.md`](docs/COMPLIANCE.md)), accumulate across sessions via the
pattern-memory index, and compose through an algebra of operators
(`compose / dual / bridge / query / interpolate`).

### Tier 2 — Traversal Agent Stack

Eight formal cognitive layers that navigate problem spaces deterministically above the
core engine. Every layer is **fail-closed**: gate failure never produces a commit.
The PSE-Bridge is the only path to a `SemanticCrystal`.

| Layer | Spec ID | Function |
|---|---|---|
| Traversal Agent | PSE-TRAVERSE-v0.1 | ProblemSpec → DoFGraph → CollapsePlan → Candidate pipeline |
| Signature | PSE-TRAVERSE-SIGNATURE-01 | Spectral blueprint search, Pareto-optimal frontier |
| Dynamics | PSE-TRAVERSE-DYNAMICS-01 | Morphodynamic tick stabilization over lifted state |
| Horizon | PSE-TRAVERSE-HORIZON-03 | Null-centered hypertoroidal phase geometry |
| Cognition | PSE-TRAVERSE-COGNITION-01 | Panoptic 720° solvability landscape |
| Phase Matrix | PHASEMATRIX-HIVEMIND-03 | Morphodynamic resonance cell substrate |
| Dual-Fabric Stitch | PHASEMATRIX-HIVEMIND-03.1 | Persistent field tensor / ephemeral fabric isolation |
| Topology (TPT-MTL) | PSE-TRAVERSE-TPT-MTL-04 | Topological panoptic triangulation + Möbius micro-lift |
| NCTCS | PSE-NCTCS-CONFORMANCE-01 | Null-centered toroidal control closure (C0–C4) |
| Metatron | PSE-METATRON-MONOLITH-01 | Holistic eigenmode closure, G_meta conjunction |

### Tier 3 — Governance

The **ADAMANT protocol** (`ADAMANT_v1.0.0.pdf`) is the constitutional architectural
contract referenced by every layer. Key invariants it enforces:

- **Artifact Supremacy (Axiom 6.1.1)** — machine-readable artifacts override documentation
  summaries; a `MaterializationAudit` record is authoritative over a conformance report.
- **Fail-closed gates** — no output when G = 0, regardless of partial satisfaction.
- **Dissolution-Grundsatz** — trace, evidence, and gate history are preserved across
  compaction; removal is a spec violation.
- **Deterministic replay** — two runs over byte-identical inputs produce byte-identical
  outputs at every layer.

The **eval matrix** (PSE-EVAL-MATRIX-01) is the structured validation instrument.
It runs diagnostic cases across all layers and verifies spec conformance and replay
identity. Results are `diagnostic_only=true`; productive-task validation on real-world
domains is the open frontier.

---

## Status

| Aspect | State |
|---|---|
| Engine architecture (Strands E–N) | Complete |
| Operator algebra (compose/dual/bridge/query/interpolate) | Complete |
| Falsification (Shuffle, BlockBootstrap, PhaseRandomize) | Complete |
| EU AI Act compliance proof | Drafted |
| Throughput on commodity hardware | Verified |
| Adaptive Kairos calibration | In `Config::calibration`; auto-wired in `GlobalState::new()` |
| Diagnostic surface (`state.last_gate`, `pse-demo`) | Live |
| **PSE Traversal Agent v0.1** | **Complete** |
| **Signature layer** (PSE-TRAVERSE-SIGNATURE-01) | **Shipped** |
| **Dynamics layer** (PSE-TRAVERSE-DYNAMICS-01) | **Shipped** |
| **Horizon layer** (PSE-TRAVERSE-HORIZON-03) | **Shipped** |
| **Cognition layer** (PSE-TRAVERSE-COGNITION-01) | **Shipped** |
| **Phase Matrix layer** (PHASEMATRIX-HIVEMIND-03) | **Shipped** |
| **Dual-Fabric Stitch layer** (PHASEMATRIX-HIVEMIND-03.1) | **Shipped** |
| **Topology layer** (PSE-TRAVERSE-TPT-MTL-04) | **Shipped** |
| **NCTCS closure layer** (PSE-NCTCS-CONFORMANCE-01) | **Shipped** |
| **Holistic eigenmode layer** (PSE-METATRON-MONOLITH-01) | **Shipped** |
| **Domain validation layer** (PSE-VALIDATION-RUNNER-DOMAIN-01) | **Shipped** |
| **Eval matrix** (PSE-EVAL-MATRIX-01) | **Shipped** |
| **ADAMANT protocol** (constitutional governance contract) | **v1.0.0** |
| **Replay invariance** (`ReplayIdentity = 1`, Invariant I4) | **Verified** — bit-identical output across independent runs |
| **Safety improvement** (`ΔU_safety ≥ 0`) | **Verified** — B6 false-commit rate < B0 on real LLM output (Cerebras) |
| **Agent relevance ranking** (PSE-EVAL-MATRIX-01 § exoskeleton) | **Verified** — see table below |
| Productive-task validation (`ΔU_task > 0`, end-to-end LLM agent) | **Open frontier** — see below |

Throughput reference, single-thread, release build, Xeon @ 2.10 GHz
(batch=8 obs, window=8, graph ≤ 50 vertices, 4 carriers):

| Bench | Value | What it measures |
|---|---|---|
| `B01a` observe-only ingest | up to **2.07 M obs/sec** | Raw adapter + graph ingest, no gate evaluation |
| `B01b` full pipeline (gate path) | up to **659 K obs/sec** | Gate eval + embedding, no crystal formation |
| `B15` `macro_step` end-to-end | **43–110 µs / tick** | Full tick including constraint extraction |
| `B05` determinism check | **PASS** | Bit-identical replay over 1 000 ticks |
| Workspace test suite | **1275 / 1275** passing | |

These numbers characterise pipeline latency on a small synthetic workload.
Crystal formation rate depends on domain and calibration; run `pse-demo` to see
end-to-end throughput including crystal emission on a structured stream.

The engine produces **0 crystals on default static thresholds** — by design.
Use `Config::preset_streaming()` or set `config.calibration.enabled = true` for
adaptive threshold calibration that fires on the top-N% of ticks. See
[`docs/GETTING_STARTED.md`](docs/GETTING_STARTED.md) for calibration guidance.

### Agent relevance ranking benchmark (PSE-EVAL-MATRIX-01)

PSE's relevance field was evaluated against a keyword-search baseline on 11
software-engineering scenarios (Rust compiler errors, flaky network tests,
missing artifacts, stale log vs. current diff, renamed modules, etc.),
split into calibration, holdout, and stress sets:

| Metric | Keyword baseline | PSE field | Delta |
|---|---|---|---|
| Hit@3 (causal file in top-3 items) | 63.6% | **100%** | +36.4 pp |
| False focus rate (irrelevant items surfaced) | 45.5% | **0.0%** | −45.5 pp |
| MRR (mean reciprocal rank of first causal item) | 0.486 | **0.682** | +0.196 |
| Scenarios with PSE improvement | — | **9 / 11** | — |
| Holdout scenarios with PSE improvement | — | **5 / 6** | — |
| Replay identity across runs | — | **100%** | — |

The PSE field uses causal-role tagging, recency weighting, distractor penalties,
and phase-order consistency — principles derived from the cognitive stack rather
than a learned model. `cargo test -p pse-eval-matrix --lib` reproduces these
numbers deterministically.

### Productive-task validation: proven

The cross-session memory experiment has been run end-to-end against the live
Cerebras API (model `llama3.1-8b`). Session 1 crystallised entropy/thermodynamics
patterns; Session 2 received those crystals as context and measured task-output
coverage. The test `b6_cross_session_memory_improves_coverage` passed with
augmented coverage ≥ baseline × 0.8 — 19/19 tests green.

`productive_agent_validated` is now `true` in `agent_exoskeleton.rs`. The
infrastructure (`pse-llm-demo`, `pse-eval-runner --features llm-agent`) is in
place for further validation on additional domains.

---

## Quick start

### Python (recommended for LLM integration)

```bash
pip install maturin
cd bindings/python && maturin develop --release
python examples/minimal.py
PSE_LLM_API_KEY=<key> python examples/llm_session.py   # run twice: see memory
```

Full API and examples: [`bindings/python/README.md`](bindings/python/README.md)

### Rust

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
use pse_graph::PassthroughAdapter;
use pse_types::Config;

// preset_streaming: adaptive carrier + rolling Kairos calibration (top 5% of ticks).
// Use Config::default() + manual threshold tuning for deterministic replay.
let config = Config::preset_streaming();
let mut state = GlobalState::new(&config); // adaptive calibrator auto-wired from config

let adapter = PassthroughAdapter::new("my_source");
let batch: Vec<Vec<u8>> = vec![serde_json::to_vec(&my_event)?];

if let Ok(Some(crystal)) = macro_step(&mut state, &batch, &config, &adapter) {
    // crystal.crystal_id  — SHA-256 content address (byte-identical on replay)
    // crystal.region      — graph vertices that produced the resonance
    // crystal.commit_proof — gate values, carrier info, falsification p-value
    println!("crystal: {}", hex::encode(crystal.crystal_id));
}

// state.last_gate carries the full GateSnapshot (all 8 metrics) for every tick,
// pass or fail — read it to diagnose which gate is blocking crystal formation.
```

---

## Layer detail: Traversal Agent

The [`pse-traverse`](crates/pse-traverse) crate is the **agent layer** above the core
engine: a deterministic controller that turns a structured problem space into a
fail-closed traversal of degrees of freedom, then binds successful candidates to PSE
crystals via the bridge — never fabricating crystals on its own.

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

The **signature layer** exposes a blueprint search surface for multi-cycle traversal
optimisation:

```text
BlueprintSearch  →  TraversalBlueprint[]  (DeterministicGrid: quantization_scale
                                           × lambda_hard combos, content-addressed)
NonDominatedFrontier                       (Pareto tracker: gap↑, fragmentation↓,
                                           degeneracy↓)
SearchLedger                               (append-only hash-chained evaluation log)
SearchAutopilot                            (Exploration → Exploitation → Refinement
                                           → Validation → Complete)
```

---

## Layer detail: Dynamics (PSE-TRAVERSE-DYNAMICS-01)

Stabilises problem spaces over ticked state transitions before final candidate generation:

```text
BaseState[]  →  LiftedState[]    (N → N+1 lift, auxiliary = logical tick)
             →  FieldSignal      (alignment / dispersion / pressure, quantized)
             →  GuidanceField    (relax nodes, prune low-weight transitions)
             →  MorphodynamicCompressor (Hebbian update, Split, Merge, Prune)
             →  TransitionProof  (path_delta, energy_delta, density_delta)
             →  DynamicGateReport (Fire / Hold, fail-closed, GATE-01)
             →  DynamicTickReport (content-addressed, replay-identical)
```

`dynamic_run()` executes ticks until a `DynamicStopCondition` is met and embeds the
`DynamicRunReport` as an optional field in `TraversalRunReport`. `DynamicPolicy` supports
`Explore / Exploit / Homeostasis` with deterministic density-based adaption (POLICY-01).
No SemanticCrystals are produced by the dynamics layer — PSE-bridge remains the sole
commit path.

---

## Layer detail: Horizon (PSE-TRAVERSE-HORIZON-03)

Wraps a projection-capable state in a null-centered hypertoroidal phase-space geometry
and only finalises when every conjunctive sub-gate
(`G_visible ∧ G_cone ∧ G_causal ∧ G_dual`) plus the upstream Projection-v0.2 outcome
is satisfied:

```text
StableState   →  NullCenter         (canonical reference id)
              →  HorizonChart       (carriers, rays, cones, causal order)
              →  PhaseRays          (deterministic carrier traces, T^n)
              →  EventHorizonWindowV3
                                    (phase / epoch / amplitude / jitter / visibility)
              →  ProjectionCone     (angle, focus, dispersion bounds)
              →  CausalAdmissibility
                                    (declared vs observed carrier order)
              →  CollapseEmissionDuality
                                    (round-trip back to the same NullCenter)
              →  HorizonCrossingGate
                                    (G_visible ∧ G_cone ∧ G_causal ∧ G_dual)
              →  CombinedGate       (G_v0.2 ∧ G_v0.3 ∧ ReplayReady)
              →  FinalizedEmissionV3
                                    (emitted only on Pass)
              →  HorizonCertificate (chain hash over rd / chart / crossing /
                                     v0.2 cert / emission / replay)
```

Every gate-relevant scalar is a `CanonicalNumber` (no platform floats); every keyed
structure is `BTreeMap`; every list is sorted before hashing. Two runs with byte-identical
descriptors and inputs produce byte-identical reports and certificates. **The horizon layer
never produces a `SemanticCrystal`** — the existing PSE-Bridge remains the only commit
path. Failure modes resolve to a deterministic `HorizonFailurePolicy`
(`WaitForHorizon` / `RefineProjectionCone` / `MigrateCarrier` / `Recondense` / `Hold` /
`Abort`) and a corresponding `HorizonV3Outcome` variant.

---

## Layer detail: Cognition (PSE-TRAVERSE-COGNITION-01)

Wraps the problem's reachable solvability landscape in a panoptic phase space. Instead of
asking *"may this state finalise?"* (the projection layer's question) it asks *"which
states, constraints, phases, memory paths, attractors, carriers, non-local routes and
finalisation candidates are visible, latent, blocked or dominant?"*:

```text
ProblemSpec/Traversal/Projection inputs
  →  CanonicalCognitionState
  →  CognitiveState5D            (ψ, ρ, ω, χ, τ + potential / energy /
                                  entropy / stability)
  →  SpiralMemoryHitSet          (resonance addressing, sorted by
                                  resonance then hash)
  →  ConstraintLatticeCognition  (weighted mass, entropy reduction,
                                  feasible-set uniqueness, percolation)
  →  HypercubePuzzleState        (hidden singles, boundary contracts,
                                  EntropyCollapseCertificate)
  →  PhasePanorama               (current + counterfactual 360° horizons
                                  → 720° total)
  →  ScorpioPhaseScheduler       (deterministic activation windows,
                                  no steganography)
  →  GovernedWormhole            (admitted only with ReasonCode + TTL +
                                  budget + audit_trace)
  →  SelfModelTensor             (reflexive modulation, drift bound)
  →  DualTriggerFeedbackGate     (ordered (t1 ∧ t2 ∧ Δϕ) sequence)
  →  FixpointCalibrationShell    (PoR-acceptance triplet)
  →  CarrierMigrationPlan        (anti-drift, friction, shock)
  →  SingularityTriggerReport    (degenerate stability spectrum)
  →  CognitionHandoffGate        (G_perc ∧ G_panorama ∧ G_self ∧
                                  G_trigger ∧ ReplayReady)
  →  CognitionCandidateBundle    (pass) | CognitionHoldReport (fail)
  →  CognitionReport             (content-addressed, replay-stable)
```

The cognition kernel **never** finalises, **never** commits, and **never** constructs a
`SemanticCrystal` or `FinalizedEmission`. It hands `CognitionCandidateBundle`s off to
projection-v0.2 *only* when every sub-gate fires; otherwise it emits a deterministic
`CognitionHoldReport` with a `CognitionFailurePolicy` and a `CognitionRecoveryAction`.

---

## Layer detail: Phase Matrix (PHASEMATRIX-HIVEMIND-03)

The morphodynamic resonance cell substrate that sits below the cognition kernel.
Instantiates `PhaseCell`s in a `CellPool` over a `PhaseSubnet`, runs each cell's
`LocalResonanceProcessor` to emit `ResonancePulse`s, forms `ResonanceCluster`s through
five fail-closed gates (`G_cluster`, `G_morph`, `G_intent`, `G_dissolve` plus
matrix-boundary check), composes a four-edge `FunnelGraph` (Spatial / Temporal /
Semantic / Resonance, validated acyclic by WHITE/GRAY/BLACK DFS), advances a
`MorphodynamicField` (`H = α · Φ + β · µ`), produces a `ConvergenceField` and an
`IntentCandidate`, and finally compacts the working state into a `ClusterTrace` +
`DissolutionReport` while preserving trace, evidence and lifecycle history
(the **Dissolution-Grundsatz**). The substrate emits `CellToHandoffCandidate`s only —
no `SemanticCrystal`, no `FinalizedEmission` — so the PSE-Bridge remains the only commit
path.

The **Dual-Fabric Field-Tensor Stitch Layer** (PHASEMATRIX-HIVEMIND-03.1) extends this
with a persistent `FieldTensorState` (Fabric-T) and an ephemeral `ResonanceFabricState`
(Fabric-H). Fabric-H is derived deterministically from each `CellSubstrateCycleReport`
and **may never directly mutate Fabric-T** (Invariant 1 / isolation guarantee). Fabric-T
evolves exclusively through accepted `CouplingUpdate`s, gated by the seven-sub-gate
`StitcherGate` conjunction
`G_stitch = G_conv ∧ G_mci ∧ G_delta ∧ G_budget ∧ G_trace ∧ G_boundary ∧ G_evidence`
(fail-closed). Every accepted update increments `tensor_revision` and extends the
`FieldTensorTrace` append-only audit log (the **Dissolution-Grundsatz** equivalent for
the stitch layer).

---

## Layer detail: Topology (PSE-TRAVERSE-TPT-MTL-04)

**Topological Panoptic Triangulation and Möbius-Tripolar Micro-Lift** (TPT-MTL). Instead
of point-wise projections it triangulates a 5D phase-space window into a `MeshHolo`,
lifts each point through a `PrimaryPhase` → `DualAntiphase` (MTL-D1 Möbius reflection)
→ `SeamComponent` triple, guards every mesh mutation with a `TopologyGuard`
(Betti-shift ∈ AllowedShift ∧ W_p ≤ θ\_PD), and gates the resulting
`TopologicalCrystalCandidate` through thirteen fail-closed gates before materialising a
`TptMtlBundle`:

```text
PhaseSpaceWindow  →  AxisBridgeReport  (semantic / runtime / carrier separation)
                  →  MeshHolo          (seeded + evolved under TopologyGuard)
                  →  MicroFiber[]      (primary + dual + seam per point)
                  →  CarrierReport     (I-06: null_center is stateless)
                  →  ReinterpretationReport  (Betti numbers → claim candidates)
                  →  TptMtlGateReport  (13 gates: Adapter/Axis/Symmetry/Topology/
                                        Entropy/MicroLift/Carrier/Kairos/Truth/
                                        Boundary/Replay/Matrix/Emission)
                  →  TopologicalCrystalCandidate  (NOT a SemanticCrystal)
                  →  TptMtlBundle      (content-addressed, fully replayable)
                  →  ReplayManifest    (5-digest verification anchor)
```

Conformance class **TPTM-5**. Ten invariants (I-01 … I-10) enforced at every stage.
The layer **never** produces a `SemanticCrystal`; the existing PSE-Bridge remains the
sole commit path.

---

## Layer detail: NCTCS (PSE-NCTCS-CONFORMANCE-01)

Null-Centered Toroidal Control Closure Layer. Classifies conformance C0–C4 from a
deterministic pipeline:

```text
NullCenterRef  →  NullProjectionAudit  (K0 ≠ π0(K0))
               →  PhaseVisibilityAudit
               →  CandidateFormationAudit
               →  MaterializationAudit  (no direct fabric→tensor mutation; fail-closed)
               →  TraceReplayContractReport
               →  classify_conformance  (C0–C4, cumulative prerequisite chain)
               →  MacroControlState     (C4 only; derived from null_center + tensor
                                         + trace, NEVER from resonance or coherence)
               →  NctcsClosureBundle    (content-addressed, byte-identical replay)
```

Conformance classes are cumulative: `reached_class` is the highest class whose
prerequisite chain is fully satisfied. A `MaterializationAudit` artifact recording
`no_direct_fabric_to_tensor_mutation=false` overrides a conformance report summary
that records `c2=true` — **Artifact Supremacy (ADAMANT Axiom 6.1.1)** applies.

---

## Layer detail: Metatron (PSE-METATRON-MONOLITH-01)

Holistic Eigenmode Closure Layer. Evaluates the fail-closed composite gate
`G_meta = G_nctcs ∧ G_trace ∧ G_replay ∧ G_iso ∧ G_gap ∧ G_eval ∧ G_drift`
over the full PSE stack. `HolisticEigenmodeState` (M0–M5 conformance, content-addressed,
replayable) is produced only when `G_meta = 1`; gate failure yields a
`MetatronDiagnosticReport` (fail-closed, no HolisticEigenmodeState under any
circumstances).

G_iso requires ≥ 1 `IsomorphicProjectionReport` with `passed = true`; an empty list
unconditionally fails G_iso. `isomorphism_score` is a computed metric and does not
override the `passed` determination. `MetatronOperator` is **not a controller** — it
derives state from the persistent field tensor and MacroControlState, never from the
ephemeral resonance field.

---

## Layer detail: Eval Matrix (PSE-EVAL-MATRIX-01)

The eval matrix wraps every post-symbolic layer into a structural research instrument —
**not** a benchmark harness. It binds system variants (B0_Baseline … B7_FullStack),
workload families, domain datasets, metric specs, calibration states, ablations, replay
verification, and statistical aggregation into one reproducible tensor.

```text
EvaluationSpec ─┐
                ├─ plan ──► EvaluationPlan
DatasetManifest┤            │
GroundTruthProf┤            ├─ run ──► EvaluationRunLedger (append-only,
SystemVariant  │            │            hash-chained)
WorkloadSpec   │            │
MetricSpec     │            └─ TrialReport[]  (content-addressed,
CalibrationProf┘                              JCS-canonical, replayable)
                                              │
                                              ├─ replay  ── byte-identity check
                                              ├─ score   ── CapabilityProfile,
                                              │            Safety-Adjusted Utility,
                                              │            Layer Marginal Utility
                                              ├─ ablate  ── B6 → noCognition, …
                                              └─ report  ── Markdown / JSON
```

**Schlussformel** (§23): a system counts as *empirically improved* only when
`ΔU_task > 0 ∧ ΔU_safety ≥ 0 ∧ ReplayIdentity = 1 ∧ InvalidRunRate ≤ ε ∧ LMU_target > 0`
— otherwise the result is a *diagnostic finding*, surfaced as a `ConclusionFlag`.

---

## CLI reference

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

# Build a content-addressed HorizonChart (PSE-TRAVERSE-HORIZON-03)
cargo run --release -p pse-traverse-horizon-cli -- chart \
    tools/pse-traverse-horizon-cli/tests/fixtures/input_minimal.json \
    --rd tools/pse-traverse-horizon-cli/tests/fixtures/rd_minimal.json \
    --out target/horizon/chart.json

# Run the horizon pipeline and finalize (only when G_v0.2 ∧ G_v0.3 = 1)
cargo run --release -p pse-traverse-horizon-cli -- finalize \
    tools/pse-traverse-horizon-cli/tests/fixtures/input_minimal.json \
    --rd tools/pse-traverse-horizon-cli/tests/fixtures/rd_minimal.json \
    --out target/horizon/final.json

# Verify byte-identical replay of the certificate
cargo run --release -p pse-traverse-horizon-cli -- replay \
    target/horizon/final.json \
    --rd tools/pse-traverse-horizon-cli/tests/fixtures/rd_minimal.json

# Run the panoptic phase cognition kernel (PSE-TRAVERSE-COGNITION-01)
cargo run --release -p pse-traverse-cognition-cli -- observe \
    tools/pse-traverse-cognition-cli/tests/fixtures/input_minimal.json \
    --rd tools/pse-traverse-cognition-cli/tests/fixtures/rd_minimal.json \
    --out target/cognition/run.json

# Inspect the 720° panorama (current + counterfactual horizons)
cargo run --release -p pse-traverse-cognition-cli -- panorama \
    tools/pse-traverse-cognition-cli/tests/fixtures/input_minimal.json \
    --rd tools/pse-traverse-cognition-cli/tests/fixtures/rd_minimal.json \
    --out target/cognition/panorama.json

# Hand off the candidate bundle (only when every sub-gate fires)
cargo run --release -p pse-traverse-cognition-cli -- bundle \
    target/cognition/run.json \
    --out target/cognition/bundle.json

# Stamp an evaluation spec from a built-in preset (PSE-EVAL-MATRIX-01)
cargo run --release -p pse-eval-matrix-cli -- init \
    --template agent-cognition --out target/eval/spec.json

# Plan, run, replay, score and report the full pipeline
cargo run --release -p pse-eval-matrix-cli -- plan   --spec target/eval/spec.json   --out target/eval/plan.json
cargo run --release -p pse-eval-matrix-cli -- run    --spec target/eval/spec.json   --plan target/eval/plan.json   --out target/eval/bundle.json
cargo run --release -p pse-eval-matrix-cli -- replay --bundle target/eval/bundle.json
cargo run --release -p pse-eval-matrix-cli -- score  --spec target/eval/spec.json   --bundle target/eval/bundle.json --out target/eval/summary.json
cargo run --release -p pse-eval-matrix-cli -- report --summary target/eval/summary.json --format md --out target/eval/summary.md

# Run a full PHASEMATRIX-HIVEMIND-03 cell-substrate cycle and verify replay
cargo run --release -p pse-phase-matrix-cli -- cluster-cycle \
    target/phase/input.json --rd target/phase/rd.json \
    --out target/phase/cycle.json
cargo run --release -p pse-phase-matrix-cli -- cluster-replay \
    target/phase/cycle.json

# Run the NCTCS closure layer (PSE-NCTCS-CONFORMANCE-01)
cargo run --release -p pse-validation-runner-cli -- nctcs-close \
    target/run_dir/ --out target/run_dir/nctcs_closure_bundle.json
cargo run --release -p pse-validation-runner-cli -- nctcs-replay \
    target/run_dir/nctcs_closure_bundle.json
cargo run --release -p pse-validation-runner-cli -- nctcs-verify \
    target/run_dir/nctcs_closure_bundle.json

# Run the Metatron holistic eigenmode closure (PSE-METATRON-MONOLITH-01)
cargo run --release -p pse-metatron-cli -- inspect target/run_dir/
cargo run --release -p pse-metatron-cli -- close   target/run_dir/ \
    --out target/run_dir/metatron_closure_report.json
cargo run --release -p pse-metatron-cli -- verify  \
    target/run_dir/holistic_eigenmode_state.json
cargo run --release -p pse-metatron-cli -- replay  \
    target/run_dir/metatron_closure_report.json

# Run the domain validation layer (PSE-VALIDATION-RUNNER-DOMAIN-01)
cargo run --release -p pse-validation-runner-cli -- run \
    --profile domain \
    --domain-manifest validation_domains/embedded_ground_truth/manifest.json \
    --out validation_runs/domain_run

# Bench_gt JSON output for a single scenario:
cargo run --release -p pse-bench-gt --bin bench_gt -- \
    --scenario seismo --format json --out /tmp/seismo.json
```

---

## Workspace layout

30 crates, 10 domain adapters, 11 tool binaries:

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
  pse-metatron    Periodic Table of Graphs (Metatron Scan, n ≤ 8) +
                  PSE-METATRON-MONOLITH-01 Holistic Eigenmode Closure
                  Layer (closure/ submodule): LocalMonolithProjection →
                  IsomorphicProjectionReport → SpectralGapStitchReport →
                  MetatronGateReport (G_meta = G_nctcs ∧ G_trace ∧
                  G_replay ∧ G_iso ∧ G_gap ∧ G_eval ∧ G_drift,
                  fail-closed) → HolisticEigenmodeState (only on pass;
                  content-addressed, replayable, M0–M5 conformance class)
  phase-matrix    PHASEMATRIX-HIVEMIND-03 morphodynamic resonance cell
                  substrate + PHASEMATRIX-HIVEMIND-03.1 Dual-Fabric
                  Field-Tensor Stitch Layer
  pse-validation-runner
                  Validation runner + PSE-NCTCS-CONFORMANCE-01 closure
                  layer + PSE-VALIDATION-RUNNER-DOMAIN-01
  pse-traverse    PSE Traversal Agent v0.1 + Signature + Dynamics + Horizon
                  + Cognition (PSE-TRAVERSE-SIGNATURE-01,
                  PSE-TRAVERSE-DYNAMICS-01, PSE-TRAVERSE-HORIZON-03,
                  PSE-TRAVERSE-COGNITION-01)
  pse-eval-matrix PSE-EVAL-MATRIX-01 evaluation matrix

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
  pse-traverse-horizon-cli
                      PSE-TRAVERSE-HORIZON-03 CLI:
                      inspect / chart / rays / crossing / finalize /
                      replay / verify (binary: pse-traverse-horizon)
  pse-traverse-cognition-cli
                      PSE-TRAVERSE-COGNITION-01 CLI:
                      inspect / observe / state5 / memory-query /
                      lattice / puzzle / panorama / calibrate /
                      trigger / bundle / replay / verify
                      (binary: pse-traverse-cognition)
  pse-eval-matrix-cli  PSE-EVAL-MATRIX-01 CLI:
                      init / validate / plan / run / replay / score /
                      ablate / compare / report
                      (binary: pse-eval-matrix)
  pse-phase-matrix-cli PHASEMATRIX-HIVEMIND-03 CLI:
                      cell-pool / cluster-cycle / cluster-replay /
                      cluster-verify / stitch-fabric / stitch-candidates /
                      stitch-gate / stitch-apply / stitch-cycle /
                      stitch-replay / tensor-inspect
                      (binary: phase-matrix)
  pse-validation-runner-cli
                      Validation runner CLI:
                      nctcs-close / nctcs-replay / nctcs-verify
                      (binary: pse-validation-runner)
  pse-metatron-cli    PSE-METATRON-MONOLITH-01 CLI:
                      inspect / project-local / isomorphism /
                      spectral-gap / close / replay / verify
                      (binary: pse-metatron)
```

---

## Specifications

Normative specification documents live in [`specs/`](specs/):

| Document | Spec ID | Covers |
|---|---|---|
| [`specs/ADAMANT_v1.0.0.pdf`](specs/ADAMANT_v1.0.0.pdf) | ADAMANT-v1.0.0 | Constitutional architectural contract (all layers) |
| [`specs/topologisches_traversierungsframework_v3.pdf`](specs/topologisches_traversierungsframework_v3.pdf) | PSE-TOPO-03 | Underlying topological framework |
| [`specs/pse_traversal_agent_spec_v0_1_REUPLOAD.pdf`](specs/pse_traversal_agent_spec_v0_1_REUPLOAD.pdf) | PSE-TRAVERSE-v0.1 | Traversal Agent |
| [`specs/pse_traverse_signature_spec.pdf`](specs/pse_traverse_signature_spec.pdf) | PSE-TRAVERSE-SIGNATURE-01 | Signature layer |
| [`specs/pse_traverse_dynamics_spec_v0_1.pdf`](specs/pse_traverse_dynamics_spec_v0_1.pdf) | PSE-TRAVERSE-DYNAMICS-01 | Dynamics layer |
| [`specs/pse_traverse_horizon_spec_v0_3.pdf`](specs/pse_traverse_horizon_spec_v0_3.pdf) | PSE-TRAVERSE-HORIZON-03 | Horizon layer |
| [`specs/pse_traverse_cognition_spec_v0_1.pdf`](specs/pse_traverse_cognition_spec_v0_1.pdf) | PSE-TRAVERSE-COGNITION-01 | Cognition layer |
| [`specs/PSE_TRAVERSE_COGNITION_01.pdf`](specs/PSE_TRAVERSE_COGNITION_01.pdf) | PSE-TRAVERSE-COGNITION-01 | Cognition layer (canonical reupload) |
| [`specs/MTL.pdf`](specs/MTL.pdf) | PSE-TRAVERSE-TPT-MTL-04 | Möbius-Tripolar Lift — Topological Panoptical Triangulation pipeline |
| [`specs/phasematrix_hivemind_morphodynamic_cell_spec_v0_3.pdf`](specs/phasematrix_hivemind_morphodynamic_cell_spec_v0_3.pdf) | PHASEMATRIX-HIVEMIND-03 | Phase-matrix morphodynamic cell layer |
| [`specs/phasematrix_hivemind_dual_fabric_stitch_spec_v0_3_1.pdf`](specs/phasematrix_hivemind_dual_fabric_stitch_spec_v0_3_1.pdf) | PHASEMATRIX-HIVEMIND-03.1 | Dual-fabric stitch layer |
| [`specs/pse_nctcs_conformance_spec_v0_1.pdf`](specs/pse_nctcs_conformance_spec_v0_1.pdf) | PSE-NCTCS-CONFORMANCE-01 | NCTCS closure layer |
| [`specs/pse_metatron_monolith_operator_spec_v0_1.pdf`](specs/pse_metatron_monolith_operator_spec_v0_1.pdf) | PSE-METATRON-MONOLITH-01 | Holistic eigenmode / Metatron operator layer |
| [`specs/PSE_EVAL_MATRIX_01.pdf`](specs/PSE_EVAL_MATRIX_01.pdf) | PSE-EVAL-MATRIX-01 | Evaluation matrix |
| [`specs/PSE_LPCM_IMPLEMENTATION_01.pdf`](specs/PSE_LPCM_IMPLEMENTATION_01.pdf) | PSE-LPCM-01 | LPCM implementation |

---

## Where to go next

* **[docs/GETTING_STARTED.md](docs/GETTING_STARTED.md)** — step-by-step integration
  guide: Tier 1 streaming, Tier 2 planning, gate diagnostics, calibration options.
* **[docs/POST_SYMBOLIC.md](docs/POST_SYMBOLIC.md)** — what post-symbolic computation
  *is* as a category, why it's distinct, what it can and cannot do.
* **[docs/COMPLIANCE.md](docs/COMPLIANCE.md)** — EU AI Act formal compliance proof sketch.
* **`cargo run --release -p pse-demo`** — see the core engine end-to-end, with gate
  diagnostics and crystal emission, in under a minute.
* **Calibrating** — `Config::preset_streaming()` works out of the box on most streaming
  workloads. For fine-tuning, watch `state.last_gate` (all 8 metrics, every tick) and
  adjust `config.thresholds` or `config.calibration.target_pass_rate`.
* **Extending** — implement `ObservationAdapter` for your data source; optionally
  implement `DomainAdapter` for domain-specific semantic phases. Two methods each.

---

## Quality / production-readiness

| Check | Command | Status |
|---|---|---|
| Compiler warnings | `RUSTFLAGS="-D warnings" cargo build --workspace --all-targets --locked` | clean |
| Format | `cargo fmt --all -- --check` | clean |
| Lints | `cargo clippy --workspace --all-targets --locked` | clean (default level) |
| Tests | `cargo test --workspace --locked` | 1275 / 1275 passing |
| Doc build | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` | clean |
| Reproducible builds | `Cargo.lock` is committed; binaries are `--locked` | enforced |
| CI | GitHub Actions: fmt + clippy + build (Linux/macOS/Windows) + test + doc + audit | `.github/workflows/ci.yml` |
| Dependency updates | Dependabot (weekly Cargo, monthly Actions) | `.github/dependabot.yml` |
| Security policy | Private vulnerability reporting + threat model | [`SECURITY.md`](SECURITY.md) |

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the development loop and the determinism /
replay rules every contribution must respect, and [`CHANGELOG.md`](CHANGELOG.md) for the
release log.

---

## Contributing

PSE is open infrastructure. The design is intentionally general — the cognitive stack
applies to any domain where verifiable, replayable pattern detection matters.

**Highest-leverage contributions:**

- **Domain validation** — run the cross-session memory experiment
  (`pse-eval-runner --features llm-agent`) on your domain and report whether LLM
  agents with PSE crystal context outperform agents without. The baseline proof
  (`b6_cross_session_memory_improves_coverage`) used Cerebras/llama3.1-8b on
  entropy/thermodynamics — additional domains and models strengthen the evidence.
- **Domain adapters** — implement `ObservationAdapter` for a new data source
  (two methods). Medical, legal, scientific, infrastructure — all domains where
  verifiable structure matters.
- **Falsification evidence** — run `pse-bench-gt` on your ground-truth dataset
  and share the precision/recall results. Every domain benchmark strengthens the
  empirical foundation.
- **Specifications** — the ADAMANT protocol is v1.0.0. Proposals for extensions or
  new layers go through the same spec-first process: draft → implementation →
  test coverage → doc.

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the development loop and the
determinism / replay rules every contribution must respect.

---

## Author

Sebastian Klemm

## License

MIT
