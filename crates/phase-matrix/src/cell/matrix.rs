//! Matrix base types — the minimal substrate the cell layer requires
//! (PHASEMATRIX-HIVEMIND-03 §6, ADAMANT §17 federation hooks).
//!
//! Every cell, cluster, claim and dissolution event lives **inside** a
//! `PhaseSubnet` belonging to a `PhaseMatrixNode`. The `MatrixGatePolicy`
//! is the single declarative policy that the matrix-boundary gate
//! consults for trace, truth and conflict requirements.

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};

/// Trust state of a `PhaseMatrixNode` (§6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NodeTrustState {
    /// Trust state has not been established.
    Unknown,
    /// Node admitted as a candidate but not yet confirmed.
    Candidate,
    /// Node is active and seam-locked with the matrix.
    Active,
    /// Node has been quarantined after a symmetry break.
    Quarantined,
    /// Node has been retired and may not contribute claims.
    Retired,
}

/// `PhaseMatrixNode` (§6.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseMatrixNode {
    /// Content-addressed node id.
    pub node_id: Hash256,
    /// Matrix the node belongs to.
    pub matrix_id: Hash256,
    /// Constitutional null-center digest (ADAMANT §29).
    pub null_center_id: Hash256,
    /// Local phase offset on the constitutional clock.
    pub local_phase: Fixed,
    /// Current trust state.
    pub trust_state: NodeTrustState,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl PhaseMatrixNode {
    /// Recompute the content-addressed `node_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.node_id = Hash256::zero();
        self.node_id = content_address(&probe)?;
        Ok(self)
    }
}

/// `PhaseSubnet` (§6.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseSubnet {
    /// Content-addressed subnet id.
    pub subnet_id: Hash256,
    /// Anchor matrix.
    pub matrix_id: Hash256,
    /// Member node ids (sorted).
    pub member_nodes: Vec<Hash256>,
    /// Hash of the federated claim graph.
    pub claim_graph_hash: Hash256,
    /// Hash of the truth-maintenance state.
    pub truth_state_hash: Hash256,
    /// Hash of the active boundary policy.
    pub boundary_policy_hash: Hash256,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl PhaseSubnet {
    /// Recompute `subnet_id` after sorting the member node list.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.member_nodes.sort();
        let mut probe = self.clone();
        probe.subnet_id = Hash256::zero();
        self.subnet_id = content_address(&probe)?;
        Ok(self)
    }

    /// Build a synthetic subnet (used by the synthetic executor and the
    /// CLI golden fixtures). Every field is content-addressed.
    pub fn synthetic(matrix_id: Hash256, member_count: u8) -> Result<Self, PhaseError> {
        let mut members = Vec::with_capacity(member_count as usize);
        for i in 0..member_count {
            let node = PhaseMatrixNode {
                node_id: Hash256::zero(),
                matrix_id: matrix_id.clone(),
                null_center_id: matrix_id.clone(),
                local_phase: Fixed::Rational {
                    num: i as i128,
                    den: (member_count as i128).max(1),
                },
                trust_state: NodeTrustState::Active,
                evidence_refs: Vec::new(),
            }
            .with_id()?;
            members.push(node.node_id);
        }
        let subnet = PhaseSubnet {
            subnet_id: Hash256::zero(),
            matrix_id,
            member_nodes: members,
            claim_graph_hash: Hash256::zero(),
            truth_state_hash: Hash256::zero(),
            boundary_policy_hash: Hash256::zero(),
            evidence_refs: Vec::new(),
        };
        subnet.with_id()
    }
}

/// Matrix claim kinds per §6.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MatrixClaimKind {
    /// Plain observation injected into the claim graph.
    Observation,
    /// Intent candidate emitted by the cell substrate.
    IntentCandidate,
    /// Cluster trace summary (post-dissolution).
    ClusterTraceSummary,
    /// Boundary-related event (e.g. quarantine).
    BoundaryEvent,
    /// Handoff candidate registered for downstream layers.
    HandoffCandidate,
}

/// `MatrixClaim` (§6.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixClaim {
    /// Content-addressed claim id.
    pub claim_id: Hash256,
    /// Source reference (cluster, cell, or external).
    pub source_ref: Hash256,
    /// Claim kind.
    pub claim_kind: MatrixClaimKind,
    /// Hash of the claim payload.
    pub payload_hash: Hash256,
    /// Confidence value in `[0, 1]`.
    pub confidence: Fixed,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl MatrixClaim {
    /// Recompute `claim_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.claim_id = Hash256::zero();
        self.claim_id = content_address(&probe)?;
        Ok(self)
    }
}

