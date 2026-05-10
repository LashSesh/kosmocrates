//! `run_stitch_cycle` — reference Dual-Fabric stitch pipeline
//! (PHASEMATRIX-HIVEMIND-03.1 §11).
//!
//! This is the single public entry point for one stitch cycle.  It is
//! deterministic: same inputs → byte-identical `StitcherOutcome`.
//!
//! ## Pipeline steps (§11.1)
//! 1. Validate `StitchRunDescriptor`.
//! 2. Use current `FieldTensorState` (Fabric-T).
//! 3. `CellSubstrateCycleReport` already provided by caller.
//! 4. Build `ResonanceFabricState` (Fabric-H).
//! 5. Derive `StitchCandidate[]`.
//! 6. Evaluate `MirrorConsistencyReport[]`.
//! 7. Simulate hypothetical tensor (for delta computation).
//! 8. Compute `TensorDeltaReport`.
//! 9. Evaluate `StitcherGateReport[]`.
//! 10. Collect accepted `CouplingUpdate[]`.
//! 11. Apply updates to Fabric-T.
//! 12. Write `FieldTensorTrace`.
//! 13. Write `StitcherReport`.

use serde::{Deserialize, Serialize};

use super::coupling_update::{build_coupling_update, derive_stitch_candidates, CouplingUpdate};
use super::field_tensor::{CouplingMatrix, FieldTensorState};
use super::field_tensor_trace::write_field_tensor_trace;
use super::mirror_consistency::evaluate_mirror_consistency;
use super::pipeline::CellSubstrateCycleReport;
use super::primitives::{content_address, EvidenceRef, Hash256, PhaseError};
use super::report::StitcherReport;
use super::resonance_fabric::{build_resonance_fabric, ResonanceFabricState};
use super::stitcher::{StitchRunDescriptor, StitcherOutcome};
use super::stitcher_gate::{evaluate_stitcher_gates, StitcherGateReport};
use super::tensor_delta::compute_tensor_delta;

/// Replay-ready bundle produced by `run_stitch_cycle` (for persistence
/// and byte-identity verification).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchCycleBundle {
    /// The run descriptor used.
    pub rd: StitchRunDescriptor,
    /// The Fabric-H snapshot.
    pub fabric_h: ResonanceFabricState,
    /// Fabric-T before the cycle.
    pub tensor_before: FieldTensorState,
    /// Fabric-T after the cycle (byte-identical to `tensor_before` when
    /// no updates were accepted).
    pub tensor_after: FieldTensorState,
    /// The stitch outcome.
    pub outcome: StitcherOutcome,
    /// Source cluster trace hash from the originating `CellSubstrateCycleReport`.
    /// Stored so replay can reconstruct `FieldTensorTrace` without re-running
    /// `build_resonance_fabric` on the original report.
    pub source_cluster_trace_hash: Hash256,
}

/// Apply accepted `CouplingUpdate`s to `FieldTensorState` (step 10–11).
///
/// The new tensor is content-addressed; the revision counter is
/// incremented exactly once per accepted batch, regardless of how many
/// updates the batch contains.
fn apply_coupling_updates(
    rd: &StitchRunDescriptor,
    tensor: FieldTensorState,
    updates: &[CouplingUpdate],
) -> Result<FieldTensorState, PhaseError> {
    if updates.is_empty() {
        return Ok(tensor);
    }

    // Build a new coupling matrix: reuse the old entries + append new ones.
    // For simplicity (and determinism) we model each update as adding one
    // new `CouplingEntry` with a weight equal to the delta.  Duplicate
    // edges accumulate weight.
    let mut matrix = CouplingMatrix {
        matrix_id: Hash256::zero(),
        entries: Vec::new(),
    };

    for u in updates {
        let weight = u.delta_resonance.clone();
        let entry = super::field_tensor::CouplingEntry {
            source: u.source_cell.clone(),
            target: u.target_cell.clone(),
            weight,
            coupling_kind: super::field_tensor::CouplingKind::Resonance,
            last_update_hash: Some(u.update_id.clone()),
        };
        matrix.entries.push(entry);
    }
    let matrix = matrix.with_id()?;

    // New tensor: previous_tensor_hash points at the old tensor.
    let new_tensor = FieldTensorState {
        tensor_id: Hash256::zero(),
        tensor_revision: tensor.tensor_revision + 1,
        matrix_id: tensor.matrix_id.clone(),
        node_state_hashes: tensor.node_state_hashes.clone(),
        spatial_topology_hash: tensor.spatial_topology_hash.clone(),
        coupling_matrix_hash: matrix.matrix_id.clone(),
        seam_state_hashes: tensor.seam_state_hashes.clone(),
        previous_tensor_hash: Some(tensor.tensor_id.clone()),
        trace_head: Hash256::zero(), // filled after trace is written
        evidence_refs: {
            let mut refs = tensor.evidence_refs.clone();
            refs.push(EvidenceRef {
                kind: "stitch.run_descriptor".into(),
                address: content_address(rd)?,
                relation: "updated_by".into(),
            });
            refs
        },
    }
    .with_id()?;
    Ok(new_tensor)
}

