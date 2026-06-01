# Implementation Status

## Current Phase
**Vision chain COMPLETE** — Topology ✅ → Energy ✅ → Blueprint ✅ → Roundtrip ✅ → Feedback ✅  
"Echte Topologie rein, tripolare Energie darauf, Blueprint raus, Realitätstest drüber, Wissen zurück ins Substrat."

- **744 substrate tests** (kosmo-core 339, kosmo-hyphae 169, kosmo-pse-bridge 35, kosmo-kcube 46, kosmo-systemcube 25, kosmo-parseback 17, kosmo-operator 8, kosmo-workbench 20, kosmo-store 7, kosmo-pipeline 65) — 0 failures
- **106/106 eval scenarios** pass (kosmo-eval KOSMO-OPS-01 full benchmark)
- **Every Q16-score substrate type in kosmo-hyphae has `energy_assessment`** ✅
- **`MicroTopologyIndex` assembled in pipeline** ✅
- **`SurgeryWorkbenchTask` conversion wired as pipeline Step 3e** ✅
- **`NormFitnessTrace` from prior feedback wired as pipeline Step 5c** ✅ — "Wissen zurück ins Substrat" loop closed

### NormFitnessTrace Pipeline Integration — Step 5c (2026-06-01)
- [x] `prior_feedback: Vec<PromotionFeedback>` added to `IntegrationRunOptions` (default empty)
- [x] Pipeline Step 5c: for each `NormGeneCandidate`, fold matching feedback via `NormFitnessTrace::observe_from_feedback`; only traces with ≥1 observation included
- [x] `IntegrationRunReport.norm_fitness_traces: Vec<NormFitnessTrace>` — one per candidate with matched feedback
- [x] `ReportContent.norm_fitness_trace_count` participates in `report_id`
- [x] `verify_policy_consistency()` covers `norm_fitness_traces[i].policy_id`
- [x] `summary()` reports `norm_candidates: N (traces: M)`
- [x] 3 new pipeline tests (65 total); 2 new `RX:Pipeline` eval scenarios (106 total)

### SurgeryWorkbenchTask Pipeline Integration — Step 3e (2026-06-01)
- [x] Pipeline Step 3e: `surgery_workbench_tasks: Vec<SurgeryWorkbenchTask>` — 1:1 from `surgery_options` via `SurgeryWorkbenchTask::from_option()`; same energy-ranked order
- [x] `IntegrationRunReport.surgery_workbench_tasks` (empty when `surgery_options` is empty)
- [x] `ReportContent.surgery_workbench_task_count` participates in `report_id`
- [x] `verify_policy_consistency()` covers `surgery_workbench_tasks[i].policy_id`
- [x] `summary()` reports `surgery: N (tasks: M)`
- [x] 3 new pipeline tests (62 total); 2 new `RX:Pipeline` eval scenarios (104 total)

### MicroTopologyIndex Pipeline Integration — Step 3d (2026-06-01)
- [x] Pipeline Step 3d: fold `(micrograph, fingerprint, diagnostic)` triples into `MicroTopologyIndex` after metatron loop
- [x] `IntegrationRunReport.metatron_index: MicroTopologyIndex` (empty-state when Metatron disabled)
- [x] `ReportContent.metatron_index_id` participates in `report_id`
- [x] `verify_policy_consistency()` covers `metatron_index.policy_id`
- [x] `summary()` reports `metatron_index.index_id` prefix
- [x] 4 new pipeline tests (59 total); 2 new `RX:Pipeline` eval scenarios (102 total)

### TopologyAmbiguityProfile + ComplementVoidHypothesis energy_assessment (2026-06-01)
- [x] `TopologyAmbiguityProfile::energy_assessment(gate)` — ψ=`confidence_score`; `evidence_bundle_id=micrograph_id` (CROSS-006)
- [x] `ComplementVoidHypothesis::energy_assessment(gate)` — ψ=`confidence_score`; evidence = first non-ZERO `evidence_ids` entry, else `micrograph_id` (CROSS-006)
- [x] 4 new metatron.rs tests (169 hyphae tests total)

### SemanticLossRecord + MicrographLiftReport energy_assessment + Pipeline Step 3c (2026-06-01)
- [x] `SemanticLossRecord::energy_assessment(gate)` — ψ=`loss_ratio`; `evidence_bundle_id=region_id` (CROSS-006)
- [x] `MicrographLiftReport::energy_assessment(gate)` — ψ=`loss_ratio`; `evidence_bundle_id=micrograph_id` (CROSS-006)
- [x] Pipeline Step 3c: `lift_reports: Vec<MicrographLiftReport>` in `IntegrationRunReport` — one per void when Metatron enabled, energy-ranked by `loss_ratio`
- [x] `ReportContent.lift_report_count` participates in `report_id`
- [x] `summary()` reports lift_reports count alongside metatron diagnostics
- [x] 4 new metatron.rs tests + 3 new pipeline tests; 2 new `RX:Pipeline` eval scenarios (100 total)

### Resonite + CubeMandorla + CompositeSupportCube energy_assessment (2026-06-01)
- [x] `Resonite::energy_assessment(gate)` — ψ=`resonance_score`; symmetric (r(a,b)≡r(b,a)); `evidence_bundle_id=resonite_id`
- [x] `CubeMandorla::energy_assessment(gate)` — ψ=`overlap_score`; `evidence_bundle_id=mandorla_id`
- [x] `CompositeSupportCube::energy_assessment(gate)` — ψ=`aggregate_support`; `evidence_bundle_id=composite_id`
- [x] All three use self-referential content addresses as evidence (CROSS-006: no new fields needed)
- [x] 3 crystal.rs tests + 4 swarm.rs tests (6 total new tests)

