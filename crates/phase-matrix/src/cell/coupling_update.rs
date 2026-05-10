//! `StitchCandidate`, `CouplingUpdate` and candidate-derivation logic
//! (PHASEMATRIX-HIVEMIND-03.1 §7).
//!
//! A `StitchCandidate` is a *proposed* coupling change derived from
//! Fabric-H. It becomes a `CouplingUpdate` only after the
//! `StitcherGate` passes.

use serde::{Deserialize, Serialize};

use super::field_tensor::{CouplingKind, FieldTensorState};
use super::primitives::{content_address, fixed_abs_diff, EvidenceRef, Fixed, Hash256, PhaseError};
use super::resonance_fabric::ResonanceFabricState;
use super::stitcher::StitchRunDescriptor;

/// A proposed coupling change derived from Fabric-H (§7.1).
///
/// This is **not** a tensor change; it is only a candidate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StitchCandidate {
    /// Content address of this candidate.
    pub candidate_id: Hash256,
    /// Content address of the `EphemeralResonanceLink` that spawned this
    /// candidate.
    pub source_link_id: Hash256,
    /// Source cell hash.
    pub source_cell: Hash256,
    /// Target cell hash.
    pub target_cell: Hash256,
    /// Proposed coupling kind.
    pub proposed_coupling_kind: CouplingKind,
    /// Proposed coupling weight delta (float-free).
    pub proposed_delta: Fixed,
    /// Content address of the reason artefact (e.g. convergence field).
    pub reason_hash: Hash256,
    /// Content address of the `ClusterTrace` that supports this proposal.
    pub supporting_cluster_trace_hash: Hash256,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl StitchCandidate {
    /// Recompute `candidate_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.candidate_id = Hash256::zero();
        self.candidate_id = content_address(&probe)?;
        Ok(self)
    }
}

