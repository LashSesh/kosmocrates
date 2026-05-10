//! IsomorphicProjectionReport — verifies that a local field operator
//! implements the same operatorik as the global closure path.
//!
//! φ ∘ Π_U = O_global ∘ ι_U (on artifact level, not as ontological claim).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, MetatronError};

/// Report verifying local ↔ global operator isomorphism.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsomorphicProjectionReport {
    pub report_id: Hash256,
    pub local_operator_ref: Hash256,
    pub global_operator_ref: Hash256,
    pub local_path_hash: Hash256,
    pub global_path_hash: Hash256,
    /// Same operator path or compatible schema.
    pub operator_path_compatible: bool,
    /// Gate order preserved across local and global path.
    pub gate_order_preserved: bool,
    /// Trace dependencies preserved.
    pub trace_dependency_preserved: bool,
    /// Replay dependencies preserved.
    pub replay_dependency_preserved: bool,
    /// Euclidean distance between signature metrics (CanonicalNumber).
    pub signature_distance: Fixed,
    /// Composite isomorphism score (0–1 fixed-point).
    pub isomorphism_score: Fixed,
    pub passed: bool,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl IsomorphicProjectionReport {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        local_operator_ref: Hash256,
        global_operator_ref: Hash256,
        local_path_hash: Hash256,
        global_path_hash: Hash256,
        operator_path_compatible: bool,
        gate_order_preserved: bool,
        trace_dependency_preserved: bool,
        replay_dependency_preserved: bool,
        signature_distance: Fixed,
        isomorphism_score: Fixed,
        evidence_refs: Vec<EvidenceRef>,
    ) -> Result<Self, MetatronError> {
        let passed = operator_path_compatible
            && gate_order_preserved
            && trace_dependency_preserved
            && replay_dependency_preserved;

        let partial = IsomorphicProjectionReport {
            report_id: Hash256::zero(),
            local_operator_ref,
            global_operator_ref,
            local_path_hash,
            global_path_hash,
            operator_path_compatible,
            gate_order_preserved,
            trace_dependency_preserved,
            replay_dependency_preserved,
            signature_distance,
            isomorphism_score,
            passed,
            evidence_refs,
        };
        let id = content_address(&partial)?;
        Ok(IsomorphicProjectionReport {
            report_id: id,
            ..partial
        })
    }
}
