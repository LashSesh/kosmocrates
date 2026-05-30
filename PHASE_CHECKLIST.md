# Phase Checklist

Phase-by-phase exit criteria tracker.

## Phase 0 — Orientation and Repo Survey ✅ COMPLETE

- [x] Read `specs/kosmocrates_spec_corpus_implementation_handoff.md`
- [x] Inspected repository layout
- [x] Identified language, crates/modules, test structure
- [x] Found existing content-addressing, ledger, evidence, HDAG, Metatron primitives
- [x] Created `IMPLEMENTATION_STATUS.md`
- [x] Created `SPEC_TRACEABILITY.md`
- [x] Created `IMPLEMENTATION_DECISIONS.md`
- [x] Created `SAFETY_POLICY.md`
- [x] Created `PHASE_CHECKLIST.md`
- [x] No runtime behavior implemented

**Exit criteria met:**
- Repository map exists ✅
- Spec precedence recorded ✅
- Safety policy exists ✅
- Next concrete implementation target identified ✅

---

## Phase 1 — Core Substrate Types ✅ COMPLETE

Target: `crates/kosmo-core`

- [x] `Digest` newtype and canonical serialization (`digest.rs`)
- [x] `canonical_bytes` — JCS RFC 8785 profile (`digest.rs`)
- [x] `Q16` fixed-point numeric type (`fixed_point.rs`)
- [x] `EvidenceRef` (`evidence.rs`)
- [x] `EvidenceBundle` with `ReplayStatus` (`evidence.rs`)
- [x] `AuthorityLabel` (`authority.rs`)
- [x] `TaintLabel` (`authority.rs`)
- [x] `LicenseStatus` (`authority.rs`)
- [x] `CapabilityLock` / `Capability` (`authority.rs`)
- [x] `PolicyProfile` / `ImplementationMode` / `PolicyViolation` (`policy.rs`)
- [x] `RunDescriptor` (HYPHAE) (`run.rs`)
- [x] `GateResult` with merge semantics (`run.rs`)
- [x] `LedgerEvent` / `LedgerEventKind` (`run.rs`)
- [x] `FoundryCheckResult` / `FoundryOutcome` / `FoundryCheckKind` (`run.rs`)
- [x] Digest/canonicalization unit tests (5 tests)
- [x] PolicyProfile default-is-ReportOnly test (CROSS-001, CROSS-002)
- [x] No host mutation enforced by `check_host_write()` (test passes)
- [x] Crate added to workspace `Cargo.toml`
- [x] `cargo test -p kosmo-core` → 43 passed, 0 failed

**Exit criteria:**
- Core types compile ✅
- Digest/canonicalization tests pass ✅
- Policy default is ReportOnly ✅
- No host mutation exists ✅

---

## Phase 2 — Workbench MVP Skeleton ✅ COMPLETE

Target: `crates/kosmo-workbench`

- [x] `WorkspaceIndex` — scan_path + from_entries, deterministic sort, content-addressed
- [x] `TaskSpec` / `TaskKind` — content-addressed task declaration
- [x] `ContextPack` with permitted-use labels — CROSS-005 enforced
- [x] Isolated dry-run — FoundryRunner respects ReportOnly (Skipped) / DryRun (executes)
- [x] `FoundryRunner` — standard_checks, run_check, run_all, EvidenceBundle emission
- [x] `RunReport` — content-addressed, to_text() human-readable output
- [x] `cargo test -p kosmo-workbench` → 20 passed, 0 failed

**Exit criteria:**
- Workbench can produce a dry-run report ✅
- Foundry check skeleton returns Skipped in ReportOnly, Unavailable if command missing ✅
- EvidenceBundle is emitted with check result refs ✅
- ContextPack rejects external-tainted content by default (CROSS-005) ✅

---

## Phase 3 — HYPHAE v0.3 Passive Run ✅ COMPLETE

Target: `crates/kosmo-hyphae` (v0.3 modules)