/// An accepted, persistent coupling change for Fabric-T (§7.2).
///
/// `CouplingUpdate` is content-addressed and MUST reference exactly one
/// `StitcherGateReport` (Invariant 4).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CouplingUpdate {
    /// Content address of this update.
    pub update_id: Hash256,
    /// Content address of the originating `StitchCandidate`.
    pub candidate_id: Hash256,
    /// Source cell hash.
    pub source_cell: Hash256,
    /// Target cell hash.
    pub target_cell: Hash256,
    /// Content address of the old `CouplingEntry` (None = new edge).
    pub old_coupling_hash: Option<Hash256>,
    /// Content address of the new `CouplingEntry`.
    pub new_coupling_hash: Hash256,
    /// Net resonance delta applied.
    pub delta_resonance: Fixed,
    /// L1 norm of this update.
    pub delta_norm: Fixed,
    /// Content address of the reason artefact.
    pub reason_hash: Hash256,
    /// Content address of the `StitcherGateReport` that admitted this update.
    pub gate_report_hash: Hash256,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl CouplingUpdate {
    /// Recompute `update_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.update_id = Hash256::zero();
        self.update_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Derive `StitchCandidate`s from Fabric-H (pipeline step 5 in §11.1).
///
/// One candidate is emitted per transient link whose `resonance_score`
/// is strictly positive. Derivation is fully deterministic.
pub fn derive_stitch_candidates(
    rd: &StitchRunDescriptor,
    fabric_h: &ResonanceFabricState,
    tensor: &FieldTensorState,
) -> Result<Vec<StitchCandidate>, PhaseError> {
    let zero = Fixed::Rational { num: 0, den: 1 };
    let mut candidates = Vec::new();

    for link in &fabric_h.transient_links {
        // Skip zero-strength links.
        if link.resonance_score <= zero {
            continue;
        }
        let reason_hash =
            content_address(&("candidate.reason", &link.link_id, &fabric_h.fabric_h_id))?;
        let delta = link.resonance_score.clone();

        let candidate = StitchCandidate {
            candidate_id: Hash256::zero(),
            source_link_id: link.link_id.clone(),
            source_cell: link.source_cell.clone(),
            target_cell: link.target_cell.clone(),
            proposed_coupling_kind: CouplingKind::Resonance,
            proposed_delta: delta,
            reason_hash,
            supporting_cluster_trace_hash: fabric_h.trace_hash.clone(),
            evidence_refs: {
                let mut refs = link.evidence_refs.clone();
                refs.push(super::primitives::EvidenceRef {
                    kind: "stitch.run_descriptor".into(),
                    address: content_address(rd)?,
                    relation: "governed_by".into(),
                });
                refs.push(super::primitives::EvidenceRef {
                    kind: "field_tensor".into(),
                    address: tensor.tensor_id.clone(),
                    relation: "targets".into(),
                });
                refs
            },
        }
        .with_id()?;
        candidates.push(candidate);
    }

    // Deterministic order: sort by candidate_id.
    candidates.sort();
    Ok(candidates)
}

/// Build a `CouplingUpdate` from an accepted candidate and its gate
/// report hash (called after the gate passes).
pub fn build_coupling_update(
    candidate: &StitchCandidate,
    gate_report_hash: Hash256,
) -> Result<CouplingUpdate, PhaseError> {
    let new_coupling_hash = content_address(&(
        "coupling.new",
        &candidate.source_cell,
        &candidate.target_cell,
        &candidate.proposed_delta,
        &candidate.proposed_coupling_kind,
    ))?;
    let delta_norm = fixed_abs_diff(
        &candidate.proposed_delta,
        &Fixed::Rational { num: 0, den: 1 },
    );
    CouplingUpdate {
        update_id: Hash256::zero(),
        candidate_id: candidate.candidate_id.clone(),
        source_cell: candidate.source_cell.clone(),
        target_cell: candidate.target_cell.clone(),
        old_coupling_hash: None,
        new_coupling_hash,
        delta_resonance: candidate.proposed_delta.clone(),
        delta_norm,
        reason_hash: candidate.reason_hash.clone(),
        gate_report_hash,
        evidence_refs: candidate.evidence_refs.clone(),
    }
    .with_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::field_tensor::CouplingMatrix;
    use crate::cell::pipeline::{
        run_cell_substrate_cycle, CellSubstrateInput, CellSubstrateOutcome,
    };
    use crate::cell::resonance_fabric::build_resonance_fabric;
    use crate::cell::run_descriptor::{
        CellSubstratePolicies, CellSubstrateThresholds, PhaseMatrixRunDescriptorV3,
    };
    use crate::cell::stitcher::StitchRunDescriptor;
    use std::collections::BTreeMap;

    fn setup() -> (StitchRunDescriptor, ResonanceFabricState, FieldTensorState) {
        let pm_rd = PhaseMatrixRunDescriptorV3 {
            run_id: "coupling.test".into(),
            matrix_id: Hash256::zero(),
            parent_profile_hash: None,
            operator_versions: BTreeMap::new(),
            canonicalization_version: "phase-matrix-v0.3".into(),
            thresholds: CellSubstrateThresholds::permissive(),
            policies: CellSubstratePolicies::strict(),
            feature_flags: Default::default(),
            evidence_refs: Vec::new(),
        };
        let stitch_rd = StitchRunDescriptor::default_for(&pm_rd).unwrap();
        let input = CellSubstrateInput::synthetic(pm_rd.matrix_id.clone(), 3, 3).unwrap();
        let report = match run_cell_substrate_cycle(&pm_rd, &input).unwrap() {
            CellSubstrateOutcome::Completed(r) => r,
            other => panic!("{other:?}"),
        };
        let fabric = build_resonance_fabric(&stitch_rd, &report).unwrap();
        let cm = CouplingMatrix::empty().unwrap();
        let tensor =
            FieldTensorState::genesis(pm_rd.matrix_id.clone(), cm.matrix_id.clone()).unwrap();
        (stitch_rd, fabric, tensor)
    }

    #[test]
    fn stitch_candidate_derivation_is_deterministic() {
        let (rd, fabric, tensor) = setup();
        let c1 = derive_stitch_candidates(&rd, &fabric, &tensor).unwrap();
        let c2 = derive_stitch_candidates(&rd, &fabric, &tensor).unwrap();
        assert_eq!(c1, c2, "candidate derivation must be byte-identical");
    }

    #[test]
    fn coupling_update_without_gate_report_hash_would_be_wrong() {
        // If we build a CouplingUpdate we MUST supply a real gate_report_hash.
        // This test verifies the build function produces a non-zero update_id.
        let (rd, fabric, tensor) = setup();
        let candidates = derive_stitch_candidates(&rd, &fabric, &tensor).unwrap();
        if let Some(c) = candidates.first() {
            let gate_hash = content_address(&("gate.report", &c.candidate_id)).unwrap();
            let upd = build_coupling_update(c, gate_hash).unwrap();
            assert_ne!(upd.update_id, Hash256::zero());
            assert_eq!(upd.candidate_id, c.candidate_id);
        }
    }
}
