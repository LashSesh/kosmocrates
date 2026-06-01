# Spec Traceability

Maps implemented modules, types, and tests to source specification sections.

## Legend
- ✅ Implemented and tested
- 🔶 Skeleton / stub
- ❌ Not yet started
- 🚫 Explicitly deferred

---

## MVP-0 / Phase 1 — Core Substrate Types
Target crate: `crates/kosmo-core`

| Type / Module | Spec Source | Status |
|---|---|---|
| `Digest` | Handoff §4 MVP-0; Handoff §7 PolicyProfile.id type | ✅ `kosmo-core/src/digest.rs` |
| `canonical_bytes` (JCS RFC 8785) | Handoff §4 MVP-0 | ✅ `kosmo-core/src/digest.rs` |
| `Q16` fixed-point | Handoff §4 MVP-0; CROSS-007 | ✅ `kosmo-core/src/fixed_point.rs` |
| `EvidenceRef` | Handoff §4 MVP-0 | ✅ `kosmo-core/src/evidence.rs` |
| `EvidenceBundle` | Handoff §4 MVP-0; CROSS-006 | ✅ `kosmo-core/src/evidence.rs` |
| `ReplayStatus` | Handoff §4 MVP-0; CROSS-015 | ✅ `kosmo-core/src/evidence.rs` |
| `AuthorityLabel` | Handoff §4 MVP-0 | ✅ `kosmo-core/src/authority.rs` |
| `TaintLabel` | Handoff §4 MVP-0; CROSS-011 | ✅ `kosmo-core/src/authority.rs` |
| `LicenseStatus` | Handoff §4 MVP-0 | ✅ `kosmo-core/src/authority.rs` |
| `CapabilityLock` / `Capability` | Handoff §4 MVP-0 | ✅ `kosmo-core/src/authority.rs` |
| `PolicyProfile` / `ImplementationMode` | Handoff §3, §7 | ✅ `kosmo-core/src/policy.rs` |
| `PolicyViolation` | Handoff §2 Hard Boundaries | ✅ `kosmo-core/src/policy.rs` |
| `RunDescriptor` (HYPHAE) | Handoff §4 MVP-0 | ✅ `kosmo-core/src/run.rs` |
| `GateResult` | Handoff §4 MVP-0 | ✅ `kosmo-core/src/run.rs` |
| `LedgerEvent` / `LedgerEventKind` | Handoff §4 MVP-0 | ✅ `kosmo-core/src/run.rs` |
| `FoundryCheckResult` / `FoundryOutcome` | Handoff §4 MVP-0 | ✅ `kosmo-core/src/run.rs` |

## MVP-1 / Phase 2 — Workbench Dry-Run Substrate
Target crate: `crates/kosmo-workbench`

| Type / Module | Spec Source | Status |
|---|---|---|
| `WorkspaceIndex` | Handoff §4 MVP-1; Workbench spec v0.1 | ✅ `kosmo-workbench/src/workspace.rs` |
| `TaskSpec` | Handoff §4 MVP-1; Workbench spec v0.1 | ✅ `kosmo-workbench/src/task_spec.rs` |
| `ContextPack` | Handoff §4 MVP-1; Workbench spec v0.1 | ✅ `kosmo-workbench/src/context_pack.rs` |
| `FoundryRunner` | Handoff §4 MVP-1; Workbench spec v0.1 | ✅ `kosmo-workbench/src/foundry.rs` |
| `RunReport` | Handoff §4 MVP-1 | ✅ `kosmo-workbench/src/report.rs` |

## MVP-2 / Phase 3 — HYPHAE v0.3 Passive Run
Target crate: `crates/kosmo-hyphae`

| Type / Module | Spec Source | Status |
|---|---|---|
| `HostBinding` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/host.rs` |
| `HostCube` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/host.rs` |
| `TopologicalVoidMap` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/void_map.rs` |
| `DeficiencyVector` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/deficiency.rs` + always-on in pipeline Step 1c |
| `SourceIntent` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/frontier.rs` |
| `SourceFrontierGraph` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/frontier.rs` |
| `SourceEvidence` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/frontier.rs` |
| `CodeObservation` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/code_hdag.rs` |
| `CodeHDAG` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/code_hdag.rs` |
| `MotifCandidate` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/motif.rs` |
| `StructuralYield` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/structural_yield.rs` |
| `GateCascade` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/gates.rs` |
| `AssimilationDecision` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/assimilation.rs` |
| `NegativeEvidenceRecord` | HYPHAE v0.3 spec; CROSS-012 | ✅ `kosmo-hyphae/src/assimilation.rs` |
| `HyphaeRunResult` / `passive_run` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/run.rs` |

