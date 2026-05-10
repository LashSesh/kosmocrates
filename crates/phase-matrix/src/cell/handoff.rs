//! `CellToHandoffCandidate` (§20.2).
//!
//! Spec invariant: handoff produces only **candidates**; it MUST NOT
//! emit external commit or finalisation artefacts.

use serde::{Deserialize, Serialize};

use super::cluster_trace::ClusterTrace;
use super::primitives::{content_address, EvidenceRef, Hash256, PhaseError};
use super::tension_to_intent::IntentCandidate;

/// Handoff candidate (§20.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellToHandoffCandidate {
    /// Content-addressed handoff id.
    pub handoff_id: Hash256,
    /// Source intent candidate id.
    pub intent_candidate_id: Hash256,
    /// Cluster trace id.
    pub cluster_trace_id: Hash256,
    /// Sorted matrix-claim references.
    pub matrix_claim_refs: Vec<Hash256>,
    /// Optional adapter recommendation (free text id).
    pub recommended_pse_adapter: Option<String>,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl CellToHandoffCandidate {
    /// Recompute `handoff_id` after sorting claim refs.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.matrix_claim_refs.sort();
        let mut probe = self.clone();
        probe.handoff_id = Hash256::zero();
        self.handoff_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Build a handoff candidate from an intent + cluster trace.
pub fn handoff_candidate_for(
    intent: &IntentCandidate,
    trace: &ClusterTrace,
) -> Result<CellToHandoffCandidate, PhaseError> {
    CellToHandoffCandidate {
        handoff_id: Hash256::zero(),
        intent_candidate_id: intent.candidate_id.clone(),
        cluster_trace_id: trace.trace_id.clone(),
        matrix_claim_refs: intent.claim_refs.clone(),
        recommended_pse_adapter: None,
        evidence_refs: vec![],
    }
    .with_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::primitives::Fixed;

    #[test]
    fn handoff_does_not_create_external_commit_artifacts() {
        let intent = IntentCandidate {
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
        let trace = ClusterTrace {
            trace_id: Hash256::zero(),
            cluster_id: Hash256::zero(),
            formation_report_hash: Hash256::zero(),
            funnel_graph_hash: Hash256::zero(),
            convergence_field_hash: Hash256::zero(),
            morphology_decision_hashes: vec![],
            intent_candidate_hashes: vec![],
            dissolution_report_hash: None,
            evidence_refs: vec![],
        }
        .with_id()
        .unwrap();
        let h = handoff_candidate_for(&intent, &trace).unwrap();
        let bytes = pse_traverse::canonical::canonical_bytes(&h).unwrap();
        let s = String::from_utf8_lossy(&bytes);
        // The handoff candidate must NOT contain commit / finalised
        // emission markers.
        assert!(!s.contains("SemanticCrystal"));
        assert!(!s.contains("FinalizedEmission"));
        assert!(!s.contains("commit"));
    }
}
