//! `ResonanceCluster` and `ClusterFormationReport` (§11).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};
use super::trident_vector::TridentVector;

/// Cluster lifecycle (§11.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ClusterLifecycle {
    /// Proposed but not yet evaluated.
    Proposed,
    /// Forming — gate evaluation in progress.
    Forming,
    /// Active — passing the formation gate.
    Active,
    /// Stabilised — high coherence over multiple ticks.
    Stabilized,
    /// Splitting morphology event in progress.
    Splitting,
    /// Fusing morphology event in progress.
    Fusing,
    /// Decaying — losing coherence.
    Decaying,
    /// Compacted into a trace.
    Compacted,
    /// Dissolved — working state cleared.
    Dissolved,
    /// Rejected by the formation gate.
    Rejected,
}

/// `ResonanceCluster` (§11.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResonanceCluster {
    /// Content-addressed cluster id.
    pub cluster_id: Hash256,
    /// Parent subnet id.
    pub parent_subnet_id: Hash256,
    /// Member cell ids (sorted).
    pub member_cells: Vec<Hash256>,
    /// Aggregate cluster trident.
    pub cluster_trident: TridentVector,
    /// Cluster coherence score.
    pub coherence_score: Fixed,
    /// Cluster conflict score.
    pub conflict_score: Fixed,
    /// Morphodynamic potential `H`.
    pub morphodynamic_potential: Fixed,
    /// Hash of the funnel graph.
    pub funnel_graph_hash: Hash256,
    /// Hash of the convergence field.
    pub convergence_field_hash: Hash256,
    /// Cluster lifecycle stage.
    pub lifecycle: ClusterLifecycle,
    /// Hash of the cluster trace artefact.
    pub trace_hash: Hash256,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl ResonanceCluster {
    /// Recompute `cluster_id` after sorting the member-cell list.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.member_cells.sort();
        let mut probe = self.clone();
        probe.cluster_id = Hash256::zero();
        self.cluster_id = content_address(&probe)?;
        Ok(self)
    }
}

/// `ClusterFormationReport` (§11.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterFormationReport {
    /// Content-addressed report id.
    pub report_id: Hash256,
    /// Source pool id.
    pub pool_id: Hash256,
    /// Cell ids the formation pass considered.
    pub proposed_member_cells: Vec<Hash256>,
    /// Cell ids actually accepted into the cluster.
    pub accepted_member_cells: Vec<Hash256>,
    /// Cell ids rejected by the formation gate.
    pub rejected_member_cells: Vec<Hash256>,
    /// Phase alignment score.
    pub phase_alignment_score: Fixed,
    /// Coherence score.
    pub coherence_score: Fixed,
    /// Morphodynamic potential.
    pub morphodynamic_potential: Fixed,
    /// Hash of the cluster purpose.
    pub purpose_hash: Hash256,
    /// Whether the trace material is ready.
    pub trace_ready: bool,
    /// Whether the formation gate passed.
    pub passed: bool,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl ClusterFormationReport {
    /// Recompute `report_id` after sorting.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.proposed_member_cells.sort();
        self.accepted_member_cells.sort();
        self.rejected_member_cells.sort();
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
    fn cluster_id_changes_with_membership() {
        let mut a = ResonanceCluster {
            cluster_id: Hash256::zero(),
            parent_subnet_id: Hash256::zero(),
            member_cells: vec![],
            cluster_trident: TridentVector::zero(),
            coherence_score: Fixed::Rational { num: 0, den: 1 },
            conflict_score: Fixed::Rational { num: 0, den: 1 },
            morphodynamic_potential: Fixed::Rational { num: 0, den: 1 },
            funnel_graph_hash: Hash256::zero(),
            convergence_field_hash: Hash256::zero(),
            lifecycle: ClusterLifecycle::Proposed,
            trace_hash: Hash256::zero(),
            evidence_refs: Vec::new(),
        };
        a = a.with_id().unwrap();
        let id_a = a.cluster_id.clone();
        let mut alt = [0u8; 32];
        alt[0] = 1;
        a.member_cells.push(Hash256(alt));
        let a = a.with_id().unwrap();
        assert_ne!(id_a, a.cluster_id);
    }
}
