# Changelog

All notable changes to PSE are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Pre-1.0 caveat: while every change tries to preserve the
content-addressing and replay contracts, the public Rust API surface
is still open to breaking changes between 0.x releases. Crystal IDs
and report bytes will only break across minor versions if a release
note explicitly says so.

## [Unreleased]

### Added

* **PHASEMATRIX-HIVEMIND-03** — morphodynamic resonance cell substrate.
  New crate `phase-matrix` and CLI tool `pse-phase-matrix-cli` (binary:
  `phase-matrix`). Implements the spec's full cell-pool → pulses →
  cluster → funnel-graph → morphology → convergence → intent → trace
  → dissolution pipeline as a deterministic `run_cell_substrate_cycle`
  that the runner / replay / verify subcommands all share.
  * Data model: `PhaseCell` (with deterministic `synthetic` factory and
    `PhaseCellRole` covering Sensor / Resonator / Router / Validator /
    MemoryProbe / BoundaryGuard / CandidateEmitter /
    MorphologyRegulator), `CellPool` with matrix-boundary enforcement
    at insertion (foreign-parent cells are rejected), `TridentVector`
    (semantic_density × structural_coherence × temporal_phase →
    activation_potential), `LocalResonanceProcessor` +
    `ResonanceNonlinearity` (Logistic / TanhApprox / SaturatingLinear
    / PiecewiseFixed), `ResonancePulse` with `PhaseBin` quantisation
    (Continuous / KPolar / Tripolar / Quadrupolar), `ResonanceCluster`
    + `ClusterLifecycle` (Proposed / Forming / Active / Stabilized /
    Splitting / Fusing / Decaying / Compacted / Dissolved / Rejected)
    + `ClusterFormationReport`, `FunnelGraph` with four edge families
    (Spatial / Temporal / Semantic / Resonance) and DFS-based
    WHITE/GRAY/BLACK acyclicity validation, `MorphodynamicField`
    (`H = α · Φ + β · µ`) + `ClusterMorphologyEvent` (Grow / Split /
    Fuse / Decay / Replicate / Stabilize / DissolveWorkingState /
    CompactToTrace) + `MorphologyDecision`, `ConvergenceField`,
    `TensionToIntentOperator` + `IntentCandidate` (sorted claim refs),
    `RecursiveFeedbackReport` (Ouroboros loop with bounded learning
    rate), `ClusterTrace` tying every artefact hash together,
    `DissolutionMode` (DropWorkingState / CompactToTrace /
    PersistEvidenceOnly / PersistClusterSummary / ArchiveFullState) +
    `DissolutionReport.validate_trace_preservation` enforcing the
    Dissolution-Grundsatz (working state may be compacted but trace +
    evidence + lifecycle history MUST be preserved),
    `CellToHandoffCandidate`, `PhaseSubnet` /
    `PhaseMatrixNode` / `NodeTrustState`, `MatrixClaim` /
    `TruthMaintenanceReport` / `MatrixBoundaryReport`,
    `CycleReportSummary`, `PhaseMatrixRunDescriptorV3` with
    `CellSubstrateThresholds::permissive()` /
    `CellSubstratePolicies::strict()` / `MatrixGatePolicy::strict()`,
    `ReplayObservation` / `verify_cycle_replay`.
  * Five fail-closed gates: `G_cluster` (phase ∧ coherence ∧ morpho ∧
    purpose ∧ trace), `G_morph` (endo ∧ exo ∧ boundary-safe),
    `G_intent` (tension ∧ convergence ∧ conflict ∧ trace-ready),
    `G_dissolve` (working-state-eligible ∧ trace-persisted ∧
    evidence-persisted ∧ gate-history-persisted), plus the
    matrix-boundary check at the pool layer.
  * `pipeline::run_cell_substrate_cycle` drives the full deterministic
    cycle and returns a `CellSubstrateOutcome` (Completed / Hold /
    Rejected / Compacted / MatrixBoundaryViolation /
    DeterminismViolation). Two runs over the same `(input, rd)` are
    byte-identical.
  * **No `SemanticCrystal` and no `FinalizedEmission`** are
    constructed in any cell-substrate module — the substrate emits
    handoff candidates only; the PSE-Bridge remains the only commit
    path. The `no_commit_artefacts_appear_in_outcome_bytes` test
    guards this invariant against canonical bytes.
  * Feature flags: `cell-substrate` (default-on), `cell-cli`,
    `cell-funnel-graph`, `cell-morphodynamics`, `cell-convergence`,
    `cell-handoff`.
  * Float-free in every gate / score path: `Fixed` (`CanonicalNumber`)
    rationals normalised by gcd, JCS-canonical reports, sorted lists
    before hashing, `BTreeMap`-keyed structures, no wall-clock in the
    audit pathway, no platform RNG.
  * CLI `phase-matrix`: `cluster-cycle`, `cluster-replay`,
    `cluster-verify`, `cell-pool`. Four CLI smoke tests cover the full
    cycle / replay / verify / pool flow.
  * Tests: 34 unit tests + 6 end-to-end integration tests + 4 CLI
    smoke tests.

