//! `StitcherGateReport`, `StitchFailurePolicy` and gate evaluation
//! (PHASEMATRIX-HIVEMIND-03.1 §9).
//!
//! **Gate formula** (§9.1):
//! ```text
//! G_stitch = G_conv ∧ G_mci ∧ G_delta ∧ G_budget ∧ G_trace ∧ G_boundary ∧ G_evidence
//! ```
//! Every partial gate failure must leave `FieldTensorState` byte-identical
//! and be persisted as a `StitcherGateReport` + `StitcherReport`.

use serde::{Deserialize, Serialize};

use super::coupling_update::StitchCandidate;
use super::mirror_consistency::MirrorConsistencyReport;
use super::primitives::{content_address, fixed_ge, Fixed, Hash256, PhaseError};
use super::stitcher::StitchRunDescriptor;
use super::tensor_delta::TensorDeltaReport;

/// Failure policy communicated by the gate (§9.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StitchFailurePolicy {
    /// Retain the candidate for the next cycle.
    Hold,
    /// Discard the candidate permanently.
    RejectCandidate,
    /// Keep Fabric-T byte-identical, no update emitted.
    KeepTensorUnchanged,
    /// Force a full recompute of Fabric-H before retrying.
    RequireRecompute,
    /// Matrix-boundary seam was violated.
    BoundaryViolation,
    /// Determinism invariant was violated.
    DeterminismViolation,
}

/// Per-candidate gate report (§9.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StitcherGateReport {
    /// Content address of this report.
    pub report_id: Hash256,
    /// Content address of the `StitchCandidate` evaluated.
    pub candidate_id: Hash256,
    /// Convergence score observed for this candidate.
    pub convergence_score: Fixed,
    /// Mirror-consistency index observed.
    pub mirror_consistency_index: Fixed,
    /// Tensor-delta norm for the full batch.
    pub tensor_delta_norm: Fixed,
    /// True iff `update_count ≤ max_updates_per_cycle`.
    pub update_budget_ok: bool,
    /// True iff the trace is ready (non-zero trace hash).
    pub trace_ready: bool,
    /// True iff the boundary check passed.
    pub boundary_passed: bool,
    /// True iff evidence refs are non-empty (or policy doesn't require them).
    pub evidence_ready: bool,
    /// Conjunction of all sub-gates.
    pub passed: bool,
    /// Failure policy if `!passed`.
    pub failure_policy: StitchFailurePolicy,
}

impl StitcherGateReport {
    /// Recompute `report_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        self.report_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Evaluate the `StitcherGate` for every candidate (pipeline step 9).
///
/// Returns one `StitcherGateReport` per candidate, sorted by
/// `candidate_id` for determinism.
pub fn evaluate_stitcher_gates(
    rd: &StitchRunDescriptor,
    candidates: &[StitchCandidate],
    mirror_reports: &[MirrorConsistencyReport],
    delta: &TensorDeltaReport,
    trace_hash: &Hash256,
    boundary_passed: bool,
) -> Result<Vec<StitcherGateReport>, PhaseError> {
    // Build a lookup from candidate_id → mirror report.
    let mut mci_map = std::collections::BTreeMap::new();
    for mr in mirror_reports {
        mci_map.insert(mr.candidate_id.clone(), mr);
    }

    let budget_ok = (candidates.len() as u64) <= rd.thresholds.max_updates_per_cycle;
    let trace_ready = *trace_hash != Hash256::zero();
    let zero = Fixed::Rational { num: 0, den: 1 };

    let mut reports = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        // G_conv: convergence score ≥ θ_conv.
        // We use the candidate's proposed_delta as a proxy for the
        // convergence score (the convergence field fed Fabric-H; its
        // scalar is encoded in the delta).
        let convergence_score = candidate.proposed_delta.clone();
        let g_conv = fixed_ge(&convergence_score, &rd.thresholds.min_convergence_score);

        // G_mci: mirror consistency index ≥ θ_mci.
        let (mci, g_mci) = if let Some(mr) = mci_map.get(&candidate.candidate_id) {
            (mr.mirror_consistency_index.clone(), mr.passed)
        } else {
            // No mirror report → fail-closed.
            (zero.clone(), false)
        };

        // G_delta: tensor_delta_norm ≤ θ_delta.
        let g_delta = delta.passed;

        // G_budget: update_count ≤ max_updates_per_cycle.
        let g_budget = budget_ok;

        // G_trace.
        let g_trace = if rd.policies.require_trace_ready {
            trace_ready
        } else {
            true
        };

        // G_boundary.
        let g_boundary = if rd.policies.require_boundary_pass {
            boundary_passed
        } else {
            true
        };

        // G_evidence.
        let g_evidence = if rd.policies.require_evidence_refs {
            !candidate.evidence_refs.is_empty()
        } else {
            true
        };

        let passed = g_conv && g_mci && g_delta && g_budget && g_trace && g_boundary && g_evidence;

        let failure_policy = if !passed {
            if !g_boundary {
                StitchFailurePolicy::BoundaryViolation
            } else if !g_delta {
                StitchFailurePolicy::KeepTensorUnchanged
            } else if !g_trace {
                StitchFailurePolicy::RequireRecompute
            } else {
                StitchFailurePolicy::RejectCandidate
            }
        } else {
            StitchFailurePolicy::KeepTensorUnchanged // nominal / unused when passed=true
        };

        let report = StitcherGateReport {
            report_id: Hash256::zero(),
            candidate_id: candidate.candidate_id.clone(),
            convergence_score,
            mirror_consistency_index: mci,
            tensor_delta_norm: delta.delta_norm.clone(),
            update_budget_ok: g_budget,
            trace_ready,
            boundary_passed,
            evidence_ready: g_evidence,
            passed,
            failure_policy,
        }
        .with_id()?;
        reports.push(report);
    }

