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

## Phase 1 — Core Substrate Types ❌ NOT STARTED

Target: `crates/kosmo-core`

- [ ] `Digest` newtype and canonical serialization
- [ ] `CanonicalSerializationProfile`
- [ ] `Q16` fixed-point numeric type
- [ ] `EvidenceRef`
- [ ] `EvidenceBundle` with `ReplayStatus`
- [ ] `AuthorityLabel`
- [ ] `TaintLabel`
- [ ] `LicenseStatus`
- [ ] `CapabilityLock`
- [ ] `PolicyProfile` / `ImplementationMode`
- [ ] `RunDescriptor` (HYPHAE)
- [ ] `GateResult`
- [ ] `LedgerEvent`
- [ ] `FoundryCheckResult`
- [ ] Digest/canonicalization unit tests
- [ ] PolicyProfile default-is-ReportOnly test
- [ ] No host mutation test
- [ ] Crate added to workspace `Cargo.toml`
- [ ] `cargo test -p kosmo-core` passes

**Exit criteria:**
- Core types compile
- Digest/canonicalization tests pass
- Policy default is ReportOnly
- No host mutation exists

---

## Phase 2 — Workbench MVP Skeleton ❌ NOT STARTED

Target: `crates/kosmo-workbench`

- [ ] `WorkspaceIndex` scan skeleton
- [ ] `TaskSpec`
- [ ] `ContextPack` with permitted-use labels
- [ ] Isolated dry-run worktree concept / placeholder
- [ ] `FoundryRunner` command/check interface skeleton
- [ ] `EvidenceBundle` emission for Workbench operations
- [ ] `RunReport` (dry-run output)
- [ ] `cargo test -p kosmo-workbench` passes

**Exit criteria:**
- Workbench can produce a dry-run report
- Foundry check skeleton can execute or report unavailable
- EvidenceBundle is emitted
- ContextPack rejects raw untrusted external code by default

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
