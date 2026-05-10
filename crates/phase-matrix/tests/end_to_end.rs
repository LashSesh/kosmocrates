//! End-to-end tests for PHASEMATRIX-HIVEMIND-03 and PHASEMATRIX-HIVEMIND-03.1.
//!
//! Drives the full cell-substrate cycle and verifies the spec's
//! invariants: deterministic plan, ADAMANT-aligned trace contract,
//! fail-closed cluster formation, dissolution trace preservation,
//! handoff candidates carrying no commit artefacts.
//!
//! Additionally drives the Dual-Fabric Field-Tensor Stitch Layer
//! (PHASEMATRIX-HIVEMIND-03.1): full stitch-cycle pipeline, replay
//! byte-identity, Fabric-H isolation invariant.

use phase_matrix::cell::field_tensor::FieldTensorState;
use phase_matrix::cell::pipeline::CellSubstrateCycleReport;
use phase_matrix::cell::pipeline::{
    run_cell_substrate_cycle, CellSubstrateInput, CellSubstrateOutcome,
};
use phase_matrix::cell::primitives::{Fixed, Hash256};
use phase_matrix::cell::run_descriptor::{
    CellSubstratePolicies, CellSubstrateThresholds, PhaseMatrixRunDescriptorV3,
};
use phase_matrix::cell::stitch_pipeline::{run_stitch_cycle, verify_stitch_replay};
use phase_matrix::cell::stitcher::{StitchRunDescriptor, StitcherOutcome};
use std::collections::{BTreeMap, BTreeSet};

fn rd() -> PhaseMatrixRunDescriptorV3 {
    PhaseMatrixRunDescriptorV3 {
        run_id: "phase.it.test".into(),
        matrix_id: Hash256::zero(),
        parent_profile_hash: None,
        operator_versions: BTreeMap::new(),
        canonicalization_version: "phase-matrix-v0.3".into(),
        thresholds: CellSubstrateThresholds::permissive(),
        policies: CellSubstratePolicies::strict(),
        feature_flags: BTreeSet::new(),
        evidence_refs: vec![],
    }
}

