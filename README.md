# PSE — Post-Symbolic Engine

**PSE is a deterministic epistemic substrate for AI systems.**

It turns stateless LLM interactions into persistent, verifiable, topology-aware
knowledge structures: content-addressed crystals, ledgered evidence, causal
retrieval paths, QTIC conformance certificates, and self-maintaining epistemic
health signals.

PSE is not another vector memory, prompt wrapper, or RAG convenience layer.
It is an execution substrate for deciding **what may become memory**, **why it
was allowed to persist**, **how it can be replayed**, **where it sits in a causal
knowledge graph**, and **what must be repaired before the knowledge base can move
toward closure**.

At the core, PSE ingests observation streams from any model or application,
tests them through fail-closed topological gates, and crystallizes only stable
patterns into SHA-256 content-addressed records. With Infinity Ledger, HDAG,
QTIC, Pfauenthron++ (thunderbolt retrieval), and the PSE+IL Intelligence Layer, those records
become an auditable, causally navigable, self-monitoring knowledge field for
LLMs and agent systems.

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                              Your Application                               │
├────────────────────────────────┬─────────────────────────────────────────────┤
│           LLM / Agent           │                PSE Substrate                │
│ GPT / Claude / Gemini / Llama   │ deterministic, auditable, fail-closed       │
│ local models / tool agents      │                                             │
├────────────────────────────────┼─────────────────────────────────────────────┤
│ generates text                  │ ingests observation streams                 │
│ reasons over context       ────┼──▶ tests topology + gate stability           │
│ calls tools                     │ crystallizes stable patterns                │
│ receives grounded memory   ◀───┼── commits to Infinity Ledger                 │
│ follows agenda signals          │ builds HDAG causal topology                 │
│                                │ certifies QTIC Q0–Q5 conformance            │
│                                │ retrieves via D = ψ · ρ · ω                 │
│                                │ monitors health, lifecycle, clusters        │
│                                │ emits an epistemic action agenda            │
└────────────────────────────────┴─────────────────────────────────────────────┘

## What PSE gives an AI system

PSE gives an LLM or agent what the model itself does not structurally possess:

- **Persistent memory** across sessions, runs, and model versions.
- **Content-addressed evidence** for what was processed and committed.
- **Deterministic replay**: byte-identical inputs produce byte-identical outputs.
- **Fail-closed crystallization**: unstable patterns emit no persistent memory.
- **Topology-aware knowledge** instead of flat chunks or unverified embeddings.
- **Causal retrieval** over ancestors, descendants, refinements, and related crystals.
- **QTIC conformance certificates** classifying committed knowledge from Q0 to Q5.
- **Constitutional pre-commit checks** for blocking, required, and advisory rules.
- **Epistemic health monitoring** for uncertainty, risk, attribution, and stability.
- **Lifecycle management** for stale, redundant, aging, and vital crystals.
- **Knowledge clustering** with bridge-crystal detection.
- **Epistemic agenda generation**: concrete actions to refresh, reinforce,
  consolidate, guard, or explore knowledge toward fixpoint closure.

PSE does not make an LLM omniscient, sentient, or automatically correct.
It makes the surrounding cognitive process **auditable, replayable, structurally
filtered, and self-maintaining**.

## What PSE is and is not

PSE alone is a **topological pattern-memory engine**. It crystallizes structurally
stable observation sequences into content-addressed records and recognizes them
deterministically when the same sequence is replayed.

PSE alone is **not** fuzzy semantic memory. Exact replay and structural stability
are its base guarantees.

**PSE + Infinity Ledger** extends the core with semantic and causal retrieval:

D = ψ · ρ · ω


where:

| Axis | Symbol | Meaning |
|---|---:|---|
| Semantic similarity | ψ | cosine similarity over IL semantic vectors |
| Structural stability | ρ | PSE gate-certified crystal stability |
| Temporal / HDAG coherence | ω | normalized HDAG coherence potential |

This means retrieval is not ranked by similarity alone. A candidate must be
semantically relevant, structurally stable, and temporally/topologically coherent.
If one axis collapses, the whole score collapses.

## Why PSE exists

LLMs are powerful plausibility engines. They can generate useful answers, but they
do not natively provide persistent proof of what was processed, why something was
stored, whether a memory is stale, which prior state caused a later state, or what
must be repaired before the system can safely continue.

PSE addresses that missing substrate.

The difference is:

| Without PSE | With PSE |
|---|---|
| “This looks right.” | “This passed structural gates and can be replayed.” |
| Context window only | Persistent content-addressed memory |
| Similarity-only retrieval | Semantic × structural × temporal retrieval |
| Flat memory chunks | Crystals in causal HDAG topology |
| No commit proof | SHA-256 IDs, evidence chains, replay packs |
| No memory health | uncertainty, lifecycle, clustering, agenda |
| No closure condition | explicit epistemic fixpoint criteria |

## Core architecture

PSE is organized into three tiers.

### Tier 1 — Core Engine

The observation-to-crystal pipeline. It feeds observation streams into a
5D-embedded topological graph, evaluates resonance and Kairos gate stability,
runs consensus and falsification checks, and emits a content-addressed crystal
only when the pattern is stable enough to persist.

Core properties:

- model-agnostic byte-stream ingestion
- SHA-256 / JCS content addressing
- fail-closed gate semantics
- deterministic replay
- evidence chains and commit proofs
- pattern-memory lookup
- operator algebra: `compose`, `dual`, `bridge`, `query`, `interpolate`

### Tier 2 — Cognitive / Traversal Stack

The deterministic agent-facing layer above the core engine. It contains formal
modules for traversal, signature search, dynamics, horizon geometry, cognition,
Phase Matrix, Dual-Fabric Stitch, TPT-MTL topology, NCTCS closure, and Metatron
holistic eigenmode closure.

