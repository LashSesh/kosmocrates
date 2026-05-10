//! HolisticEigenmodeState — the content-addressed global closure state.
//!
//! The MetatronOperator is NOT a controller. It is the deterministic,
//! content-addressed, replayable projection of global system coherence
//! from: persistent field tensor, MacroControlState, Trace, Conformance,
//! spectral gap, Mirror/Stitcher consistency and local Monolith projections.

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, MetatronError, StableId};

/// Conformance class of the Metatron closure run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MetatronConformanceClass {
    M0,
    M1,
    M2,
    M3,
    M4,
    M5,
}

impl MetatronConformanceClass {
    pub fn as_str(self) -> &'static str {
        match self {
            MetatronConformanceClass::M0 => "M0",
            MetatronConformanceClass::M1 => "M1",
            MetatronConformanceClass::M2 => "M2",
            MetatronConformanceClass::M3 => "M3",
            MetatronConformanceClass::M4 => "M4",
            MetatronConformanceClass::M5 => "M5",
        }
    }
}

/// Validation status for the Metatron closure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MetatronValidationStatus {
    Rejected,
    Diagnostic,
    Closed,
}

/// Aggregate gate status across the Metatron gate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateGateStatus {
    pub total_gates: u64,
    pub passed_gates: u64,
    pub failed_gates: u64,
}

/// The HolisticEigenmodeState — content-addressed output of the Metatron closure.
///
/// Only produced when G_meta = 1. Fail-closed: never produced with productive
/// status when any gate fails.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolisticEigenmodeState {
    pub state_id: Hash256,
    pub parent_run_id: Hash256,
    pub null_center_ref: Hash256,
    pub field_tensor_ref: Hash256,
    pub macro_control_state_ref: Hash256,
    pub trace_head: Hash256,
    pub local_monolith_refs: Vec<Hash256>,
    pub isomorphic_projection_refs: Vec<Hash256>,
    pub spectral_gap_report_ref: Hash256,
    pub aggregate_gate_status: AggregateGateStatus,
    pub global_coherence: Fixed,
    pub conformance_class: MetatronConformanceClass,
    pub validation_status: MetatronValidationStatus,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl HolisticEigenmodeState {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        parent_run_id: Hash256,
        null_center_ref: Hash256,
        field_tensor_ref: Hash256,
        macro_control_state_ref: Hash256,
        trace_head: Hash256,
        mut local_monolith_refs: Vec<Hash256>,
        mut isomorphic_projection_refs: Vec<Hash256>,
        spectral_gap_report_ref: Hash256,
        aggregate_gate_status: AggregateGateStatus,
        global_coherence: Fixed,
        conformance_class: MetatronConformanceClass,
        validation_status: MetatronValidationStatus,
        evidence_refs: Vec<EvidenceRef>,
    ) -> Result<Self, MetatronError> {
        local_monolith_refs.sort();
        isomorphic_projection_refs.sort();

        let partial = HolisticEigenmodeState {
            state_id: Hash256::zero(),
            parent_run_id,
            null_center_ref,
            field_tensor_ref,
            macro_control_state_ref,
            trace_head,
            local_monolith_refs,
            isomorphic_projection_refs,
            spectral_gap_report_ref,
            aggregate_gate_status,
            conformance_class,
            validation_status,
            global_coherence,
            evidence_refs,
        };
        let id = content_address(&partial)?;
        Ok(HolisticEigenmodeState {
            state_id: id,
            ..partial
        })
    }

    pub fn content_hash(&self) -> Result<Hash256, MetatronError> {
        content_address(self)
    }
}