### NormGeneCandidate Pipeline Integration — Step 5b (2026-06-01)
- [x] `enable_norm_candidates: bool` in `IntegrationRunOptions` (default false)
- [x] Pipeline Step 5b: one `NormGeneCandidate` per accepted decision; initial `fitness_score = Q16::ONE`; `evidence_bundle_id = decision.evidence_bundle_id` (CROSS-006)
- [x] `IntegrationRunReport.norm_candidates: Vec<NormGeneCandidate>` (energy-ranked)
- [x] `ReportContent.norm_candidate_count` participates in `report_id`
- [x] `verify_policy_consistency()` covers `norm_candidates[i].policy_id`
- [x] `summary()` reports `norm_candidates` count
- [x] 3 new pipeline unit tests (52 total); 2 new `RX:Pipeline` eval scenarios (98 total)

### Void Priority Ranking — Pipeline Step 1b (2026-06-01)
- [x] `HostVoid::energy_assessment(gate, policy_id)` — ψ=`severity`; `evidence_bundle_id=void_id` (CROSS-006 self-referential content address)
- [x] `TopologicalVoidMap::priority_ranking(gate)` — severity-ordered void repair queue via `rank_by_energy`; ties broken by `void_id`
- [x] Pipeline Step 1b: `void_priority_ranking: Vec<Digest>` in every `IntegrationRunReport`
- [x] `ReportContent.void_priority_count` participates in `report_id`
- [x] `summary()` reports `voids: N (priority ranked)`
- [x] 5 new void_map.rs unit tests; 2 new `RX:Pipeline` eval scenarios (96 total, 709 substrate tests)

### Surgery Energy Assessment + Pipeline Step 3b (2026-06-01)
- [x] `TopologicalSurgeryOption::energy_assessment(gate)` — ψ=`confidence_score`; `evidence_bundle_id=diagnostic_id` (CROSS-006)
- [x] Pipeline Step 3b: energy-ranked surgery options from Metatron diagnostics
- [x] `IntegrationRunOptions.enable_surgery: bool` (default false; requires `enable_metatron`)
- [x] `IntegrationRunReport.surgery_options: Vec<TopologicalSurgeryOption>` (energy-ranked)
- [x] `verify_policy_consistency()` covers surgery options
- [x] 4 new surgery.rs unit tests; 3 new `RX:Pipeline` eval scenarios (94 total)

### from_host_and_composite removed + MorphogenicCorpusUpdate as Step 4d (2026-06-01)
- [x] `HostTargetDelta::from_host_and_composite` removed (raw `max_by_key` violation); tests migrated to `from_source_cubes`
- [x] Pipeline Step 4d: `MorphogenicCorpusUpdate::skeleton(cartography_update_id, collapse_plan_id, policy_id)`
- [x] `IntegrationRunReport.morphogenic_update` + `verify_policy_consistency()` + `summary()` + `ReportContent`
- [x] 2 new `RX:Pipeline` eval scenarios (91 total)

### JsonlCartographyStore Persistence in Pipeline (2026-06-01)
- [x] `CartographyEntryKind::CartographyUpdate` added to kosmo-core
- [x] `kosmo-store` dep added to kosmo-pipeline
- [x] `persist_cartography_update(update, path, scope, policy)` in new `persistence` module
- [x] Policy-gated: fails with `PolicyDenied` if `allow_host_write == false`
- [x] CROSS-006: `evidence_bundle_id = update.update_id` (non-ZERO content ref)
- [x] 3 unit tests in persistence.rs; 2 new `RX:Pipeline` eval scenarios (89 total)

### StructuralCrystalCandidate Energy Assessment (2026-06-01)
- [x] `energy_assessment(gate)` added to `StructuralCrystalCandidate`
- [x] taint = Q16::ONE (quarantined yields rejected at gate cascade, never candidates)
- [x] 3 new crystal.rs unit tests; 2 new `RX:EnergyRanking` scenarios (87 total)

### Phase 4c HostTargetCollapsePlan Pipeline Weld (2026-06-01)
- [x] `run_dry_pipeline` Step 4c: `HostTargetCollapsePlan::from_delta(&void_fill_delta, policy.id)` — zero-cost, no host writes
- [x] `IntegrationRunReport` carries `collapse_plan: HostTargetCollapsePlan`
- [x] `ReportContent` gains `collapse_plan_id` — collapse plan changes the report_id
- [x] `verify_policy_consistency()` covers `collapse_plan.policy_id`
- [x] `summary()` reports step count and `CollapsePlanStatus`
- [x] 3 new `RX:Pipeline` eval scenarios (85 total): PlanningOnly status, policy traceability, content-addressing

### MotifCandidate Policy Alignment + SeamGraph → Energy Wiring (2026-06-01)
- [x] `MotifCandidate` gains `policy_id` field, updated content addressing, `energy_assessment(gate)`
- [x] `SourceCube::energy_assessment` gains `seam_coherence: Q16` parameter
- [x] `HostTargetDelta::from_source_cubes` gains `seam_map: &BTreeMap<Digest, Q16>`
- [x] Pipeline Step 4b moved after LPCM; seam_map built from lpcm_reports
- [x] 2 new `RX:EnergyRanking` scenarios (82/82 total); 682 substrate tests, 0 failures

### Phase 4 CubeSwarm + HostTargetDelta Pipeline Integration (2026-06-01)
- [x] `run_dry_pipeline` Step 2b: accepted decisions → SourceCubes → CubeSwarm → CompositeSupportCube
- [x] `HostTargetDelta::from_source_cubes` called (energy-ranked, not raw Q16)
- [x] `IntegrationRunReport` carries `swarm_composite` + `void_fill_delta`
- [x] `verify_policy_consistency()` covers swarm + delta policy_ids
- [x] 4 `RX:Pipeline` scenarios all pass (80/80 total)