## MVP-3 / Phase 4 — CubeSwarm MVP

| Type / Module | Spec Source | Status |
|---|---|---|
| `RepositoryCube` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/cube.rs` |
| `CubeDimensionProfile` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/cube.rs` |
| `SourceCube` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/cube.rs` |
| `SourceCubeWorker` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/swarm.rs` |
| `CubeSwarm` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/swarm.rs` |
| `CubeMandorla` + `energy_assessment` | HYPHAE v0.3 spec + KOSMO-TOPO-ENERGY-01 | ✅ `kosmo-hyphae/src/swarm.rs` |
| `CompositeSupportCube` + `energy_assessment` | HYPHAE v0.3 spec + KOSMO-TOPO-ENERGY-01 | ✅ `kosmo-hyphae/src/swarm.rs` |
| `HostTargetDelta` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/delta.rs` |

## MVP-4 / Phase 5 — HYPHAE v0.4 Persistence

| Type / Module | Spec Source | Status |
|---|---|---|
| `CorpusCartography` | HYPHAE v0.4 spec | ✅ `kosmo-hyphae/src/corpus.rs` |
| `CorpusEntity` / `CorpusRelation` | HYPHAE v0.4 spec | ✅ `kosmo-hyphae/src/corpus.rs` |
| `CorpusCartographyUpdate` | HYPHAE v0.4 spec | ✅ `kosmo-hyphae/src/corpus.rs` |
| `CartographyPrecheck` / `ReplayManifest` | HYPHAE v0.4 spec | ✅ `kosmo-hyphae/src/corpus.rs` |
| `StructuralCrystalCandidate` | HYPHAE v0.4 spec | ✅ `kosmo-hyphae/src/crystal.rs` + certification work queue in pipeline Step 5d |
| `ConstraintProgram` / `AssimilationCertificate` | HYPHAE v0.4 spec | ✅ `kosmo-hyphae/src/crystal.rs` |
| `StructuralCrystalRecord` / `Resonite` + `energy_assessment` | HYPHAE v0.4 spec + KOSMO-TOPO-ENERGY-01 | ✅ `kosmo-hyphae/src/crystal.rs` |
| `DualFabricGateCascade` | HYPHAE v0.4 spec | ✅ `kosmo-hyphae/src/crystal.rs` |
| `NormGeneCandidate` | HYPHAE v0.4 spec | ✅ `kosmo-hyphae/src/norm.rs` |
| `NormFitnessTrace` | HYPHAE v0.4 spec | ✅ `kosmo-hyphae/src/norm.rs` |
| `HostTargetCollapsePlan` | HYPHAE v0.4 spec | ✅ `kosmo-hyphae/src/collapse.rs` |
| `MorphogenicCorpusUpdate` | HYPHAE v0.4 spec | ✅ `kosmo-hyphae/src/collapse.rs` |

## MVP-5 / Phase 6 — Metatron v0.4.1

