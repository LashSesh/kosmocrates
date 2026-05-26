# Getting Started with PSE (Post-Symbolic Engine)

PSE is a two-tier cognitive substrate for structured stream analysis and constraint-driven planning.

- **Tier 1 (pse-core)**: Observation streams → 5D graph embedding → 8-metric Kairos gate → SemanticCrystal
- **Tier 2 (pse-traverse)**: ProblemSpec JSON → FieldCube → DoFGraph → CollapsePlan → Candidate

---

## Prerequisites

- Rust toolchain **1.82 or newer** (`rustup update stable`)
- Clone the repository and `cd` into it

```bash
git clone https://github.com/lashsesh/pse
cd pse
```

No external C libraries or network access required for any core path.

---

## Quick Start: your first crystal in under 5 minutes

The demo drives a synthetic damped oscillator through the full Tier 1 pipeline and
reports throughput, crystal formation rate, and the SHA-256 of the first crystal.

```bash
cargo run --release -p pse-demo
```

Expected output (exact values vary by hardware; crystal count is deterministic):

```
PSE Demo — synthetic damped oscillator with regime shift
=========================================================

Ticks: 600, sliding window: 8
Mode: adaptive Kairos (target_pass_rate=0.30, window=150, warmup=50)

Results
-------
Wall time:          0.766s
Observations:       4772 (across 600 macro_steps)
Throughput:         6229 obs/sec
Kairos passes:      41 / 600
Crystals formed:    7
Crystal rate:       3.36 /sec
First crystal SHA:  53e550d2d1344128…

Gate diagnostics (600 ticks observed)
------------------------------------
metric       mean  threshold    fails
d          0.0478     0.0000        0
q          0.6755     0.5000       84
r          0.8792     0.5000        0
g          0.5376     0.5000      158
j          0.8475     0.5000        2
p          0.8792     0.5000        0
n          0.7304     0.5000        0
k          0.6755     0.5000       84

Bottleneck gate:    g (failed 158/600 ticks; …)

Wrote pse-demo.json
```

You will see the SHA of the first crystal and a `pse-demo.json` artifact. The exact
number of crystals (typically 5–15) depends on how the adaptive calibration converges
across the four regime shifts in the synthetic oscillator.

### Why metric `d` has threshold 0.0

The deformation metric `d` measures graph-structure change from one sliding window to
the next. In streaming mode this converges to near-zero as the window stabilises, so
any positive threshold would permanently silence the gate. The demo sets `d = 0.0`
(always pass) and lets the other seven metrics — especially `q` (coherence) and `g`
(gradient) — do the discriminating. This is documented in
`Config::preset_anomaly_detection()` and applies to all sliding-window workloads.

If you disable adaptive mode (`PSE_DEMO_ADAPTIVE=0 cargo run --release -p pse-demo`)
you will see 0 crystals with all static thresholds at 0.5 — useful for observing which
metrics are closest to their cut points without the adaptive calibrator adjusting them.

---

## Tier 1: Feeding your own data

### Minimal integration

```rust
use pse_core::{macro_step, GlobalState};
use pse_graph::PassthroughAdapter;
use pse_types::Config;

fn main() {
    let mut config = Config::default();

    // Enable adaptive calibration — required for crystal emission on most
    // real-world workloads without manual threshold tuning.
    // See "Calibration guide" below for alternatives.
    config.calibration.enabled = true;
    config.calibration.target_pass_rate = 0.05;  // fire on top 5% of ticks

    // Enable adaptive carrier tracking (Strand J).
    // Re-selects the active carrier each tick to maximize coherence with
    // the incoming data stream. Recommended for any streaming workload.
    config.carrier.adaptive = true;

    let mut state = GlobalState::new(&config);
    let adapter = PassthroughAdapter::new("my-source");

    loop {
        let batch: Vec<Vec<u8>> = collect_next_batch(); // your data here
        match macro_step(&mut state, &batch, &config, &adapter) {
            Ok(Some(crystal)) => {
                let id: String = crystal.crystal_id.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect();
                println!("Crystal: {} stability={:.3}", id, crystal.stability_score);
            }
            Ok(None) => { /* gate did not fire this tick */ }
            Err(e) => eprintln!("engine error: {}", e),
        }
    }
}
```

### Using a domain adapter with semantic phases

`PassthroughAdapter` treats observations as opaque bytes. To unlock the full
coherence path, implement `DomainAdapter` and provide a semantic phase hint so
similar observations receive similar phases (instead of uncorrelated SHA-256
avalanche phases):

```rust
use pse_core::DomainAdapter;
use pse_types::SemanticCrystal;

pub struct MyAdapter;

impl DomainAdapter for MyAdapter {
    fn domain_name(&self) -> &str { "my-domain" }

    fn encode_observation(&self, raw: &[u8]) -> Vec<u8> {
        // normalise, project, or enrich the payload here
        raw.to_vec()
    }

    fn validate(&self, crystal: &SemanticCrystal) -> bool {
        crystal.stability_score > 0.4
    }
}
```

