//! MacroControlState — the content-addressed closure of the global system
//! state, derived exclusively from tensor history, trace, policies and
//! evaluation result (C4 condition).

use serde::{Deserialize, Serialize};

use super::conformance::{NctcsConformanceClass, ValidationClosureStatus};
use super::primitives::{content_address, EvidenceRef, Hash256, NctcsError, StableId};

/// Aggregate gate status across all gates in the run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateGateStatus {
    pub total_gates: u64,
    pub passed_gates: u64,
    pub failed_gates: u64,
    pub held_gates: u64,
    pub quarantined_candidates: u64,
    pub no_update_count: u64,
    pub gate_coverage: pse_traverse::dynamic_state::CanonicalNumber,
}

impl AggregateGateStatus {
    pub fn gate_pass_rate(&self) -> f64 {
        if self.total_gates == 0 {
            return 1.0;
        }
        self.passed_gates as f64 / self.total_gates as f64
    }
}

/// The verdichteter MacroControlState (C4).
///
/// MUST NOT be derived from resonance or ephemeral fabric directly.
/// MUST be derived from null_center_ref, field_tensor (via tensor history),
/// trace, policies, certificates and evaluation result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroControlState {
    pub macro_state_id: Hash256,
    pub run_id: StableId,
    pub null_center_ref: Hash256,
    pub field_tensor_ref: Option<Hash256>,
    pub trace_head: Hash256,
    pub orientation_map_hash: Option<Hash256>,
    pub active_horizons: Vec<Hash256>,
    pub blocked_horizons: Vec<Hash256>,
    pub attractor_map_hash: Option<Hash256>,
    pub aggregate_gate_status: AggregateGateStatus,
    pub conformance_class: NctcsConformanceClass,
    pub validation_status: ValidationClosureStatus,
    pub eval_summary_hash: Option<Hash256>,
    pub domain_validation_hash: Option<Hash256>,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl MacroControlState {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        run_id: StableId,
        null_center_ref: Hash256,
        field_tensor_ref: Option<Hash256>,
        trace_head: Hash256,
        orientation_map_hash: Option<Hash256>,
        mut active_horizons: Vec<Hash256>,
        mut blocked_horizons: Vec<Hash256>,
        attractor_map_hash: Option<Hash256>,
        aggregate_gate_status: AggregateGateStatus,
        conformance_class: NctcsConformanceClass,
        validation_status: ValidationClosureStatus,
        eval_summary_hash: Option<Hash256>,
        domain_validation_hash: Option<Hash256>,
        evidence_refs: Vec<EvidenceRef>,
    ) -> Result<Self, NctcsError> {
        active_horizons.sort();
        blocked_horizons.sort();

        let partial = MacroControlState {
            macro_state_id: Hash256::zero(),
            run_id,
            null_center_ref,
            field_tensor_ref,
            trace_head,
            orientation_map_hash,
            active_horizons,
            blocked_horizons,
            attractor_map_hash,
            aggregate_gate_status,
            conformance_class,
            validation_status,
            eval_summary_hash,
            domain_validation_hash,
            evidence_refs,
        };
        let id = content_address(&partial)?;
        Ok(MacroControlState {
            macro_state_id: id,
            ..partial
        })
    }

    pub fn content_hash(&self) -> Result<Hash256, NctcsError> {
        content_address(self)
    }
}