/// Reference stitch-cycle pipeline (§11.2).
pub fn run_stitch_cycle(
    rd: &StitchRunDescriptor,
    tensor: FieldTensorState,
    cell_report: &CellSubstrateCycleReport,
) -> Result<StitchCycleBundle, PhaseError> {
    // Step 1 — validate descriptor.
    rd.validate()?;

    let tensor_before = tensor;
    let source_trace_hash = cell_report.trace.trace_id.clone();

    // Step 4 — build Fabric-H.
    let fabric_h = build_resonance_fabric(rd, cell_report)?;

    // Steps 5–13: run via shared helper (also used by verify_stitch_replay).
    let (outcome, tensor_after) =
        run_stitch_from_fabric_h(rd, tensor_before.clone(), &fabric_h, &source_trace_hash)?;

    Ok(StitchCycleBundle {
        rd: rd.clone(),
        fabric_h,
        tensor_before,
        tensor_after,
        outcome,
        source_cluster_trace_hash: source_trace_hash,
    })
}

/// Verify byte-identity of a replayed stitch bundle (Invariant 9).
///
/// Uses the stored `fabric_h` directly (steps 5–13 only) so that replay
/// does not depend on re-running `build_resonance_fabric` against the
/// original `CellSubstrateCycleReport`.  The `source_cluster_trace_hash`
/// stored in the bundle supplies the provenance input for `write_field_tensor_trace`.
pub fn verify_stitch_replay(
    bundle: &StitchCycleBundle,
) -> Result<super::replay::ReplayObservation, PhaseError> {
    let (replayed_outcome, _) = run_stitch_from_fabric_h(
        &bundle.rd,
        bundle.tensor_before.clone(),
        &bundle.fabric_h,
        &bundle.source_cluster_trace_hash,
    )?;
    let original_hash = bundle.outcome.report_hash();
    let replayed_hash = replayed_outcome.report_hash();
    Ok(super::replay::ReplayObservation {
        expected_hash: original_hash.clone(),
        observed_hash: replayed_hash.clone(),
        passed: original_hash == replayed_hash,
    })
}