For a semantic phase hint, use `EventScopedAdapter` from `pse-bench-gt` (as used
in the demo) or set `observation.phase_hint = Some(phi)` on each `Observation`
before ingestion.

### Batch sizing

PSE ingests observations in batches per `macro_step` call. Each call is one tick.
Typical guidance:

| Workload | Batch size |
|---|---|
| Real-time events | 1–8 observations per tick |
| IoT / telemetry streams | 10–50 observations per tick |
| Historical replay | 50–200 observations per tick |

Use a sliding window (as the demo does) when observations are a continuous stream:
submit `payloads[k-W+1..=k]` at tick `k`.

---

## Tier 2: Solving a planning problem

Tier 2 takes a `ProblemSpec` JSON, builds a constraint lattice (FieldCube → DoFGraph),
and emits a deterministic `CollapsePlan`.

### Run the minimal example

```bash
# inspect the FieldCube and DoFGraph derived from the spec
cargo run --release -p pse-traverse-cli -- inspect \
    --problem crates/pse-traverse/examples/problem_minimal.json

# emit a CollapsePlan as canonical JSON
cargo run --release -p pse-traverse-cli -- plan \
    --problem crates/pse-traverse/examples/problem_minimal.json \
    --out plan.json

# full run: CollapsePlan + PSE Tier 1 bridge attempt + report
cargo run --release -p pse-traverse-cli -- run \
    --problem crates/pse-traverse/examples/problem_minimal.json \
    --out report.json
```

### Structure of a ProblemSpec

```json
{
  "id": "my.problem.v1",
  "title": "Example planning problem",
  "objective": "Select an implementation layout satisfying all hard constraints.",
  "domain": "software_synthesis",
  "inputs": [
    { "id": "readme", "kind": "text", "path": "README.md" }
  ],
  "constraints": [
    {
      "id": "c.no_network",
      "kind": "hard",
      "predicate": "core_has_no_network_dependency",
      "weight": 1.0,
      "dimensions": ["d.layout"]
    }
  ],
  "dimensions": [
    {
      "id": "d.layout",
      "label": "Crate Layout",
      "kind": "Enum",
      "values": { "Enum": ["single_crate", "split_crates"] },
      "required": true,
      "source": "user"
    }
  ],
  "desired_outputs": [
    { "id": "o.plan", "kind": "collapse_plan" }
  ],
  "risk_policy": { "fail_closed": true, "allow_oracle": false },
  "replay": { "seed": 42, "canonical": true },
  "metadata": {}
}
```

The 10 formal layers (signature, dynamics, horizon, cognition, phase-matrix, topology,
and others) are derived automatically from the spec. `fail_closed: true` means the
planner rejects candidates that cannot prove constraint satisfaction rather than
guessing.

### Tier 2 layer pipeline at a glance

1. **Signature** — extracts the operator algebra from constraints and dimensions
2. **Dynamics** — models how dimension values evolve under operator application
3. **Horizon** — bounds the reachable plan space (causal admissibility)
4. **Cognition** — 5D cognitive state tracks uncertainty across candidate steps
5. **Phase-matrix** — dual-antiphase interference between candidate projections
6. **Topology** — Betti/spectral certificates on the plan graph

`inspect` dumps all six layers as JSON so you can see what the engine derived.

---

## Understanding gate diagnostics

The Kairos gate fires when **all 8 metrics simultaneously exceed their thresholds**.
The demo writes `pse-demo.json` with per-metric means and fail counts. Enable
`tracing` to see per-tick detail:

```bash
RUST_LOG=pse_core=debug cargo run --release -p pse-demo 2>&1 | head -40
```

Each rejected tick logs the metric values alongside thresholds:

```
DEBUG kairos rejected tick=17 d=0.7500 d_thr=0.5 q=0.0000 q_thr=0.5 ...
```

### Metric reference

| Metric | Name | What it measures |
|---|---|---|
| `d` | Deformation | Mean relative embedding change across vertices; vertex-set churn. High at regime shifts. |
| `q` | Coherence | Fraction of the carrier's intrinsic coherence preserved by the data stream's phase distribution. |
| `r` | Resonance | Exponential decay of the 5D state vector's distance from the origin. |
| `g` | Readiness | Composite: `γ_d·d + γ_q·q + γ_r·r`. Overall readiness signal. |
| `j` | Double-kick | Edge/vertex ratio in the observation graph. Tracks relational density. |
| `p` | Projection | Stability of the H5 state as measured from the origin. |
| `n` | Seam | Phase coherence of the Mandorla delta-phi. High when the carrier-data interference is clean. |
| `k` | Crystal score | `λ_C·q + λ_E·(1 − Δφ/π)`. Direct precursor to crystal emission. |

