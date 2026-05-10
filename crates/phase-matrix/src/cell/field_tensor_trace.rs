//! `FieldTensorTrace` and trace-writing logic
//! (PHASEMATRIX-HIVEMIND-03.1 §10).
//!
//! The trace is an append-only audit log of what changed in Fabric-T
//! during one stitch cycle. It preserves the Dissolution-Grundsatz:
//! trace + evidence must never be lost.

use serde::{Deserialize, Serialize};

use super::coupling_update::CouplingUpdate;
use super::field_tensor::FieldTensorState;
use super::primitives::{content_address, EvidenceRef, Hash256, PhaseError};
use super::stitcher::StitchRunDescriptor;
use super::stitcher_gate::StitcherGateReport;

/// Append-only trace of one stitch cycle's effect on Fabric-T (§10).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FieldTensorTrace {
    /// Content address of this trace.
    pub trace_id: Hash256,
    /// Content address of `FieldTensorState` before this cycle.
    pub tensor_before_hash: Hash256,
    /// Content address of `FieldTensorState` after this cycle.
    pub tensor_after_hash: Hash256,
    /// Content addresses of accepted `CouplingUpdate`s (sorted).
    pub coupling_update_hashes: Vec<Hash256>,
    /// Content addresses of rejected `StitchCandidate`s (sorted).
    pub rejected_candidate_hashes: Vec<Hash256>,
    /// Content addresses of the `ClusterTrace`s that fed Fabric-H.
    pub source_cluster_trace_hashes: Vec<Hash256>,
    /// Content addresses of boundary reports referenced.
    pub boundary_report_hashes: Vec<Hash256>,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl FieldTensorTrace {
    /// Recompute `trace_id`.
    ///
    /// All hash-lists are sorted before hashing to ensure determinism.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.coupling_update_hashes.sort();
        self.rejected_candidate_hashes.sort();
        self.source_cluster_trace_hashes.sort();
        self.boundary_report_hashes.sort();
        let mut probe = self.clone();
        probe.trace_id = Hash256::zero();
        self.trace_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Write a `FieldTensorTrace` for the completed stitch cycle (step 11).
pub fn write_field_tensor_trace(
    rd: &StitchRunDescriptor,
    tensor_before: &FieldTensorState,
    tensor_after: &FieldTensorState,
    accepted_updates: &[CouplingUpdate],
    gate_reports: &[StitcherGateReport],
    source_cluster_trace_hashes: Vec<Hash256>,
) -> Result<FieldTensorTrace, PhaseError> {
    let mut coupling_update_hashes: Vec<Hash256> = accepted_updates
        .iter()
        .map(|u| u.update_id.clone())
        .collect();
    coupling_update_hashes.sort();

    // Rejected = candidates for which gate passed=false.
    let mut rejected_candidate_hashes: Vec<Hash256> = gate_reports
        .iter()
        .filter(|g| !g.passed)
        .map(|g| g.candidate_id.clone())
        .collect();
    rejected_candidate_hashes.sort();

    FieldTensorTrace {
        trace_id: Hash256::zero(),
        tensor_before_hash: tensor_before.tensor_id.clone(),
        tensor_after_hash: tensor_after.tensor_id.clone(),
        coupling_update_hashes,
        rejected_candidate_hashes,
        source_cluster_trace_hashes,
        boundary_report_hashes: Vec::new(),
        evidence_refs: vec![EvidenceRef {
            kind: "stitch.run_descriptor".into(),
            address: content_address(rd)?,
            relation: "governed_by".into(),
        }],
    }
    .with_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::field_tensor::CouplingMatrix;
    use crate::cell::primitives::Hash256;

    #[test]
    fn field_tensor_trace_preserves_evidence() {
        let cm = CouplingMatrix::empty().unwrap();
        let tb = FieldTensorState::genesis(Hash256::zero(), cm.matrix_id.clone()).unwrap();
        let ta = FieldTensorState::genesis(Hash256([1u8; 32]), cm.matrix_id).unwrap();
        let trace = FieldTensorTrace {
            trace_id: Hash256::zero(),
            tensor_before_hash: tb.tensor_id.clone(),
            tensor_after_hash: ta.tensor_id.clone(),
            coupling_update_hashes: vec![],
            rejected_candidate_hashes: vec![],
            source_cluster_trace_hashes: vec![],
            boundary_report_hashes: vec![],
            evidence_refs: vec![EvidenceRef {
                kind: "test.evidence".into(),
                address: Hash256::zero(),
                relation: "test".into(),
            }],
        }
        .with_id()
        .unwrap();
        // Evidence must survive re-id.
        assert_eq!(trace.evidence_refs.len(), 1);
        assert_ne!(trace.trace_id, Hash256::zero());
    }

    #[test]
    fn field_tensor_trace_is_deterministic() {
        let cm = CouplingMatrix::empty().unwrap();
        let t = FieldTensorState::genesis(Hash256::zero(), cm.matrix_id).unwrap();
        let make = || {
            FieldTensorTrace {
                trace_id: Hash256::zero(),
                tensor_before_hash: t.tensor_id.clone(),
                tensor_after_hash: t.tensor_id.clone(),
                coupling_update_hashes: vec![Hash256([2u8; 32]), Hash256([1u8; 32])],
                rejected_candidate_hashes: vec![],
                source_cluster_trace_hashes: vec![],
                boundary_report_hashes: vec![],
                evidence_refs: vec![],
            }
            .with_id()
            .unwrap()
        };
        assert_eq!(make().trace_id, make().trace_id);
    }
}