| Type / Module | Spec Source | Status |
|---|---|---|
| `TopologyRegionRef` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/metatron.rs` |
| `RegionExtractionProfile` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/metatron.rs` |
| `ProjectionProfile` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/metatron.rs` |
| `SemanticLossRecord` + `energy_assessment` | Metatron v0.4.1 spec + KOSMO-TOPO-ENERGY-01 | ✅ `kosmo-hyphae/src/metatron.rs` |
| `MetatronMicrograph` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/metatron.rs` |
| `MicrographLiftReport` + `energy_assessment` | Metatron v0.4.1 spec + KOSMO-TOPO-ENERGY-01 | ✅ `kosmo-hyphae/src/metatron.rs` |
| `MetatronRegionFingerprint` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/metatron.rs` |
| `MicroTopologyDiagnostic` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/metatron.rs` |
| `TopologyAmbiguityProfile` + `energy_assessment` | Metatron v0.4.1 spec + KOSMO-TOPO-ENERGY-01 | ✅ `kosmo-hyphae/src/metatron.rs` + energy-ranked in pipeline Step 3f |
| `ComplementVoidHypothesis` + `energy_assessment` | Metatron v0.4.1 spec + KOSMO-TOPO-ENERGY-01 | ✅ `kosmo-hyphae/src/metatron.rs` + energy-ranked in pipeline Step 3f |
| `MicroTopologyIndex` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/metatron.rs` |
| `MicroTopologyIndex` (assembled in pipeline Step 3d) | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/metatron.rs` + `kosmo-pipeline/src/lib.rs` |
| `lift_region()` M1 pipeline | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/metatron.rs` |
| `diagnose_micrograph()` M2 pipeline | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/metatron.rs` |

## Phase 7 — Metatron Planning-only Surgery

| Type / Module | Spec Source | Status |
|---|---|---|
| `TopologicalSurgeryKind` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/surgery.rs` |
| `SurgeryPrecondition` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/surgery.rs` |
| `SurgeryEffect` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/surgery.rs` |
| `SurgeryRisk` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/surgery.rs` |
| `TopologicalSurgeryOption` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/surgery.rs` |
| `SurgeryBackedCollapseStep` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/surgery.rs` |
| `SurgeryWorkbenchTask` | Metatron v0.4.1 spec | ✅ `kosmo-hyphae/src/surgery.rs` + assembled in pipeline Step 3e |

## MVP-6 / Phase 8 — LPCM v0.4.2

| Type / Module | Spec Source | Status |
|---|---|---|
| `FragmentField` | LPCM v0.4.2 spec | ✅ `kosmo-hyphae/src/lpcm.rs` |
| `LocalCondensationCandidate` | LPCM v0.4.2 spec | ✅ `kosmo-hyphae/src/lpcm.rs` |
| `MonotoneContractiveFilter` | LPCM v0.4.2 spec | ✅ `kosmo-hyphae/src/lpcm.rs` |

## MVP-7 / Phase 9 — SystemCube v0.4.3

| Type / Module | Spec Source | Status |
|---|---|---|
| `SystemCube` | SystemCube v0.4.3 spec | ✅ `kosmo-systemcube/src/` |
| `BlueprintUnit` | SystemCube v0.4.3 spec | ✅ `kosmo-systemcube/src/` |
| `SystemCubeManifest` | SystemCube v0.4.3 spec | ✅ `kosmo-systemcube/src/` |

---

## Cross-Cutting Acceptance Tests

| ID | Description | Status |
|---|---|---|
| CROSS-001 | Default mode is ReportOnly | ✅ `policy::tests::cross_001_default_is_report_only` |
| CROSS-002 | Host mutation impossible without PolicyProfile | ✅ `policy::tests::cross_002_host_mutation_blocked_by_default` |
| CROSS-003 | External acquisition without capability blocked | 🔶 PolicyViolation type exists; enforcement in Phase 2+ |
| CROSS-004 | Acquired source never executes by default | 🔶 Policy flag exists; enforcement in Phase 3+ |
| CROSS-005 | Raw external code never enters default ContextPack | ✅ `context_pack::tests::cross_005_external_taint_rejected` |
| CROSS-006 | Every durable object has digest, evidence, policy, replay status | ✅ `evidence::tests::cross_006_bundle_has_digest_evidence_policy_replay` |
| CROSS-007 | Gate-relevant numerics use fixed-point / rational | ✅ `fixed_point::tests::q16_comparison_is_integer_only` |
| CROSS-008 | Every materialization path declares Foundry checks | ✅ `kosmo-pipeline/src/materialization.rs` |
| CROSS-009 | Topology-changing materialization declares parse-back | ✅ `kosmo-pipeline/src/materialization.rs` |
| CROSS-010 | No numeric score bypasses gates | ✅ `motif::tests::cross_010_high_support_does_not_bypass_gates` |
| CROSS-011 | Synthetic artifacts are low-authority and tainted | 🔶 `TaintLabel::Synthetic` enforced in passive_run yields |
| CROSS-012 | Negative evidence persisted and affects ranking | ✅ `assimilation::tests::cross_012_negative_evidence_representable` |
| CROSS-013 | Report-only produces diagnostics without host writes | ✅ `report::tests::cross_013_report_only_produces_text_without_writes` |
| CROSS-014 | Implementation can replay from content-addressed artifacts | 🔶 All artifacts content-addressed; replay path Phase 5+ |
| CROSS-015 | Non-replayable objects marked replay-incomplete | 🔶 `ReplayStatus::ReplayIncomplete` default in EvidenceBundle |

