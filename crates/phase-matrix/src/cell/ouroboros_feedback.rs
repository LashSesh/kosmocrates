//! `RecursiveFeedbackReport` — bounded feedback (§15).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};

/// `RecursiveFeedbackReport` (§15.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecursiveFeedbackReport {
    /// Content-addressed report id.
    pub report_id: Hash256,
    /// Subject id (cell or cluster).
    pub target_id: Hash256,
    /// Hash of the previous state.
    pub previous_state_hash: Hash256,
    /// Hash of the trident response.
    pub trident_response_hash: Hash256,
    /// Bounded learning rate `η`.
    pub learning_rate: Fixed,
    /// Hash of the delta produced by the feedback step.
    pub delta_hash: Hash256,
    /// Hash of the new state.
    pub new_state_hash: Hash256,
    /// Whether the bounded-step constraint held.
    pub bounded: bool,
    /// Whether the feedback gate passed.
    pub passed: bool,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl RecursiveFeedbackReport {
    /// Recompute `report_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        self.report_id = content_address(&probe)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursive_feedback_report_is_content_addressed() {
        let r = RecursiveFeedbackReport {
            report_id: Hash256::zero(),
            target_id: Hash256::zero(),
            previous_state_hash: Hash256::zero(),
            trident_response_hash: Hash256::zero(),
            learning_rate: Fixed::Rational { num: 1, den: 100 },
            delta_hash: Hash256::zero(),
            new_state_hash: Hash256::zero(),
            bounded: true,
            passed: true,
            evidence_refs: Vec::new(),
        }
        .with_id()
        .unwrap();
        assert_eq!(r.report_id, r.clone().with_id().unwrap().report_id);
    }
}
