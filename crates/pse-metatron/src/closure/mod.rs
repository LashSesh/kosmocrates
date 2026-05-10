//! PSE-METATRON-MONOLITH-01 — Holistic Eigenmode Closure Layer.
//!
//! Additive closure layer over the existing PSE stack. Does not introduce
//! a new engine. Reads NCTCS/PhaseMatrix/Stitch/TPT/Eval artifacts, evaluates
//! local Monolith projections, isomorphism, spectral gap, trace and replay,
//! and produces a content-addressed HolisticEigenmodeState on gate pass.
//!
//! Entry point: [`pipeline::run_metatron_closure`].

pub mod gates;
pub mod holistic_state;
pub mod isomorphic;
pub mod local_projection;
pub mod metrics;
pub mod pipeline;
pub mod primitives;
pub mod replay;
pub mod run_descriptor;
pub mod spectral_gap;

pub use gates::MetatronGateReport;
pub use holistic_state::{
    HolisticEigenmodeState, MetatronConformanceClass, MetatronValidationStatus,
};
pub use isomorphic::IsomorphicProjectionReport;
pub use local_projection::LocalMonolithProjection;
pub use pipeline::{
    run_metatron_closure, MetatronClosureInput, MetatronClosureOutcome, MetatronDiagnosticReport,
};
pub use primitives::{
    EvidenceRef, Fixed, MetatronError, MetatronPolicies, MetatronThresholds, NctcsClass,
};
pub use run_descriptor::MetatronRunDescriptor;
pub use spectral_gap::SpectralGapStitchReport;

#[cfg(test)]
mod tests;
