# Kosmocrates Production Substrate

> **Standalone documentation for the `kosmo-*` crate layer.**
>
> This document covers the five new crates implemented against the
> *Kosmocrates Spec Corpus Implementation Handoff* (see `specs/`).
> It is kept separate from the PSE base-system documentation in
> [`README.md`](README.md) and [`docs/OVERVIEW.md`](docs/OVERVIEW.md)
> until the substrate has been empirically validated and the two
> layers are ready to be treated as a unified whole.

---

## What this layer is

The production substrate is a **policy-governed, content-addressed,
fail-closed execution layer** that sits above the PSE crystallization
engine and below any domain-specific application.

Its job is to answer one question reliably: *has a structural yield
from a host workspace been shown to be safe enough, evidence-bound
enough, and operator-approved enough to materialize into the host file
system?*

The answer is almost always **no** — and that is by design.
The substrate emits planning artifacts, diagnostics, gate traces, and
content-addressed reports. It does not patch files, execute generated
code, or write to disk without an explicit operator-issued approval
token and a Foundry validation gate.

> **This file documents the CAD/metrology half** — topology analysis and
> ranking, fail-closed, writing nothing on its own. The *generative* half — the
> closed loop that turns a stated intent into a validated workspace change ("the
> wish-to-system machine", CAD/CAM for software) — is documented in
> [`docs/WISH_TO_SYSTEM.md`](docs/WISH_TO_SYSTEM.md).

---

## Crate map

```
kosmo-core          ─── substrate types: Digest, Q16, PolicyProfile,
│                        EvidenceBundle, GateResult, AuthorityLabel, …
│
kosmo-workbench     ─── WorkspaceIndex, FoundryRunner, RunReport
│
kosmo-hyphae        ─── HYPHAE v0.3/v0.4 · Metatron v0.4.1 · LPCM v0.4.2
│
kosmo-systemcube    ─── BlueprintUnit, SystemCubeManifest, KcubeExportReport
│
kosmo-pipeline      ─── run_dry_pipeline(), GateTraceAggregator,
                         IntegrationRunReport, MaterializationPlan
```

All five crates are members of the workspace; none have external
network dependencies. Each crate's public API is stable within the
`claude/youthful-cannon-fzfu7` branch and pinned to the spec sections
listed in [`SPEC_TRACEABILITY.md`](SPEC_TRACEABILITY.md).

---

## Getting started

Four entry points — pick whichever fits your context. No configuration
required beyond a Rust workspace on the filesystem.

### Install

```bash
# From this repo (local)
bash install.sh

# From upstream git
bash install.sh --git

# In Docker (CLI)
docker build -f docker/Dockerfile.kosmo -t kosmo-substrate .
docker run --rm -v $(pwd):/workspace kosmo-substrate /workspace

# In Docker (server)
docker run --rm -p 7777:7777 \
  --entrypoint kosmo-server kosmo-substrate --host 0.0.0.0
```

### 1. CLI — `kosmo-substrate`

```bash
# Analyse current workspace (rich terminal output)
kosmo-substrate .

# Enable all analysis layers; persist crystal CAD library across runs
kosmo-substrate . --all --operator --store ~/.kosmo/cadlib.jsonl

# Generate a Markdown topology report (paste into PR descriptions)
kosmo-substrate . --output markdown > TOPOLOGY_REPORT.md

# JSON dump (pipe to jq, feed into tooling)
kosmo-substrate . --output json | jq '.void_priority_ranking | length'

# CI gate — exit 1 when gate is Reject
kosmo-substrate . --output summary --fail-on-reject
```

### 2. TUI — `kosmo-tui`

```bash
# Interactive terminal dashboard
kosmo-tui .

# With all layers enabled
kosmo-tui . --all

# Keybindings: q=quit  r=rerun  ↑↓/jk=navigate  PgUp/PgDn=page  g/G=top/bottom
```

### 3. Browser UI — `kosmo-server`

```bash
# Start server + open browser automatically
kosmo-server --open

# Specify port / bind address
kosmo-server --port 8080 --host 0.0.0.0
```

Open `http://localhost:7777` — enter a path, toggle flags, click Analyse.

### 4. REST API — `POST /api/analyse`

```bash
curl -s -X POST http://localhost:7777/api/analyse \
  -H 'Content-Type: application/json' \
  -d '{
    "path": "/path/to/workspace",
    "flags": {
      "crystals": true,
      "metatron": true,
      "operator": false
    }
  }' | jq '{gate, void_count, action_count: (.action_items | length)}'
```

Response fields: `gate`, `void_count`, `total_severity`, `action_items[]`,
`void_ranking[]`, `certified_crystals`, `resonite_pairs`, plus optional-layer counts.

`GET /api/health` returns `{ "status": "ok", "version": "..." }`.

### 5. Promotion — `kosmo-promote` (substrate→core)

```bash
# Report-only (default, fail-closed): list what WOULD be offered to PSE;
# the engine is never touched.
kosmo-promote . --all-kinds

# Actually feed the PSE engine (DryRun profile; in-memory, no host writes).
# PSE's own gate cascade alone decides crystallization.
kosmo-promote . --all-kinds --offer

# Machine-readable verdicts (Accepted | Deferred | Rejected | Skipped…)
kosmo-promote . --offer --json | jq '.offers[] | {label, outcome}'

# Accumulate engine memory across sessions (the flag authorizes the write):
kosmo-promote . --offer --state ~/.kosmo/pse-archive.json

# Offer the CAD library (kosmo-substrate --store) to the engine, read-only:
kosmo-promote . --offer --store ~/.kosmo/cadlib.jsonl

# Close the memory→action loop: engine verdicts feed the next run's pipeline
kosmo-promote . --offer --feedback ~/.kosmo/feedback.json

# Resonance: co-observe the candidates as ONE ensemble under the substrate
# calibration — this is the configuration in which substrate knowledge
# actually crystallizes (real SemanticCrystals):
kosmo-promote . --all-kinds --offer --batch --calibration substrate \
    --state ~/.kosmo/pse-archive.json

# Full QTIC: anchor accepted crystals in the Infinity Ledger — ledger block,
# IL-HDAG node, path invariance — lifting certificates from Q3 to Q5:
kosmo-promote . --all-kinds --offer --batch --calibration substrate \
    --ledger ~/.kosmo/il

# Recall: the anchored memory is queryable — Pfauenthron++ (D = ψ·ρ·ω)
# over the ledger's crystals, with the top hit's causal lineage:
kosmo-promote --recall "missing test coverage for a module" \
    --ledger ~/.kosmo/il --top 5
```

Runs the full pipeline, filters its `pse_candidates` (default:
`CertifiedCrystal` only; `--all-kinds` adds Structural/Topology
observations), and offers them through `pse-adapter-kosmo` — the
sanctioned crossing into the PSE crystallization engine.

With `--state`, the engine's crystal archive persists across sessions and
warm-starts `PatternMemory` on the next run (the `pse-core` cross-session
mechanism) — repeated promotions of recurring substrate output build the
resonance that can eventually flip `Deferred` into `Accepted`. Fail-closed:
report-only mode never writes, and a corrupt archive is a hard error, never
silently cold-started over.

With `--store`, the durable CAD library is itself a promotion source:
`StructuralCrystalRecord`s are **directly evidence-bound** (each carries its
certifying candidate's `evidence_bundle_id` — CROSS-006 as a first-class
field), so store-loaded crystals wrap into bridge candidates without
resolving their candidates. The store is integrity-checked before any record
is offered; a tampered record is a hard error.

With `--feedback`, the loop closes in the other direction — **memory shapes
action**: the engine's verdicts are persisted as `PromotionFeedback`
(`Accepted` → full fitness, `Deferred` → ¼, `Rejected`/`Skipped` → zero;
CROSS-010 analogue) and loaded into the *next* run's
`IntegrationRunOptions::prior_feedback`, where pipeline Step 5c folds them
into `NormFitnessTrace`s. Norm-derived candidates key their feedback to the
originating `NormGeneCandidate`; the merge is idempotent by feedback id.
Same authorization discipline as `--state`: report-only never writes,
corruption is a hard error.

**Resonance — how substrate knowledge actually crystallizes.** The engine
forms graph edges *pairwise within a batch*, and each candidate is its own
engine vertex (`KosmoBridgeAdapter::observation_source_id`), so single-step
offers structurally cap the connectivity metric `j` at zero and the 8-fold
conjunctive Kairos gate can never fire. `--batch` (`offer_batch`) co-observes
the candidates as one ensemble — N vertices, pairwise edges, live `j` — with
attribution staying per-candidate and honest: exactly the candidates whose
vertex lies in `crystal.region` are `Accepted`. `--calibration substrate`
(an explicit operator choice, like every threshold change) composes the
planning preset with the `preset_anomaly_detection` rationale: the carrier-
physics consensus stands down and the fully-armed Kairos gate is the
discriminant. `--ticks n` re-observes the ensemble (crystallization is
temporal). The gate diagnostics (`gate (last step): d … j … → kairos | engine`)
show per-metric values against the effective thresholds, so "why deferred" is
always visible. Under `--batch --calibration substrate` a polyglot workspace's
certified structure commits real `SemanticCrystal`s; the conservative default
calibration remains fail-closed and commits nothing.

Every crystal the engine commits receives a **QTIC conformance certificate**
(Q0–Q5, `pse-adapter-il::qtic`). Classes are earned, never granted: the
engine's commitment *is* gate-passed condensation (Q3), and without anchors
the certificate honestly **caps at Q3**, its `trace_ready`/`path_inv` fields
showing exactly what is missing. With `--ledger <path>` every accepted
crystal is **anchored in the Infinity Ledger** — a ledger block (the
canonical trace anchor), an IL-HDAG node (the 5D resonance tensor), and a
path-invariance check — and the certificate lifts to **Q5, full QTIC**: the
same conformance class the cognition system awards its own best memory.
Anchoring is idempotent (an identical crystal re-anchors to the same block;
the ledger does not grow) and a host write, so the flag is the operator's
authorization and nothing is written outside `--offer` mode. `kosmo-promote`
reports class and block per accepted crystal (`QTIC Q5 — …`, `IL: block …`;
`qtic_class`/`block_hash` in `--json`).

And the anchored memory is **queryable**: `--recall <query>` embeds the query
(`text_to_vector8`), ranks every ledger crystal by the Pfauenthron++ tripolar
score `D = ψ·ρ·ω` (semantic × structural × temporal —
`build_context_entries`), and returns the top hits with QTIC class,
stability, provenance (the promotion scope travels as the crystal's
`question`), and the **causal lineage** of the best hit. Recall is read-only
by contract: it never creates a ledger; a missing path is a hard error,
never a silent empty store.

---


## Design invariants

These properties hold for every type in every crate. They are not
conventions — the test suite enforces them structurally.

### Content-addressing everywhere

Every durable object carries an `id: Digest` field computed as
`SHA-256(JCS(content_fields))` where `content_fields` excludes the
`id` itself. Two objects with identical semantic content will always
produce the same `Digest`. The implementation uses `serde_jcs` (RFC
8785 canonical JSON) to guarantee field-order independence.

```rust
// Every ID is deterministic and verifiable.
let p1 = PolicyProfile::default_report_only();
let p2 = PolicyProfile::default_report_only();
assert_eq!(p1.id, p2.id);    // always true
assert!(p1.verify_id());      // content matches stored digest
```

### Q16 fixed-point arithmetic — no floats in audit paths

All gate-relevant numerics use `Q16`: a 64-bit integer scaled by
2^16. Division and ratio operations stay in integer arithmetic;
floating-point never appears in any content-addressed structure or
gate decision. This satisfies CROSS-007.

```rust
let threshold = Q16::from_ratio(51, 100);   // 0.51 exactly
let score     = Q16::from_ratio(73, 100);   // 0.73 exactly
assert!(score > threshold);
```

### PolicyProfile — fail-closed defaults

The default `PolicyProfile` is `ReportOnly`. Every `allow_*` flag is
`false`; every `require_*` flag is `true`. No subsystem may escalate
its own policy.

```rust
let p = PolicyProfile::default();
assert_eq!(p.mode, ImplementationMode::ReportOnly);
assert!(p.check_host_write().is_err());   // CROSS-002
assert!(p.check_network().is_err());
```

The four implementation modes, in order of escalating privilege:

| Mode | Host writes | Execution | Requires |
|---|---|---|---|
| `ReportOnly` | no | no | — |
| `DryRun` | no | isolated sandbox | — |
| `OperatorApproved` | yes | yes | operator token |
| `AutonomousBounded` | yes | yes | pre-approved bounds |

### Evidence-bound durable objects

Every record that survives a run must carry at least one
`evidence_id: Digest` pointing to an `EvidenceBundle`. Structures
without evidence cannot be certified or replayed. This satisfies
CROSS-006 and CROSS-015.

### Deterministic replay

Identical inputs produce byte-identical outputs. All collections are
sorted before hashing; all maps use `BTreeMap`; no `HashMap` or
`HashSet` appears in any content-addressed path.

### Unified tripolar energy — ranks but never gates

`kosmo-core::energy` provides the single selection core `D = ψ · ρ · ω`
(meaning · coherence · phase), in `Q16` integer arithmetic, modulated by
six fail-closed `[0,1]` factors (gate, taint, license, foundry, seam,
contradiction). It replaces the substrate's previously-fragmented
heuristics with one consistent scalar.

The hard invariant (CROSS-010): **energy ranks, it never gates.** A
`Reject` `GateResult` forces the `gate` factor to zero, so a rejected
candidate's energy is zero and can never out-rank a passing one — but a
high `D` can never flip a `Reject` into an `Accept`. There is no method
on the kernel that turns an energy value into a decision or policy
escalation. Selection by energy is always a choice *among* gate-passed
candidates.

```rust
let k = EnergyKernel::new(
    TripolarEnergy::unit(),                 // D = 1, maximal
    EnergyFactors::derive(
        &GateResult::Reject { reason: "no evidence".into() },
        &TaintLabel::Clean, &LicenseStatus::Permissive { spdx: "MIT".into() },
        FoundrySurvival::Passed, Q16::ONE, Q16::ZERO,
    ),
);
assert!(k.is_zeroed());                      // Reject ⇒ zero energy, always
```

---

## Cross-cutting acceptance constraints

The spec defines 15 cross-cutting constraints (CROSS-001 through
CROSS-015). The ones with the highest architectural impact:

| ID | Summary | Where enforced |
|---|---|---|
| CROSS-001 | Default mode is `ReportOnly` | `PolicyProfile::default()` |
| CROSS-002 | Host mutation impossible without explicit policy | `check_host_write()` |
| CROSS-005 | External-tainted context rejected by default | `ContextPack::from_tainted()` |
| CROSS-006 | Every durable record is evidence-bound | `EvidenceBundle` fields |
| CROSS-007 | No floats in gate/digest paths | `Q16`, no `f32`/`f64` in hashed structs |
| CROSS-010 | 51% majority → candidate only, never gate bypass | `local_majority_candidate()` |
| CROSS-012 | Rejected yields have persisted negative evidence | `NegativeEvidenceRecord` |
| CROSS-013 | Report-only mode produces diagnostics, zero host writes | `allow_host_write=false` in all sub-reports |
| CROSS-015 | Every record carries replay status | `ReplayStatus` on `EvidenceBundle` |

---

## Layer 1 — `kosmo-core`

Foundation types used by every other crate in this layer. No
application logic; only data structures, serialization, and
policy enforcement.

**Key modules:**

| Module | Contents |
|---|---|
| `digest.rs` | `Digest` (SHA-256 newtype), `canonical_bytes` (JCS), `Digest::of<T>()` |
| `fixed_point.rs` | `Q16` (i64 × 2^16), arithmetic ops, `from_ratio`, `ratio` |
| `energy.rs` | `TripolarEnergy` (`D = ψ·ρ·ω`), `EnergyFactors`, `EnergyKernel`, `EnergyAssessment`, `rank_by_energy` — the unified selection core (ranks, never gates) |
| `evidence.rs` | `EvidenceRef`, `EvidenceBundle`, `ReplayStatus` |
| `authority.rs` | `AuthorityLabel`, `TaintLabel`, `LicenseStatus`, `CapabilityLock` |
| `policy.rs` | `PolicyProfile`, `ImplementationMode`, `PolicyViolation` |
| `run.rs` | `RunDescriptor`, `GateResult` (merge semantics), `LedgerEvent`, `FoundryCheckResult` |

`GateResult` merges by worst-wins: `Reject > Warn > Pass`. Two gate
traces merged always produce the most restrictive outcome.

```rust
let a = GateResult::Pass;
let b = GateResult::Warn { message: "marginal".into() };
let c = GateResult::Reject { reason: "missing evidence".into() };

assert_eq!(a.merge(&b), GateResult::Warn { .. });
assert_eq!(b.merge(&c), GateResult::Reject { .. });
```

**Test count:** 49 passing, 0 failing.

---

## Layer 2 — `kosmo-workbench`

Workspace scanning, isolated dry-run execution, Foundry checks, and
structured run reports.

**Key types:**

| Type | Role |
|---|---|
| `WorkspaceIndex` | Content-addressed index of workspace files; `scan_path` + `from_entries` |
| `TaskSpec` | Content-addressed task declaration with `TaskKind` |
| `ContextPack` | Evidence-bound context with permitted-use labels; rejects external taint (CROSS-005) |
| `FoundryRunner` | Executes `FoundryCheckSpec`s; respects `ReportOnly` (→ Skipped) vs `DryRun` |
| `RunReport` | Content-addressed run summary; `to_text()` human-readable output |

`FoundryRunner` never mutates the host. In `ReportOnly` mode every
check returns `FoundryOutcome::Skipped`; in `DryRun` mode checks
execute in an isolated environment.

**Test count:** 20 passing, 0 failing (2 integration tests ignored pending
live Foundry environment).

---

## Layer 3 — `kosmo-hyphae`

The largest and most complex crate. Implements four sub-specifications:

### HYPHAE v0.3 — Passive topology assimilation

Pipeline: `HostCube → TopologicalVoidMap → DeficiencyVector →
SourceFrontierGraph → GateCascade → AssimilationDecision`

The `GateCascade` evaluates five gates in sequence — `TaintGate`,
`EvidenceGate`, `VoidRefGate`, `AuthorityGate`, `PolicyGate` — with
no short-circuit: all five always run, the final decision is their
worst-wins merge.

Rejected `StructuralYield`s produce a `NegativeEvidenceRecord` (CROSS-012).
No yield is silently discarded.

`passive_run()` is the entry point. It performs the full v0.3 pipeline
without any host writes.

### HYPHAE v0.4 — Persistent layer

Builds on v0.3 to add:
- `CorpusCartography` — append-only entity/relation store; idempotent
- `StructuralCrystalCandidate` / `StructuralCrystalRecord` / `Resonite`
- `ConstraintProgram` — all_satisfied gate over arbitrary constraint sets
- `AssimilationCertificate` — issued only when `program.all_satisfied()`
- `NormGeneCandidate` / `NormFitnessTrace` — no `is_trusted` field;
  trust escalation requires a full governance path
- `HostTargetCollapsePlan` — planning-only artifact (`PlanningOnly` flag)
- `MorphogenicCorpusUpdate` skeleton

`HostTargetCollapsePlan` is deliberately not executable. It describes
what *would* change; execution requires Phase 11 materialization governance.

### Metatron v0.4.1 — Microtopology diagnostics

M1 pipeline (`lift_region`):
`HostVoidRegion → MetatronMicrograph → MicrographLiftReport →
MetatronRegionFingerprint → MicroTopologyIndex`

M2 pipeline (`diagnose_micrograph`):
`MetatronMicrograph → MicroTopologyDiagnostic → TopologyAmbiguityProfile
→ ComplementVoidHypothesis`

Surgery planning (`TopologicalSurgeryOption::from_diagnostic`):
`MicroTopologyDiagnostic → TopologicalSurgeryOption[]` — planning-only,
no host modifications. Surgery options feed into `SurgeryBackedCollapseStep`
inside a `HostTargetCollapsePlan`.

### LPCM v0.4.2 — Controlled fragmentation

Pipeline: `FragmentField → SupportMassVector → SeamGraph →
monotone_contractive_filter → DoFContractionReport → LpcmPassiveReport`

`local_majority_candidate()` requires strict majority:
`mass.raw() * 2 > total.raw()` — integer arithmetic only. A candidate
with 51% mass is a `CandidateDirection`, never a gate bypass (CROSS-010).

`monotone_contractive_filter` rejects any sequence of masses that is
non-contractive; `MonotoneFilterOutcome::Rejected` carries the first
violating index.

`LpcmPassiveReport::build()` runs the full pipeline. `allow_host_write`
is hardcoded `false` on the output report (CROSS-013).

### CubeSwarm

| Type | Role |
|---|---|
| `SourceCube` | Content-addressed, Q16 support score |
| `CubeSwarm` | Sorted by `cube_id` for deterministic replay |
| `CubeMandorla` | Shared-void detection, sorted `cube_ids` |
| `CompositeSupportCube` | Integer-averaged Q16 aggregate support |
| `HostTargetDelta` | Planning-only, `from_host_and_composite` |

### Cross-language extraction (`xlang`) — polyglot hypercube materialization

Everything from `SourceCube` upward is language-agnostic: the cube pipeline
consumes a `CodeHDAG` and the ρ/ω poles derived from it, never the source text.
The only Rust-specific link was the extractor at the head of the chain. The
`xlang` module removes it, so a Python, JavaScript, Go, C, Java, or C++ file
lifts into the **same** content-addressed `CodeHDAG` a Rust file would — and
flows into the identical hypercube with no downstream change.

| Type / fn | Role |
|---|---|
| `SourceLanguage` | `Rust`, `Python`, `JavaScript`, `Go`, `C`, `Java`, `Cpp`; `from_path` detects by extension, fail-closed (`None` for unknown) |
| `CodeHDAG::extract_from_source` | Language-dispatched lexical extraction; Rust delegates verbatim to `extract_from_rust_source` (byte-identical ids) |
| `CodeHDAG::extract_auto` | Detect language from path and extract, or `None` — the host-scan integration entry point |
| `CrossLanguageFingerprint` | Content-addressed `Q16` structural-ratio vector (function/type/import/test density); `similarity` is integer-only (CROSS-007) |

The taxonomy is mined from the PSE-Codex corpus (10 algorithms × 4 languages)
and its `normalize` Rosetta table — *what counts as* a function, a type, an
import, a test, per language. PSE-Codex's tree-sitter + `f64` spectral/Kuramoto
machinery is deliberately **not** ported: it is float-heavy and dependency-bound,
which the substrate forbids (CROSS-007, no external deps). What carries over is
the taxonomy, re-expressed as a deterministic, dependency-free lexical extractor.

Rust, Python, JavaScript, and Go are **keyword-anchored** (`fn`/`def`/`function`/
`func`) and validated against the PSE-Codex corpus. C, Java, and C++ extend
coverage to the rest of the `normalize` taxonomy; their function definitions
carry no leading keyword, so they use a deliberately **conservative** heuristic
(`detect_clike_function`) that **under-counts rather than emit a false positive** —
a control statement, call, or initialiser is never misread as a definition. Type
and import detection for the C family is exact (`#include`/`import`/`using`,
`struct`/`class`/`interface`/`enum`).

Wiring: `kosmo-workbench`'s scanner now classifies `.py`/`.js`/`.go` (and the
common variants) as source/test files per each language's convention, and
`HostCube` dispatches extraction by detected language. `HostCube` also stores a
`CrossLanguageFingerprint` per void (`fingerprint_by_void_id`, content-addressed
into `cube_id`), and `run_dry_pipeline` adds a `cross_language_resonance`
`SourceCube` dimension — the maximum fingerprint similarity to any *other* file
in the workspace, across languages. So a Python file structurally echoing a Go
file resonates and ranks higher; a structural outlier does not. (Energy ranks,
never gates — CROSS-010.)