### Energy Kernel Adoption in Selection Paths (2026-06-01)
- [x] `SourceCube::energy_assessment(gate, license, foundry)` — ψ=support_score, ρ=avg profile coverage
- [x] `NormGeneCandidate::energy_assessment(gate)` — ψ=fitness_score, fail-closed gate
- [x] `HostTargetDelta::from_source_cubes` replaces raw `.max_by_key` with `rank_by_energy`
- [x] 4 `RX:EnergyRanking` scenarios all pass (76/76 total)
- [x] Proven: quarantined taint beats higher raw support_score in ranking

### PSE Feedback Loop — "Wissen zurück ins Substrat" (2026-06-01)
- [x] `FeedbackOutcome` + `PromotionFeedback` in `kosmo-core` (circular dep avoidance)
- [x] `fitness_signal` mapping: Accepted→energy, Deferred→¼, Rejected/Skipped→0
- [x] `CartographyEntryKind::PromotionFeedback` variant
- [x] `build_promotion_feedback` in `kosmo-pse-bridge`
- [x] `NormFitnessTrace::observe_from_feedback` in `kosmo-hyphae`
- [x] 4 `RX:FeedbackLoop` eval scenarios all pass (72/72 total)

## Completed Steps

### Phase 0 — Orientation (2026-05-30)
- [x] Read `specs/kosmocrates_spec_corpus_implementation_handoff.md`
- [x] Inspected full repository layout (crates, adapters, tools, vendors, bindings)
- [x] Identified existing PSE primitives and their alignment with spec
- [x] Created control files: IMPLEMENTATION_STATUS, SPEC_TRACEABILITY, IMPLEMENTATION_DECISIONS, SAFETY_POLICY, PHASE_CHECKLIST
- [x] Confirmed branch: `claude/amazing-allen-Gvldy`

## Repository Map

### Language / Toolchain
- Rust, workspace resolver v2, edition 2021, MSRV 1.82, stable toolchain
- `serde_jcs` in workspace deps → deterministic JSON canonicalization already available
- `sha2 = "0.11"` in workspace deps → SHA-256 hashing available

### Existing Crates (PSE substrate)
| Crate | Role |
|---|---|
| `pse-types` | Shared data model: `Hash256`, `content_address`, `RunDescriptor`, `EvidenceChain`, `GateSnapshot`, `CommitProof` |
| `pse-core` | PSE analytical engine, filter, explore, metatron attach, topology ops |
| `pse-evidence` | Crystal archival, evidence chain verification, `Archive` |
| `pse-metatron` | Graph-theoretic Metatron scan (spectral, platonic, scaffold) |
| `pse-cascade` | Cascade operators, phase ladder, mandorla, dual consensus |
| `pse-graph` | Persistent graph, observation ingestion |
| `pse-memory` | Pattern memory |
| `pse-replay` | Replay verification |
| `pse-topology` | Topology operations |
| `pse-traverse` | Traversal cognition |
| `pse-gateway` | Gateway / routing |

### Existing Types Relevant to Spec
| Existing | Spec Target | Status |
|---|---|---|
| `Hash256 = [u8; 32]` | `Digest` | Alias needed; type exists |
| `content_address<T>()` (JCS+SHA-256) | canonical serialization profile | Exists |
| `RunDescriptor` (PSE domain) | `RunDescriptor` (HYPHAE) | Different; new type needed |
| `EvidenceChain`, `EvidenceEntry` | `EvidenceBundle` / `EvidenceRef` | Partial; spec types differ |
| `GateSnapshot` | `GateResult` | Related; new type needed |
| `CommitProof` | — | PSE-specific |
| — | `AuthorityLabel` | Missing |
| — | `TaintLabel` | Missing |
| — | `CapabilityLock` | Missing |
| — | `PolicyProfile` / `ImplementationMode` | Missing |
| — | `LedgerEvent` | Missing |
| — | `FoundryCheckResult` | Missing |
| — | `Q16` fixed-point | Missing |

### Missing Crates (to be created)
- `crates/kosmo-core` — new substrate types (Phase 1)
- `crates/kosmo-workbench` — Workbench/Foundry (Phase 2)
- `crates/kosmo-hyphae` — HYPHAE v0.3/v0.4+ (Phase 3+)
- `crates/kosmo-systemcube` — SystemCube v0.4.3 (Phase 9)

### Phase 1 — Core Substrate Types (2026-05-30)
- [x] Created `crates/kosmo-core` crate with 6 modules
- [x] `Digest` newtype (SHA-256 + JCS), hex serde, `ZERO` sentinel
- [x] `canonical_bytes` (JCS RFC 8785)
- [x] `Q16` fixed-point (`i64`-backed, 2^16 scale), no-float arithmetic
- [x] `EvidenceRef`, `EvidenceBundle` (content-addressed, policy-scoped, replay-status)
- [x] `AuthorityLabel`, `TaintLabel`, `LicenseStatus`, `CapabilityLock`, `Capability`
- [x] `ImplementationMode`, `PolicyProfile`, `PolicyViolation` (fail-closed defaults)
- [x] `GateResult` (with merge semantics), `LedgerEvent`, `LedgerEventKind`
- [x] `FoundryCheckResult`, `FoundryOutcome`, `FoundryCheckKind`
- [x] `RunDescriptor` (HYPHAE — distinct from pse-types::RunDescriptor)
- [x] 43 tests pass (0 failures)
- [x] Fixed pre-existing duplicate `readme` key in 68 workspace crate Cargo.toml files
- [x] Added `crates/kosmo-core` to workspace members

### Phase 2 — Workbench MVP Skeleton (2026-05-30)
- [x] Created `crates/kosmo-workbench` crate with 5 modules
- [x] `workspace.rs`: `WorkspaceIndex` (scan_path + from_entries, content-addressed, sorted)
- [x] `task_spec.rs`: `TaskSpec` / `TaskKind` (content-addressed, label-keyed)
- [x] `context_pack.rs`: `ContextPack` with CROSS-005 enforcement (ExternalContentDenied), taint propagation
- [x] `foundry.rs`: `FoundryRunner` (ReportOnly → Skipped, DryRun → executes), `FoundryRunOutput` with EvidenceBundle
- [x] `report.rs`: `RunReport` (content-addressed, human-readable `to_text()`)
- [x] 20 tests pass (0 failures), 2 `#[ignore]` integration tests
- [x] CROSS-005 and CROSS-013 verified by named tests

