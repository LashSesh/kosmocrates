//! End-to-end tests for PHASEMATRIX-HIVEMIND-03.
//!
//! Drives the full cell-substrate cycle and verifies the spec's
//! invariants: deterministic plan, ADAMANT-aligned trace contract,
//! fail-closed cluster formation, dissolution trace preservation,
//! handoff candidates carrying no commit artefacts.

use phase_matrix::cell::pipeline::{
    run_cell_substrate_cycle, CellSubstrateInput, CellSubstrateOutcome,
};
use phase_matrix::cell::primitives::{Fixed, Hash256};
use phase_matrix::cell::run_descriptor::{
    CellSubstratePolicies, CellSubstrateThresholds, PhaseMatrixRunDescriptorV3,
};
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