The fingerprint also travels into the durable CAD library: a certified
`StructuralCrystalCandidate`/`StructuralCrystalRecord` carries the originating
file's `CrossLanguageFingerprint` (`with_fingerprint`, content-addressed into the
record id). `StructuralCrystalRecord::fingerprint_resonance` then lets the
pipeline's `crystal_resonance` dimension match a void against certified crystals
**across languages and across runs** — a Go crystal can resonate with a
structurally-similar Python void — using the richer four-axis fingerprint when
both sides carry one, and falling back to the two-axis ρ/ω proximity otherwise.

Running `kosmo-substrate` over a polyglot workspace therefore produces voids,
HDAG-scaled severities, fingerprints, and energy-ranked `SourceCube`s for all
supported languages, materializing into the same `.kcube` archive.

**Test count:** 127 passing, 0 failing (HYPHAE core) + 22 cross-language
(`xlang`) + host/scanner integration tests.

---

## Layer 4 — `kosmo-systemcube`

Exportable blueprint layer for producing `.kcube` manifests.

| Type | Role |
|---|---|
| `BlueprintUnit` | Evidence-bound unit; `Accepted`, `RejectedOpaque`, `AcceptedWithTaint` |
| `SystemCubeManifest` | Sorted accepted unit IDs; JSON round-trip stable |
| `ContradictionEnergyReport` | Q16 weight sum, sorted contradiction pairs |
| `CompatibilityProfileReport` | Q16 compatibility score, gaps by unit ID |
| `DDensityReport` | `Q16::ratio(accepted, capacity)`; `Available` or `Unavailable` |
| `SystemCube` | Entry point; `export_dry_run()` |
| `KcubeExportReport` | `DryRun` or `BlockedByPolicy` — never direct disk write |

