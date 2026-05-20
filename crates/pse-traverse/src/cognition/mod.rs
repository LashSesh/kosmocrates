//! Panoptic Phase Cognition Kernel — `pse-traverse-cognition` v0.1.
//!
//! See `pse_traverse_cognition_spec_v0_1.pdf` in the repo root for the
//! normative specification this module realises.
//!
//! The cognition kernel turns problem, traversal, dynamics, reactive
//! and projection state into a *canonical map* of the reachable
//! solvability landscape — not just the active path. It carries:
//!
//! * a 5D operational state (`ψ`, `ρ`, `ω`, `χ`, `τ`) with derived
//!   potential / energy / entropy / stability observables;
//! * a typed operator algebra over `State5D / SpiralPath /
//!   ConstraintLattice / HypercubePuzzle / PhasePanorama / SelfModel /
//!   CandidateBundle`;
//! * spiral memory and resonance addressing;
//! * a constraint lattice with weighted mass, entropy reduction,
//!   feasible-set uniqueness and a perkolation gate;
//! * a hypercube puzzle with hidden-single detection and an
//!   `EntropyCollapseCertificate`;
//! * a 720° phase panorama (current + counterfactual horizons);
//! * the neutralised "Scorpio" phase scheduler (no steganography, no
//!   hidden channels);
//! * governed wormhole shortcuts under a strict admission gate;
//! * a self-model tensor + dual-trigger feedback (ordered trigger
//!   sequence);
//! * fixpoint calibration with PoR acceptance and carrier migration
//!   evaluation;
//! * a singularity trigger, an attractor map, and a fail-closed
//!   `ProjectionHandoffPolicy` that hands a `CognitionCandidateBundle`
//!   off to projection-v0.2 when every sub-gate fires.
//!
//! ## Hard architectural rules
//!
//! * The cognition kernel **never** finalises a state, **never**
//!   commits, and **never** constructs a `SemanticCrystal`. It produces
//!   reports, evidence and `CognitionCandidateBundle`s only.
//! * Every gate-relevant scalar is a [`crate::cognition::primitives::Fixed`]
//!   (the workspace `CanonicalNumber`) — no platform floats touch the
//!   audit pathway.
//! * Every keyed collection is a `BTreeMap`; every list whose order is
//!   not semantically declared is sorted before hashing.
//!
//! Use [`crate::cognition::pipeline::run_cognition`] for the canonical
//! end-to-end pass.

pub mod attractor_map;
pub mod filter;
pub mod canonical_state;
pub mod cognitive_state_5d;
pub mod constraint_lattice;
pub mod fixpoint_calibration;
pub mod handoff;
pub mod hypercube_puzzle;
pub mod operator_algebra;
pub mod phase_panorama;
pub mod pipeline;
pub mod primitives;
pub mod replay;
pub mod report;
pub mod run_descriptor;
pub mod scorpio_phase_scheduler;
pub mod self_model_tensor;
pub mod singularity_trigger;
pub mod spiral_memory;
pub mod wormhole_hypernet;

pub use attractor_map::{AttractorEntry, AttractorMap};
pub use canonical_state::CanonicalCognitionState;
pub use cognitive_state_5d::{detect_singularity, CognitiveState5D, SingularityDetectorReport};
pub use constraint_lattice::{
    build_lattice_minimal, evaluate_perkolation, AdmissibleRegion, ConstraintLatticeCognition,
    Infogene, InfogenePolicy, Resonite,
};
pub use fixpoint_calibration::{
    calibrate, evaluate_migration, evaluate_por_acceptance, CarrierMigrationPlan,
    FixpointCalibrationShell, PerformanceTriplet, ResonanceImpulse,
};
pub use handoff::{
    CognitionCandidate, CognitionCandidateBundle, CognitionHandoffGate, ProjectionHandoffPolicy,
};
pub use hypercube_puzzle::{
    build_puzzle_minimal, BoundaryContract, CandidateSet, CognitiveDimension,
    EntropyCollapseCertificate, HiddenSingle, HypercubePuzzleState, NegativeTopologyWitness,
    PartialAssignment,
};
pub use operator_algebra::{
    CognitionSimulationSpec, IntegrationMode, IntegratorKind, InvariantSpec, OperatorDeclaration,
    OperatorFamily, OperatorType,
};
pub use phase_panorama::{
    build_panorama_minimal, build_panorama_with_quality, AttractorCandidate, ChoiceGeometryReport, Horizon360, PhasePanorama,
    PhasePath, PhasePathStatus, PhasePathSummary, RecognitionBoundary,
};
pub use filter::{CognitionFilter, FilterResult};
pub use pipeline::{run_cognition, CognitionInput, CognitionRunResult, CognitiveComponents};
pub use primitives::{
    fixed_abs_diff, fixed_add, fixed_ge, fixed_le, fixed_sub, max_fixed, min_fixed, CognitionError,
    EvidenceRef, Fixed, StableId,
};
pub use replay::{assert_replay_match, replay_hash_of};
pub use report::{
    CognitionDiagnostic, CognitionHoldReport, CognitionOutcome, CognitionRecoveryAction,
    CognitionReport,
};
pub use run_descriptor::{
    CognitionFailurePolicy, CognitionGateKind, CognitionPolicies, CognitionRunDescriptor,
    CognitionThresholds,
};
pub use scorpio_phase_scheduler::{
    build_scheduler_minimal, ActivationWindow, ReasonCode, ResonanceOffset, ScorpioPhaseScheduler,
    TransportPolicy, VectorTunnelTransport,
};
pub use self_model_tensor::{
    build_self_model, evaluate_dual_trigger, DualTriggerFeedbackGate, ReflexiveModulation,
    SelfModelTensor,
};
pub use singularity_trigger::{evaluate_singularity_trigger, SingularityTriggerReport};
pub use spiral_memory::{
    resonance_distance, spiral_memory_query, SpiralMemoryAddress, SpiralMemoryHitSet, SpiralSegment,
};
pub use wormhole_hypernet::{admit_wormhole, GovernedWormhole};
