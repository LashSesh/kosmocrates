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
//! operates within. See the `pse_traversal_agent_spec_v0_1.pdf` in the
//! repository root for the spec this implementation realises.

pub mod canonical;
pub mod spec;
pub mod field_cube;
pub mod dof;
pub mod plan;
pub mod excision;
pub mod gate;
pub mod report;
pub mod solver;
pub mod norms;
pub mod operator;
pub mod signature;
pub mod signature_diag;
pub mod signature_gate;
pub mod blueprint_search;
pub mod search_ledger;
pub mod search_autopilot;
pub mod dynamic_state;
pub mod dynamic_policy;
pub mod field;
pub mod guidance_field;
pub mod compressor;
pub mod transition_proof;
pub mod plan_delta;
pub mod dynamic_report;
pub mod dynamic_tick;

#[cfg(feature = "pse-commit")]
pub mod bridge;

pub use canonical::{canonical_bytes, content_address, hex_address};
pub use spec::{
    ConstraintKind, ConstraintSpec, DimensionKind, DimensionSource, DimensionSpec,
    InputRef, OutputSpec, ProblemSpec, ReplayPolicy, RiskPolicy, TreeValue, ValueDomain,
};
pub use field_cube::{
    CarrierKind, CarrierState, CarrierStatus, Coupling, EvidenceRef, FieldCube,
    FieldCubeBuilder, DefaultFieldCubeBuilder, PathSpec, TopologySummary,
};
pub use dof::{DoFEdge, DoFGraph, DoFNode, DoFNodeKind, NodeId, NodeStatus};
pub use plan::{
    CollapseEffect, CollapsePlan, CollapsePlanner, CollapseStep, CollapseStepKind,
    DefaultCollapsePlanner, FailurePolicy, OrderingPolicy,
};
pub use excision::{detect_path_excision, OperationalImpact, PathExcision};
pub use gate::{
    Candidate, GateCheck, GateEngine, GateReport, GateSeverity, MciGate,
};
pub use report::{
    CarrierMigrationReport, CommitOutcome, MigrationTrigger, RefinementRequest,
    TraversalRunDescriptor, TraversalRunReport,
};
pub use solver::{
    NullSolver, OracleSolver, ProjectedContext, Solver, SolverContext, TemplateSolver,
};
pub use norms::{
    CollapseCertificate, ConstraintLattice, NormFitness, NormKind, NormSpec,
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
    fn from(e: std::io::Error) -> Self { TraverseError::Io(e.to_string()) }
}
impl From<serde_json::Error> for TraverseError {
    fn from(e: serde_json::Error) -> Self { TraverseError::Parse(e.to_string()) }
}

/// Result alias.
pub type Result<T> = std::result::Result<T, TraverseError>;