`export_dry_run()` under a `ReportOnly` policy always returns
`KcubeExportMode::BlockedByPolicy`, even when D-density is 1.0
(CROSS-010: metric saturation does not bypass the policy gate).

**Test count:** 36 passing, 0 failing.

---

## Layer 5 — `kosmo-pipeline`

Wires all sub-systems under a single `PolicyProfile` and aggregates
gate results into a unified report.

### `run_dry_pipeline()`

```rust
pub fn run_dry_pipeline(
    index: &WorkspaceIndex,
    options: &IntegrationRunOptions,
    policy: &PolicyProfile,
) -> IntegrationRunReport
```

Execution order:
1. HYPHAE passive run + v0.4 corpus update
2. Metatron diagnostics (if `enable_metatron`)
3. LPCM passive reports (if `enable_lpcm`)
4. SystemCube dry-run export (if `enable_systemcube`)
5. Gate aggregation → `AggregatedGateResult` → `final_result`

Every sub-report carries the same `policy_id`. The pipeline verifies
this invariant via `verify_policy_consistency()`.

### `GateTraceAggregator`

Merges gate traces from multiple layers:
- Worst-wins: `Reject > Warn > Pass`
- Layer summaries sorted by `gate_trace_id` for deterministic output
- Single `Reject` in any layer propagates to `final_result`