### Phase 3 — HYPHAE v0.3 Passive Run (2026-05-30)
- [x] Created `crates/kosmo-hyphae` crate with 10 modules
- [x] `void_map.rs`: `HostVoid`, `HostVoidKind`, `TopologicalVoidMap` (content-addressed, sorted)
- [x] `deficiency.rs`: `DeficiencyEntry`, `DeficiencyKind`, `DeficiencyVector` (from_void_map, integer Q16 severity)
- [x] `frontier.rs`: `SourceIntent`, `SourceIntentKind`, `SourceEvidence`, `SourceFrontierGraph`
- [x] `code_hdag.rs`: `CodeObservation`, `HDAGNode`, `HDAGEdge`, `CodeHDAG` (skeleton, source backref)
- [x] `motif.rs`: `MotifCandidate` (Q16 support score, CROSS-010 test)
- [x] `structural_yield.rs`: `StructuralYield` (workbench-usability gate, void-ref requirement)
- [x] `gates.rs`: `GateKind`, `GateCascade`, `GateTrace` (TaintGate, EvidenceGate, VoidRefGate, AuthorityGate, PolicyGate)
- [x] `assimilation.rs`: `AssimilationDecision`, `AssimilationOutcome`, `NegativeEvidenceRecord` (CROSS-012)
- [x] `host.rs`: `HostBinding`, `HostCube` (from_workspace_index, structural void analysis)
- [x] `run.rs`: `HyphaeRunResult`, `passive_run()` (full pipeline, no host writes)
- [x] 36 tests pass (0 failures)
- [x] CROSS-005, CROSS-010, CROSS-012 verified by named tests
- [x] Added `crates/kosmo-hyphae` to workspace members

## Open Blockers
- None.

### Phase 4 — CubeSwarm MVP (2026-05-30)
- [x] `cube.rs`: `CubeDimensionProfile`, `RepositoryCube`, `SourceCube` (content-addressed, BTreeMap dimensions)
- [x] `swarm.rs`: `SourceCubeWorker`, `CubeMandorla`, `CompositeSupportCube`, `CubeSwarm`
- [x] `delta.rs`: `DeltaAction`, `VoidFillDelta`, `DeltaStatus`, `HostTargetDelta`
- [x] CubeSwarm sorts cubes by cube_id at construction (deterministic replay)
- [x] CubeMandorla detection: cubes targeting the same void form a mandorla
- [x] CompositeSupportCube: integer-averaged Q16 aggregate support (no floats)
- [x] HostTargetDelta: planning-only, report-only artifact (no host mutation)
- [x] 53 tests pass (0 failures)

### Phase 5 — HYPHAE v0.4 Persistent Layer (2026-05-30)
- [x] `corpus.rs`: `CorpusEntity`, `CorpusRelation`, `CorpusCartography` (append-only, entity+relation dedup)
- [x] `SourceCubeIndex`, `MotifIndex`, `NegativeEvidenceIndex` (filtered views)
- [x] `CartographyPrecheck`, `CorpusCartographyUpdate`, `ReplayManifest`
- [x] `crystal.rs`: `StructuralCrystalCandidate`, `ConstraintProgram`, `AssimilationCertificate`
- [x] `ReplayProof`, `StructuralCrystalRecord`, `Resonite` (symmetric)
- [x] `DualFabricGateCascade` (merges two GateTraces)
- [x] `norm.rs`: `NormGeneCandidate` (not trusted — governance path required), `NormFitnessTrace`
- [x] `collapse.rs`: `CollapseStep`, `HostTargetCollapsePlan` (PlanningOnly status), `MorphogenicCorpusUpdate` skeleton
- [x] `update_from_run` is idempotent (relation dedup fixed)
- [x] 79 tests pass (0 failures)

### Phase 6 — Metatron v0.4.1 M1/M2 (2026-05-30)
- [x] `metatron.rs`: `TopologyRegionRef`, `RegionExtractionProfile`, `ProjectionProfile`
- [x] `SemanticLossRecord` (Q16 loss_ratio, integer arithmetic — CROSS-007)
- [x] `MetatronMicrograph` (source_evidence_id backref — CROSS-006)
- [x] `MicrographLiftReport`, `MetatronRegionFingerprint` (structural hash, not semantic)
- [x] `AnomalyRecord`, `TopologyAmbiguityProfile`, `ComplementVoidHypothesis`
- [x] `MicroTopologyDiagnostic` (sorted anomalies/ambiguities/hypotheses)
- [x] `MicroTopologyIndex` (idempotent add)
- [x] `lift_region()` M1 pipeline, `diagnose_micrograph()` M2 pipeline
- [x] Fingerprint equality ≠ semantic equivalence documented as invariant
- [x] 92 tests pass (0 failures)

### Phase 7 — Metatron Planning-only Surgery (2026-05-30)
- [x] `surgery.rs`: `TopologicalSurgeryKind`, `SurgeryPrecondition`, `SurgeryEffect`, `SurgeryRisk`
- [x] `TopologicalSurgeryOption` (from_diagnostic, sorted by option_id, source_id disambiguator)
- [x] `SurgeryBackedCollapseStep` (CollapseStep + surgery option provenance)
- [x] `SurgeryTaskStatus` / `SurgeryWorkbenchTask` (PlanningOnly status, from_option)
- [x] 103 tests pass (0 failures)

