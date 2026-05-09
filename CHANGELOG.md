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