- [x] `HostBinding`
- [x] `HostCube` skeleton (from_workspace_index, structural void analysis)
- [x] `TopologicalVoidMap` (content-addressed, sorted voids)
- [x] `DeficiencyVector` (from_void_map, Q16 severity, integer arithmetic)
- [x] `SourceIntent` / `SourceIntentKind`
- [x] `SourceFrontierGraph` (from_void_map, sorted by intent_id)
- [x] `SourceEvidence` (local only, no network acquisition)
- [x] `CodeObservation` (with source_evidence_id backref for Metatron Phase 6)
- [x] `CodeHDAG` lowering skeleton (skeleton_for_source, single-node, no real parser)
- [x] `MotifCandidate` (Q16 support score, CROSS-010)
- [x] `StructuralYield` (void-ref requirement, gate_trace_id gating)
- [x] `GateCascade` (TaintGate, EvidenceGate, VoidRefGate, AuthorityGate, PolicyGate, no short-circuit)
- [x] `AssimilationDecision` (content-addressed, evidence-bound, CROSS-012)
- [x] `NegativeEvidenceRecord` (persisted rejected yields — CROSS-012)
- [x] `HyphaeRunResult` + `passive_run()` (full pipeline, no host writes)
- [x] `cargo test -p kosmo-hyphae` → 36 passed, 0 failed

**Exit criteria:**
- Local host scan produces VoidMap / DeficiencyVector / report ✅
- GateCascade can reject/downgrade/pass mocked StructuralYields ✅
- Negative evidence representable ✅

---

## Phase 4 — CubeSwarm MVP ✅ COMPLETE

- [x] `RepositoryCube` (`cube.rs`)
- [x] `CubeDimensionProfile` (BTreeMap<String, Q16>, insertion-order-independent)
- [x] `SourceCube` (content-addressed, support_score as Q16)
- [x] `SourceCubeWorker` (content-addressed per cube+policy)
- [x] `CubeSwarm` (sorts by cube_id for deterministic replay)
- [x] `CubeMandorla` (sorted cube_ids, mandorla detection for shared void)
- [x] `CompositeSupportCube` (integer-averaged Q16 aggregate support)
- [x] `HostTargetDelta` (planning-only, report-only, from_host_and_composite)
- [x] `fixture_source_cubes_merge_deterministically` test ✅
- [x] `HostTargetDelta` emitted as report-only artifact ✅
- [x] `cargo test -p kosmo-hyphae` → 53 passed, 0 failed

**Exit criteria:**
- Fixture SourceCubes merge deterministically ✅
- HostTargetDelta emitted as report-only artifact ✅

---

## Phase 5 — HYPHAE v0.4 Persistent Layer ✅ COMPLETE

- [x] `CorpusCartography` (append-only, entity+relation dedup, idempotent)
- [x] `CorpusEntity` / `CorpusRelation`
- [x] `SourceCubeIndex` / `MotifIndex` / `NegativeEvidenceIndex`
- [x] `CartographyPrecheck`
- [x] `CorpusCartographyUpdate` (before_id/after_id, update_id content-addressed)
- [x] `ReplayManifest` (artifact_digests sorted, from_run)
- [x] `StructuralCrystalCandidate` (from_decision, certification_status)
- [x] `ConstraintProgram` (standard + evaluate, all_satisfied gate)
- [x] `DualFabricGateCascade` (merges two GateTraces)
- [x] `AssimilationCertificate` (issued only when program.all_satisfied)
- [x] `ReplayProof` (replayable flag)
- [x] `StructuralCrystalRecord` (from_certificate)
- [x] `Resonite` (symmetric: canonical id order)
- [x] `NormGeneCandidate` (no is_trusted field — governance path required)
- [x] `NormFitnessTrace` (integer-averaged Q16, non-mutating observe)
- [x] `HostTargetCollapsePlan` (from_delta, PlanningOnly)
- [x] `MorphogenicCorpusUpdate` skeleton
- [x] `cargo test -p kosmo-hyphae` → 79 passed, 0 failed

**Exit criteria:**
- v0.3 run updates CorpusCartography append-only ✅ (`corpus_update_from_run_adds_entities`)
- StructuralYield can become EvidenceOnly / rejected / certified candidate ✅ (`StructuralCrystalCandidate::from_decision`)
- HostTargetCollapsePlan emitted as planning artifact ✅ (`collapse_plan_from_delta_is_planning_only`)