### Phase 8 — LPCM v0.4.2 Passive Report (2026-05-30)
- [x] `lpcm.rs`: `Fragment`, `FragmentKind`, `FragmentField` (sorted by fragment_id, HDAG node backrefs)
- [x] `SupportMassVector` (Q16-scaled integer masses, `local_majority_candidate()`, no floats)
- [x] `CandidateDirection` / `CandidateDirectionReason` (LocalMajority = candidate only, not truth)
- [x] `LocalCondensationCandidate` (derived from CandidateDirection, gate-pending)
- [x] `SeamGraph` / `SeamEdge` (Q16 compatibility scores, threshold-filtered)
- [x] `monotone_contractive_filter()` — `MonotoneFilterOutcome` (Contractive / SpuriousExpansion / Insufficient)
- [x] `DoFContractionReport` (advisory only, content-addressed, `summary()`)
- [x] `LpcmPassiveReport::build()` — full passive pipeline, no host writes
- [x] CROSS-010: 51% majority → CandidateDirection only, never gate bypass
- [x] CROSS-013: LpcmPassiveReport has no host-mutation interface; `allow_host_write = false`
- [x] `allow_synthetic_sourcecube = false` enforced in default policy
- [x] 127 tests pass (0 failures, 0 warnings); +24 LPCM tests

### Phase 9 — SystemCube v0.4.3 Passive Export (2026-05-30)
- [x] New crate `crates/kosmo-systemcube` added to workspace
- [x] `blueprint_unit.rs`: `BlueprintUnit` / `BlueprintUnitKind` / `BlueprintUnitStatus`
      (evidence-bound; opaque units → `RejectedOpaque`; tainted units → `AcceptedWithTaint`)
- [x] `manifest.rs`: `SystemCubeManifest` (accepted-only, sorted IDs, JSON round-trip stable)
- [x] `energy.rs`: `ContradictionEnergyReport` / `ContradictionRecord` / `EnergyStatus`
      (Q16 weight sum, sorted by (unit_a_id, unit_b_id), advisory only)
- [x] `compatibility.rs`: `CompatibilityProfileReport` / `CompatibilityGap` / `CompatibilityStatus`
      (Q16 score, gaps sorted by unit_id, no-host-snapshot stub)
- [x] `lib.rs`: `DDensityReport` (Q16::ratio, Available/Unavailable), `SystemCube`,
      `KcubeExportReport` / `KcubeExportMode` (DryRun / BlockedByPolicy)
- [x] `SystemCube::export_dry_run()` — full passive pipeline, no disk I/O
- [x] CROSS-010: D-density=1.0 does NOT authorise materialization; mode=BlockedByPolicy
- [x] CROSS-013: no host-write interface; `allow_host_write = false`
- [x] `allow_systemcube_materialization = false` in default PolicyProfile → BlockedByPolicy
- [x] 36 tests pass (0 failures, 0 warnings)

### Phase 10 — Integration Hardening (2026-05-30)
- [x] New crate `crates/kosmo-pipeline` added to workspace
- [x] `aggregator.rs`: `GateTraceAggregator`, `AggregatedGateResult`, `LayerGateSummary`
      — fail-closed worst-wins merge (Reject > Warn > Pass), sorted by trace_id, content-addressed
- [x] `IntegrationRunOptions` — flags for optional layers (Metatron, LPCM, SystemCube)
      with `report_only()` and `all_layers()` constructors
- [x] `IntegrationRunReport` — unified content-addressed report with `verify_policy_consistency()`
      proving single PolicyProfile governs every layer
- [x] `run_dry_pipeline()` — single entry point wiring all layers:
      1. HYPHAE v0.3 passive run → gate contribution
      2. CorpusCartography::empty + update_from_run (append-only)
      3. Optional Metatron: lift_region + diagnose_micrograph per void
      4. Optional LPCM: FragmentField + SupportMassVector + SeamGraph + build() per void
      5. Optional SystemCube: BlueprintUnit per accepted decision + export_dry_run()
      6. GateTraceAggregator → AggregatedGateResult → final_result
- [x] CROSS-002: allow_host_write = false in default policy (structural + tested)
- [x] CROSS-013: no host-write interface in IntegrationRunReport (structural + tested)
- [x] Traceability: verify_policy_consistency() checks policy_id in every sub-report
- [x] Determinism: all-layers run with same inputs → identical report_id
- [x] Fail-closed: single gate Reject propagates to final_result (tested)
- [x] 24 tests pass (0 failures, 0 warnings); +6 aggregator tests, +18 pipeline tests

## Completed Phase Summary
| Phase | Crate | Tests |
|---|---|---|
| 0 | Control files | — |
| 1 | kosmo-core | 43 |
| 2 | kosmo-workbench | 20 |
| 3 | kosmo-hyphae (v0.3) | — |
| 4 | kosmo-hyphae (CubeSwarm) | — |
| 5 | kosmo-hyphae (v0.4) | — |
| 6 | kosmo-hyphae (Metatron) | — |
| 7 | kosmo-hyphae (Surgery) | 127 total |
| 8 | kosmo-hyphae (LPCM) | 127 total |
| 9 | kosmo-systemcube | 36 |
| 10 | kosmo-pipeline | 24 |

**Total: 230 tests across 4 new crates, 0 failures, 0 warnings.**

### Phase 11 — Operator-Approved Materialization (2026-05-30)
- [x] `PolicyProfile::operator_approved()` added to `kosmo-core`
      (allow_host_write=true, all require_* guards retained, no network/memory-promotion/synthetic)
- [x] `materialization.rs` in `kosmo-pipeline`:
  - `OperatorApprovalToken` (covers specific plan_id, Human/Operator authority required, content-addressed)
  - `ParseBackExpectation` (topology before/after declaration per step, content-addressed)
  - `WorkbenchMaterializationTask` (step + token + foundry_checks + parse_back, content-addressed)
  - `MaterializationOutcome` (Blocked / FoundryRequired)
  - `MaterializationPlan::evaluate()` — full governance chain:
    - Blocked: no token / wrong plan / agent authority / wrong mode / allow_host_write=false
    - FoundryRequired: valid token + OperatorApproved + allow_host_write=true
  - `simulate_foundry_check()` — Passed in OperatorApproved, Skipped in ReportOnly
