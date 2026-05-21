#![allow(
    clippy::derivable_impls,
    clippy::doc_overindented_list_items,
    clippy::field_reassign_with_default,
    clippy::get_first,
    clippy::iter_cloned_collect,
    clippy::iter_kv_map,
    clippy::manual_div_ceil,
    clippy::manual_range_contains,
    clippy::needless_range_loop,
    clippy::ptr_arg,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::unnecessary_cast,
    clippy::unnecessary_map_or,
    clippy::unwrap_or_default,
    clippy::useless_vec
)]
//! PSE Traversal Agent v0.1
//!
//! A post-symbolic traversal layer over PSE. The kernel models a problem
//! space as a [`FieldCube`] (dimensions, constraints, couplings, paths,
//! carriers, evidence), constructs a [`DoFGraph`] (degree-of-freedom
//! graph), produces a deterministic [`CollapsePlan`], detects
//! [`PathExcision`] (formal options without admissible operational
//! paths), gates candidates **fail-closed**, and binds successful
//! candidates to PSE crystals via the [`bridge`] module — which uses
//! `pse_core::macro_step` as the only legitimate source of
//! `SemanticCrystal`s.
//!
//! Determinism is a structural invariant, not a convention: every map
//! that affects canonical output is a `BTreeMap`, every list that
//! affects canonical output is sorted before serialisation, and every
//! report is round-tripped through JCS (RFC 8785) before its content
//! address is computed.
//!
//! See `docs/POST_SYMBOLIC.md` for the paradigm definition this layer
//! operates within. See `specs/pse_traversal_agent_spec_v0_1_REUPLOAD.pdf` for the
//! spec this implementation realises.

pub mod blueprint_search;
pub mod canonical;
pub mod compressor;
pub mod dof;
pub mod dynamic_policy;
pub mod dynamic_report;
pub mod dynamic_state;
pub mod dynamic_tick;
pub mod excision;
pub mod field;
pub mod field_cube;
pub mod gate;
pub mod guidance_field;
pub mod norms;
pub mod operator;
pub mod plan;
pub mod plan_delta;
pub mod report;
pub mod search_autopilot;
pub mod search_ledger;
pub mod signature;
pub mod signature_diag;
pub mod signature_gate;
pub mod solver;
pub mod spec;
pub mod transition_proof;

/// Null-Centered Horizon Geometry (PSE-TRAVERSE-HORIZON-03).
///
/// Optional v0.3 layer that adds a hypertoroidal phase-space, phase
/// rays, projection cones, causal admissibility, collapse–emission
/// duality and a fail-closed `HorizonCrossingGate`. Activated with the
/// `horizon` feature (default-on). See `horizon::pipeline::run_horizon_v3`.
#[cfg(feature = "horizon")]
pub mod horizon;

/// Panoptic Phase Cognition Kernel (PSE-TRAVERSE-COGNITION-01).
///
/// Optional v0.1 layer that maps a problem's reachable solvability
/// landscape into a canonical phase space: 5D state geometry, spiral
/// memory, constraint lattice, hypercube puzzle, 720° phase panorama,
/// scorpio phase scheduler, governed wormholes, self-model tensor,
/// fixpoint calibration, singularity trigger and a `CognitionCandidateBundle`
/// handoff to projection-v0.2. Activated with the `cognition` feature
/// (default-on). See `cognition::pipeline::run_cognition`.
#[cfg(feature = "cognition")]
pub mod cognition;

#[cfg(feature = "pse-commit")]
pub mod bridge;

/// Topological Panoptic Triangulation and Möbius-Tripolar Micro-Lift
/// (PSE-TRAVERSE-TPT-MTL-04).
///
/// Additive topology-orientation layer above the Cognition / PhaseMatrix /
/// Dual-Fabric-Stitch stack. Converts panoptic cognitive states into
/// deterministic topological orientation artefacts:
/// `PhaseSpaceWindow`, `MeshHolo`, `MicroFiberSet`, `ReinterpretationReport`,
/// `TopologicalCrystalCandidate` and `TptMtlBundle`.
///
/// The layer generates **no** `SemanticCrystal` and triggers **no** external
/// commits. Activated with the `topology` feature (default-on).
/// See [`topology::run_tpt_mtl`] for the canonical end-to-end pass.
#[cfg(feature = "topology")]
pub mod topology;