---

## KOSMO-OPS-01 — Operationalization (R0–RX)
Target spec: `specs/kosmocrates_production_substrate_operationalization_spec_v0_1.pdf`

### R1 — Real Foundry MVP
Target: `kosmo-core/src/foundry.rs`

| Type / Module | Status |
|---|---|
| `FoundryExecutionPlan`, `FoundryExecutionReport`, `FoundryExecutionOutcome` | ✅ |
| `FoundrySandboxSpec`, `FoundryCommandPolicy`, `FoundryTimeoutPolicy`, `FoundryEnvironmentPolicy` | ✅ |
| `FoundryCheckSpec` | ✅ |

### R2 — Parse-Back MVP
Target: `kosmo-core/src/parseback.rs`

| Type / Module | Status |
|---|---|
| `ParseBackPlan`, `ParseBackReport`, `ParseBackOutcome` | ✅ |
| `ParseBackTopologyDelta`, `TopologyChangeKind`, `ParseBackSeverity`, `ParseBackScanScope` | ✅ |

### R3 — Validation Closure
Target: `kosmo-core/src/validation.rs`

| Type / Module | Status |
|---|---|
| `ValidationClosureReport`, `ValidationClosureStatus`, `determine_closure_status` | ✅ |

### R4 — Persistent CorpusCartography Store
Target: `kosmo-core/src/cartography.rs`

| Type / Module | Status |
|---|---|
| `CorpusCartographyStore` (trait), `CartographyStorageManifest`, `CartographyStoreCommit` | ✅ |
| `CartographyIntegrityReport`, `InMemoryCartographyStore`, `CorpusScope` | ✅ |

### R5 — Isolated Worktree Materialization
Target: `kosmo-core/src/materialization.rs`

| Type / Module | Status |
|---|---|
| `IsolatedWorktreeSpec`, `MaterializationExecutionPlan`, `WorkbenchTaskApplication` | ✅ |
| `MaterializationExecutionReport` | ✅ |

### R6 — SystemCube Disk Export
Target: `kosmo-core/src/kcube.rs`

| Type / Module | Status |
|---|---|
| `KcubePackage`, `KcubeExportPolicy`, `KcubeWriteReport`, `KcubeRoundtripVerification` | ✅ |

### R7 — PSE Bridge
Target: `crates/kosmo-pse-bridge/src/lib.rs`

| Type / Module | Status |
|---|---|
| `PseBridgeCandidate`, `PseBridgeCandidateKind`, `PseBridgePolicy`, `PromotionRequest`, `PromotionOutcome` | ✅ |
| `validate_candidate` | ✅ |
| `PseBridgeCandidate` assembled from pipeline observations in Step 6b (`kosmo-pipeline`) | ✅ |

### R8 — Controlled Acquisition
Target: `kosmo-core/src/acquisition.rs`

| Type / Module | Status |
|---|---|
| `SourceAcquisitionCapability`, `AcquisitionSandbox`, `AcquiredSource`, `AcquisitionTaint` | ✅ |

### R9 — Evaluation Harness
Target: `kosmo-core/src/evaluation.rs`

| Type / Module | Status |
|---|---|
| `EvaluationScenario`, `EvaluationRunReport`, `EvaluationMetrics`, `EvaluationHarness` | ✅ |
| `EvaluationSuiteReport`, `StubEvaluationHarness` | ✅ |

### RX — Real Foundry Executor
Target: `crates/kosmo-foundry/src/lib.rs`

| Type / Module | Status |
|---|---|
| `FoundryExecutor`, `map_kind_to_subcommand`, `standard_cargo_plan` | ✅ |

### RX — Persistent JSONL Store
Target: `crates/kosmo-store/src/lib.rs`