Every layer is fail-closed. Candidate structures may be formed, inspected, traced,
and replayed, but the PSE-Bridge remains the only valid path to a committed
`SemanticCrystal`.

### Tier 3 — Governance

The governance layer defines the architectural contract for safe materialization:

- **ADAMANT protocol** as the constitutional governance contract.
- **Artifact Supremacy**: machine-readable artifacts override prose summaries.
- **Dissolution-Grundsatz**: compaction may remove working state, never trace,
  evidence, or gate history.
- **Replay identity** as a non-negotiable invariant.
- **Eval Matrix** as structured conformance and validation instrument.
- **Domain Validation Layer** for empirical benchmark-backed conclusions.

## Infinity Ledger, HDAG, and QTIC

When Infinity Ledger is active, gate-passing `SemanticCrystal` records are
committed into a private append-only ledger. Each ledger block carries semantic
vectors, topology signatures, stability scores, Metatron hashes, and QTIC
classification metadata.

HDAG builds a 5D resonance graph over those ledgered crystals. Edges encode
sequential commits, resonance proximity, refinement, and Metatron-isomorphic
structure. Path invariance checks ensure that equivalent causal routes condense
to equivalent canonical structure.

QTIC — Quasi-Temporal Information Crystal — provides the conformance model.
Every committed crystal receives a `QticCertificate` classifying it from Q0 to Q5.
A Q5 crystal is a gate-passed, auditable, replayable, path-invariant information
attractor between extrinsic ledger time and intrinsic phase coherence.

## PSE+IL Intelligence Layer

The Intelligence Layer turns Infinity Ledger from a passive record store into an
active epistemic system. It includes ten modules:

1. **Context compression** — budget-aware `[PSE-CONTEXT]` generation.
2. **Causal graph** — content-addressed lineage DAG over crystals.
3. **Agent layer** — per-agent provenance and cross-agent causal links.
4. **Constitutional AI substrate** — blocking pre-commit rule checks.
5. **Epistemic health monitoring** — uncertainty and risk metrics.
6. **Crystal lifecycle management** — decay, staleness, redundancy, refresh need.
7. **Prompt grounding** — system prompt assembly from context and agenda.
8. **Causal retrieval** — semantic seeds expanded through causal ancestors and descendants.
9. **Knowledge clustering** — semantic islands, bridge crystals, unification checks.
10. **Epistemic agenda** — prioritized actions toward knowledge fixpoint.

The IL store reaches epistemic fixpoint only when all four conditions hold:

| Condition | Meaning |
|---|---|
| Constitutional closure | all blocking rules pass |
| Lifecycle closure | no stale or redundant crystals remain |
| Topological unification | knowledge forms one coherent island |
| Agenda empty | no remaining action is needed |

At fixpoint, the knowledge base has no currently known constitutional, lifecycle,
topological, or agenda-level defect.

## Current status

PSE is implemented as a Rust workspace with the core engine, traversal stack,
Infinity Ledger adapter, HDAG, QTIC conformance engine, PSE+IL Intelligence Layer,
server routes, validation tooling, and governance layers.

Current validation status:

- Workspace test suite: **1315 / 1315 passing**
- Replay invariance: **verified**
- Safety improvement over B0 baseline on real LLM output: **verified**
- Agent relevance ranking benchmark: **verified**
- Productive cross-session memory demo with live Cerebras output: **verified**
- Domain validation layer: **shipped**
- NCTCS closure layer: **shipped**
- Metatron holistic eigenmode closure: **shipped**
- Phase Matrix and Dual-Fabric Stitch: **shipped**
- TPT-MTL topology layer: **shipped**

PSE remains pre-1.0. The public Rust API may still evolve, but replay,
content-addressing, and report-byte contracts are treated as core invariants unless
a release note explicitly declares a breaking change.

For what post-symbolic computation means as a category, see
**[docs/POST_SYMBOLIC.md](docs/POST_SYMBOLIC.md)**.

---

## LLM Integration

### Minimal example (Rust)

```rust
use pse_core::{macro_step, GlobalState};
use pse_memory::{MemoryConfig, PatternMemory};
use pse_types::Config;

// For LLM text: use a semantic-phase adapter (see tools/pse-llm-demo/src/observe.rs).
// PassthroughAdapter uses avalanche-hash phases that prevent carrier alignment.
// TextPhaseAdapter computes phase as the circular mean of per-token FNV hashes,
// so sentences sharing vocabulary cluster in the same phase region.
// Chunks are also content-addressed (one vertex per unique sentence), creating
// real co-occurrence topology instead of a single self-loop.
let config = /* see pse_config() in pse-llm-demo/src/main.rs */ Config::default();
let mut state = GlobalState::new(&config);
let mut memory = PatternMemory::new(MemoryConfig::default());
let adapter = TextPhaseAdapter::new("llm-session"); // implements ObservationAdapter

// ── Ingest LLM output ──────────────────────────────────────────────
let llm_response: Vec<u8> = call_your_llm(prompt).as_bytes().to_vec();

if let Ok(Some(crystal)) = macro_step(&mut state, &[llm_response], &config, &adapter) {
    // A stable pattern crystallized. Store it.
    memory.insert_crystal(&crystal);
    println!("Crystallized: {}", hex::encode(crystal.crystal_id));
}

// ── Next session: replay prior text to warm PatternMemory ────────────
// Replaying the *same* prior response text produces bit-identical crystal IDs.
// PatternMemory then reports a hit — proving the topology is stable across sessions.
// (PSE does not do fuzzy/semantic matching; recall requires the same byte sequence.)
let prior_response: Vec<u8> = load_prior_response_from_disk();
if let Ok(Some(replay_crystal)) = macro_step(&mut state, &[prior_response], &config, &adapter) {
    if let Some(_prior_id) = memory.lookup_crystal(&replay_crystal) {
        // Hit confirmed: topology is stable. Now inject prior crystals as context.
        let context = render_crystal_context(&crystal_records, 5);
        let response = call_your_llm_with_context(new_prompt, context);
    }
}
```