- [x] Blocked invariants tested: 4 distinct block cases
- [x] FoundryRequired invariants: parse-back declared per step, ≥1 Foundry check per task
- [x] Token authority: Operator/Human sufficient; Agent insufficient
- [x] OperatorApproved policy: allow_host_write=true, require_foundry=true, require_parseback=true
- [x] 46 tests pass in kosmo-pipeline (0 failures, 0 warnings); +22 materialization tests

## Final Status — All Phases Complete

| Phase | Crate | Key Artifact |
|---|---|---|
| 0 | Control files | Repo survey + control docs |
| 1 | kosmo-core | Digest, Q16, PolicyProfile, EvidenceBundle |
| 2 | kosmo-workbench | WorkspaceIndex, TaskSpec, FoundryRunner |
| 3-7 | kosmo-hyphae | HYPHAE v0.3+v0.4, Metatron, Surgery (127 tests) |
| 8 | kosmo-hyphae | LPCM v0.4.2 (127 tests incl.) |
| 9 | kosmo-systemcube | SystemCube, BlueprintUnit, KcubeExportReport (36 tests) |
| 10 | kosmo-pipeline | run_dry_pipeline, GateTraceAggregator (46 tests) |
| 11 | kosmo-pipeline | OperatorApprovalToken, MaterializationPlan (46 tests incl.) |

**Total: ~254 tests across 5 new production crates. 0 failures. 0 warnings.**

## Open Blockers
None. Entire spec corpus (Phases 0–11) is implemented.

---

# KOSMO-OPS-01 — Operationalization Staircase

Spec: `KOSMO_OPS_01_STATUS.md`. Branch: `claude/kosmo-ops-01-operationalization`.

## Phases R0–R9 (Data Model)

### Phase R0 — Foundation Types (2026-05-31)
- [x] `Digest` newtype (32-byte SHA-256), `Q16` fixed-point (`i64 × 2^16`)
- [x] `PolicyProfile` — `ReportOnly` / `DryRun` / `OperatorApproved`
- [x] `EvidenceBundle` — content-addressed evidence record
- [x] `ParseBackTopologyDelta` / `ParseBackSeverity` — topology diff types
- [x] `FoundryCheckKind` / `FoundryCheckSpec` / `FoundryCommandPolicy` / `FoundryTimeoutPolicy`
- [x] Hosted in `crates/kosmo-core`

### Phase R1 — Foundry Execution Data Types (2026-05-31)
- [x] `FoundryExecutionPlan` (content-addressed over check specs + policy refs)
- [x] `FoundryCheckOutcome` (`Passed` / `Failed` / `SkippedByReportOnly` / `CommandDeniedByPolicy`)
- [x] `FoundryExecutionReport` (content-addressed; `verify_id()`)
- [x] `worst_outcome()` worst-wins aggregation
- [x] 8 unit tests

### Phase R2 — CorpusCartography Data Types (2026-05-31)
- [x] `CartographyEntryKind` / `CartographyStoreCommit` (content-addressed)
- [x] `CorpusCartographyStore` trait (`append`, `read_manifest`, `verify_integrity`)
- [x] `CorpusDiagnosticReport` (content-addressed)
- [x] `CartographyStorageManifest` (monotone head sequence)
- [x] 12 unit tests

### Phase R3 — ParseBack Data Types (2026-05-31)
- [x] `ParseBackPlan` / `ParseBackReport` (content-addressed; `verify_id()`)
- [x] `ParseBackOutcome` (`Passed` / `Failed` / `Inconclusive` / `TopologyUnchanged` / `SkippedByReportOnly`)
- [x] `ParseBackSeverity` worst-wins severity ladder
- [x] `ParseBackTopologyDelta` (`NodeAdded` / `NodeRemoved` / `EdgeAdded` / `EdgeRemoved` / `NodeModified`)
- [x] 9 unit tests

### Phase R4 — Operator Orchestration Data Types (2026-05-31)
- [x] `ValidationClosureReport` (content-addressed over all sub-report IDs)
- [x] `OperatorGateOutcome` (`Passed` / `Failed` / `Inconclusive`)
- [x] Gate synthesis logic (`worst_outcome_of_closure()`)
- [x] 7 unit tests

### Phases R5–R9 — Eval Benchmark Scenarios (2026-05-31)
- [x] `tools/kosmo-eval` binary (`kosmo-eval`): 42 scenarios covering R1–R9 invariants
- [x] R1:Foundry — 9 scenarios (SkippedByReportOnly, CommandDeniedByPolicy, content-addressing, etc.)
- [x] R2:Cartography — 8 scenarios (append, manifest, integrity, worst-wins, fail-closed)
- [x] R3:ParseBack — 7 scenarios (topology delta, severity ladder, report content-addressing)
- [x] R4:Operator — 6 scenarios (closure synthesis, gate outcome, INVARIANT-007)
- [x] R5–R9 misc — 12 scenarios (cross-cutting invariants, policy contracts, round-trip)
- [x] All 42 pass with `EXIT 0`

## Phases RX — Real Executors

### Phase RX-1 — Real Foundry Executor (2026-05-31)
- [x] New crate `crates/kosmo-foundry` added to workspace
- [x] `FoundryExecutor::execute()` — runs allowlisted cargo check/test/clippy commands
      via `std::process::Command`; `ReportOnly` → `SkippedByReportOnly` (no spawn);
      `CommandDeniedByPolicy` checked before spawn
- [x] `standard_cargo_plan()` / `minimal_check_plan()` plan constructors
- [x] `map_kind_to_subcommand()` — deterministic command mapping
- [x] 8 unit tests (including real `cargo check -p kosmo-core` integration test)