* **PHASEMATRIX-HIVEMIND-03.1** — Dual-Fabric Field-Tensor Stitch
  Layer. Additive patch on the PHASEMATRIX-HIVEMIND-03 cell-substrate
  implementation. Plugs cleanly into the existing `phase-matrix` crate
  without duplicating or parallelising any existing architecture.
  * **Data model**: `FieldTensorState` (Fabric-T — persistent;
    content-addressed; carries `tensor_revision`, `coupling_matrix_hash`,
    `previous_tensor_hash` chain, `trace_head`), `CouplingMatrix` +
    `CouplingEntry` (five coupling kinds: Structural / Resonance /
    Temporal / Semantic / Boundary), `ResonanceFabricState` (Fabric-H
    — ephemeral; derived deterministically from each
    `CellSubstrateCycleReport`; carries mandatory `trace_hash` per §5.2
    Invariant), `EphemeralResonanceLink` (source / target / resonance_score
    / phase_alignment / ttl_ticks), `StitchCandidate` (proposed coupling
    change; never touches Fabric-T directly), `CouplingUpdate` (accepted
    change; references exactly one `StitcherGateReport`),
    `MirrorConsistencyReport` (MCI per candidate), `TensorDeltaReport`
    (cumulative L1 norm + per-edge max + hypothetical tensor-after hash),
    `StitcherGateReport` (per candidate; all seven sub-gate booleans),
    `FieldTensorTrace` (append-only audit log; sorted before hashing),
    `StitcherReport` (content-addressed outcome; sorted accepted_updates
    / rejected_candidates / gate_reports before hashing),
    `StitcherOutcome` (Completed / Hold / Rejected — all carry the report),
    `StitchRunDescriptor` (replay anchor), `StitchThresholds` /
    `StitchPolicies`, `StitchCycleBundle` (replay-ready artefact containing
    rd, fabric_h, tensor_before, tensor_after, outcome,
    source_cluster_trace_hash).
  * **Key invariants**:
    * Invariant 1 — Fabric-H isolation: Fabric-H MUST NEVER directly
      mutate Fabric-T; all changes route through the StitcherGate.
    * Invariant 2 — StitcherGate is fail-closed:
      `G_stitch = G_conv ∧ G_mci ∧ G_delta ∧ G_budget ∧ G_trace ∧ G_boundary ∧ G_evidence`.
    * Invariant 3 — tensor_revision increments exactly once per
      accepted batch.
    * Invariant 4 — previous_tensor_hash chain is preserved.
    * Invariant 5 — when no updates accepted, tensor_after is
      byte-identical to tensor_before (no trace_head mutation).
    * Invariant 9 — `CouplingUpdate`s sorted before hashing when
      `sort_updates_before_hash = true`.
  * **Pipeline** (`run_stitch_cycle`): validate descriptor → build
    Fabric-H → derive candidates → mirror consistency → tensor delta →
    gate evaluation → collect accepted updates → apply to Fabric-T →
    write FieldTensorTrace → write StitcherReport. Replay path
    (`verify_stitch_replay`) reuses the stored Fabric-H directly
    (bypassing `build_resonance_fabric`) for byte-identity verification.
  * **New modules** in `crates/phase-matrix/src/cell/`:
    `field_tensor`, `resonance_fabric`, `coupling_update`, `stitcher`,
    `stitcher_gate`, `mirror_consistency`, `tensor_delta`,
    `field_tensor_trace`, `stitch_pipeline`.
  * **New CLI subcommands** in `pse-phase-matrix-cli`:
    `stitch-fabric`, `stitch-candidates`, `stitch-gate`, `stitch-apply`,
    `stitch-cycle`, `stitch-replay`, `tensor-inspect`.
  * **New Cargo feature**: `cell-stitch` (default-on; depends on all
    four prior cell features).
  * Float-free in every gate / score path (all `Fixed`); no wall-clock;
    `BTreeMap` for all keyed collections; sorted-before-hashing for all
    lists; JCS-canonical reports.
  * 9 unit tests in `stitch_pipeline.rs`; 5 integration tests in
    `end_to_end.rs`; 3 CLI smoke tests in `cli_smoke.rs`.