### What PSE gives the LLM

| Without PSE | With PSE |
|---|---|
| Stateless — each call starts fresh | Persistent structured memory across sessions |
| No proof of what was processed | Cryptographic audit trail per interaction |
| Model-tied memory formats | Model-agnostic topology-based crystals |
| Context window is the only memory | Persistent archive; deterministic recall via PatternMemory (content-addressed, not semantic) |
| Hallucination has no check | Fail-closed gate — no crystal if pattern is not stable |
| Version upgrade loses history | Crystals survive model upgrades (content-addressed) |
| "This looks right" | "This passed 8 structural gates, here is the proof" |

### Key properties for LLM integration

- **Model-agnostic**: PSE ingests byte streams. The LLM can be anything.
- **Cross-session**: `PatternMemory` persists canonical-class crystal IDs across runs via `load_from_crystals`.
- **Deterministic recall**: The same observation sequence always produces the same crystal ID (content-addressed via SHA-256).  `PatternMemory` detects exact replays across sessions — bit-identical input → guaranteed hit.  Semantic generalisation (recognising *similar* but not identical content) is outside PSE's scope in the current release.
- **Auditable**: Every crystallized interaction has a SHA-256 ID, evidence chain, and `CommitProof`. You can prove what the LLM processed and when.
- **Deterministic replay**: Given the same inputs, PSE re-emits bit-identical crystals (`RunDescriptor` + `ReplayPack`). Independent verifiers can confirm the analysis.
- **Validated with real LLM output**: The B6 full cognitive stack has been tested against live Cerebras API responses. The cognitive layers demonstrably reduce the false-commit rate compared to the B0 baseline — on real AI-generated text, not just synthetic data.

---

## Infinity Ledger + HDAG Integration

### What Infinity Ledger is

Infinity Ledger (IL) is a private content-addressed ledger bundled with PSE as a zip
distribution in `vendors/infinityledger/`. Cloning `lashsesh/pse` is all you need — the
zip is the complete distribution. IL extends PSE's crystallization pipeline with three
capabilities:

1. **Persistent cross-session ledger** — every `SemanticCrystal` that passes the Kairos
   gate is committed to an append-only block chain. Each block carries the crystal's 8D
   semantic vector, topology signature, stability score, and Metatron canonical hash.

2. **HDAG (Hierarchical Directed Acyclic Graph)** — a 5D resonance-tensor graph built
   over the ledger. Nodes are `ResonanceTensor` embeddings of crystals; directed edges
   encode causal and topological relationships between them.

3. **Pfauenthron++ Unified Retrieval** — the primary retrieval path when IL is active.
   Scores every candidate via the multiplicative tripolar formula `D = ψ · ρ · ω`.

### HDAG — 5D Resonance Graph

Each `SemanticCrystal` maps to a 5-dimensional resonance tensor:

```
T = [mean_propagation_time,  kuramoto_coherence,  cheeger_estimate,  spectral_gap,  1−stability_score]
  = [temporal,               morphic,             relational,        topological,   entropic          ]
```

When Metatron data is present, `cheeger_estimate` and `spectral_gap` are replaced by
graph-theoretic equivalents from the Metatron scan (`algebraic_connectivity/n`,
`spectral_radius/n`).

**Coherence potential** ψ = `kuramoto_coherence − (1 − stability_score)`. A crystal is
in the S_coh class (eligible for edges) if ψ > −0.1 or its Kairos gate passed.

**Acyclicity is emergent**: an edge i→j is only added when ψ(j) ≥ ψ(i). No timestamp
checks are needed — acyclicity follows from ψ-monotonicity by construction.

**Four edge causes:**

| Cause | When added |
|---|---|
| `sequential_commit` | Crystal B was committed immediately after A in the same session |
| `resonance_proximity` | ‖T_A − T_B‖ ≤ 0.35 and both crystals are in S_coh |
| `refinement` | B lists A in its `parent_crystal_ids` (IL→PSE feedback-loop refinement) |
| `metatron_isomorphic` | A and B share the same Metatron canonical hash (graph-isomorphic topology) |

**Path invariance** (`∮Φ·dl = 0`): every two HDAG paths between the same nodes produce
canonically equivalent condensations — enforced by `HDAG::verify_path_invariance()`.

### ValidationFeedback + Crystal Refinement

IL commit returns a `ValidationFeedback`:

```rust
pub struct ValidationFeedback {
    pub block_hash: String,
    pub converged: bool,
    pub coherence_potential: f64,  // ψ = kuramoto − (1 − stability)
    pub gate_passed: bool,
    pub hdag_node_id: String,
    pub il_stability: f64,         // IL quality signal ∈ [0, 1]
}
```

If the IL stability signal diverges from the original crystal's stability by more than
0.02, `refine_crystal()` produces a new crystal with blended stability `0.7·PSE + 0.3·IL`,
a fresh SHA-256 content address, and the original listed in `parent_crystal_ids`. The
refined crystal is also committed to IL, creating a `refinement` HDAG edge.

### Pfauenthron++ Unified Retrieval — D = ψ · ρ · ω

Defined in `specs/TheTimelessMonolith_bySebastianKlemm_v1.0.pdf`:

| Axis | Symbol | Source | Meaning |
|---|---|---|---|
| Semantic | ψ | cosine(query_vec, crystal_vec8) | IL vector similarity |
| Structural | ρ | `crystal.stability_score` | PSE gate-certified stability |
| Temporal | ω | `(hdag_ψ + 1) / 2` | normalized HDAG coherence potential |

The **Gabriel4D Funnel** requires all three axes to be non-trivial: a near-zero on any
axis collapses D to near-zero. A low-stability crystal (ρ≈0), a semantically irrelevant
one (ψ≈0), or a temporally incoherent one (ω≈0) cannot rank highly regardless of the
other two scores.