### Phase RX-2 — Persistent CorpusCartography Store (2026-05-31)
- [x] New crate `crates/kosmo-store` added to workspace
- [x] `JsonlCartographyStore` — JSONL append-only durable backend
- [x] `verify_integrity()` — re-reads disk, detects digest mismatch and sequence gaps
- [x] Emergent invariant: `DryRun` cannot persist (`allow_host_write = false`); only `OperatorApproved`
- [x] 9 unit tests (including real filesystem round-trip)

### Phase RX-3 — Real ParseBack Executor (2026-05-31)
- [x] New crate `crates/kosmo-parseback` added to workspace
- [x] `ParseBackExecutor::snapshot()` — `cargo metadata --format-version 1 --no-deps`
      snapshots crate-level topology; `INVARIANT-007`: identical inputs → identical IDs
- [x] `CrateFingerprint` — SHA-256(name + files_id + dep_names); content-addressed
- [x] `TopologySnapshot` — content-addressed; `diff_snapshots()` produces delta list
- [x] `ParseBackExecutor::execute()` — takes pre-snapshot explicitly, takes post-snapshot
      internally, diffs, maps severity (NodeRemoved/EdgeRemoved → Critical, NodeAdded/EdgeAdded
      → Warning, NodeModified → Info)
- [x] `ReportOnly` → `SkippedByReportOnly`; baseline mismatch → `Inconclusive`
- [x] 17 unit tests (including real workspace integration tests)

### Phase RX-4 — Operator Orchestrator (2026-05-31)
- [x] New crate `crates/kosmo-operator` added to workspace
- [x] `OperatorExecutor::execute()` — R1→R2→R3 full pipeline:
      1. ParseBack pre-snapshot
      2. Foundry check execution
      3. ParseBack post-snapshot + diff
      4. `ValidationClosureReport` synthesis
      5. Optional JSONL store persistence (only if `OperatorApproved` + `allow_host_write`)
- [x] `OperationPlan` / `OperationReport` (both content-addressed, `verify_id()`)
- [x] `standard_plan()` convenience constructor
- [x] 8 unit tests (including real `cargo check -p kosmo-parseback` integration + temp store)

### Phase RX-BENCH — Eval Benchmark Extended (2026-05-31)
- [x] `tools/kosmo-eval` extended to 52 scenarios (10 new RX scenarios)
- [x] RX:ParseBackExec — 6 scenarios (report-only skip, baseline mismatch, node-added warning,
      node-removed critical, deterministic snapshot, unchanged-workspace passes)
- [x] RX:Operator — 4 scenarios (report-only inconclusive, content-addressed report,
      full-cycle dry-run, approved-persists-closure)
- [x] All 52 scenarios pass with `EXIT 0` in < 30 s on a warm build

## Completed OPS-01 Summary

| Phase | Crate | Tests |
|---|---|---|
| R0 | kosmo-core (ext) | 43 total |
| R1 | kosmo-core | 8 |
| R2 | kosmo-core | 12 |
| R3 | kosmo-core | 9 |
| R4 | kosmo-core | 7 |
| R5–R9 | kosmo-eval (42 scenarios) | — |
| RX-1 | kosmo-foundry | 8 |
| RX-2 | kosmo-store | 9 |
| RX-3 | kosmo-parseback | 17 |
| RX-4 | kosmo-operator | 8 |
| RX-BENCH | kosmo-eval (52 scenarios) | — |

**Total: 614 tests workspace-wide. 0 failures. 0 warnings. 52/52 eval scenarios pass.**

## Open Blockers
None. KOSMO-OPS-01 staircase R0–RX is fully implemented.

---

# KOSMO-TOPO-ENERGY-01 — Real Topology In, Tripolar Energy On It (2026-05-31)

Front of the production-machine vision chain:
`real topology in → tripolar energy → (blueprint out → validation → feedback)`.
This round delivered the first two links.

### TE-1 — Unified tripolar energy kernel (`kosmo-core::energy`)
- [x] `TripolarEnergy { psi, rho, omega }` — `D = ψ·ρ·ω`, Q16 integer arithmetic, no floats (CROSS-007)
- [x] `EnergyFactors` — gate/taint/license/foundry/seam/contradiction, each `[0,1]`, derived fail-closed from `GateResult`/`TaintLabel`/`LicenseStatus`/`FoundrySurvival`
- [x] `EnergyKernel` — `D · ∏ factors`; `EnergyAssessment` content-addressed + evidence-bound
- [x] `rank_by_energy` — deterministic, tie-break on `subject_id`, never drops a candidate
- [x] **Non-bypass invariant (CROSS-010):** `Reject` → gate factor 0 → energy 0; energy ranks but never gates
- [x] 20 unit tests

### TE-2 — Real code topology extraction (`kosmo-hyphae::code_hdag`)
- [x] `CodeHDAG::extract_from_rust_source` — dependency-free lexical extractor
- [x] Nodes: modules, imports, fn defs, type defs, tests; edges: `Imports`/`Contains`/`Tests`/`Implements`
- [x] Content-addressed to `location:line:text`; deterministic (INVARIANT-007); new `Contains` edge kind; content-address now covers full edge wiring
- [x] Topology→energy bridge: `rho_coherence()`, `omega_phase()`, `energy_kernel()`, `energy_assessment()` (ψ caller-supplied; ρ, ω derived from graph)
- [x] 12 new unit tests

### TE-3 — Empirical benchmark (`tools/kosmo-eval`)
- [x] +5 `RX:Energy` scenarios (tripolar exactness, gate non-bypass, hard-state zeroing, content-addressing, deterministic ranking)
- [x] +3 `RX:Topology` scenarios (real-graph extraction, deterministic extraction, full topology→energy chain)
- [x] 60/60 scenarios pass, EXIT 0; `kosmo-eval` now depends on `kosmo-hyphae`