* **PSE-EVAL-MATRIX-01 — PHASEMATRIX-HIVEMIND-03.1 closure.** Extended
  the eval matrix so the system stays empirically closed across the
  new stitch layer (additive on top of the HIVEMIND-03 closure):
  * New `WorkloadFamily::DualFabricStitch` (the matrix now lists eleven
    mandatory families) plus `WorkloadSpec::dual_fabric_stitch`
    constructor with hold-correctness / no-false-commit /
    replay-byte-identical success criteria.
  * Six new `CellSubstrateMetricKind` variants for the stitch layer:
    `StitcherGatePassRate`, `CouplingUpdateTraceCoverage`,
    `TensorRevisionMonotonicity`, `MirrorConsistencyCompliance`,
    `StitchReplayIdentity`, `FabricHIsolationRate`.  Three are primary:
    `StitcherGatePassRate`, `StitchReplayIdentity`,
    `FabricHIsolationRate`. `FabricHIsolationRate` is always 1.0 (hard
    invariant; any deviation would be a critical failure).
  * New `dual_fabric_stitch_metric_specs()` (6 metrics) and
    `b9_metric_specs()` (16 metrics: 10 cell-substrate + 6 stitch).
  * New `LayerMask::DUAL_FABRIC_STITCH` bit (1 << 13) and
    `B9_DualFabricStitch` ladder rung (= `B8_PhaseMatrix |
    DUAL_FABRIC_STITCH`), `SystemVariantSpec::dual_fabric_stitch()`
    constructor, and `VariantLadder::full_with_dual_fabric_stitch()`.
  * New `dual-fabric-stitch` preset (B0 / B8 / B9 over the
    `DualFabricStitch` workload, scored against all 16 B9 metrics).
  * `SyntheticTrialExecutor` now emits stitch-layer metric observations
    for `DualFabricStitch` workloads: the six stitch metrics for
    stitch-active variants (B9); the ten cell-substrate metrics for
    substrate-active variants (B8+); `FabricHIsolationRate = 1.0`
    always.
  * `preset_dual_fabric_stitch` exported from `pse-eval-matrix` crate root.
  * 4 new tests in `presets.rs` and `cell_substrate_metrics.rs`.

* **PSE-EVAL-MATRIX-01 — PHASEMATRIX-HIVEMIND-03 closure.** Extended
  the eval matrix so the system stays empirically closed across the
  new substrate:
  * New `WorkloadFamily::MorphoCellSubstrate` (the matrix now lists
    ten mandatory families) plus `WorkloadSpec::morpho_cell_substrate`
    constructor with the standard hold-correctness / no-false-commit /
    replay-byte-identical success criteria.
  * New `cell_substrate_metrics` module with the canonical
    PHASEMATRIX-HIVEMIND-03 metric set (ten metrics:
    `cluster_formation_rate`, `morphology_gate_compliance`,
    `convergence_stability`, `intent_generation_rate`,
    `dissolution_trace_preservation`, `funnel_acyclicity_rate`,
    `matrix_boundary_violation_rate`,
    `working_state_compaction_efficiency`,
    `handoff_candidate_utility`, `substrate_self_coherence`).
  * New `LayerMask::CELL_SUBSTRATE` bit and `B8_PhaseMatrix` ladder
    rung (= `B7_FullStack | CELL_SUBSTRATE`),
    `SystemVariantSpec::phase_matrix_substrate()` constructor, and
    `VariantLadder::full_with_phase_matrix()` for the extended
    nine-rung ladder.
  * New `phase-matrix-substrate` preset (B0 / B7 / B8 over the new
    workload, scored against the full cell-substrate metric set).
  * `SyntheticTrialExecutor` now emits the cell-substrate metric
    observations whenever the workload is `MorphoCellSubstrate`,
    pinned to the fail-closed floor for variants without the
    `CELL_SUBSTRATE` bit and monotonically uplifted for the B8
    variant. Two regression tests guard the uplift on
    `cluster_formation_rate` and the lower-is-better behaviour of
    `matrix_boundary_violation_rate`.

