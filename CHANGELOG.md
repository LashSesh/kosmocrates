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

### Changed

* Workspace builds and tests now run warning-free under
  `RUSTFLAGS="-D warnings"`.
* `Cargo.lock` is now committed. The workspace ships binaries
  (`pse-cli`, `pse-demo`, `pse-traverse-cli`, `pse-bench-bbo`) where
  a reproducible build is a hard requirement.
* `cargo fmt --all` applied across the workspace; CI now enforces it.

### Added

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
