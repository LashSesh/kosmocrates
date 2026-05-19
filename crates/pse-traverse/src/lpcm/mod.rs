//! LPCM — Fragmented 51% Condensation Layer (PSE-LPCM-IMPLEMENTATION-01).
//!
//! Additive, ablatable, fail-closed layer that partitions a bounded phase-space
//! window into local fragments, constructs topology patches, generates candidate
//! directions, computes normalized support-mass vectors, applies a hysteretic
//! 51% local majority gate, links accepted condensations via seam percolation,
//! coarse-grains into hierarchical condensates, and emits a replayable
//! HierarchicalCollapseReport.
//!
//! PRIMARY INVARIANT: Local51(U) ⟹ Candidate(U), Candidate(U) ≠ Truth.
//! A 51% local condensation bit is a local dominance signal, never a commit
//! authority or final output.
//!
//! MUST NOT create SemanticCrystal, FinalizedEmission, MacroControlState,
//! HolisticEigenmodeState, or any persistent tensor update.

pub mod adapters;
pub mod candidate;
pub mod coarse_grain;
pub mod evidence;
pub mod fragment;
pub mod local_gate;
pub mod metrics;
pub mod partition;
pub mod percolation;
pub mod pipeline;
pub mod primitives;
pub mod replay;
pub mod report;
pub mod run_descriptor;
pub mod seam;
pub mod support_mass;
pub mod topology_patch;

#[cfg(test)]
mod tests;

pub use evidence::LpcmInputWindow;
pub use fragment::FragmentedPhaseSpaceWindow;
pub use local_gate::{LocalCondensationBit, LocalCondensationCandidate, LocalCondensationStatus};
pub use metrics::LpcmMetrics;
pub use percolation::PercolativePath;
pub use pipeline::run_lpcm;
pub use primitives::{EvidenceRef, Fixed, Hash256, LpcmError, LpcmResult, StableId};
pub use replay::{verify_lpcm_replay, LpcmReplayReport};
pub use report::HierarchicalCollapseReport;
pub use run_descriptor::{LpcmPolicies, LpcmRunDescriptor, LpcmThresholds};
pub use seam::SeamPercolationGraph;
pub use support_mass::SupportMassVector;
pub use topology_patch::LocalTopologyPatch;
pub use candidate::CandidateDirection;
pub use coarse_grain::CoarseGrainCondensate;

/// Failure kinds for the LPCM layer (§8).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum LpcmFailureKind {
    NoProvenance,
    NonTriangulablePatch,
    TopologyGuardFailed,
    NoCandidateDirections,
    ZeroAdmissibleSupport,
    NoMajority,
    TieNoCondense,
    OscillatoryFlip,
    SeamBreak,
    FalsePercolation,
    CoarseGrainConflict,
    ReplayMismatch,
    InvalidDescriptor,
}

/// Pipeline outcome classification (§8).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum LpcmOutcome {
    NoFragments,
    DiagnosticHold,
    LocalOnly,
    SeamLinked,
    Percolating,
    CoarseGrained,
    CollapseCandidateReady,
    InvalidInput,
    DeterminismViolation,
}