* **PSE-EVAL-MATRIX-01** — empirical benchmark matrix for
  post-symbolic cognition systems. New crate `pse-eval-matrix` and
  CLI tool `pse-eval-matrix-cli` (binary: `pse-eval-matrix`).
  * Data model: `EvaluationSpec` (content-addressed, validatable),
    `SystemVariantSpec` over the B0 → B7 variant ladder with
    explicit `LayerMask` bitset, `WorkloadSpec` over nine mandatory
    families (`StreamEvent` / `AnomalyRegime` / `TraversalPuzzle` /
    `CodeAgentPatch` / `DocSynthesis` / `MemoryReuse` /
    `HorizonFinalization` / `CognitionPanorama` / `MultiAgent`),
    `DatasetManifest` with `calibration` / `validation` / `test`
    splits, `GroundTruthProfile` (synthetic-exact, semi-synthetic
    injection, historical, unit-test oracle, human-adjudicated),
    `MetricSpec` (family / direction / primary flag / aggregation /
    invalidation rules), `MetricObservation`,
    `EvaluationRunLedger` (append-only, hash-chained) with
    `EvaluationRunEntry` and `RunStatus`, `TrialReport` with
    `TrialOutputs` / `GateObservationSet` / `ReplayObservation` /
    `DiagnosticRecord`, `EvaluationSummaryReport`,
    `CapabilityProfile`, `AblationSummary` + `MetricDelta` +
    `AblationConclusion`, `StatisticalSummary`,
    `ReviewerReport` (qualitative rubric), `FailureRecord` /
    `FailureKind` (replay mismatch, false crystal, missed event,
    false handoff, over-hold, under-hold, memory mislead, wormhole
    abuse, calibration leakage), `CalibrationLedgerEntry` /
    `CalibrationProfile` / `CalibrationReason`.
  * Operators: `plan_runs` (deterministic plan), `run_trial` +
    `TrialExecutor` trait (pluggable, with reference
    `SyntheticTrialExecutor`), `init_ledger` / `append_to_ledger` /
    `verify_ledger_chain` (rolling chain hash),
    `verify_trial_replay` (byte-identity check), `score_ledger`
    (aggregates strictly from declared `MetricObservation`s — never
    recomputes), `score_capability_profile` (`U_task / U_replay /
    U_safety / U_cognition / U_efficiency / U_calibration /
    U_robustness` + Safety-Adjusted Utility),
    `safety_adjusted_utility`, `cognition_uplift`,
    `layer_marginal_utility`, `summarize_ablation`,
    `build_ablation_ladder` (eight ablation rungs per §3.2),
    `bootstrap_mean_ci` (deterministic seeded LCG — no platform
    RNG), `exact_binomial_ci`, `paired_mean_diff`,
    `render_markdown_summary`, `render_json_summary`.
  * Three built-in presets (§18): `agent-cognition`,
    `streaming-event-detection`, `post-symbolic-ablation`. Each
    preset stamps a content-addressed spec; the CLI's `init
    --template <preset>` is the canonical entry point.
  * Feature flags: `eval-matrix` (default-on), `eval-cli`,
    `eval-agent`, `eval-cognition`, `eval-streams`,
    `eval-statistics`, `eval-reports`.
  * Float-free in every score / metric / gate hash:
    `CanonicalNumber` only, gcd-normalised rationals to keep i128
    arithmetic safe under composition, `BTreeMap` keyed,
    sorted lists before hashing, JCS-canonical reports, no
    wall-clock timestamps in the audit pathway, no platform RNG.
  * `Schlussformel` (§23) enforced: a system counts as *empirically
    improved* only when `ΔU_task > 0 ∧ ΔU_safety ≥ 0 ∧
    ReplayIdentity = 1 ∧ InvalidRunRate ≤ ε ∧ LMU_target > 0`,
    surfaced as `ConclusionFlag::EmpiricalImprovement` vs.
    `DiagnosticFinding` / `InvalidatedByReplay` /
    `InvalidatedByLeakage`.
  * CLI: `pse-eval-matrix init|validate|plan|run|replay|score|ablate|compare|report`.
  * 49 unit tests + 6 end-to-end integration tests + 4 CLI smoke
    tests; workspace test count rises to **839 / 839** passing.

* **PSE-TRAVERSE-COGNITION-01** — panoptic phase cognition kernel
  layer in `crates/pse-traverse/src/cognition/`:
  * Layered data model `C0–C10`: `CognitionRunDescriptor`,
    `CanonicalCognitionState`, `CognitiveState5D`
    (`ψ, ρ, ω, χ, τ` + derived potential / energy / entropy /
    stability_index), `SingularityDetectorReport`,
    typed `OperatorDeclaration` / `OperatorFamily` / `OperatorType` /
    `IntegrationMode` / `IntegratorKind` / `CognitionSimulationSpec`,
    `SpiralMemoryAddress` / `SpiralMemoryHitSet` / `SpiralSegment`,
    `ConstraintLatticeCognition` / `Resonite` / `Infogene` /
    `AdmissibleRegion` / `InfogenePolicy`,
    `HypercubePuzzleState` / `CognitiveDimension` /
    `PartialAssignment` / `CandidateSet` / `HiddenSingle` /
    `BoundaryContract` / `NegativeTopologyWitness` /
    `EntropyCollapseCertificate`,
    `PhasePanorama` / `Horizon360` / `PhasePath` /
    `AttractorCandidate` / `RecognitionBoundary` /
    `ChoiceGeometryReport`,
    `ScorpioPhaseScheduler` / `ActivationWindow` / `ResonanceOffset` /
    `TransportPolicy` / `VectorTunnelTransport` / `ReasonCode`,
    `GovernedWormhole`,
    `SelfModelTensor` / `ReflexiveModulation` /
    `DualTriggerFeedbackGate`,
    `FixpointCalibrationShell` / `PerformanceTriplet` /
    `ResonanceImpulse` / `CarrierMigrationPlan`,
    `AttractorMap` / `AttractorEntry`,
    `SingularityTriggerReport`,
    `CognitionHandoffGate` / `ProjectionHandoffPolicy` /
    `CognitionCandidate` / `CognitionCandidateBundle`,
    `CognitionReport` / `CognitionHoldReport` /
    `CognitionDiagnostic` / `CognitionRecoveryAction` /
    `CognitionOutcome`.
  * Operators: `null_center_unfold`-style derivations,
    `detect_singularity`, `spiral_memory_query`,
    `build_lattice_minimal`, `evaluate_perkolation`,
    `build_puzzle_minimal`, `build_panorama_minimal`,
    `build_scheduler_minimal`, `admit_wormhole`, `build_self_model`,
    `evaluate_dual_trigger`, `calibrate`, `evaluate_por_acceptance`,
    `evaluate_migration`, `evaluate_singularity_trigger`,
    `CognitionHandoffGate::evaluate`.
  * `pipeline::run_cognition` — total reference pipeline (per §16:
    canonicalize → 5D state → spiral memory query → constraint
    lattice → hypercube puzzle → perkolation → scheduler → panorama
    → wormholes → self-model → dual-trigger feedback → fixpoint
    calibration → carrier migration → attractor ranking →
    singularity trigger → handoff gate → bundle-or-hold → report →
    replay).
  * `replay_hash_of` / `assert_replay_match` for byte-identity audit.
  * Feature flags: `cognition` (default-on), `cognition-cli`,
    `cognition-simulation`, `cognition-spiral-memory`,
    `cognition-hypercube`, `cognition-scorpio-phase`,
    `cognition-wormholes`, `cognition-calibration`,
    `cognition-projection-handoff`.
  * Float-free everywhere: gate-relevant scalars are `Fixed`
    (`CanonicalNumber`); rationals are normalised by gcd to keep
    i128 arithmetic safe under composition; keyed structures are
    `BTreeMap`; lists are sorted before hashing; reports are
    JCS-canonicalised.
  * **No `SemanticCrystal` and no `FinalizedEmission`** are
    constructed in any cognition module — the kernel hands a
    `CognitionCandidateBundle` to projection-v0.2, which alone may
    finalise; the PSE-Bridge remains the only commit path.
* **`pse-traverse-cognition-cli`** tool binary (binary name
  `pse-traverse-cognition`) with the spec's twelve subcommands
  (§18): `inspect`, `observe`, `state5`, `memory-query`, `lattice`,
  `puzzle`, `panorama`, `calibrate`, `trigger`, `bundle`, `replay`,
  `verify`. Golden fixtures in
  `tools/pse-traverse-cognition-cli/tests/fixtures/` and
  end-to-end CLI smoke tests for every subcommand.
* **PSE-TRAVERSE-HORIZON-03** — null-centered horizon geometry layer
  in `crates/pse-traverse/src/horizon/`:
  * Data model: `HorizonRunDescriptorV3`, `HorizonThresholdsV3`,
    `HorizonPoliciesV3`, `HorizonFailurePolicy`, `CarrierPolicyV3`,
    `HorizonWindowPolicyV3`, `ProjectionConePolicyV3`,
    `CausalPolicyV3`, `DualityPolicyV3`, `CrossingPolicyV3`,
    `RationalFixed`, `EpochRange`, `HorizonEvidenceRef`,
    `HorizonError`.
  * Operators: `NullCenterUnfold`, `PhaseRayLift` (hypertorus T^n,
    default n = 4), `HorizonVisibility`, `ProjectionConeCheck`,
    `CausalOrderCheck`, `CollapseEmissionDualityCheck`,
    `HorizonCrossingGate`, combined gate
    `G_v0.3 = G_projection_v2 ∧ G_cross ∧ ReplayReady`.
  * Reports & artefacts: `HorizonChart` (content-addressed),
    `PhaseRay`, `EventHorizonWindowV3` /
    `HorizonWindowReportV3`, `ProjectionCone` /
    `ProjectionConeReport`, `CausalAdmissibilityReport` (with
    `CausalViolation`), `DualityReport`, `HorizonCrossingReport`,
    `FinalizedEmissionV3`, `HorizonHoldReport`, `HorizonV3Outcome`
    (`Finalized` / `Hold` / `WaitForHorizon` / `RefineCone` /
    `NeedsCarrierMigration` / `Recondense` / `ProjectionOnly` /
    `InvalidInput` / `DeterminismViolation`),
    `HorizonCertificate`, `replay_hash_of`, `assert_replay_match`.
  * `pipeline::run_horizon_v3` — total reference pipeline
    (canonicalize → null-resolve → unfold → ray-lift →
    window-evaluate → cone-check → causal-check → duality-check →
    crossing-gate → projection-v0.2 merge → finalize-or-hold →
    certify → replay).
  * Feature flags: `horizon` (default-on), `horizon-cli`,
    `horizon-projection-v2`, `horizon-pse-bridge`,
    `horizon-adapters`.
  * Float-free in every audit path: gate-relevant scalars are
    `CanonicalNumber` (`Fixed`); rationals are `RationalFixed` with
    decimal-string i128 serialisation; keyed collections are
    `BTreeMap`; ray / window / kind lists are sorted before
    hashing; every report is JCS-canonicalised.
  * No `SemanticCrystal` is constructed in any horizon module — the
    PSE-Bridge remains the only commit path.
* **`pse-traverse-horizon-cli`** tool binary (binary name
  `pse-traverse-horizon`) implementing the spec's seven subcommands:
  `inspect`, `chart`, `rays`, `crossing`, `finalize` (refuses when
  `G_v0.3 = 0`), `replay` (byte-identity check), `verify`
  (certificate-chain audit). Golden fixtures in
  `tools/pse-traverse-horizon-cli/tests/fixtures/`. End-to-end CLI
  smoke tests cover every subcommand.
* GitHub Actions CI workflow (`.github/workflows/ci.yml`):
  fmt, clippy, build (Linux / macOS / Windows), test, doc, and a
  non-blocking `cargo audit` job.
* Dependabot configuration for weekly Cargo and monthly Actions
  updates, grouped by patch / minor.
* `CONTRIBUTING.md` with the determinism / replay ground rules,
  PR checklist, and adapter recipe.
* `SECURITY.md` with vulnerability-reporting flow, in-scope
  components, threat model, and primitive inventory.
* `CHANGELOG.md` (this file).

### Changed

* Workspace builds and tests now run warning-free under
  `RUSTFLAGS="-D warnings"`.
* `Cargo.lock` is now committed. The workspace ships binaries
  (`pse-cli`, `pse-demo`, `pse-traverse-cli`,
  `pse-traverse-horizon`, `pse-traverse-cognition`,
  `pse-eval-matrix`, `pse-bench-bbo`) where a reproducible build is
  a hard requirement.
* `cargo fmt --all` applied across the workspace; CI now enforces it.
* README and CHANGELOG document the eval matrix, cognition and
  horizon layers alongside the existing signature and dynamics
  layers; workspace test suite now reports **839 / 839** passing.

### Fixed

* Various clippy warnings: removed dead `crystal_count` in the
  Binance adapter test, replaced an `if_same_then_else` in the
  Metatron platonic classifier (the `is_iso ⇒ is_sub` collapse was
  redundant), used `std::f64::consts::PI` instead of an inline literal
  in `pse-traverse`, removed a placeholder `assert!(true)` test in
  `pse-cli` and replaced it with a real one, annotated the
  NaN-handling `!(hi > lo)` in the IsoForest baseline.

## [0.1.0] — 2026-05

Initial public iteration. Highlights below — see `README.md`
"What's new since the last README" for the full strand-letter log
(E through P, plus the signature and dynamics layers).

### Added (top-level)

* Engine architecture, strands E through N: real Mandorla / cascade,
  5D state, `CrystalAdapter`, resonance fingerprint query,
  resonance-landscape-aware TRITON.
* Operator algebra: `compose / dual / bridge / query / interpolate`.
* Falsification: `Shuffle`, `BlockBootstrap`, `PhaseRandomize`.
* AdaptiveCalibrator (P.3): rolling-history quantile thresholds,
  opt-in.
* `state.last_gate` diagnostic surface (P.2).
* PSE Traversal Agent v0.1 (`pse-traverse` crate): full
  ProblemSpec → FieldCube → DoFGraph → CollapsePlan → Candidate →
  GateReport → PSE-bridge pipeline, fail-closed.
* PSE-TRAVERSE-SIGNATURE-01: signature layer
  (`StructuralOperator` → `Signature` → `SignatureDiagnostics` →
  `SignatureGate`), `BlueprintSearch`, `NonDominatedFrontier`,
  `SearchLedger`, `SearchAutopilot`.
* PSE-TRAVERSE-DYNAMICS-01: morphodynamic tick engine
  (`CanonicalNumber`, `Hash256`/`StableId`, `BaseState`/`LiftedState`,
  `FieldAbsorber`, `GuidanceField`, `MorphodynamicCompressor`,
  `TransitionProof`, `DynamicGate`, `dynamic_tick`/`dynamic_run`).
* Ten domain adapters (Binance, ENTSO-E, Seismo, Weather, AirQuality,
  IoT, Syslog, Vitals, Tabular, ModelMon).
* Four tool binaries (`pse-bench-gt`, `pse-bench-bbo`, `pse-audit`,
  `pse-demo`).
* `pse-traverse-cli` with `inspect / plan / run / replay / search /
  dynamics` subcommands.
* `docs/POST_SYMBOLIC.md`, `docs/COMPLIANCE.md`.

[Unreleased]: https://github.com/lashsesh/pse/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lashsesh/pse/releases/tag/v0.1.0
