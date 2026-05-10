//! `ClusterTrace` (§17).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Hash256, PhaseError};

/// `ClusterTrace` (§17).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterTrace {
    /// Content-addressed trace id.
    pub trace_id: Hash256,
    /// Cluster the trace belongs to.
    pub cluster_id: Hash256,
    /// Hash of the formation report.
    pub formation_report_hash: Hash256,
    /// Hash of the funnel graph.
    pub funnel_graph_hash: Hash256,
    /// Hash of the convergence field.
    pub convergence_field_hash: Hash256,
    /// Sorted morphology decision hashes.
    pub morphology_decision_hashes: Vec<Hash256>,
    /// Sorted intent candidate hashes.
    pub intent_candidate_hashes: Vec<Hash256>,
    /// Optional dissolution report hash.
    pub dissolution_report_hash: Option<Hash256>,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl ClusterTrace {
    /// Recompute `trace_id` after sorting decision + intent lists.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.morphology_decision_hashes.sort();
        self.intent_candidate_hashes.sort();
        let mut probe = self.clone();
        probe.trace_id = Hash256::zero();
        self.trace_id = content_address(&probe)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_trace_is_content_addressed() {
        let a = ClusterTrace {
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
        assert_eq!(a.trace_id, a.clone().with_id().unwrap().trace_id);
    }
}
