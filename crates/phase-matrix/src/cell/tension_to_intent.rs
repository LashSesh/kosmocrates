//! `TensionToIntentOperator` and `IntentCandidate` (§14).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};

/// `IntentCandidate` (§14.2). Produced by the operator only when the
/// intent gate fires; never directly executed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentCandidate {
    /// Content-addressed candidate id.
    pub candidate_id: Hash256,
    /// Source cluster id.
    pub source_cluster_id: Hash256,
    /// Hash of the cluster purpose.
    pub purpose_hash: Hash256,
    /// Hash of the intent vector.
    pub intent_vector_hash: Hash256,
    /// Confidence in `[0, 1]`.
    pub confidence: Fixed,
    /// Sorted claim references.
    pub claim_refs: Vec<Hash256>,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl IntentCandidate {
    /// Recompute `candidate_id` after sorting claim refs.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.claim_refs.sort();
        let mut probe = self.clone();
        probe.candidate_id = Hash256::zero();
        self.candidate_id = content_address(&probe)?;
        Ok(self)
    }
}

/// `TensionToIntentOperator` (§14.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TensionToIntentOperator {
    /// Content-addressed operator id.
    pub operator_id: Hash256,
    /// Cluster the operator was applied to.
    pub cluster_id: Hash256,
    /// Field tension at evaluation time.
    pub field_tension: Fixed,
    /// Convergence score at evaluation time.
    pub convergence_score: Fixed,
    /// Tension threshold the gate compared against.
    pub intent_threshold: Fixed,
    /// Hash of the intent candidate (when the gate passed).
    pub intent_candidate_hash: Option<Hash256>,
    /// Whether the intent gate passed.
    pub passed: bool,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl TensionToIntentOperator {
    /// Recompute `operator_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.operator_id = Hash256::zero();
        self.operator_id = content_address(&probe)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_candidate_is_content_addressed() {
        let c = IntentCandidate {
            candidate_id: Hash256::zero(),
            source_cluster_id: Hash256::zero(),
            purpose_hash: Hash256::zero(),
            intent_vector_hash: Hash256::zero(),
            confidence: Fixed::Rational { num: 1, den: 2 },
            claim_refs: vec![],
            evidence_refs: vec![],
        }
        .with_id()
        .unwrap();
        let again = c.clone().with_id().unwrap();
        assert_eq!(c.candidate_id, again.candidate_id);
    }
}
