//! Topological Panoptic Triangulation and Möbius-Tripolar Micro-Lift
//! (PSE-TRAVERSE-TPT-MTL-04).
//!
//! This module adds a deterministic topology-orientation layer above
//! the existing PSE/Traversal/Projection/Cognition/PhaseMatrix stack.
//!
//! ## Pipeline
//!
//! ```text
//! Σ_cog/matrix ──BuildWindow──▶ W_H5,K ──Triangulate──▶ MH
//!   ──LiftMicro──▶ {µi} ──Reinterpret──▶ (Claims, H, H̄, G) ──Gate──▶ Bundle
//! ```
//!
//! Use [`pipeline::run_tpt_mtl`] for the canonical end-to-end pass.
//!
//! ## Invariants
//!
//! * **I-01** State before topology — no window without valid, traceable state.
//! * **I-02** Window before point — isolated point is not triangulatable.
//! * **I-03** Axis separation — semantic / operative / carrier axes MUST be
//!   declared separately.
//! * **I-06** No stateful null center — `null_center_ref` is an ordering anchor,
//!   not a vertex.
//! * **I-07** Antiphase by proof — DualAntiphase requires a declared rule +
//!   proof + digest.
//! * **I-08** TopologyGuard on mutation — every mesh mutation produces a proof.
//! * **I-09** Budgeted recursion — all recursive refinement is bounded.
//! * **I-10** Replay identity — identical inputs + RD → identical digests.
//!
//! ## Conformance class implemented
//!
//! TPTM-5 (all required types, gates, replay, CLI, integration with
//! eval-matrix).
//!
//! ## Feature flags
//!
//! * `topology` — base types, PhaseSpaceWindow, MeshHolo, gates (default-on).
//! * `topology-cli` — builds `pse-traverse-topology` CLI.
//! * `topology-mtl` — MicroFiber, MTL-D1, SeamComponent, MicroLiftGate.
//! * `topology-refine` — recursive refinement operators.
//! * `topology-carrier` — CarrierGate, KairosGate, CarrierMigrationReports.
//! * `topology-reinterpret` — reinterpretation into Claims/Horizons/Gates.
//! * `topology-handoff` — handoff candidates without external commit.

pub mod artifact;
pub mod axis_bridge;
pub mod carrier;
pub mod dual_antiphase;
pub mod gates;
pub mod mesh_holo;
pub mod mesh_weights;
pub mod micro_fiber;
pub mod persistence;
pub mod phase_window;
pub mod pipeline;
pub mod primary_phase;
pub mod primitives;
pub mod refine;
pub mod reinterpret;
pub mod replay;
pub mod run_descriptor;
pub mod seam;
pub mod subwindow;
pub mod topology_guard;

// ── Convenience re-exports ────────────────────────────────────────────────────

pub use artifact::{
    form_topological_crystal_candidate, materialize_bundle, CandidateStatus,
    TopologicalCrystalCandidate, TptMtlBundle,
};
pub use axis_bridge::{build_axis_bridge_report, AxisBridgeReport, AxisLayer, AxisOriginDecl};
pub use carrier::{evaluate_carrier, CarrierContext, CarrierReport};
pub use dual_antiphase::{apply_mtl_d1, DualAntiphase, DualizationFailure, DualizationProof};
pub use gates::{gate_all, FailurePolicy, GateResult, TptMtlGateReport, TptMtlOutcomeKind};
pub use mesh_holo::{
    evolve_mesh, seed_mesh_holo, EntropyClass, MeshEdge, MeshHolo, MeshProof, MeshVertex, Simplex,
    TopologicalInvariants,
};
pub use mesh_weights::weight_mesh_with_microfibers;
pub use micro_fiber::{gate_micro_lift, lift_micro_fiber, lift_micro_fibers, MicroFiber};
pub use persistence::{PersistenceDiagram, PersistencePair};
pub use phase_window::{
    build_phase_space_window, PhaseSpaceWindow, PointMeta, PointStatus, SamplePoint, TptMtlInput,
    TptPoint,
};
pub use pipeline::{run_tpt_mtl, TptMtlOutcome, TptMtlPipelineResult};
pub use primary_phase::{compute_primary_phase, PrimaryPhase};
pub use primitives::{
    tpt_content_address, Fixed, Hash256, RationalFixed, TopologyError, TptEvidenceRef,
};
pub use refine::{refine_recursive, BudgetState};
pub use reinterpret::{
    reinterpret_mesh_to_panoptic, ClaimCandidate, ClaimStatus, HorizonUpdate, HorizonUpdateKind,
    ReinterpretationReport,
};
pub use replay::ReplayManifest;
pub use run_descriptor::{
    AxisPolicy, MeshPolicy, MicroLiftPolicy, NumericPolicy, TieBreakPolicy, TptMtlBudgets,
    TptMtlRunDescriptor, TptMtlThresholds,
};
pub use seam::{compute_seam, SeamComponent, SeamProof};
pub use subwindow::{build_local_subwindow, compute_centroid, LocalSubwindow};
pub use topology_guard::{check_topology_guard, TopologyGuardProof, TopologyGuardResult};
