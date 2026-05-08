//! Null-Centered Horizon Geometry — `pse-traverse-horizon` v0.3.
//!
//! See `pse_traverse_horizon_spec_v0_3.pdf` in the repo root for the
//! normative specification this module realises.
//!
//! The horizon layer answers a single question:
//!
//! > *When is a stabilised, projection-capable state in the
//! > hypertoroidal phase space visible, causally admissible, dually
//! > recoverable, and finalisable?*
//!
//! It does **not** generate `SemanticCrystal` and does **not** replace
//! `pse-core`. The PSE-Bridge remains the only path from a finalised
//! emission to a crystal.
//!
//! ## Pipeline
//!
//! ```text
//! StableState → NullCenter → HorizonChart → PhaseRays → ProjectionCone
//!   → HorizonCrossing → DualityCheck → CombinedGate → FinalizedEmission
//! ```
//!
//! Use [`crate::horizon::pipeline::run_horizon_v3`] for the canonical
//! end-to-end pass.
//!
//! ## Determinism contract
//!
//! Every map is a `BTreeMap`, every list is sorted before hashing, every
//! gate-relevant scalar is a [`crate::horizon::primitives::Fixed`]
//! (`CanonicalNumber` from the dynamics layer — never a platform float),
//! and every report is content-addressed via JCS + SHA-256. Two runs
//! with byte-identical descriptors and inputs MUST produce
//! byte-identical reports.

pub mod causal_admissibility;
pub mod certificate;
pub mod collapse_emission_duality;
pub mod gate;
pub mod horizon_chart;
pub mod horizon_crossing;
pub mod horizon_window;
pub mod outcome;
pub mod phase_carrier_v3;
pub mod phase_ray;
pub mod pipeline;
pub mod primitives;
pub mod projection_cone;
pub mod replay;
pub mod run_descriptor;

// Re-export the most commonly used types so downstream code can write
// `use pse_traverse::horizon::*` and reach everything in §5–§9.
pub use causal_admissibility::{CausalAdmissibilityReport, CausalViolation};
pub use certificate::HorizonCertificate;
pub use collapse_emission_duality::{
    check_collapse_emission_duality, CollapseEmissionDualitySpec, DualityReport,
};
pub use gate::{merge_projection_and_horizon_gates, CombinedGateOutcome, ProjectionV2OutcomeRef};
pub use horizon_chart::{null_center_unfold, HorizonChart};
pub use horizon_crossing::{horizon_crossing_gate, HorizonCrossingReport};
pub use horizon_window::{
    evaluate_horizon_windows, evaluate_window, EventHorizonWindowV3, HorizonWindowReportV3,
};
pub use outcome::{
    outcome_for_policy, FinalizedEmissionV3, HorizonDiagnostic, HorizonDiagnosticKind,
    HorizonHoldReport, HorizonV3Outcome,
};
pub use phase_carrier_v3::{PhaseCarrierRole, PhaseCarrierSpecV3, PhaseQuantizationV3};
pub use phase_ray::{attach_rays_to_chart, lift_phase_rays, PhaseRay};
pub use pipeline::{run_horizon_v3, HorizonInput, HorizonRunResult};
pub use primitives::{
    fixed_abs_diff, fixed_ge, fixed_le, fixed_sub, EpochRange, Fixed, HorizonError,
    HorizonEvidenceRef, RationalFixed, StableId,
};
pub use projection_cone::{check_projection_cone, ProjectionCone, ProjectionConeReport};
pub use replay::{assert_replay_match, replay_hash_of};
pub use run_descriptor::{
    CarrierPolicyV3, CausalPolicyV3, CrossingPolicyV3, DualityPolicyV3, HorizonFailurePolicy,
    HorizonPoliciesV3, HorizonRunDescriptorV3, HorizonThresholdsV3, HorizonWindowPolicyV3,
    ProjectionConePolicyV3,
};
