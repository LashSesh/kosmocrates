//! PHASEMATRIX-HIVEMIND-03 / 03.1 — Morphodynamic Resonance Cell Substrate
//! with Dual-Fabric Field-Tensor Stitch Layer.
//!
//! See `specs/phasematrix_hivemind_morphodynamic_cell_spec_v0_3.pdf` for the
//! v0.3 normative specification, `specs/phasematrix_hivemind_dual_fabric_stitch_spec_v0_3_1.pdf`
//! for the v0.3.1 additive patch, and `specs/ADAMANT_v1.0.0.pdf` for the
//! constitutional architectural contract this crate aligns to.
//!
//! The cell substrate is **not** a static graph layer. It is a
//! controlled morphodynamic field:
//!
//! ```text
//! PhaseSubnet ─► CellPool ─► PhaseCells (TridentVector, lifecycle)
//!                       └─► LocalResonanceProcessor ─► ResonancePulse
//!                                                       │
//!                                                       ▼
//! ClusterFormationGate ──► ResonanceCluster ──► FunnelGraph
//!                                          ├─► MorphodynamicField ──► MorphologyEvent
//!                                          ├─► ConvergenceField   ──► TensionToIntent
//!                                          ├─► RecursiveFeedback  (bounded)
//!                                          └─► Dissolution        (trace-preserving)
//!                                                                      │
//!                                                                      ▼
//!                                                                  ClusterTrace
//!                                                                      │
//!                                              ┌───────────────────────┴───────────────────────┐
//!                                              ▼                                               ▼
//!                                       MatrixBoundaryGate                                   Handoff
//!                                              │                                              candidate
//!                                              ▼                                              (no commit)
//!                                       MatrixClaim
//!
//! ── PHASEMATRIX-HIVEMIND-03.1 Dual-Fabric Stitch Layer ──────────────────
//!
//! CellSubstrateCycleReport
//!         │
//!         ▼
//!  ResonanceFabricState (Fabric-H — ephemeral)
//!         │
//!         ▼  StitcherGate (G_conv ∧ G_mci ∧ G_delta ∧ G_budget ∧ G_trace
//!         ▼               ∧ G_boundary ∧ G_evidence)
//!         │
//!         ▼
//!  CouplingUpdate[] ──► FieldTensorState (Fabric-T — persistent)
//!         │
//!         ▼
//!  StitcherReport + FieldTensorTrace (replay-ready)
//! ```
//!
//! ## Hard rules (PHASEMATRIX-HIVEMIND-03 §5.1 / 03.1 §3, ADAMANT §2)
//!
//! * Every gate-relevant scalar is a [`primitives::Fixed`]
//!   (`CanonicalNumber`) — no platform floats touch the audit
//!   pathway.
//! * Every keyed collection is a `BTreeMap`; every list whose order
//!   is not semantically declared is sorted before hashing.
//! * Every report is content-addressed and JCS-canonical.
//! * Cluster formation, morphology events, intent emission, dissolution
//!   and matrix-boundary checks are **fail-closed**.
//! * **Fabric-H events MUST NOT mutate Fabric-T directly** (Invariant 1).
//!   Fabric-T may only change via an accepted `CouplingUpdate` that
//!   references a passed `StitcherGateReport`.
//! * Dissolution may compact working state but **must** preserve trace,
//!   evidence and gate history (the spec's *Dissolution-Grundsatz*).
//! * Handoff produces only candidates; it **never** creates external
//!   commit or finalisation artefacts (PSE-Bridge remains the only
//!   crystal-commit path).
//! * Domain-specific adapters live outside the core; the core's APIs,
//!   reports and CLI use only the neutral types declared in this
//!   crate (PHASEMATRIX-HIVEMIND-03 §2.2).

#![deny(missing_docs)]

pub mod cell;

pub use cell::*;
