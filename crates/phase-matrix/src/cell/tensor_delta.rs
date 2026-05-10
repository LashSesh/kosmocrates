//! `TensorDeltaReport` and delta computation
//! (PHASEMATRIX-HIVEMIND-03.1 §8.2).
//!
//! `TensorDelta` caps the magnitude of structural change per stitch
//! cycle. If the cumulative L1 norm of all proposed updates exceeds
//! `max_tensor_delta_norm`, the cycle is held or rejected.

use serde::{Deserialize, Serialize};

use super::coupling_update::StitchCandidate;
use super::field_tensor::FieldTensorState;
use super::primitives::{
    content_address, fixed_add, fixed_le, max_fixed, Fixed, Hash256, PhaseError,
};
use super::stitcher::StitchRunDescriptor;

/// Tensor-delta report for one stitch cycle (§8.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TensorDeltaReport {
    /// Content address of this report.
    pub report_id: Hash256,
    /// Content address of `FieldTensorState` before candidate application.
    pub tensor_before_hash: Hash256,
    /// Content address of the hypothetical `FieldTensorState` after
    /// all accepted candidates are applied.
    pub proposed_tensor_after_hash: Hash256,
    /// Cumulative L1 norm of all accepted candidate deltas.
    pub delta_norm: Fixed,
    /// Threshold copied from the run descriptor.
    pub max_allowed_delta_norm: Fixed,
    /// Maximum per-edge delta observed across all candidates.
    pub per_edge_delta_max: Fixed,
    /// Number of updates in this batch.
    pub update_count: u64,
    /// True iff `delta_norm ≤ max_allowed_delta_norm` and
    /// `per_edge_delta_max ≤ max_coupling_delta_per_edge`.
    pub passed: bool,
}

impl TensorDeltaReport {
    /// Recompute `report_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        self.report_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Compute the tensor-delta report for a set of candidates (step 8).
///
/// `candidates` is the full proposed set (before gate evaluation). The
/// report represents the *hypothetical* tensor after all candidates
/// would be applied; the gate then decides which are accepted.
pub fn compute_tensor_delta(
    rd: &StitchRunDescriptor,
    tensor: &FieldTensorState,
    candidates: &[StitchCandidate],
) -> Result<TensorDeltaReport, PhaseError> {
    let zero = Fixed::Rational { num: 0, den: 1 };

    // Cumulative L1 norm = sum of |proposed_delta| across all candidates.
    let mut cumulative_norm = zero.clone();
    let per_edge_deltas: Vec<Fixed> = candidates
        .iter()
        .map(|c| {
            let d = &c.proposed_delta;
            if d < &zero {
                // abs: negate
                Fixed::Rational {
                    num: match d {
                        Fixed::Rational { num, .. } => -num,
                        Fixed::FixedI64 { raw, .. } => -(*raw as i64) as i128,
                    },
                    den: match d {
                        Fixed::Rational { den, .. } => *den,
                        Fixed::FixedI64 { scale, .. } => 10i128.pow(*scale),
                    },
                }
            } else {
                d.clone()
            }
        })
        .collect();
    for d in &per_edge_deltas {
        cumulative_norm = fixed_add(&cumulative_norm, d);
    }

    let per_edge_delta_max = max_fixed(&per_edge_deltas);

    // Hypothetical tensor hash = hash of (tensor_id, sorted candidate ids).
    let mut cand_ids: Vec<Hash256> = candidates.iter().map(|c| c.candidate_id.clone()).collect();
    cand_ids.sort();
    let proposed_tensor_after_hash =
        content_address(&("tensor.after", &tensor.tensor_id, &cand_ids))?;

    let max_norm = &rd.thresholds.max_tensor_delta_norm;
    let max_per_edge = &rd.thresholds.max_coupling_delta_per_edge;

    let passed =
        fixed_le(&cumulative_norm, max_norm) && fixed_le(&per_edge_delta_max, max_per_edge);

    TensorDeltaReport {
        report_id: Hash256::zero(),
        tensor_before_hash: tensor.tensor_id.clone(),
        proposed_tensor_after_hash,
        delta_norm: cumulative_norm,
        max_allowed_delta_norm: max_norm.clone(),
        per_edge_delta_max,
        update_count: candidates.len() as u64,
        passed,
    }
    .with_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::coupling_update::derive_stitch_candidates;
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

    fn setup() -> (StitchRunDescriptor, Vec<StitchCandidate>, FieldTensorState) {
        let pm_rd = PhaseMatrixRunDescriptorV3 {
            run_id: "delta.test".into(),
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
        let tensor = FieldTensorState::genesis(pm_rd.matrix_id.clone(), cm.matrix_id).unwrap();
        let candidates = derive_stitch_candidates(&stitch_rd, &fabric, &tensor).unwrap();
        (stitch_rd, candidates, tensor)
    }

    #[test]
    fn tensor_delta_blocks_large_update() {
        let (mut rd, candidates, tensor) = setup();
        // Set the delta threshold to zero → all updates must be blocked.
        rd.thresholds.max_tensor_delta_norm = Fixed::Rational { num: 0, den: 1 };
        let report = compute_tensor_delta(&rd, &tensor, &candidates).unwrap();
        // Zero threshold blocks any update with candidates.
        if !candidates.is_empty() {
            assert!(!report.passed, "zero threshold must block updates");
        }
    }

    #[test]
    fn tensor_delta_passes_with_permissive_threshold() {
        let (rd, candidates, tensor) = setup();
        let report = compute_tensor_delta(&rd, &tensor, &candidates).unwrap();
        assert!(report.passed, "permissive threshold must pass");
    }

    #[test]
    fn tensor_delta_report_is_content_addressed() {
        let (rd, candidates, tensor) = setup();
        let r1 = compute_tensor_delta(&rd, &tensor, &candidates).unwrap();
        let r2 = compute_tensor_delta(&rd, &tensor, &candidates).unwrap();
        assert_eq!(r1.report_id, r2.report_id);
    }
}