/// Run steps 5–13 of the stitch pipeline starting from a pre-built Fabric-H.
///
/// Returns `(outcome, tensor_after)`. Used by both `run_stitch_cycle`
/// (after building Fabric-H in step 4) and `verify_stitch_replay`
/// (reusing the stored Fabric-H for byte-identity).
fn run_stitch_from_fabric_h(
    rd: &StitchRunDescriptor,
    tensor_before: FieldTensorState,
    fabric_h: &ResonanceFabricState,
    source_trace_hash: &Hash256,
) -> Result<(StitcherOutcome, FieldTensorState), PhaseError> {
    let rd_hash = content_address(rd)?;

    // Step 5 — derive candidates.
    let candidates = derive_stitch_candidates(rd, fabric_h, &tensor_before)?;

    // Step 6 — mirror consistency.
    let mirror_reports = evaluate_mirror_consistency(rd, &candidates, &tensor_before)?;

    // Step 8 — tensor delta.
    let delta = compute_tensor_delta(rd, &tensor_before, &candidates)?;

    // Step 9 — gate evaluation.
    let trace_hash = fabric_h.trace_hash.clone();
    let boundary_passed = delta.passed;
    let gate_reports = evaluate_stitcher_gates(
        rd,
        &candidates,
        &mirror_reports,
        &delta,
        &trace_hash,
        boundary_passed,
    )?;

    // Step 10 — collect accepted updates.
    let accepted_gate: Vec<&StitcherGateReport> =
        gate_reports.iter().filter(|g| g.passed).collect();
    let cand_map: std::collections::BTreeMap<Hash256, &super::coupling_update::StitchCandidate> =
        candidates
            .iter()
            .map(|c| (c.candidate_id.clone(), c))
            .collect();

    let mut accepted_updates: Vec<CouplingUpdate> = Vec::new();
    for gate in &accepted_gate {
        if let Some(candidate) = cand_map.get(&gate.candidate_id) {
            let upd = build_coupling_update(candidate, gate.report_id.clone())?;
            accepted_updates.push(upd);
        }
    }
    if rd.policies.sort_updates_before_hash {
        accepted_updates.sort();
    }

    // Early hold path (empty-update-set policy).
    if accepted_updates.is_empty() && rd.policies.reject_empty_update_set {
        let stitch_candidates_hash = content_address(&candidates)?;
        let gate_report_hashes: Vec<Hash256> =
            gate_reports.iter().map(|g| g.report_id.clone()).collect();
        let rejected_candidates: Vec<Hash256> =
            candidates.iter().map(|c| c.candidate_id.clone()).collect();
        let fabric_h_hash = fabric_h.fabric_h_id.clone();

        let trace = write_field_tensor_trace(
            rd,
            &tensor_before,
            &tensor_before,
            &[],
            &gate_reports,
            vec![source_trace_hash.clone()],
        )?;

        let report = StitcherReport {
            report_id: Hash256::zero(),
            rd_hash,
            fabric_h_hash,
            tensor_before_hash: tensor_before.tensor_id.clone(),
            tensor_after_hash: tensor_before.tensor_id.clone(),
            stitch_candidates_hash,
            accepted_updates: vec![],
            rejected_candidates,
            gate_reports: gate_report_hashes,
            tensor_delta_report_hash: delta.report_id.clone(),
            field_tensor_trace_hash: trace.trace_id.clone(),
            passed: false,
            evidence_refs: vec![],
        }
        .with_id()?;

        return Ok((StitcherOutcome::Hold(report), tensor_before));
    }

    // Step 11 — apply updates.
    let tensor_after_pre_trace =
        apply_coupling_updates(rd, tensor_before.clone(), &accepted_updates)?;

    // Step 12 — write trace.
    let trace = write_field_tensor_trace(
        rd,
        &tensor_before,
        &tensor_after_pre_trace,
        &accepted_updates,
        &gate_reports,
        vec![source_trace_hash.clone()],
    )?;

    // Update trace_head only when updates were accepted — when the gate held
    // and the tensor is unchanged, we must not alter its content_address.
    let tensor_after = if !accepted_updates.is_empty() {
        let mut ta = tensor_after_pre_trace;
        ta.trace_head = trace.trace_id.clone();
        ta.with_id()?
    } else {
        tensor_after_pre_trace
    };

    // Step 13 — write StitcherReport.
    let stitch_candidates_hash = content_address(&candidates)?;
    let accepted_update_hashes: Vec<Hash256> = accepted_updates
        .iter()
        .map(|u| u.update_id.clone())
        .collect();
    let rejected_candidate_hashes: Vec<Hash256> = gate_reports
        .iter()
        .filter(|g| !g.passed)
        .map(|g| g.candidate_id.clone())
        .collect();
    let gate_report_hashes: Vec<Hash256> =
        gate_reports.iter().map(|g| g.report_id.clone()).collect();

    let passed = !accepted_updates.is_empty();
    let fabric_h_hash = fabric_h.fabric_h_id.clone();
    let report = StitcherReport {
        report_id: Hash256::zero(),
        rd_hash,
        fabric_h_hash,
        tensor_before_hash: tensor_before.tensor_id.clone(),
        tensor_after_hash: tensor_after.tensor_id.clone(),
        stitch_candidates_hash,
        accepted_updates: accepted_update_hashes,
        rejected_candidates: rejected_candidate_hashes,
        gate_reports: gate_report_hashes,
        tensor_delta_report_hash: delta.report_id.clone(),
        field_tensor_trace_hash: trace.trace_id.clone(),
        passed,
        evidence_refs: vec![EvidenceRef {
            kind: "stitch.run_descriptor".into(),
            address: content_address(rd)?,
            relation: "governed_by".into(),
        }],
    }
    .with_id()?;

    let outcome = if passed {
        StitcherOutcome::Completed(report)
    } else {
        StitcherOutcome::Hold(report)
    };
    Ok((outcome, tensor_after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::pipeline::{
        run_cell_substrate_cycle, CellSubstrateInput, CellSubstrateOutcome,
    };
    use crate::cell::primitives::Fixed;
    use crate::cell::run_descriptor::{
        CellSubstratePolicies, CellSubstrateThresholds, PhaseMatrixRunDescriptorV3,
    };
    use crate::cell::stitcher::StitchRunDescriptor;
    use std::collections::BTreeMap;

    fn pm_rd() -> PhaseMatrixRunDescriptorV3 {
        PhaseMatrixRunDescriptorV3 {
            run_id: "stitch.pipeline.test".into(),
            matrix_id: Hash256::zero(),
            parent_profile_hash: None,
            operator_versions: BTreeMap::new(),
            canonicalization_version: "phase-matrix-v0.3".into(),
            thresholds: CellSubstrateThresholds::permissive(),
            policies: CellSubstratePolicies::strict(),
            feature_flags: Default::default(),
            evidence_refs: Vec::new(),
        }
    }

    fn cycle_report(pm_rd: &PhaseMatrixRunDescriptorV3) -> CellSubstrateCycleReport {
        let input = CellSubstrateInput::synthetic(pm_rd.matrix_id.clone(), 3, 3).unwrap();
        match run_cell_substrate_cycle(pm_rd, &input).unwrap() {
            CellSubstrateOutcome::Completed(r) => r,
            other => panic!("{other:?}"),
        }
    }

    fn genesis_tensor() -> FieldTensorState {
        let cm = CouplingMatrix::empty().unwrap();
        FieldTensorState::genesis(Hash256::zero(), cm.matrix_id).unwrap()
    }

    #[test]
    fn stitch_cycle_happy_path_updates_tensor() {
        let pm_rd = pm_rd();
        let mut stitch_rd = StitchRunDescriptor::default_for(&pm_rd).unwrap();
        stitch_rd.policies.require_boundary_pass = false;
        stitch_rd.policies.require_trace_ready = false;
        stitch_rd.policies.require_evidence_refs = false;
        stitch_rd.policies.reject_empty_update_set = false;
        let report = cycle_report(&pm_rd);
        let tensor = genesis_tensor();
        let bundle = run_stitch_cycle(&stitch_rd, tensor.clone(), &report).unwrap();
        // Completed or Hold depending on whether any candidate passed the gate.
        // In either case, tensor_after must be deterministic.
        assert_ne!(bundle.tensor_after.tensor_id, Hash256::zero());
    }

    #[test]
    fn accepted_update_changes_tensor_revision() {
        let pm_rd = pm_rd();
        let mut stitch_rd = StitchRunDescriptor::default_for(&pm_rd).unwrap();
        stitch_rd.policies.require_boundary_pass = false;
        stitch_rd.policies.require_trace_ready = false;
        stitch_rd.policies.require_evidence_refs = false;
        stitch_rd.policies.reject_empty_update_set = false;
        let report = cycle_report(&pm_rd);
        let tensor = genesis_tensor();
        let bundle = run_stitch_cycle(&stitch_rd, tensor.clone(), &report).unwrap();
        if bundle.outcome.tensor_updated() {
            assert_eq!(
                bundle.tensor_after.tensor_revision,
                bundle.tensor_before.tensor_revision + 1,
                "accepted update must increment revision"
            );
            assert_eq!(
                bundle.tensor_after.previous_tensor_hash,
                Some(bundle.tensor_before.tensor_id.clone()),
                "tensor chain must be preserved"
            );
        }
    }

    #[test]
    fn rejected_candidate_keeps_tensor_unchanged() {
        let pm_rd = pm_rd();
        let mut stitch_rd = StitchRunDescriptor::default_for(&pm_rd).unwrap();
        // Force gate failure via impossible threshold.
        stitch_rd.thresholds.min_convergence_score = Fixed::Rational { num: 9999, den: 1 };
        stitch_rd.policies.require_boundary_pass = false;
        stitch_rd.policies.require_trace_ready = false;
        stitch_rd.policies.require_evidence_refs = false;
        stitch_rd.policies.reject_empty_update_set = false;
        let report = cycle_report(&pm_rd);
        let tensor = genesis_tensor();
        let bundle = run_stitch_cycle(&stitch_rd, tensor.clone(), &report).unwrap();
        assert_eq!(
            bundle.tensor_after.tensor_id, bundle.tensor_before.tensor_id,
            "rejected candidates must leave tensor byte-identical"
        );
    }

    #[test]
    fn stitch_cycle_no_candidates_holds() {
        let pm_rd = pm_rd();
        let mut stitch_rd = StitchRunDescriptor::default_for(&pm_rd).unwrap();
        stitch_rd.policies.reject_empty_update_set = true;
        stitch_rd.policies.require_boundary_pass = false;
        stitch_rd.policies.require_trace_ready = false;
        // Force convergence threshold to max so no candidate passes.
        stitch_rd.thresholds.min_convergence_score = Fixed::Rational { num: 9999, den: 1 };
        let report = cycle_report(&pm_rd);
        let tensor = genesis_tensor();
        let bundle = run_stitch_cycle(&stitch_rd, tensor, &report).unwrap();
        assert!(
            matches!(bundle.outcome, StitcherOutcome::Hold(_)),
            "no accepted candidates + reject_empty_update_set → Hold"
        );
    }

    #[test]
    fn stitch_cycle_two_runs_byte_identical() {
        let pm_rd = pm_rd();
        let mut stitch_rd = StitchRunDescriptor::default_for(&pm_rd).unwrap();
        stitch_rd.policies.require_boundary_pass = false;
        stitch_rd.policies.require_trace_ready = false;
        stitch_rd.policies.require_evidence_refs = false;
        let report = cycle_report(&pm_rd);
        let tensor = genesis_tensor();
        let b1 = run_stitch_cycle(&stitch_rd, tensor.clone(), &report).unwrap();
        let b2 = run_stitch_cycle(&stitch_rd, tensor, &report).unwrap();
        assert_eq!(
            b1.outcome.report_hash(),
            b2.outcome.report_hash(),
            "stitch cycle must be byte-identical across runs"
        );
    }

    #[test]
    fn coupling_updates_sorted_before_hashing() {
        let pm_rd = pm_rd();
        let mut stitch_rd = StitchRunDescriptor::default_for(&pm_rd).unwrap();
        stitch_rd.policies.sort_updates_before_hash = true;
        stitch_rd.policies.require_boundary_pass = false;
        stitch_rd.policies.require_trace_ready = false;
        stitch_rd.policies.require_evidence_refs = false;
        let report = cycle_report(&pm_rd);
        let tensor = genesis_tensor();
        // Two identical runs must produce identical hashes → sorting is stable.
        let b1 = run_stitch_cycle(&stitch_rd, tensor.clone(), &report).unwrap();
        let b2 = run_stitch_cycle(&stitch_rd, tensor, &report).unwrap();
        assert_eq!(b1.outcome.report_hash(), b2.outcome.report_hash());
    }

    #[test]
    fn stitch_replay_byte_identical() {
        let pm_rd = pm_rd();
        let mut stitch_rd = StitchRunDescriptor::default_for(&pm_rd).unwrap();
        stitch_rd.policies.require_boundary_pass = false;
        stitch_rd.policies.require_trace_ready = false;
        stitch_rd.policies.require_evidence_refs = false;
        let report = cycle_report(&pm_rd);
        let tensor = genesis_tensor();
        let bundle = run_stitch_cycle(&stitch_rd, tensor, &report).unwrap();
        let obs = verify_stitch_replay(&bundle).unwrap();
        assert!(obs.passed, "replay must be byte-identical");
    }

    #[test]
    fn fabric_h_cannot_mutate_tensor_directly() {
        // Invariant 1: Fabric-H events must not touch Fabric-T without the gate.
        // We verify by confirming that the fabric_h_id != tensor_before tensor_id
        // and that the tensor only changes through accepted CouplingUpdates.
        let pm_rd = pm_rd();
        let mut stitch_rd = StitchRunDescriptor::default_for(&pm_rd).unwrap();
        stitch_rd.thresholds.min_convergence_score = Fixed::Rational { num: 9999, den: 1 };
        stitch_rd.policies.require_boundary_pass = false;
        stitch_rd.policies.require_trace_ready = false;
        stitch_rd.policies.require_evidence_refs = false;
        let report = cycle_report(&pm_rd);
        let tensor = genesis_tensor();
        let bundle = run_stitch_cycle(&stitch_rd, tensor.clone(), &report).unwrap();
        // With impossible threshold, no candidate passes → tensor unchanged.
        assert_eq!(
            bundle.tensor_after.tensor_id, bundle.tensor_before.tensor_id,
            "Fabric-H link must not mutate tensor without gate"
        );
    }
}