```
[pse-llm-demo]  [Unified retrieval: 5 record(s), top D=0.847]
                [Unified Retrieval — Pfauenthron++ D=ψ·ρ·ω]
```

### PSE Server — HTTP API

`tools/pse-server` exposes PSE+IL over HTTP. Eight routes:

| Method | Path | Description |
|---|---|---|
| `POST` | `/ingest` | Ingest observation bytes → crystals → optional IL commit |
| `GET` | `/status` | Engine status + crystal count |
| `GET` | `/crystals` | List all crystals in the registry |
| `POST` | `/retrieve` | Vector-based retrieval (legacy, PSE only) |
| `GET` | `/il/status` | IL ledger status + HDAG edge counts by cause |
| `POST` | `/il/retrieve` | Pfauenthron++ tripolar retrieval (`D = ψ·ρ·ω`) |
| `GET` | `/il/hdag/coherence` | Mean HDAG coherence potential across all nodes |
| `GET` | `/il/hdag/order` | Topological order of HDAG nodes (Kahn's algorithm) |

IL routes are only active when the server starts with a valid `PSE_IL_PATH` environment
variable:

```bash
# Start with IL enabled
PSE_IL_STORE=./il_data cargo run --release -p pse-server

# Retrieve via Pfauenthron++
curl -X POST http://localhost:8765/il/retrieve \
     -H 'Content-Type: application/json' \
     -d '{"question": "entropy and thermodynamics", "top_k": 5}'

# Inspect HDAG
curl http://localhost:8765/il/hdag/coherence
curl http://localhost:8765/il/hdag/order
```

---

## QTIC — Theory and Implementation

`specs/QTIC.pdf` (Sebastian Klemm, 11 May 2026) provides the formal theoretical
framework underlying PSE+IL. QTIC = *Quasi-Zeitinformationskristall* (Quasi-Temporal
Information Crystal). The framework is not only mapped but fully implemented in
`adapters/pse-adapter-il/src/qtic.rs` — every `SemanticCrystal` committed to IL
receives a `QticCertificate` classifying it as Q0–Q5.

**Central formula** (QTIC §21):

```
QTIC = Fix(Tα) ∩ Int(ℋΣ⁻) ∩ Pass(GΓ) ∩ Pass(GΣ) ∩ Mirror(S,M) ∩ TraceReplay ∩ PathInvariant
```

**Main theorem** (QTIC §16, Satz 16.1): *A valid QTIC is a seam-stable, path-invariant
and replayable information attractor between extrinsic revision time and intrinsic spiral
phase.*

### Implementation: `pse-adapter-il/src/qtic.rs`

```rust
// Every commit_with_feedback() call produces a QticCertificate:
let fb = store.commit_with_feedback(&crystal, &chunks, session, question)?;
let cert = fb.qtic_certificate.unwrap();

println!("Class: {:?}", cert.conformance_class);   // Q0–Q5
println!("MCI:   {:.3}", cert.mci);                // Mirror-Consistency Index
println!("ψ:     {:.3}", cert.psi);                // coherence potential
println!("t:     {}",    cert.extrinsic_t);        // IL block index (extrinsic time)
println!("θ:     {:.3}", cert.intrinsic_theta);    // kuramoto_coherence (intrinsic phase)
println!("Q5:    {}",    cert.is_full_qtic());     // seam-stable attractor?
```

The conformance class is also stored as `qtic_class: Option<u8>` in the IL index entry,
persisted to disk on every commit (backward-compatible with existing ledger files).

**`mirror_consistency_index(σ_H, σ_T)`** = 1 − |PSE_stability − IL_stability| — the
concrete MCI implementation. Threshold η = 0.7 for Q4.

**`HDAG::check_node_path_invariance(node_id)`** — the Q5 gate: checks that all direct
predecessors' paths to the node produce the same canonical gradient condensation (QTIC
§13.3 HDAG path neutrality invariant). Trivially passes for nodes with ≤ 1 predecessor.

### QTIC ↔ PSE+IL Mapping

| QTIC Concept | PSE+IL Implementation |
|---|---|
| TIC — Fix(Tα) | `SemanticCrystal` (content-addressed, gate-validated fixed point) |
| Extrinsic revision time t | IL ledger block index / commit sequence |
| Intrinsic spiral phase θ | HDAG coherence potential ψ (kuramoto_coherence axis) |
| Dual time T_dual = ℝ≥0 × S¹ | IL block index × HDAG ψ phase |
| DualFabric F_H (ephemeral) | PSE graph observations (resonance fabric) |
| DualFabric F_T (persistent) | IL ledger (persistent field tensor) |
| Seam Γ / Gate G_Γ | PSE Kairos Gate (8-metric fail-closed AND) |
| Mandorla-Zone M_t,θ = A(t)∩B(θ) | Crystal must satisfy extrinsic gate metrics AND intrinsic phase stability |
| Mandorla-Brane Σ = ∂M | Gate membrane — only Gate-passing events cross from F_H to F_T |
| Mirror-Consistency MCI = 1 − dist(σ_H, σ_T) | `mirror_consistency_index(pse_stability, il_stability)` |
| Proof-of-Resonance Θ(x) = αψ+βρ+γω | Pfauenthron++ D = ψ·ρ·ω |
| MEF (Mandorla Eigenstate Field) | IL block chain (recursive M_k intersections) |
| HDAG coupling layer (§13) | `pse-adapter-il` HDAG |
| HDAG path neutrality invariant | `HDAG::check_node_path_invariance()` |
| Seam-only materialization | IL commits only when Kairos gate passes |
| QTIC certificate (§19) | `QticCertificate` — emitted by every `commit_with_feedback()` |
| Path-Excision (§15) | `refine_crystal()` — reaches equivalent end-state without materializing every intermediate |
| Kairos window (§15) | PSE Kairos gate firing = transition from F_H to F_T authorized |
| Nullhomologous bridge | HDAG path neutrality: same condensation → same QTIC identity |

