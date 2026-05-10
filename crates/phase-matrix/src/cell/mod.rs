//! Cell substrate root module — re-exports the entire data + pipeline
//! surface defined in PHASEMATRIX-HIVEMIND-03 §6 through §22.
//!
//! ADAMANT alignment: the trace contract (§6.3), change-set discipline
//! (§6.4), replay contract (§6.6) and fail-closed gate semantics
//! (§3.4.4) are realised by [`cluster_trace`], [`replay`], [`pipeline`]
//! and [`cluster_gate`] respectively.

pub mod cell_pool;
pub mod cluster_gate;
pub mod cluster_trace;
pub mod convergence_field;
pub mod coupling_update;
pub mod dissolution;
pub mod field_tensor;
pub mod field_tensor_trace;
pub mod funnel_graph;
pub mod handoff;
pub mod local_resonance_processor;
pub mod matrix;
pub mod mirror_consistency;
pub mod morphodynamic_field;
pub mod ouroboros_feedback;
pub mod phase_cell;
pub mod pipeline;
pub mod primitives;
pub mod replay;
pub mod report;
pub mod resonance_cluster;
pub mod resonance_fabric;
pub mod resonance_pulse;
pub mod run_descriptor;
pub mod stitch_pipeline;
pub mod stitcher;
pub mod stitcher_gate;
pub mod tension_to_intent;
pub mod tensor_delta;
pub mod trident_vector;

pub use cell_pool::{CellPool, CellPoolBuildOptions};
pub use cluster_gate::{
    ClusterFormationGateOutcome, ClusterFormationPolicy, ClusterTracePolicy, DissolutionPolicy,
    IntentPolicy, MorphologyPolicy,
};
pub use cluster_trace::ClusterTrace;
pub use convergence_field::ConvergenceField;
pub use coupling_update::{
    build_coupling_update, derive_stitch_candidates, CouplingUpdate, StitchCandidate,
};
pub use dissolution::{DissolutionMode, DissolutionReport};
pub use field_tensor::{CouplingEntry, CouplingKind, CouplingMatrix, FieldTensorState};
pub use field_tensor_trace::{write_field_tensor_trace, FieldTensorTrace};
pub use funnel_graph::{FunnelEdge, FunnelEdgeKind, FunnelGraph, FunnelNode, FunnelNodeKind};
pub use handoff::{handoff_candidate_for, CellToHandoffCandidate};
pub use local_resonance_processor::{LocalResonanceProcessor, ResonanceNonlinearity};
pub use matrix::{
    MatrixBoundaryReport, MatrixClaim, MatrixClaimKind, MatrixGatePolicy, MatrixTrace,
    NodeTrustState, PhaseMatrixNode, PhaseSubnet, TruthMaintenanceReport,
};
pub use mirror_consistency::{evaluate_mirror_consistency, MirrorConsistencyReport};
pub use morphodynamic_field::{ClusterMorphologyEvent, MorphodynamicField, MorphologyDecision};
pub use ouroboros_feedback::RecursiveFeedbackReport;
pub use phase_cell::{CellLifecycle, PhaseCell, PhaseCellRole};
pub use pipeline::{
    run_cell_substrate_cycle, CellSubstrateCycleReport, CellSubstrateInput, CellSubstrateOutcome,
    ClusterHoldReport, ClusterRejectionReport, DeterminismReport,
};
pub use primitives::{
    canonical_bytes, content_address, fixed_abs_diff, fixed_add, fixed_ge, fixed_le, fixed_sub,
    max_fixed, min_fixed, normalize_rational, EvidenceRef, Fixed, Hash256, PhaseError, StableId,
};
pub use replay::{verify_cycle_replay, ReplayObservation};
pub use report::{CycleReportSummary, StitcherReport};
pub use resonance_cluster::{ClusterFormationReport, ClusterLifecycle, ResonanceCluster};
pub use resonance_fabric::{build_resonance_fabric, EphemeralResonanceLink, ResonanceFabricState};
pub use resonance_pulse::{PhaseBin, ResonancePulse};
pub use run_descriptor::{
    CellSubstratePolicies, CellSubstrateThresholds, PhaseMatrixRunDescriptorV3,
};
pub use stitch_pipeline::{run_stitch_cycle, verify_stitch_replay, StitchCycleBundle};
pub use stitcher::{
    evidence_ref_for_rd, StitchPolicies, StitchRunDescriptor, StitchThresholds, StitcherOutcome,
};
pub use stitcher_gate::{evaluate_stitcher_gates, StitchFailurePolicy, StitcherGateReport};
pub use tension_to_intent::{IntentCandidate, TensionToIntentOperator};
pub use tensor_delta::{compute_tensor_delta, TensorDeltaReport};
pub use trident_vector::TridentVector;
