//! PSE-NCTCS-CONFORMANCE-01 — Null-Centered Toroidal Control Closure Layer.
//!
//! Implements the NCTCS closure layer as a submodule of the validation runner.
//! This layer reads existing artifacts, classifies C0–C4 conformance and
//! builds a content-addressed MacroControlState at C4.
//!
//! Entry point: [`pipeline::close_nctcs`].

pub mod candidate_audit;
pub mod conformance;
pub mod macro_control_state;
pub mod materialization_audit;
pub mod metrics;
pub mod null_center;
pub mod phase_visibility;
pub mod pipeline;
pub mod primitives;
pub mod replay;
pub mod report;
pub mod run_descriptor;
pub mod trace_replay_contract;

pub use conformance::{
    ConformanceCheck, NctcsConformanceClass, NctcsGateOutcome, ProofObligationRef,
    ValidationClosureStatus,
};
pub use macro_control_state::{AggregateGateStatus, MacroControlState};
pub use pipeline::{close_nctcs, NctcsClosureInput};
pub use primitives::{NctcsError, NctcsPolicies, NctcsThresholds};
pub use report::NctcsClosureBundle;
pub use run_descriptor::NctcsRunDescriptor;

#[cfg(test)]
mod tests;