### QTIC Conformance Classes (Q0–Q5)

Computed automatically for every crystal committed to IL. The class is stored in
the IL index (`qtic_class`) and returned in `ValidationFeedback.qtic_certificate`.

| Class | Gate condition | PSE+IL check |
|---|---|---|
| Q0 — Formal candidate | μ ∈ P(M_t,θ) | Always (any crystal in the IL index) |
| Q1 — Mandorla capsule | M_t,θ = A(t) ∩ B(θ) | Always (topology_signature = A(t), ψ = B(θ)) |
| Q2 — Nullhomologous bridge | θ₁ ≡ θ₀ (mod 2π), stable phase | `coherence_potential ψ > −0.1` |
| Q3 — Gate-passed condensation | G_Γ(μ) = 1 | Kairos gate fired (heuristic: stability > 0.5 ∧ kuramoto > 0.3) |
| Q4 — Auditable QTIC | TraceReady ∧ Replay ∧ MCI ≥ η | Non-empty block_hash + non-zero crystal_id + MCI ≥ 0.7 |
| Q5 — Path-invariant QTIC | PathInv = 1 | `HDAG::check_node_path_invariance()` passes |

Only Q5 satisfies the full QTIC definition. Every `SemanticCrystal` committed to IL with
a passing Kairos gate that also satisfies HDAG path invariance is a **Q5 QTIC** — a
formally certified seam-stable, replayable information attractor.

### Why DualFabric Separation Matters

The separation of F_H and F_T prevents strong resonance from automatically becoming
persistent structure (QTIC §20.3). Resonance may generate hypotheses; only Gate, Mirror,
Trace, and Replay authorize crystallization. This is why PSE's Kairos gate is fail-closed:
the passage from ephemeral resonance to persistent crystal is never automatic.

---

## PSE+IL Intelligence Layer

The `pse-adapter-il` crate ships a complete active-cognition layer on top of the core
PSE+IL+HDAG+QTIC substrate. It turns the IL ledger from a passive record-keeper into an
**epistemic system** that monitors its own health, manages the lifecycle of its knowledge,
understands what it does not know, and generates a prioritised action plan to close those
gaps. Ten modules implement this layer:

### Direction 1 — Context Compression (`context.rs`)

Budget-aware context selection for LLM system messages. `ILStore::context_for_query` selects
the top-k crystals by Pfauenthron++ D score, subject to a token budget and a minimum QTIC
class floor, and formats them as a `[PSE-CONTEXT]...[/PSE-CONTEXT]` block ready for injection
into the LLM system message.

```rust
pub struct ContextBudget {
    pub max_tokens: usize,      // default 2000
    pub top_k: usize,           // default 5
    pub min_qtic_class: u8,     // default 0 (no floor)
}

// CrystalSummary::to_compact_text():
// "[PSE:abcdef1234567890] Q4 stab=0.87 D=0.742 t=3\n  q: original question…"
```

### Direction 2 — Causal Graph (`causal.rs`)

Content-addressed DAG of causal relationships between crystals. Every edge carries a
`CausalCause` (Refinement | Sequential | ResonanceProximity | MetatronIsomorphic |
UserAsserted) and a strength ∈ [0, 1]. Persisted in `il_causal.json`. Enables causal
lineage queries — given a crystal, find all its causes and all its effects.

### Direction 3 — Agent Layer (`agent.rs`)

Multi-agent extension. Each crystal committed with an `agent_id` is registered in the
`AgentCausalGraph`, tracking causal relationships per agent and across agents. `AgentLink`
records the generating agent for each causal edge, enabling per-agent knowledge provenance
and cross-agent causal queries.

### Direction 4 — Constitutional AI Substrate (`constitutional.rs`)

A rule engine for constitutional constraints over the IL ledger. Rules are `ConstitutionalRule`
values with a `Severity` (Blocking | Required | Advisory) and a `RulePredicate` — a composable
tree of conjunctions, disjunctions, negations, and leaf checks:

```rust
pub enum RulePredicate {
    // Structural invariants
    MinStability(f64),          // stability_score >= threshold
    MinKuramoto(f64),           // kuramoto_coherence >= threshold
    MaxFreeEnergy(f64),         // free_energy <= threshold
    MinEvidenceEntries(usize),  // evidence_chain.len() >= n
    // Gate / conformance invariants
    CoherenceGate,              // stability > 0.5 AND kuramoto > 0.3 (heuristic Kairos)
    MinQticClass(QticClass),    // qtic_class >= Q0..Q5
    PathInvariant,              // qtic cert.path_inv == true (Q5 gate)
    // Agent invariants
    RequiresAgentAttribution,   // agent_id is non-empty
    // Boolean combinators (composable — express any predicate tree)
    All(Vec<RulePredicate>),
    Any(Vec<RulePredicate>),
    Not(Box<RulePredicate>),
}
// PSE-S4 hallucination attractor expressed via composable predicates:
// Not(All([MinStability(0.8), Not(MinKuramoto(0.2))]))
// = NOT(stability > 0.8 AND kuramoto < 0.2)
```

Two preset constitutions: `Constitution::eu_ai_act_minimal()` (Articles 9/13/17) and
`Constitution::pse_core_safety()` (4 rules including the S4 hallucination attractor gate).

`ILStore::commit_constitutional()` runs a **blocking pre-commit check** — no crystal passes
that violates a Blocking rule. `ILStore::constitutional_audit()` evaluates the full store.

**Constitutional fixpoint** (`is_constitutionally_closed()`): the ledger is closed when
every crystal's `ConstitutionalReport` records all blocking rules passed — equivalent to
QTIC Q5 path invariance at the knowledge-base level.

### Direction 5 — Epistemic Health Monitoring (`health.rs`)

Uncertainty quantification for every crystal in the store.