### `IntegrationRunReport`

Content-addressed (`report_id = Digest::of(content)`). Fields:
`policy_id`, `hyphae_result`, `cartography_update`,
`metatron_diagnostics`, `lpcm_reports`, `systemcube_export`,
`aggregated_gate`, `final_result`.

No mutation interface. `allow_host_write` is `false` in the default
pipeline policy (CROSS-013).

### Phase 11 — Operator-Approved Materialization

`MaterializationPlan::evaluate()` is the governance entry point.
It returns `MaterializationOutcome::Blocked` unless all of the
following hold:

1. An `OperatorApprovalToken` is present.
2. The token's `collapse_plan_id` matches the submitted plan.
3. The token authority is `Human` or `Operator` (not `Agent`).
4. The policy mode is `OperatorApproved`.
5. `policy.allow_host_write == true`.

When all conditions pass the outcome is
`MaterializationOutcome::FoundryRequired` — signalling that actual
execution requires a Foundry validation gate. The `MaterializationPlan`
itself never executes anything; it is a governance skeleton.

`simulate_foundry_check()` returns `FoundryCheckResult::Passed` under
an `OperatorApproved` policy and `Skipped` under `ReportOnly`.

**Test count:** 46 passing, 0 failing.

---

## Running the tests

```bash
# Individual crates
cargo test -p kosmo-core
cargo test -p kosmo-workbench
cargo test -p kosmo-hyphae
cargo test -p kosmo-systemcube
cargo test -p kosmo-pipeline

# All substrate crates at once
cargo test -p kosmo-core -p kosmo-workbench -p kosmo-hyphae \
           -p kosmo-systemcube -p kosmo-pipeline
```