---

## Phase 6 — Metatron v0.4.1 M1/M2 ✅ COMPLETE

- [x] `TopologyRegionRef`
- [x] `RegionExtractionProfile`
- [x] `ProjectionProfile`
- [x] `SemanticLossRecord`
- [x] `MetatronMicrograph`
- [x] `MicrographLiftReport`
- [x] `MetatronRegionFingerprint`
- [x] `MicroTopologyDiagnostic`
- [x] `TopologyAmbiguityProfile`
- [x] `ComplementVoidHypothesis`
- [x] `MicroTopologyIndex`
- [x] `lift_region()` M1 pipeline function
- [x] `diagnose_micrograph()` M2 pipeline function
- [x] `cargo test -p kosmo-hyphae` → 92 passed, 0 failed

**Exit criteria:**
- Small HostVoidRegion can be lifted, fingerprinted, diagnosed, stored ✅
- Ambiguity and semantic loss represented ✅

---

## Phase 7 — Metatron Planning-only Surgery ✅ COMPLETE

- [x] `TopologicalSurgeryKind`
- [x] `SurgeryPrecondition`
- [x] `SurgeryEffect`
- [x] `SurgeryRisk`
- [x] `TopologicalSurgeryOption` (from_diagnostic, sorted by option_id)
- [x] `SurgeryBackedCollapseStep`
- [x] `SurgeryTaskStatus` / `SurgeryWorkbenchTask`
- [x] `cargo test -p kosmo-hyphae` → 103 passed, 0 failed

**Exit criteria:**
- Diagnostic produces planning-only surgery option ✅
- CollapsePlan can include SurgeryBackedCollapseStep ✅
- No host files modified ✅

---

## Phase 8 — LPCM v0.4.2 Passive Report ✅ COMPLETE

Target: `crates/kosmo-hyphae/src/lpcm.rs`

- [x] `Fragment` / `FragmentKind` / `FragmentField` (HDAG node backrefs, sorted by fragment_id)
- [x] `SupportMassVector` (Q16 integer masses, `local_majority_candidate()`, no floats)
- [x] `CandidateDirection` / `CandidateDirectionReason` (51% = candidate only, CROSS-010)
- [x] `LocalCondensationCandidate` (gate-pending, never mutation authority)
- [x] `SeamEdge` / `SeamGraph` (Q16 compatibility scores, threshold filter)
- [x] `monotone_contractive_filter()` / `MonotoneFilterOutcome`
- [x] `DoFContractionReport` (advisory only, content-addressed, `summary()`)
- [x] `LpcmPassiveReport::build()` — full passive pipeline, no host writes
- [x] CROSS-010: local majority → CandidateDirection only, never gate bypass
- [x] CROSS-013: no host-write interface; `allow_host_write = false` confirmed
- [x] `cargo test -p kosmo-hyphae` → 127 passed, 0 failed

**Exit criteria:**
- LPCM consumes fixture fragments and emits passive report ✅
- Monotone contraction testable ✅ (`monotone_filter_contractive_when_non_increasing`)
- Spurious DoF reduction report generated ✅

---

## Phase 9 — SystemCube v0.4.3 Passive Export ✅ COMPLETE

Target: `crates/kosmo-systemcube`

- [x] `BlueprintUnit` / `BlueprintUnitKind` / `BlueprintUnitStatus` (evidence-bound; opaque → RejectedOpaque)
- [x] `SystemCubeManifest` (accepted-only sorted IDs, JSON round-trip stable)
- [x] `ContradictionEnergyReport` / `ContradictionRecord` / `EnergyStatus` (Q16 weight sum)
- [x] `CompatibilityProfileReport` / `CompatibilityGap` / `CompatibilityStatus` (Q16 score)
- [x] `DDensityReport` (Q16::ratio, Available/Unavailable)
- [x] `SystemCube` + `KcubeExportReport` / `KcubeExportMode` (DryRun | BlockedByPolicy)
- [x] `SystemCube::export_dry_run()` — full passive pipeline, no disk I/O
- [x] CROSS-010: D-density=1.0 does NOT bypass policy gate (BlockedByPolicy)
- [x] CROSS-013: no host-write interface in `KcubeExportReport`
- [x] `cargo test -p kosmo-systemcube` → 36 passed, 0 failed