/// `TruthMaintenanceReport` (§6.2). Records the matrix's resolution of
/// claim conflicts; `passed` is the reported truth-maintenance verdict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruthMaintenanceReport {
    /// Content-addressed report id.
    pub report_id: Hash256,
    /// Claim ids checked.
    pub checked_claims: Vec<Hash256>,
    /// Claim ids accepted.
    pub accepted_claims: Vec<Hash256>,
    /// Claim ids rejected.
    pub rejected_claims: Vec<Hash256>,
    /// Aggregated conflict score (lower is better).
    pub conflict_score: Fixed,
    /// Whether the truth-maintenance gate passed.
    pub passed: bool,
}

impl TruthMaintenanceReport {
    /// Recompute `report_id` after sorting the claim lists.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.checked_claims.sort();
        self.accepted_claims.sort();
        self.rejected_claims.sort();
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        self.report_id = content_address(&probe)?;
        Ok(self)
    }
}

/// `MatrixBoundaryReport` (§6.2). Records boundary-policy compliance
/// for a single subject (cluster, claim, dissolution).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixBoundaryReport {
    /// Content-addressed report id.
    pub report_id: Hash256,
    /// Hash of the subject the boundary check covered.
    pub subject_id: Hash256,
    /// Hash of the boundary policy that produced the verdict.
    pub boundary_policy_hash: Hash256,
    /// Sorted, deduplicated violation messages.
    pub violations: Vec<String>,
    /// Whether the boundary check passed.
    pub passed: bool,
}

impl MatrixBoundaryReport {
    /// Recompute `report_id` after sorting violations.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.violations.sort();
        self.violations.dedup();
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        self.report_id = content_address(&probe)?;
        Ok(self)
    }
}

/// `MatrixTrace` (§6.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixTrace {
    /// Content-addressed trace id.
    pub trace_id: Hash256,
    /// Subject the trace anchors to.
    pub source_id: Hash256,
    /// Event hashes (sorted).
    pub event_hashes: Vec<Hash256>,
    /// Report hashes (sorted).
    pub report_hashes: Vec<Hash256>,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl MatrixTrace {
    /// Recompute `trace_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.event_hashes.sort();
        self.report_hashes.sort();
        let mut probe = self.clone();
        probe.trace_id = Hash256::zero();
        self.trace_id = content_address(&probe)?;
        Ok(self)
    }
}

/// `MatrixGatePolicy` (§6.3).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MatrixGatePolicy {
    /// Whether the matrix-boundary gate must pass for any cluster event.
    pub require_boundary_pass: bool,
    /// Whether the truth-maintenance gate must pass.
    pub require_truth_pass: bool,
    /// Whether the active trace MUST be ready before a gate decision.
    pub require_trace_ready: bool,
    /// Maximum admissible aggregated conflict score.
    pub max_conflict_score: Fixed,
}

impl MatrixGatePolicy {
    /// Strict default policy that mirrors the spec's safe baseline.
    pub fn strict() -> Self {
        MatrixGatePolicy {
            require_boundary_pass: true,
            require_truth_pass: true,
            require_trace_ready: true,
            max_conflict_score: Fixed::Rational { num: 1, den: 4 },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_subnet_is_content_addressed() {
        let a = PhaseSubnet::synthetic(Hash256::zero(), 3).unwrap();
        let b = PhaseSubnet::synthetic(Hash256::zero(), 3).unwrap();
        assert_eq!(a.subnet_id, b.subnet_id);
    }

    #[test]
    fn synthetic_subnet_changes_with_member_count() {
        let a = PhaseSubnet::synthetic(Hash256::zero(), 3).unwrap();
        let b = PhaseSubnet::synthetic(Hash256::zero(), 5).unwrap();
        assert_ne!(a.subnet_id, b.subnet_id);
    }

    #[test]
    fn boundary_report_is_content_addressed() {
        let r = MatrixBoundaryReport {
            report_id: Hash256::zero(),
            subject_id: Hash256::zero(),
            boundary_policy_hash: Hash256::zero(),
            violations: vec!["dup".into(), "dup".into()],
            passed: true,
        }
        .with_id()
        .unwrap();
        // Duplicates removed.
        assert_eq!(r.violations.len(), 1);
    }
}