### Maintenance
- [x] Fixed two pre-existing `-D warnings` failures (unused imports in `cartography.rs` and `kosmo-operator`)

| Phase | Crate | Tests |
|---|---|---|
| TE-1 | kosmo-core (energy) | 20 |
| TE-2 | kosmo-hyphae (code_hdag) | 12 new (139 crate total) |
| TE-3 | kosmo-eval | 60 scenarios |

**Total: 646 substrate tests (was 614). 0 failures. 0 warnings. 60/60 eval scenarios pass.**

### Decisions
AD-015 (tripolar energy kernel + non-bypass invariant), AD-016 (lexical topology extraction + topology→energy bridge).

---

# KOSMO-KCUBE-01 — Real `.kcube` Archive Executor (2026-06-01)

Delivers "Blueprint raus + Realitätstest drüber": the host-capability bridge
that turns a `KcubeExportPolicy`-gated artifact list into a real `.kcube` file
on disk (and reads it back for import/verify workflows).

### KC-1 — `kosmo-kcube` executor crate
- [x] `KcubeArtifact { kind, path, bytes }` — typed input to the write operation
- [x] `KcubeExecutor::write` — deterministic framed binary archive, artifacts sorted by path, roundtrip SHA-256 verify
- [x] `KcubeExecutor::read` / `parse_kcube_file` — deserializes the manifest back to `KcubePackage`
- [x] `kcube_file_name(scope, sequence)` — safe slug + sequence → `{scope}-seq{n}.kcube`
- [x] Policy gates: `allow_write=false` → `DeniedByPolicy` (no disk touch); artifact kind allowlist; overwrite guard
- [x] `require_roundtrip_verification=true` (default): re-reads file, compares artifact-section SHA-256
- [x] `KcubeReadError` — typed error variants (too-short / bad-magic / bad-version / truncated sections / parse error)
- [x] CROSS-006: `evidence_bundle_id ≠ ZERO` in every report variant
- [x] CROSS-007: no floats (`written_bytes`, `elapsed_ms` are `u64`)
- [x] INVARIANT-007: `KcubeWriteReport.verify_id()` and `KcubePackage.verify_id()` pass after roundtrip read
- [x] 25 unit tests; 0 new external dependencies

### KC-2 — Empirical benchmark (`tools/kosmo-eval`)
- [x] +5 `RX:Kcube` scenarios: deny when `allow_write=false`, write+roundtrip, content-addressed package, overwrite guard, `read` parses manifest
- [x] 65/65 scenarios pass, EXIT 0

| Phase | Crate | Tests |
|---|---|---|
| TE-1 | kosmo-core (energy) | 20 |
| TE-2 | kosmo-hyphae (code_hdag) | 12 new (139 crate total) |
| TE-3 | kosmo-eval (energy+topology) | 60 scenarios |
| KC-1 | kosmo-kcube | 25 |
| KC-2 | kosmo-eval (kcube) | 65 scenarios |

**Total: 673 substrate tests (was 646). 0 failures. 0 warnings. 65/65 eval scenarios pass.**

---

# KOSMO-SCKCUBE-01 — SystemCube → .kcube Weld (2026-06-01)

Closes the "Blueprint raus" vision link: `SystemCube` can now write a real
`.kcube` archive to disk via the `KcubeExecutor`.

### SC-1 — `SystemCube::export_to_kcube` + `PolicyProfile::operator_approved_with_systemcube`
- [x] `SystemCube::export_to_kcube(executor, capacity, export_policy, op_policy, evidence_bundle_id, sequence) -> KcubeWriteReport`
- [x] `SystemCube::to_kcube_artifacts` — serializes manifest, export assessment, and accepted blueprint units into `Vec<KcubeArtifact>`
- [x] `PolicyProfile::operator_approved_with_systemcube` — new constructor enabling `allow_systemcube_materialization = true`
- [x] Policy gate: `allow_systemcube_materialization = false` → `SkippedByReportOnly` without touching filesystem
- [x] Artifact kinds: `CartographyManifest`, `ValidationClosureReport`, `StructuralCrystal` (one per accepted unit)
- [x] CROSS-006: evidence bound in every report variant; INVARIANT-007: `verify_id()` passes on all outputs
- [x] 5 new unit tests in `kosmo-systemcube`; `kosmo-systemcube` gains `kosmo-kcube` dependency

### SC-2 — Empirical benchmark (`tools/kosmo-eval`)
- [x] +3 `RX:SystemCubeKcube` scenarios: blocked by default policy, write creates archive, archive parses back
- [x] 68/68 scenarios pass, EXIT 0

| Phase | Crate | Tests |
|---|---|---|
| TE-1 | kosmo-core (energy) | 20 |
| TE-2 | kosmo-hyphae (code_hdag) | 12 new (139 crate total) |
| TE-3 | kosmo-eval (energy+topology) | 60 scenarios |
| KC-1 | kosmo-kcube | 25 |
| KC-2 | kosmo-eval (kcube) | 65 scenarios |
| SC-1 | kosmo-systemcube (weld) | 5 new (41 crate total) |
| SC-2 | kosmo-eval (systemcube-kcube) | 68 scenarios |

**Total: 678 substrate tests (was 673). 0 failures. 0 warnings. 68/68 eval scenarios pass.**

## Open Blockers (SCKCUBE)
None. Remaining bridges in priority order:
1. Adopt `EnergyKernel` for `SourceCube`/`BlueprintUnit`/`NormGene` ranking (kernel available; legacy heuristics still alongside it).
2. Close the PSE feedback loop (`kosmo-pse-bridge` is candidate-only / one-directional; no path from PSE `SemanticCrystal` back to `CorpusCartography`/`NormGene`).
3. Cross-language materialization: weld the `.kcube` reader/importer to a consumer outside the Rust substrate.
