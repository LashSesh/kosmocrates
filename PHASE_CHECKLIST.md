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

## Phase 3 — HYPHAE v0.3 Passive Run ❌ NOT STARTED

Target: `crates/kosmo-hyphae` (v0.3 modules)

- [ ] `HostBinding`
- [ ] `HostCube` skeleton
- [ ] `TopologicalVoidMap`
- [ ] `DeficiencyVector`
- [ ] `SourceIntent`
- [ ] `SourceFrontierGraph`
- [ ] `SourceEvidence`
- [ ] `CodeObservation`
- [ ] `CodeHDAG` lowering skeleton
- [ ] `MotifCandidate`
- [ ] `StructuralYield`
- [ ] `GateCascade`
- [ ] `AssimilationDecision`
- [ ] Run report output
- [ ] `cargo test -p kosmo-hyphae` passes

**Exit criteria:**
- Local host scan produces VoidMap / DeficiencyVector / report
- GateCascade can reject/downgrade/pass mocked StructuralYields
- Negative evidence representable

---

## Phase 4 — CubeSwarm MVP ❌ NOT STARTED

- [ ] `RepositoryCube`
- [ ] `CubeDimensionProfile`
- [ ] `SourceCube`
- [ ] `SourceCubeWorker`
- [ ] `CubeSwarm`
- [ ] `CubeMandorla`
- [ ] `CompositeSupportCube`
- [ ] `HostTargetDelta`
- [ ] Deterministic worker ordering test

**Exit criteria:**
- Fixture SourceCubes merge deterministically
- HostTargetDelta emitted as report-only artifact

---

## Phase 5 — HYPHAE v0.4 Persistent Layer ❌ NOT STARTED

- [ ] `CorpusCartography` (append-only)
- [ ] `CorpusEntity` / `CorpusRelation`
- [ ] `SourceCubeIndex` / `MotifIndex` / `NegativeEvidenceIndex`
- [ ] `CartographyPrecheck`
- [ ] `CorpusCartographyUpdate` with before/after digest
- [ ] `ReplayManifest`
- [ ] `StructuralCrystalCandidate`
- [ ] `ConstraintProgram`
- [ ] `DualFabricGateCascade`
- [ ] `AssimilationCertificate` with replay/evidence
- [ ] `ReplayProof`
- [ ] `StructuralCrystalRecord`
- [ ] `Resonite`
- [ ] `NormGeneCandidate` (not trusted)
- [ ] `NormFitnessTrace`
- [ ] `HostTargetCollapsePlan` (planning only)
- [ ] `MorphogenicCorpusUpdate` skeleton

**Exit criteria:**
- v0.3 run updates CorpusCartography append-only
- StructuralYield can become EvidenceOnly / rejected / certified candidate
- HostTargetCollapsePlan emitted as planning artifact

---

## Phase 6 — Metatron v0.4.1 M1/M2 ❌ NOT STARTED

- [ ] `TopologyRegionRef`
- [ ] `RegionExtractionProfile`
- [ ] `ProjectionProfile`
- [ ] `SemanticLossRecord`
- [ ] `MetatronMicrograph`
- [ ] `MicrographLiftReport`
- [ ] `MetatronRegionFingerprint`
- [ ] `MicroTopologyDiagnostic`
- [ ] `TopologyAmbiguityProfile`
- [ ] `ComplementVoidHypothesis`
- [ ] `MicroTopologyIndex`

**Exit criteria:**
- Small HostVoidRegion can be lifted, fingerprinted, diagnosed, stored
- Ambiguity and semantic loss represented

---

## Phase 7 — Metatron Planning-only Surgery ❌ NOT STARTED

- [ ] `TopologicalSurgeryOption`
- [ ] `TopologicalSurgeryKind`
- [ ] `SurgeryEffect` / `SurgeryRisk` / `SurgeryPrecondition`
- [ ] `SurgeryBackedCollapseStep`
- [ ] `SurgeryWorkbenchTask`

**Exit criteria:**
- Diagnostic produces planning-only surgery option
- CollapsePlan can include SurgeryBackedCollapseStep
- No host files modified

---

## Phase 8 — LPCM v0.4.2 Passive Report ❌ NOT STARTED

- [ ] `FragmentField` / `Fragment`
- [ ] `CandidateDirection`
- [ ] `SupportMassVector`
- [ ] `LocalCondensationCandidate`
- [ ] `SeamGraph`
- [ ] `DoFContractionReport`
- [ ] `MonotoneContractiveFilter`
- [ ] Passive LPCM report

**Exit criteria:**
- LPCM consumes fixture fragments and emits passive report
- Monotone contraction testable
- Spurious DoF reduction report generated

---

## Phase 9 — SystemCube v0.4.3 Passive Export ❌ NOT STARTED

Target: `crates/kosmo-systemcube`

- [ ] `SystemCube`
- [ ] `SystemCubeManifest`
- [ ] Package / canonical hashing skeleton
- [ ] `BlueprintUnit`
- [ ] D-density report
- [ ] Contradiction energy report
- [ ] Compatibility profile report
- [ ] `.kcube` export dry-run CLI

**Exit criteria:**
- Host can export dry-run `.kcube` manifest/report
- D-density and contradiction report computed or stubbed with unavailable status
- No generated code written

---

## Phase 10 — Integration Hardening ❌ NOT STARTED

- [ ] Shared `PolicyProfile` enforcement across all layers
- [ ] Shared `EvidenceBundle` propagation
- [ ] `GateTrace` aggregation
- [ ] `RunReport` integration
- [ ] Foundry check integration
- [ ] Traceability tests
- [ ] Fail-closed tests

**Exit criteria:**
- One dry-run command produces: Host scan + HYPHAE report + CorpusCartography update
  + optional Metatron diagnostics + optional LPCM report + optional SystemCube export report
  + no mutation

---

## Phase 11 — Operator-Approved Materialization 🚫 BLOCKED

**BLOCKED** until:
- All Phases 0–10 pass
- User explicitly authorizes in this conversation
- PolicyProfile with OperatorApproved mode constructed
- Foundry validation infrastructure operational
- Parse-back topology check infrastructure exists