Expected result (as of 2026-05-30):

```
kosmo-core:        49 passed,  0 failed,  0 warnings
kosmo-workbench:   20 passed,  0 failed,  2 ignored, 0 warnings
kosmo-hyphae:     127 passed,  0 failed,  0 warnings
kosmo-systemcube:  36 passed,  0 failed,  0 warnings
kosmo-pipeline:    46 passed,  0 failed,  0 warnings
─────────────────────────────────────────────────────
TOTAL:            278 passed,  0 failed,  0 warnings
```

---

## What this layer does not do (yet)

The substrate is structurally complete but empirically unvalidated.
The following capabilities are implemented as governance skeletons or
planning artifacts only — they have no live execution path:

| Capability | Status |
|---|---|
| Host file writes | `OperatorApproved` + `allow_host_write=true`; `DryRun` and `ReportOnly` cannot persist |
| Foundry execution (real) | ✅ `kosmo-foundry`: real `std::process::Command` spawn, allowlist-checked |
| Network acquisition | `allow_network = false` in all shipped profiles |
| NormGene promotion to trusted | Requires governance path not yet specified |
| AutonomousBounded mode | `ImplementationMode` variant exists; no issuing logic |
| `.kcube` disk export | `KcubeExportMode::DryRun` — no actual file I/O |
| Cross-session corpus persistence | ✅ `kosmo-store`: JSONL append-only store, `verify_integrity()` |
| ParseBack topology scan (crate-level) | ✅ `kosmo-parseback`: `cargo metadata`, `CrateFingerprint`, INVARIANT-007 |
| Intra-file code topology (module/import/fn/type/test graph) | ✅ `kosmo-hyphae::code_hdag::extract_from_rust_source` — lexical, dependency-free, content-addressed; ρ/ω feed the energy kernel |
| Cross-language code topology (Rust, Python, JavaScript, Go, C, Java, C++) | ✅ `kosmo-hyphae::xlang` — `extract_from_source`/`extract_auto`, dependency-free lexical, `Q16` `CrossLanguageFingerprint`; keyword-anchored for the first four, conservative heuristic for the C family; polyglot workspaces materialize into the same hypercube |
| Unified tripolar energy selection (`D = ψ·ρ·ω`) | ✅ `kosmo-core::energy` — Q16, content-addressed, ranks-never-gates |
| R1→R2→R3 operator pipeline | ✅ `kosmo-operator`: `OperatorExecutor::execute()`, closure synthesis |
| Empirical validation (52-scenario benchmark) | ✅ `tools/kosmo-eval`: EXIT 0, all 52 scenarios pass |