    reports.sort();
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::coupling_update::derive_stitch_candidates;
    use crate::cell::field_tensor::CouplingMatrix;
    use crate::cell::mirror_consistency::evaluate_mirror_consistency;
    use crate::cell::pipeline::{
        run_cell_substrate_cycle, CellSubstrateInput, CellSubstrateOutcome,
    };
    use crate::cell::resonance_fabric::build_resonance_fabric;
    use crate::cell::run_descriptor::{
        CellSubstratePolicies, CellSubstrateThresholds, PhaseMatrixRunDescriptorV3,
    };
    use crate::cell::stitcher::StitchRunDescriptor;
    use crate::cell::tensor_delta::compute_tensor_delta;
    use std::collections::BTreeMap;

    fn full_setup() -> (
        StitchRunDescriptor,
        Vec<StitchCandidate>,
        Vec<MirrorConsistencyReport>,
        TensorDeltaReport,
        Hash256,
    ) {
        let pm_rd = PhaseMatrixRunDescriptorV3 {
            run_id: "gate.test".into(),
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
        let mirror_reports = evaluate_mirror_consistency(&stitch_rd, &candidates, &tensor).unwrap();
        let delta = compute_tensor_delta(&stitch_rd, &tensor, &candidates).unwrap();
        let trace_hash = fabric.trace_hash.clone();
        (stitch_rd, candidates, mirror_reports, delta, trace_hash)
    }

    use super::super::field_tensor::FieldTensorState;

    #[test]
    fn stitcher_gate_fails_closed_without_trace() {
        let (mut rd, candidates, mirror_reports, delta, _trace) = full_setup();
        rd.policies.require_trace_ready = true;
        // Provide zero trace hash → G_trace fails.
        let zero_trace = Hash256::zero();
        let reports =
            evaluate_stitcher_gates(&rd, &candidates, &mirror_reports, &delta, &zero_trace, true)
                .unwrap();
        for r in &reports {
            assert!(!r.passed, "gate must fail-closed without trace");
        }
    }

    #[test]
    fn stitcher_gate_fails_closed_without_boundary() {
        let (mut rd, candidates, mirror_reports, delta, trace) = full_setup();
        rd.policies.require_boundary_pass = true;
        let reports =
            evaluate_stitcher_gates(&rd, &candidates, &mirror_reports, &delta, &trace, false)
                .unwrap();
        for r in &reports {
            assert!(
                !r.passed || !rd.policies.require_boundary_pass,
                "gate must fail-closed without boundary"
            );
            if !r.boundary_passed {
                assert!(!r.passed);
            }
        }
    }

    #[test]
    fn gate_passes_under_permissive_conditions() {
        let (mut rd, candidates, mirror_reports, delta, trace) = full_setup();
        rd.policies.require_boundary_pass = false;
        rd.policies.require_trace_ready = false;
        rd.policies.require_evidence_refs = false;
        if candidates.is_empty() {
            return; // nothing to gate
        }
        let reports =
            evaluate_stitcher_gates(&rd, &candidates, &mirror_reports, &delta, &trace, true)
                .unwrap();
        assert!(
            reports.iter().any(|r| r.passed),
            "at least one gate should pass under permissive policies"
        );
    }
}