/// Full traversal stack orchestration (§Stack-01).
///
/// Chains PSE-TRAVERSE-COGNITION-01 → PSE-TRAVERSE-HORIZON-03 → TPT-MTL-04
/// with deterministic content-addressed handoffs at each stage boundary.
/// Activated when all three layer features are enabled (default-on).
/// See [`orchestration::run_traverse_stack`] for the canonical entry point.
#[cfg(all(feature = "cognition", feature = "horizon", feature = "topology"))]
pub mod orchestration;

/// Fragmented 51% Condensation Layer (PSE-LPCM-IMPLEMENTATION-01).
///
/// Additive, ablatable, fail-closed layer that partitions a bounded phase-space
/// window into local fragments, constructs topology patches, generates candidate
/// directions, computes normalized support-mass vectors, applies a hysteretic
/// 51% local majority gate, links accepted condensations via seam percolation,
/// coarse-grains into hierarchical condensates, and emits a replayable
/// HierarchicalCollapseReport.
///
/// PRIMARY INVARIANT: Local51(U) ⟹ Candidate(U), Candidate(U) ≠ Truth.
/// Activated with the `lpcm-fragment-collapse` feature.
/// See [`lpcm::pipeline::run_lpcm`] for the canonical entry point.
#[cfg(feature = "lpcm-fragment-collapse")]
pub mod lpcm;

pub use canonical::{canonical_bytes, content_address, hex_address};
pub use dof::{DoFEdge, DoFGraph, DoFNode, DoFNodeKind, NodeId, NodeStatus};
pub use excision::{detect_path_excision, OperationalImpact, PathExcision};
pub use field_cube::{
    CarrierKind, CarrierState, CarrierStatus, Coupling, DefaultFieldCubeBuilder, EvidenceRef,
    FieldCube, FieldCubeBuilder, PathSpec, TopologySummary,
};
pub use gate::{Candidate, GateCheck, GateEngine, GateReport, GateSeverity, MciGate};
pub use norms::{CollapseCertificate, ConstraintLattice, NormFitness, NormKind, NormSpec};
pub use plan::{
    CollapseEffect, CollapsePlan, CollapsePlanner, CollapseStep, CollapseStepKind,
    DefaultCollapsePlanner, FailurePolicy, OrderingPolicy,
};
pub use report::{
    CarrierMigrationReport, CommitOutcome, MigrationTrigger, RefinementRequest,
    TraversalRunDescriptor, TraversalRunReport,
};
pub use solver::{
    NullSolver, OracleSolver, ProjectedContext, Solver, SolverContext, TemplateSolver,
};
pub use spec::{
    ConstraintKind, ConstraintSpec, DimensionKind, DimensionSource, DimensionSpec, InputRef,
    OutputSpec, ProblemSpec, ReplayPolicy, RiskPolicy, TreeValue, ValueDomain,
};

#[cfg(feature = "pse-commit")]
pub use bridge::{CrystalCommitter, PseMacroStepCommitter};

use thiserror::Error;

/// Top-level error type for the traversal kernel.
#[derive(Debug, Error)]
pub enum TraverseError {
    #[error("canonical serialisation error: {0}")]
    Canonical(String),
    #[error("input parse error: {0}")]
    Parse(String),
    #[error("invalid problem spec: {0}")]
    InvalidSpec(String),
    #[error("invalid field cube: {0}")]
    InvalidCube(String),
    #[error("plan failure: {0}")]
    Plan(String),
    #[error("gate failure: {0}")]
    Gate(String),
    #[error("PSE commit failure: {0}")]
    PseCommit(String),
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for TraverseError {
    fn from(e: std::io::Error) -> Self {
        TraverseError::Io(e.to_string())
    }
}
impl From<serde_json::Error> for TraverseError {
    fn from(e: serde_json::Error) -> Self {
        TraverseError::Parse(e.to_string())
    }
}

/// Result alias.
pub type Result<T> = std::result::Result<T, TraverseError>;