**Typical bottleneck pattern**: if the gate diagnostics show `q` failing 90% of ticks,
the data stream's phases are uncorrelated (hash-derived). Add a semantic `phase_hint`
or use an adapter that computes domain-aware phases. If `d` fails most ticks on a
streaming workload, the graph changes too incrementally — widen the sliding window or
lower `d`'s threshold.

---

## Calibration guide

### Option 1: Adaptive calibration (recommended starting point)

Adaptive calibration tracks a rolling history of gate snapshots and sets each
threshold to the `(1 − target_pass_rate)`-th quantile of that history. The gate fires
on the **top `target_pass_rate` fraction** of recent ticks — i.e., only when the
current resonance is exceptional relative to recent context.

```rust
use pse_types::Config;

let mut config = Config::default();
config.calibration.enabled = true;
config.calibration.target_pass_rate = 0.05;  // emit crystals for top 5% of ticks
config.calibration.window = 200;             // rolling history size
config.calibration.warmup_ticks = 50;        // use static thresholds during warmup
config.carrier.adaptive = true;
```

`target_pass_rate = 0.05` is a safe default. Use 0.10–0.20 if you want more frequent
crystals; use 0.01–0.02 for high-precision anomaly detection.

### Option 2: Domain presets

For common workloads, use the built-in presets instead of constructing `Config` manually:

```rust
// Streaming sensor / event data (sliding window, adaptive carrier)
let config = Config::preset_streaming();

// Constraint-driven planning / Tier 2 bridge (tighter gate, fail-closed)
let config = Config::preset_planning();
```

`preset_streaming()` enables adaptive calibration and `carrier.adaptive`.
`preset_planning()` uses static lower thresholds (0.30 per gate) without adaptive
calibration — planning artifacts have controlled distributions so static thresholds
are reproducible and deterministic.

### Option 3: Manual threshold tuning

Read the per-metric means from `pse-demo.json` after a calibration run, then set
thresholds just below the p95 of each metric's observed distribution:

```rust
let mut config = Config::default();
// Example values derived from gate diagnostics on your workload:
config.thresholds.d = 0.30;
config.thresholds.q = 0.15;
config.thresholds.r = 0.40;
config.thresholds.g = 0.25;
config.thresholds.j = 0.20;
config.thresholds.p = 0.40;
config.thresholds.n = 0.35;
config.thresholds.k = 0.10;
```

Manual thresholds are deterministically reproducible across replays (unlike adaptive
thresholds, which depend on history). Use them when you need bit-identical replay
guarantees and have pre-characterized the workload distribution.

---

## What a SemanticCrystal means

A `SemanticCrystal` is the fundamental unit of validated knowledge emitted by PSE.

```rust
pub struct SemanticCrystal {
    pub crystal_id: Hash256,           // SHA-256 content address
    pub region: Vec<VertexId>,         // observation graph vertices in this crystal
    pub constraint_program: Vec<...>,  // extracted structural constraints
    pub stability_score: f64,          // composite gate readiness at emission time
    pub topology_signature: TopologySignature, // Betti numbers, spectral gap, etc.
    pub evidence_chain: EvidenceChain, // hash-linked provenance to source observations
    pub commit_proof: CommitProof,     // gate values, consensus scores, carrier info
    pub created_at: CommitIndex,       // tick index at emission
    pub free_energy: f64,              // -(constraints × stability)
    // ...
}
```

Key guarantees:

- **Content-addressed**: `crystal_id = SHA-256(JCS(core_fields))`. Two crystals with
  the same `crystal_id` are structurally identical, regardless of when or where they
  were produced.
- **Evidence-chained**: `evidence_chain` links back to the raw observation digests that
  caused this crystal to form. Auditable without re-running the engine.
- **Gate-provable**: `commit_proof.gate_values` records the exact metric values at
  emission. You can verify that the gate passed by checking
  `gate_values.kairos == true`.
- **Reproducible**: given the same `RunDescriptor` and observation sequence, the engine
  re-emits bit-identical crystals (Invariant I4).

A crystal represents a moment when the observation stream's structure was
**simultaneously novel** (high `d`), **coherent** (high `q`, `k`), and
**consistent** (consensus passed). It is not a prediction or a label — it is a
topologically certified snapshot of a structural pattern the engine observed and could
not reduce to a previously known crystal in its pattern memory.

---

## Next steps

- `docs/POST_SYMBOLIC.md` — theoretical foundations
- `crates/pse-core/examples/` — sensor stream, financial ticks, and accumulation proofs
- `crates/pse-core/examples/cross_session_proof.rs` — pattern memory across runs
- `adapters/` — domain adapters for Binance, IoT, syslog, vitals, weather, and more
- `tools/pse-audit/` — crystal chain auditing and EU AI Act compliance reporting
