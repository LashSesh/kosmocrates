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
  `pse-traverse-horizon`, `pse-bench-bbo`) where a reproducible
  build is a hard requirement.
* `cargo fmt --all` applied across the workspace; CI now enforces it.
* README and CHANGELOG document the horizon layer alongside the
  existing signature and dynamics layers; workspace test suite now
  reports **737 / 737** passing.

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