| Type / Module | Status |
|---|---|
| `JsonlCartographyStore` | ✅ |

### RX — Real ParseBack Executor
Target: `crates/kosmo-parseback/src/lib.rs`

| Type / Module | Status |
|---|---|
| `ParseBackExecutor`, `TopologySnapshot`, `CrateFingerprint`, `diff_snapshots` | ✅ |

### RX — Operator Pipeline
Target: `crates/kosmo-operator/src/lib.rs`

| Type / Module | Status |
|---|---|
| `OperatorExecutor`, `OperationPlan`, `OperationReport`, `standard_plan` | ✅ |

## KOSMO-TOPO-ENERGY-01 — Real Topology In, Tripolar Energy On It

### TE — Unified tripolar energy kernel
Target: `crates/kosmo-core/src/energy.rs` (AD-015)

| Type / Module | Status |
|---|---|
| `TripolarEnergy` (`D = ψ·ρ·ω`, Q16, CROSS-007) | ✅ |
| `EnergyFactors` (gate/taint/license/foundry/seam/contradiction, fail-closed) | ✅ |
| `EnergyKernel`, `FoundrySurvival` | ✅ |
| `EnergyAssessment` (content-addressed, evidence-bound, CROSS-006) | ✅ |
| `rank_by_energy` (deterministic, never drops candidates) | ✅ |
| Non-bypass invariant: energy ranks, never gates (CROSS-010) | ✅ |

### TE — Energy integration: all Q16-score substrate types
Every kosmo-hyphae type carrying a Q16 score now has `energy_assessment`.

| Type | ψ (meaning) | evidence_bundle_id |
|---|---|---|
| `StructuralCrystalCandidate` | `support_score` | `evidence_bundle_id` (field) |
| `MotifCandidate` | `support_score` | `evidence_bundle_id` (field) |
| `SourceCube` | `support_score` | `evidence_bundle_id` (field) |
| `TopologicalSurgeryOption` | `confidence_score` | `diagnostic_id` (causal) |
| `HostVoid` | `severity` | `void_id` (self-ref) |
| `NormGeneCandidate` | `fitness_score` | `evidence_bundle_id` (field) |
| `Resonite` | `resonance_score` | `resonite_id` (self-ref, symmetric) |
| `CubeMandorla` | `overlap_score` | `mandorla_id` (self-ref) |
| `CompositeSupportCube` | `aggregate_support` | `composite_id` (self-ref) |
| `SemanticLossRecord` | `loss_ratio` | `region_id` (causal) |
| `MicrographLiftReport` | `loss_ratio` | `micrograph_id` (causal) |
| `TopologyAmbiguityProfile` | `confidence_score` | `micrograph_id` (causal) |
| `ComplementVoidHypothesis` | `confidence_score` | first `evidence_ids` or `micrograph_id` |
| `BlueprintUnit` | `Q16::ONE` (Accepted/AcceptedWithTaint), `Q16::ZERO` (RejectedOpaque) | `unit_id` (self-ref) |

### TE — Real code topology extraction
Target: `crates/kosmo-hyphae/src/code_hdag.rs` (AD-016)

| Type / Module | Status |
|---|---|
| `CodeHDAG::extract_from_rust_source` (lexical, dependency-free) | ✅ |
| Real nodes (module/import/fn/type/test) + edges (`Imports`/`Contains`/`Tests`/`Implements`) | ✅ |
| Content-addressed to source line; deterministic (INVARIANT-007) | ✅ |
| Topology→energy bridge: `rho_coherence`, `omega_phase`, `energy_kernel`, `energy_assessment` | ✅ |

### TE — Empirical benchmark
Target: `tools/kosmo-eval/src/main.rs`

| Scenario group | Status |
|---|---|
| `RX:Energy` (5 scenarios) | ✅ |
| `RX:Topology` (3 scenarios) | ✅ |
| `RX:Pipeline` (46 scenarios including all energy-ranked pipeline outputs + Steps 1c, 3e, 3f, 5c, 5d, 5e, 6b) | ✅ |
| `RX:BlueprintEnergy` (2 scenarios: accepted positive / opaque zero; tainted ranks below clean) | ✅ |