These are deliberate boundary conditions, not omissions. The weld
seam between planning and execution is where the governance model
earns its keep.

---

## Relationship to the PSE base system

The `kosmo-*` crates **do not modify** any `pse-*` crate. They
share the Cargo workspace but have no compile-time dependency on the
PSE engine. The relationship is conceptual, not structural: PSE
provides the crystallization substrate; this layer provides the
policy-governed topology assimilation substrate that decides what is
worth sending to PSE in the first place.

The integration path — wrapping `StructuralCrystalRecord` in a PSE
observation adapter — is **implemented end to end**, now that the
empirical validation it was gated on has passed (147/147 eval scenarios):

- **Offer side** (`kosmo-pipeline::crystal_to_pse_candidate`): every crystal
  certified in a run is wrapped as a `PseBridgeCandidate` of kind
  `CertifiedCrystal` (confidence `(ρ+ω)/2` in `Q16`, cross-language
  fingerprint as metadata, evidence-bound via the certifying candidate's
  bundle) and included in `pse_candidates` when `enable_pse_candidates`
  is set.
- **Consumption side** (`adapters/pse-adapter-kosmo`): the PSE-side
  `KosmoBridgeAdapter` canonicalizes candidates into PSE `Observation`s
  (fail-closed: unparseable, tampered, evidence-free, or disallowed-kind
  payloads are rejected) and `offer_candidate` feeds them through
  `pse_core::macro_step` under full policy gating — `ReportOnly` profiles
  and denying bridge policies never touch the engine. The `Q16→f64`
  conversion happens exactly at this seam: confidence becomes the
  observation's semantic `phase_hint`, so structurally similar crystals
  can resonate.

