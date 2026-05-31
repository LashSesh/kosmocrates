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

**Test count:** 127 passing, 0 failing.

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
| ParseBack topology scan | ✅ `kosmo-parseback`: `cargo metadata`, `CrateFingerprint`, INVARIANT-007 |
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
observation adapter — is deferred until empirical validation confirms
that the substrate's output quality warrants it.

---

## Key files

| Path | Contents |
|---|---|
| `crates/kosmo-core/src/policy.rs` | `PolicyProfile`, `ImplementationMode`, `PolicyViolation` |
| `crates/kosmo-core/src/fixed_point.rs` | `Q16` |
| `crates/kosmo-core/src/digest.rs` | `Digest`, `canonical_bytes` |
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