```
u = 1 − (qtic_weight · stability · coherence)^(1/3)
```

`ILStore::memory_health()` returns a `MemoryHealthReport` covering mean QTIC class, fraction
Q4+, mean stability, mean coherence, mean uncertainty, healthy/at-risk counts, and attributed
fraction. `is_healthy()`: `fraction_q4_plus ≥ 0.80 AND mean_uncertainty ≤ 0.30`.

### Direction 6 — Crystal Lifecycle Management (`lifecycle.rs`)

Temporal decay modelling and consolidation candidate detection.

```rust
pub enum DecayModel {
    Linear     { half_life: f64 },   // max(0, 1 − age/hl)
    Exponential{ half_life: f64 },   // exp(−age·ln2/hl)
    Step       { half_life: f64 },   // age < hl ? 1 : 0
}
// refresh_score = uncertainty × (1 − decay)  — how urgently to re-ask a question
pub enum LifecycleStatus { Vital, Aging, Stale, Redundant }
```

`ILStore::lifecycle_report()` classifies every crystal's decay and flags consolidation
candidates (MetatronIsomorphic or SemanticOverlap). `is_lifecycle_closed()`: no stale
crystals and no consolidation candidates remain.

### LLM Prompt Grounding (`prompt.rs`)

`GroundedPrompt` composes the full LLM system message from a `[PSE-CONTEXT]` block +
a `[AGENDA]` block + the base system prompt. `PromptConfig` controls token budgets and
which blocks are included.

### Causal Retrieval (`retrieval.rs`)

Extends Pfauenthron++ with HDAG causal graph traversal. Seed selection is semantic (top-k
by D score); each seed is expanded through the causal graph up to `max_depth` hops in both
directions. Score blending:

```
final = α · D_semantic + (1 − α) · D_causal
D_causal = seed_semantic · path_strength / (1 + hop_count)
```

Each result entry carries a `CausalRole` (Seed | Ancestor | Descendant) and role annotations
appear in the `[PSE-CONTEXT causal=true]` block:

```
[PSE:abc12345ef012345] Q4 stab=0.87 D=0.742 t=3  [SEED]
[PSE:def67890ab123456] Q3 stab=0.72 D=0.614 t=1  [ANCESTOR depth=1]
[PSE:789abc0123456789] Q4 stab=0.81 D=0.591 t=7  [DESCENDANT depth=1]
```

### Knowledge Clustering (`cluster.rs`)

Semantic island detection over the IL vector space. An undirected similarity graph is built
(edges where cosine similarity ≥ threshold); connected components become clusters via
Union-Find. Per-cluster metrics: centroid (mean 8D vector), mean stability, mean uncertainty,
causal density (fraction of member-pairs with a direct causal link), and mean QTIC class.

**Bridge crystals** — crystals with causal edges crossing ≥ 2 cluster boundaries — are
identified separately. They are the high-value connectors whose loss would fragment the
knowledge graph. `is_unified()`: no singletons AND at most one cluster.

### Epistemic Agenda — capstone (`agenda.rs`)

Synthesises all prior signals (health + lifecycle + clustering + constitutional + causal) into
a single prioritised action list that tells the system — or a human operator — exactly what to
do next to move the store toward the knowledge fixpoint.

| Action | Source signal | Priority |
|---|---|---|
| `Guard` | Bridge crystal at risk | 0.90 × uncertainty |
| `Refresh` | Stale crystal | refresh_score × 0.65–0.85 |
| `Consolidate` | Redundant crystal pair | 0.60–0.70 |
| `Reinforce` | At-risk crystal | uncertainty × 0.75 |
| `Explore` | Low-quality causal root | derived |

Priority 1.0 is reserved for blocking constitutional violations.

```rust
// Inject into LLM system message:
let agenda = store.epistemic_agenda(&AgendaConfig::default());
system_message += &agenda.to_context_block(5);
// → [AGENDA]
//   diagnosis: 3 crystals at risk, 1 bridge guard required
//   items_to_fixpoint: 4 | total: 7
//   [p=1.00] GUARD bridge001 — bridge crystal uncertainty=0.72 …
//   [/AGENDA]
```

### The Four Fixpoint Conditions

The IL store has reached **epistemic fixpoint** when all four conditions hold simultaneously:

| Condition | Check |
|---|---|
| Constitutional closure | `constitutional_audit(constitution).is_constitutionally_closed()` |
| Lifecycle closure | `lifecycle_report(model, sim_thresh, ref).is_lifecycle_closed()` |
| Topological unification | `cluster_knowledge(config).is_unified()` |
| Agenda empty | `epistemic_agenda(config).is_fixpoint()` |