The dependency direction holds: no `kosmo-*` crate imports `pse-*`; the
adapter lives on the PSE side and consumes the bridge. PSE runs its own
gate cascade and alone decides crystallization — a committed
`SemanticCrystal` maps to `PromotionOutcome::Accepted`, clean ingestion
without crystallization to `Deferred`, for the substrate's feedback loop.

---

## Key files

| Path | Contents |
|---|---|
| `crates/kosmo-core/src/policy.rs` | `PolicyProfile`, `ImplementationMode`, `PolicyViolation` |
| `crates/kosmo-core/src/fixed_point.rs` | `Q16` |
| `crates/kosmo-core/src/digest.rs` | `Digest`, `canonical_bytes` |
| `crates/kosmo-core/src/energy.rs` | `TripolarEnergy`, `EnergyFactors`, `EnergyKernel`, `EnergyAssessment`, `rank_by_energy` |
| `crates/kosmo-hyphae/src/code_hdag.rs` | `CodeHDAG::extract_from_rust_source`, `rho_coherence`, `omega_phase`, `energy_kernel` |
| `crates/kosmo-hyphae/src/xlang.rs` | `SourceLanguage`, `CodeHDAG::extract_from_source`/`extract_auto`, `CrossLanguageFingerprint` (cross-language extraction) |
| `crates/kosmo-hyphae/src/run.rs` | `passive_run()`, `HyphaeRunResult` |
| `crates/kosmo-hyphae/src/gates.rs` | `GateCascade` (5 gates, no short-circuit) |
| `crates/kosmo-hyphae/src/lpcm.rs` | LPCM v0.4.2 full pipeline |
| `crates/kosmo-hyphae/src/metatron.rs` | Metatron v0.4.1 M1+M2 pipelines |
| `crates/kosmo-hyphae/src/collapse.rs` | `HostTargetCollapsePlan` (planning-only) |
| `crates/kosmo-pipeline/src/lib.rs` | `run_dry_pipeline()`, `IntegrationRunReport` |
| `crates/kosmo-pipeline/src/materialization.rs` | `MaterializationPlan`, `OperatorApprovalToken` |
| `crates/kosmo-systemcube/src/lib.rs` | `SystemCube::export_dry_run()`, `KcubeExportReport` |
| `crates/kosmo-foundry/src/lib.rs` | `FoundryExecutor`, `standard_cargo_plan()`, `map_kind_to_subcommand()` |
| `crates/kosmo-store/src/lib.rs` | `JsonlCartographyStore`, `verify_integrity()` |
| `crates/kosmo-parseback/src/lib.rs` | `ParseBackExecutor`, `TopologySnapshot`, `CrateFingerprint`, `diff_snapshots()` |
| `crates/kosmo-operator/src/lib.rs` | `OperatorExecutor`, `OperationPlan`, `OperationReport`, `standard_plan()` |
| `tools/kosmo-eval/src/main.rs` | 52-scenario benchmark; EXIT 0 = all pass |
| `SPEC_TRACEABILITY.md` | Full type-to-spec-section mapping |
| `PHASE_CHECKLIST.md` | Phase-by-phase exit criteria and test counts |
| `SAFETY_POLICY.md` | Hard boundaries and safety doctrine |
| `IMPLEMENTATION_DECISIONS.md` | Rationale for non-obvious choices |