**Exit criteria:**
- Host can export dry-run `.kcube` manifest/report ✅ (`kcube_export_report_is_content_addressed`)
- D-density and contradiction report computed (Available/Insufficient) ✅
- No generated code written ✅

---

## Phase 10 — Integration Hardening ✅ COMPLETE

Target: `crates/kosmo-pipeline`

- [x] `GateTraceAggregator` — fail-closed worst-wins merge (Reject > Warn > Pass), sorted layers
- [x] `AggregatedGateResult` — content-addressed cross-layer gate result
- [x] `IntegrationRunOptions` — `report_only()` / `all_layers()`, flags for optional layers
- [x] `IntegrationRunReport` — unified content-addressed report
- [x] `verify_policy_consistency()` — proves one PolicyProfile governed every sub-report
- [x] `run_dry_pipeline()` — HYPHAE → Cartography → Metatron → LPCM → SystemCube → Aggregate
- [x] CROSS-002: `allow_host_write = false` in default policy (structural test)
- [x] CROSS-013: `IntegrationRunReport` has no mutation interface
- [x] Traceability: policy_id consistent across all sub-reports (tested)
- [x] Determinism: same inputs → same `report_id` across all layers (tested)
- [x] Fail-closed: single Reject propagates to `final_result` (tested)
- [x] `cargo test -p kosmo-pipeline` → 46 passed, 0 failed

**Exit criteria:**
- One dry-run command produces: Host scan + HYPHAE report + CorpusCartography update
  + optional Metatron diagnostics + optional LPCM report + optional SystemCube export
  + no mutation ✅ (`pipeline_all_layers_is_deterministic`, `cross_013_pipeline_no_host_write`)

---

## Phase 11 — Operator-Approved Materialization ✅ COMPLETE

Target: `crates/kosmo-core` (new constructor) + `crates/kosmo-pipeline/src/materialization.rs`

- [x] `PolicyProfile::operator_approved()` — allows host writes, all `require_*` guards retained
- [x] `OperatorApprovalToken` (bound to plan_id, Human/Operator authority, content-addressed)
- [x] `ParseBackExpectation` (before/after topology per step, satisfies `require_parseback`)
- [x] `WorkbenchMaterializationTask` (step + token + FoundryCheckSpec + ParseBack)
- [x] `MaterializationOutcome` (Blocked / FoundryRequired)
- [x] `MaterializationPlan::evaluate()` — full governance chain:
  - Blocked: no token / wrong plan / agent authority / wrong mode / no `allow_host_write`
  - FoundryRequired: valid token + OperatorApproved + `allow_host_write=true`
- [x] `simulate_foundry_check()` — Passed in OperatorApproved, Skipped in ReportOnly
- [x] Token authority: Operator/Human sufficient; Agent insufficient (tested)
- [x] OperatorApproved policy: `memory_promotion=false`, `synthetic_sourcecube=false`, `network=false`
- [x] Parse-back declared per step; ≥1 Foundry check per task (tested)
- [x] `cargo test -p kosmo-pipeline` → 46 passed, 0 failed

**Exit criteria:**
- Materialization requires policy + operator approval ✅ (4 Blocked conditions tested)
- Generated changes declared as Workbench tasks + Foundry specs ✅
- Foundry checks declared per task ✅
- Parse-back topology declared ✅
- Failures remain in planning artifacts (no host side-effects) ✅

---

## Final Verification — 2026-05-30

```
kosmo-core:        49 passed,  0 failed,  0 warnings
kosmo-workbench:   20 passed,  0 failed,  2 ignored (integration), 0 warnings
kosmo-hyphae:     127 passed,  0 failed,  0 warnings
kosmo-systemcube:  36 passed,  0 failed,  0 warnings
kosmo-pipeline:    46 passed,  0 failed,  0 warnings
─────────────────────────────────────────────────────
TOTAL:            278 passed,  0 failed,  0 warnings
```

All 11 phases of the Kosmocrates spec corpus are implemented and verified.
