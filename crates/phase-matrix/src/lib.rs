//! PHASEMATRIX-HIVEMIND-03 — Morphodynamic Resonance Cell Substrate.
//!
//! See `PHASEMATRIX_HIVEMIND_03.pdf` for the normative specification
//! and `ADAMANT_v1.0.0.pdf` for the constitutional architectural
//! contract this crate aligns to.
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
//! ```
//!
//! ## Hard rules (PHASEMATRIX-HIVEMIND-03 §5.1, ADAMANT §2)
//!
//! * Every gate-relevant scalar is a [`primitives::Fixed`]
//!   (`CanonicalNumber`) — no platform floats touch the audit
//!   pathway.
//! * Every keyed collection is a `BTreeMap`; every list whose order
//!   is not semantically declared is sorted before hashing.
//! * Every report is content-addressed and JCS-canonical.
//! * Cluster formation, morphology events, intent emission, dissolution
//!   and matrix-boundary checks are **fail-closed**.
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