#[test]
fn cell_cycle_produces_completed_report_under_permissive_thresholds() {
    let rd = rd();
    let input = CellSubstrateInput::synthetic(rd.matrix_id.clone(), 3, 3).unwrap();
    let outcome = run_cell_substrate_cycle(&rd, &input).unwrap();
    match outcome {
        CellSubstrateOutcome::Completed(r) => {
            assert!(!r.cluster.member_cells.is_empty());
            assert_ne!(r.cluster.cluster_id, Hash256::zero());
            assert_ne!(r.trace.trace_id, Hash256::zero());
        }
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[test]
fn cell_cycle_replay_byte_identical() {
    let rd = rd();
    let input = CellSubstrateInput::synthetic(rd.matrix_id.clone(), 3, 3).unwrap();
    let a = run_cell_substrate_cycle(&rd, &input).unwrap();
    let b = run_cell_substrate_cycle(&rd, &input).unwrap();
    let ah = pse_traverse::canonical::canonical_bytes(&a).unwrap();
    let bh = pse_traverse::canonical::canonical_bytes(&b).unwrap();
    assert_eq!(
        ah, bh,
        "two identical cycles must produce byte-identical bytes"
    );
}

#[test]
fn cluster_dissolve_keeps_trace_after_compaction() {
    let mut rd = rd();
    rd.thresholds.max_working_state_retention_ticks = 1;
    let mut input = CellSubstrateInput::synthetic(rd.matrix_id.clone(), 3, 3).unwrap();
    input.logical_step = 4; // > retention threshold ⇒ dissolution path
    let outcome = run_cell_substrate_cycle(&rd, &input).unwrap();
    if let CellSubstrateOutcome::Completed(r) = outcome {
        let diss = r
            .dissolution
            .as_ref()
            .expect("expected dissolution report when retention exceeded");
        diss.validate_trace_preservation()
            .expect("dissolution must preserve trace + evidence + lifecycle history");
        // The trace MUST reference the dissolution report.
        assert_eq!(
            r.trace.dissolution_report_hash.as_ref(),
            Some(&diss.report_id)
        );
    } else {
        panic!("expected Completed outcome");
    }
}

#[test]
fn matrix_boundary_violation_blocks_intent_claim() {
    // Insert a foreign cell whose parent_node_id does not belong to
    // the subnet — the pool MUST reject it.
    let rd = rd();
    let mut input = CellSubstrateInput::synthetic(rd.matrix_id.clone(), 3, 3).unwrap();
    let mut foreign = [0u8; 32];
    foreign[0] = 0xff;
    let mut bad = input.seed_cells[0].clone();
    bad.parent_node_id = Hash256(foreign);
    input.seed_cells.push(bad);
    let result = run_cell_substrate_cycle(&rd, &input);
    assert!(result.is_err(), "foreign-parent cell must be rejected");
}

#[test]
fn cycle_holds_when_thresholds_too_strict() {
    let mut rd = rd();
    rd.thresholds.min_morphodynamic_potential = Fixed::Rational { num: 100, den: 1 };
    let input = CellSubstrateInput::synthetic(rd.matrix_id.clone(), 3, 3).unwrap();
    match run_cell_substrate_cycle(&rd, &input).unwrap() {
        CellSubstrateOutcome::Hold(_) => {}
        other => panic!("expected Hold, got {other:?}"),
    }
}

#[test]
fn no_commit_artefacts_appear_in_outcome_bytes() {
    let rd = rd();
    let input = CellSubstrateInput::synthetic(rd.matrix_id.clone(), 3, 3).unwrap();
    let outcome = run_cell_substrate_cycle(&rd, &input).unwrap();
    let bytes = pse_traverse::canonical::canonical_bytes(&outcome).unwrap();
    let s = String::from_utf8_lossy(&bytes);
    assert!(!s.contains("SemanticCrystal"));
    assert!(!s.contains("FinalizedEmission"));
}

// ── PHASEMATRIX-HIVEMIND-03.1 Dual-Fabric Stitch Layer tests ─────────────────

fn genesis_tensor() -> FieldTensorState {
    use phase_matrix::cell::field_tensor::CouplingMatrix;
    let cm = CouplingMatrix::empty().unwrap();
    FieldTensorState::genesis(Hash256::zero(), cm.matrix_id).unwrap()
}

fn stitch_rd(pm_rd: &PhaseMatrixRunDescriptorV3) -> StitchRunDescriptor {
    StitchRunDescriptor::default_for(pm_rd).unwrap()
}

fn completed_cell_report(pm_rd: &PhaseMatrixRunDescriptorV3) -> CellSubstrateCycleReport {
    let input = CellSubstrateInput::synthetic(pm_rd.matrix_id.clone(), 3, 3).unwrap();
    match run_cell_substrate_cycle(pm_rd, &input).unwrap() {
        CellSubstrateOutcome::Completed(r) => r,
        other => panic!("expected Completed cell report, got {other:?}"),
    }
}

#[test]
fn stitch_cycle_completes_under_permissive_thresholds() {
    let pm_rd = rd();
    let srd = stitch_rd(&pm_rd);
    let tensor = genesis_tensor();
    let cell_report = completed_cell_report(&pm_rd);

    let bundle = run_stitch_cycle(&srd, tensor, &cell_report).unwrap();
    // Tensor must be content-addressed (non-zero id after genesis).
    assert_ne!(bundle.tensor_before.tensor_id, Hash256::zero());
    assert_ne!(bundle.tensor_after.tensor_id, Hash256::zero());
    // Fabric-H must have a valid id.
    assert_ne!(bundle.fabric_h.fabric_h_id, Hash256::zero());
}

#[test]
fn stitch_cycle_replay_byte_identical() {
    let pm_rd = rd();
    let srd = stitch_rd(&pm_rd);
    let tensor = genesis_tensor();
    let cell_report = completed_cell_report(&pm_rd);

    let bundle_a = run_stitch_cycle(&srd, tensor.clone(), &cell_report).unwrap();
    let bundle_b = run_stitch_cycle(&srd, tensor, &cell_report).unwrap();

    let ha = pse_traverse::canonical::canonical_bytes(&bundle_a.outcome).unwrap();
    let hb = pse_traverse::canonical::canonical_bytes(&bundle_b.outcome).unwrap();
    assert_eq!(
        ha, hb,
        "two identical stitch cycles must produce byte-identical outcomes"
    );
}

#[test]
fn stitch_replay_verification_passes() {
    let pm_rd = rd();
    let srd = stitch_rd(&pm_rd);
    let tensor = genesis_tensor();
    let cell_report = completed_cell_report(&pm_rd);

    let bundle = run_stitch_cycle(&srd, tensor, &cell_report).unwrap();
    let obs = verify_stitch_replay(&bundle).unwrap();
    assert!(
        obs.passed,
        "replay verification must pass for a fresh bundle"
    );
    assert_eq!(obs.expected_hash, obs.observed_hash);
}

#[test]
fn stitch_tensor_revision_increments_when_updates_accepted() {
    let pm_rd = rd();
    let srd = stitch_rd(&pm_rd);
    let tensor = genesis_tensor();
    let cell_report = completed_cell_report(&pm_rd);
    let rev_before = tensor.tensor_revision;

    let bundle = run_stitch_cycle(&srd, tensor, &cell_report).unwrap();
    match &bundle.outcome {
        StitcherOutcome::Completed(r) if !r.accepted_updates.is_empty() => {
            assert_eq!(
                bundle.tensor_after.tensor_revision,
                rev_before + 1,
                "tensor revision must increment by 1 per accepted batch"
            );
            assert_eq!(
                bundle.tensor_after.previous_tensor_hash.as_ref(),
                Some(&bundle.tensor_before.tensor_id),
                "tensor chain must be preserved"
            );
        }
        // Hold or Completed with no updates — tensor is unchanged.
        _ => {
            assert_eq!(
                bundle.tensor_after.tensor_id, bundle.tensor_before.tensor_id,
                "tensor must be byte-identical when no updates were accepted"
            );
        }
    }
}

#[test]
fn fabric_h_isolation_invariant_holds() {
    // Invariant 1: Fabric-H may never directly mutate Fabric-T.
    // The pipeline enforces this by routing all Fabric-T changes
    // through the StitcherGate; we verify that the Fabric-H hash
    // stored in the bundle matches what was used to derive candidates.
    let pm_rd = rd();
    let srd = stitch_rd(&pm_rd);
    let tensor = genesis_tensor();
    let cell_report = completed_cell_report(&pm_rd);

    let bundle = run_stitch_cycle(&srd, tensor, &cell_report).unwrap();

    // The StitcherReport must record the same fabric_h_hash as the
    // bundle's Fabric-H snapshot.
    let expected_fabric_h_hash = &bundle.fabric_h.fabric_h_id;
    let report_fabric_h_hash = match &bundle.outcome {
        StitcherOutcome::Completed(r) => &r.fabric_h_hash,
        StitcherOutcome::Hold(r) => &r.fabric_h_hash,
        StitcherOutcome::Rejected(r) => &r.fabric_h_hash,
    };
    assert_eq!(
        report_fabric_h_hash, expected_fabric_h_hash,
        "StitcherReport must reference the same Fabric-H that was used"
    );

    // Tensor before and after must differ only through accepted updates
    // (not a direct Fabric-H mutation).
    match &bundle.outcome {
        StitcherOutcome::Completed(r) if r.accepted_updates.is_empty() => {
            assert_eq!(
                bundle.tensor_before.tensor_id,
                bundle.tensor_after.tensor_id
            );
        }
        _ => {}
    }
}
