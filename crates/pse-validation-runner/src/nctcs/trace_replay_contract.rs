//! TraceReplayContractReport — verifies that every persistent artifact
//! is trace-ready, evidence-ready and replay-ready (C3 condition).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, Fixed, Hash256, NctcsError};

/// Report verifying the trace/replay contract for all persistent artifacts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceReplayContractReport {
    pub report_id: Hash256,
    pub trace_head: Hash256,
    pub evidence_root: Hash256,
    pub gate_history_root: Hash256,
    pub replay_manifest_hash: Hash256,
    pub canonicalization_hash: Hash256,
    /// Fraction of persistent artifacts with replay-identical reconstruction.
    pub replay_identity: Fixed,
    /// Gate-pass requires replay-readiness.
    pub replay_ready_required_for_gate_pass: bool,
    pub passed: bool,
}

impl TraceReplayContractReport {
    pub fn build(
        trace_head: Hash256,
        evidence_root: Hash256,
        gate_history_root: Hash256,
        replay_manifest_hash: Hash256,
        canonicalization_hash: Hash256,
        replay_identity: Fixed,
        replay_ready_required_for_gate_pass: bool,
    ) -> Result<Self, NctcsError> {
        use pse_traverse::dynamic_state::CanonicalNumber;
        let identity_val = match &replay_identity {
            CanonicalNumber::FixedI64 { raw, scale } => *raw * 10i64.pow(2 - scale.min(&2)),
            _ => 0,
        };
        let passed = identity_val >= 10000 && replay_ready_required_for_gate_pass;

        let partial = TraceReplayContractReport {
            report_id: Hash256::zero(),
            trace_head,
            evidence_root,
            gate_history_root,
            replay_manifest_hash,
            canonicalization_hash,
            replay_identity,
            replay_ready_required_for_gate_pass,
            passed,
        };
        let id = content_address(&partial)?;
        Ok(TraceReplayContractReport {
            report_id: id,
            ..partial
        })
    }
}