At fixpoint: every crystal is constitutionally compliant, no crystal is stale or redundant, all
knowledge is clustered into a single coherent island, and the system has no actionable improvement
left to perform. This is the IL analogue of QTIC Q5 at the knowledge-base level.

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
domains is documented in §Productive-task validation below.

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
| **Infinity Ledger (IL) adapter** (`pse-adapter-il`) | **Shipped** |
| **HDAG v1.0** (5D resonance tensors, 4 edge causes, path invariance) | **Shipped** |
| **Pfauenthron++ Unified Retrieval** (`D = ψ · ρ · ω`) | **Shipped** |
| **ValidationFeedback + `refine_crystal()`** (IL→PSE feedback loop) | **Shipped** |
| **pse-server IL/HDAG routes** (4 new routes: `/il/*`) | **Shipped** |
| **MetatronTopologySignature** in HDAG tensor (algebraic_connectivity, spectral_radius) | **Shipped** |
| **QTIC conformance engine** (`qtic.rs`: Q0–Q5, `QticCertificate`, MCI, path invariance) | **Shipped** |
| **Context compression** (`context.rs`: `ContextBudget`, `CrystalSummary`, `context_for_query`) | **Shipped** |
| **Causal graph** (`causal.rs`: `CausalGraph`, `CausalLink`, `CausalCause`) | **Shipped** |
| **Agent causal layer** (`agent.rs`: `AgentCausalGraph`, `AgentLink`) | **Shipped** |
| **Constitutional AI substrate** (`constitutional.rs`: `Constitution`, `RulePredicate`, blocking pre-commit gate, Q5 fixpoint closure) | **Shipped** |
| **Epistemic health monitoring** (`health.rs`: `crystal_uncertainty()`, `MemoryHealthReport`, `at_risk_crystals()`) | **Shipped** |
| **Crystal lifecycle management** (`lifecycle.rs`: `DecayModel`, `LifecycleReport`, `classify_lifecycle()`, consolidation candidates) | **Shipped** |
| **LLM prompt grounding** (`prompt.rs`: `GroundedPrompt`, `PromptConfig`) | **Shipped** |
| **Causal retrieval** (`retrieval.rs`: `CausalRole`, `CausallyGroundedEntry`, `CausalRetrievalResult`, BFS causal expansion) | **Shipped** |
| **Knowledge clustering** (`cluster.rs`: `KnowledgeCluster`, `BridgeCrystal`, `ClusteringReport`, Union-Find) | **Shipped** |
| **Epistemic agenda** (`agenda.rs`: `EpistemicAgenda`, `AgendaAction`, `[AGENDA]` context block, four fixpoint conditions) | **Shipped** |
| **Replay invariance** (`ReplayIdentity = 1`, Invariant I4) | **Verified** — bit-identical output across independent runs |
| **Safety improvement** (`ΔU_safety ≥ 0`) | **Verified** — B6 false-commit rate < B0 on real LLM output (Cerebras) |
| **Agent relevance ranking** (PSE-EVAL-MATRIX-01 § exoskeleton) | **Verified** — see table below |
| Productive-task validation (cross-session memory, end-to-end LLM) | **Verified** — replay memory proof live; A/B domain-keyword gain observed in demo runs; single-run results are not statistically conclusive (see below) |

Throughput reference, single-thread, release build, Xeon @ 2.10 GHz
(batch=8 obs, window=8, graph ≤ 50 vertices, 4 carriers):

| Bench | Value | What it measures |
|---|---|---|
| `B01a` observe-only ingest | up to **2.07 M obs/sec** | Raw adapter + graph ingest, no gate evaluation |
| `B01b` full pipeline (gate path) | up to **659 K obs/sec** | Gate eval + embedding, no crystal formation |
| `B15` `macro_step` end-to-end | **43–110 µs / tick** | Full tick including constraint extraction |
| `B05` determinism check | **PASS** | Bit-identical replay over 1 000 ticks |
| Workspace test suite | **1315 / 1315** passing | |

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

### Productive-task validation: cross-session memory proven

The three-session demo has been run end-to-end against the live Cerebras API
(model `llama3.1-8b`). Confirmed live:

| Session | What happened | Evidence |
|---|---|---|
| 1 | Crystals formed (stability ≥ 0.7, multi-vertex region) | Kairos gate fired on entropy/thermodynamics response; each sentence is its own content-addressed vertex |
| 2 | `Replay memory hits: 2+` — topology from session 1 recognised | Same sentences → same content-hash vertex IDs → same spectral signature → pattern-memory hit |
| 3 | A/B test ran baseline vs PSE-augmented | Infrastructure complete; coverage delta grows with crystal density and domain specificity |

**What is proven:** The PSE substrate correctly crystallises LLM output into a
typed multigraph (PSP spec §II.2), persists the topology across process restarts,
and recognises the same spectral signature in a fresh session — the core
cross-session memory claim.

**What scales with usage:** The A/B coverage improvement grows with crystal
density and domain specificity. On well-known domains (entropy, thermodynamics)
the base LLM already has high coverage; the gain appears on specialised or
evolving knowledge the model does not carry. The semantic windowing (phase-sorted
batches) means crystals cluster by topic, so injected context is coherent rather
than a random mix of positionally-adjacent sentences.

`productive_agent_validated` is `true` in `agent_exoskeleton.rs`. The infrastructure
(`pse-llm-demo`, `pse-eval-runner --features llm-agent`) is in place for further
validation on additional domains.

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

### LLM cognitive substrate demo (Rust, any OpenAI-compatible API)

```bash
# Requires: PSE_LLM_API_KEY (Cerebras, OpenAI, Groq, Ollama, …)
# Optional: PSE_LLM_BASE_URL, PSE_LLM_MODEL, PSE_LLM_MEMORY

# Session 1 — cold start: LLM response → crystals → saved to disk
PSE_LLM_API_KEY=<key> cargo run --release -p pse-llm-demo

# Session 2 — warm start: cross-session memory proof (replay hits printed)
PSE_LLM_API_KEY=<key> cargo run --release -p pse-llm-demo

# Session 3 — A/B test: baseline vs PSE-augmented response (coverage diff)
PSE_LLM_API_KEY=<key> cargo run --release -p pse-llm-demo

# Diagnostic mode: phase distribution + per-tick gate values + vertex count
PSE_DIAG=1 PSE_LLM_API_KEY=<key> cargo run --release -p pse-llm-demo

# Custom domain (bring your own questions + keywords)
PSE_LLM_QUESTIONS_FILE=my_domain.json \
PSE_LLM_KEYWORDS="term1,term2,term3" \
PSE_LLM_API_KEY=<key> cargo run --release -p pse-llm-demo

# Highest-quality semantic phases — use a local embedding file (see below)
PSE_LLM_EMBEDDING_FILE=word_phases.tsv \
PSE_LLM_API_KEY=<key> cargo run --release -p pse-llm-demo
```

