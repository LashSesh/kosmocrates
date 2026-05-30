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
| `DeficiencyVector` | HYPHAE v0.3 spec | ✅ `kosmo-hyphae/src/deficiency.rs` |
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
| `RepositoryCube` | HYPHAE v0.3 spec | ❌ |
| `CubeDimensionProfile` | HYPHAE v0.3 spec | ❌ |
| `SourceCube` | HYPHAE v0.3 spec | ❌ |
| `SourceCubeWorker` | HYPHAE v0.3 spec | ❌ |
| `CubeSwarm` | HYPHAE v0.3 spec | ❌ |
| `CubeMandorla` | HYPHAE v0.3 spec | ❌ |
| `CompositeSupportCube` | HYPHAE v0.3 spec | ❌ |
| `HostTargetDelta` | HYPHAE v0.3 spec | ❌ |

## MVP-4 / Phase 5 — HYPHAE v0.4 Persistence

| Type / Module | Spec Source | Status |
|---|---|---|
| `CorpusCartography` | HYPHAE v0.4 spec | ❌ |
| `StructuralCrystalCandidate` | HYPHAE v0.4 spec | ❌ |
| `AssimilationCertificate` | HYPHAE v0.4 spec | ❌ |
| `NormGeneCandidate` | HYPHAE v0.4 spec | ❌ |
| `HostTargetCollapsePlan` | HYPHAE v0.4 spec | ❌ |
| `MorphogenicCorpusUpdate` | HYPHAE v0.4 spec | ❌ |

## MVP-5 / Phase 6 — Metatron v0.4.1

| Type / Module | Spec Source | Status |
|---|---|---|
| `MetatronScanKernel` | Metatron v0.4.1 spec | ❌ |
| `MetatronMicrograph` | Metatron v0.4.1 spec | ❌ |
| `MicroTopologyDiagnostic` | Metatron v0.4.1 spec | ❌ |
| `SemanticLossRecord` | Metatron v0.4.1 spec | ❌ |

## MVP-6 / Phase 8 — LPCM v0.4.2

| Type / Module | Spec Source | Status |
|---|---|---|
| `FragmentField` | LPCM v0.4.2 spec | ❌ |
| `LocalCondensationCandidate` | LPCM v0.4.2 spec | ❌ |
| `MonotoneContractiveFilter` | LPCM v0.4.2 spec | ❌ |

## MVP-7 / Phase 9 — SystemCube v0.4.3

| Type / Module | Spec Source | Status |
|---|---|---|
| `SystemCube` | SystemCube v0.4.3 spec | ❌ |
| `BlueprintUnit` | SystemCube v0.4.3 spec | ❌ |
| `SystemCubeManifest` | SystemCube v0.4.3 spec | ❌ |

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
| CROSS-008 | Every materialization path declares Foundry checks | ❌ |
| CROSS-009 | Topology-changing materialization declares parse-back | ❌ |
| CROSS-010 | No numeric score bypasses gates | ✅ `motif::tests::cross_010_high_support_does_not_bypass_gates` |
| CROSS-011 | Synthetic artifacts are low-authority and tainted | 🔶 `TaintLabel::Synthetic` enforced in passive_run yields |
| CROSS-012 | Negative evidence persisted and affects ranking | ✅ `assimilation::tests::cross_012_negative_evidence_representable` |
| CROSS-013 | Report-only produces diagnostics without host writes | ✅ `report::tests::cross_013_report_only_produces_text_without_writes` |
| CROSS-014 | Implementation can replay from content-addressed artifacts | 🔶 All artifacts content-addressed; replay path Phase 5+ |
| CROSS-015 | Non-replayable objects marked replay-incomplete | 🔶 `ReplayStatus::ReplayIncomplete` default in EvidenceBundle |
