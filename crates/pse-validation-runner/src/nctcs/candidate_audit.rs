//! CandidateFormationAudit — verifies that candidates imply phase-gated
//! visibility, activation, boundary-readiness, evidence and trace (C1).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, Fixed, Hash256, NctcsError};

/// Audit of candidate formation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateFormationAudit {
    pub audit_id: Hash256,
    pub candidate_refs: Vec<Hash256>,
    pub rejected_refs: Vec<Hash256>,
    /// All candidates had a visible phase cell as prerequisite.
    pub candidate_requires_visibility_passed: bool,
    pub activation_gate_coverage: Fixed,
    pub boundary_ready_coverage: Fixed,
    pub evidence_ready_coverage: Fixed,
    pub trace_replay_ready_coverage: Fixed,
    pub passed: bool,
}

impl CandidateFormationAudit {
    pub fn build(
        mut candidate_refs: Vec<Hash256>,
        mut rejected_refs: Vec<Hash256>,
        candidate_requires_visibility_passed: bool,
        activation_gate_coverage: Fixed,
        boundary_ready_coverage: Fixed,
        evidence_ready_coverage: Fixed,
        trace_replay_ready_coverage: Fixed,
    ) -> Result<Self, NctcsError> {
        candidate_refs.sort();
        rejected_refs.sort();

        let passed = candidate_requires_visibility_passed;

        let partial = CandidateFormationAudit {
            audit_id: Hash256::zero(),
            candidate_refs,
            rejected_refs,
            candidate_requires_visibility_passed,
            activation_gate_coverage,
            boundary_ready_coverage,
            evidence_ready_coverage,
            trace_replay_ready_coverage,
            passed,
        };
        let id = content_address(&partial)?;
        Ok(CandidateFormationAudit {
            audit_id: id,
            ..partial
        })
    }
}