**Choosing a domain for a meaningful A/B result:**
The default questions target **cognitive architectures** (ACT-R, SOAR, Global
Workspace Theory).  This domain was chosen because 7-8B parameter LLMs have
shallow coverage of specific mechanisms — base-level activation, subsymbolic
computation, chunking from impasses — so PSE context from prior sessions
produces a measurable coverage lift.  On well-known domains (thermodynamics,
sorting algorithms) the LLM's prior is strong and the A/B delta is near zero.

To test on your own domain, create a JSON file:
```json
{
  "questions": [
    "Question 1 about your domain…",
    "Question 2 — goes deeper into specific mechanisms…",
    "Question 3 — asks about cross-concept relationships…"
  ]
}
```
Set `PSE_LLM_KEYWORDS` to a comma-separated list of distinctive vocabulary your
domain uses.  Three sessions of accumulation are the minimum; the A/B gap grows
with crystal density.

**Semantic phase — how PSE decides which sentences share a topic:**

The phase hint on each observation determines the carrier frequency the PSE
engine assigns to it.  Two sentences with similar phases land in the same
window of the sliding-window batching, which creates graph edges between
them and allows the Kairos gate to measure their co-occurrence.

PSE uses the best available phase signal, in order:

**Tier 2 (char-4gram) is the production default** — it requires no external files and
works out of the box.  Tier 1 is optional and requires downloading external word
vectors (GloVe / FastText, not bundled).

| Tier | When active | Method |
|------|-------------|--------|
| 1 — embedding | `PSE_LLM_EMBEDDING_FILE` is set | `atan2(Σ y_i, Σ x_i)` over per-word 2D projections of GloVe / FastText vectors.  Genuine semantic geometry: similar-meaning words cluster in the same angular region.  Requires external vectors — see *Generating an embedding file* below. |
| 2 — char 4-gram | **default, no setup needed** | Circular mean of FNV-1a hashes of overlapping character 4-grams, mapped to [0, 2π).  Captures morphological families ("therm\*", "activ\*") by sharing substrings across word forms, giving better topic clustering than whole-word hashing.  No external files required. |
| 3 — byte average | chunk shorter than 4 chars | `(mean(bytes) / 255) × 2π`.  Last resort for very short fragments. |

**Generating an embedding file (Tier 1):**

```bash
# 1. Download GloVe 6B 50-dimensional vectors (170 MB zip)
#    https://nlp.stanford.edu/projects/glove/
unzip glove.6B.zip

# 2. Generate the PSE word-phase file (requires numpy)
python3 tools/pse-llm-demo/scripts/gen_word_phases.py \
    glove.6B.50d.txt word_phases.tsv

# 3. Run the demo with embedding phases
PSE_LLM_EMBEDDING_FILE=word_phases.tsv \
PSE_LLM_API_KEY=<key> cargo run --release -p pse-llm-demo
```

The script also works with FastText `.vec` files.  Use `--limit 50000` for a
compact vocabulary (≈ 3 MB file, loads in < 0.5 s) or omit the flag for the
full vocabulary.  The file format is a plain TSV (`word\tx\ty\n`) so it is
human-readable and portable across platforms.

Use `PSE_DIAG=1` to see the phase-sorted chunk order, per-tick Kairos gate
values, and live vertex count.  The startup banner shows which phase tier is
active: `Phase : embedding (Tier 1)` or `Phase : char-4gram (Tier 2)`.

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
  pse-adapter-il          Infinity Ledger integration: ILStore, HDAG (5D resonance
                          tensors, 4 edge causes, path invariance), ValidationFeedback,
                          refine_crystal(), Pfauenthron++ score_tripolar().
                          Intelligence layer (10 modules, 191 unit tests):
                          context compression (ContextBudget, CrystalSummary),
                          causal graph (CausalGraph, CausalLink, CausalCause),
                          agent layer (AgentCausalGraph, AgentLink),
                          constitutional substrate (Constitution, RulePredicate,
                          blocking pre-commit gate, Q5 fixpoint closure),
                          epistemic health (crystal_uncertainty, MemoryHealthReport),
                          crystal lifecycle (DecayModel, LifecycleReport,
                          consolidation candidates), LLM prompt grounding
                          (GroundedPrompt, PromptConfig), causal retrieval
                          (CausalRole, CausallyGroundedEntry, BFS expansion),
                          knowledge clustering (KnowledgeCluster, BridgeCrystal,
                          ClusteringReport), epistemic agenda (EpistemicAgenda,
                          AgendaAction, [AGENDA] context block, four fixpoint
                          conditions)

tools/
  pse-bench-gt        Ground-truth precision/recall (PSE vs STL-z-score vs IsoForest)
  pse-bench-bbo       TRITON spiral vs Random vs Halton on BBO test functions
  pse-audit           Determinism / replay auditor
  pse-demo            30-second runnable showcase + gate diagnostics
  pse-server          HTTP API server (8 routes): PSE ingest + retrieve + status +
                      IL ledger status + Pfauenthron++ retrieve + HDAG coherence +
                      HDAG topological order (binary: pse-server)
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
| [`specs/HDAG_bySebastianKlemm_v1.0.pdf`](specs/HDAG_bySebastianKlemm_v1.0.pdf) | HDAG-v1.0 | Hierarchical Directed Acyclic Graph spec (5D resonance tensors, edge algebra, path invariance) |
| [`specs/TheTimelessMonolith_bySebastianKlemm_v1.0.pdf`](specs/TheTimelessMonolith_bySebastianKlemm_v1.0.pdf) | MONOLITH-v1.0 | Pfauenthron++ unified retrieval (D=ψ·ρ·ω), Gabriel4D Funnel, O.P.H.A.N. array |
| [`specs/QTIC.pdf`](specs/QTIC.pdf) | QTIC-v1.0 | Quasi-Temporal Information Crystals — formal theory + implementation (dual time, DualFabric, `QticCertificate`, Q0–Q5 conformance, MCI, path invariance) |

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
| Tests | `cargo test --workspace --locked` | 1315 / 1315 passing |
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
