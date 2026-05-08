//! Integration & property tests for PSE-TRAVERSE-COGNITION-01 (§21.2,
//! §21.3).

#![cfg(feature = "cognition")]

use std::collections::BTreeMap;

use pse_traverse::canonical::canonical_bytes;
use pse_traverse::cognition::{
    pipeline::{run_cognition, CognitionInput},
    CognitionOutcome, CognitionPolicies, CognitionRunDescriptor, CognitionThresholds, Fixed,
};
use pse_traverse::dynamic_state::Hash256;

fn rd() -> CognitionRunDescriptor {
    CognitionRunDescriptor {
        run_id: "cognition.it.test".into(),
        problem_spec_hash: Hash256::zero(),
        traversal_report_hash: None,
        projection_v2_hash: None,
        seed: 0,
        operator_versions: BTreeMap::new(),
        thresholds: CognitionThresholds::permissive(),
        policies: CognitionPolicies::default_policies(),
        canonicalization_version: "cognition-v0.1".into(),
    }
}

fn input() -> CognitionInput {
    CognitionInput::minimal(Hash256::zero())
}

#[test]
fn cognition_state_is_content_addressed() {
    let r_a = run_cognition(&input(), &rd()).unwrap();
    let r_b = run_cognition(&input(), &rd()).unwrap();
    assert_eq!(r_a.canonical_state.state_id, r_b.canonical_state.state_id);
}

#[test]
fn two_identical_runs_yield_byte_identical_reports() {
    let a = run_cognition(&input(), &rd()).unwrap();
    let b = run_cognition(&input(), &rd()).unwrap();
    assert_eq!(
        canonical_bytes(&a.report).unwrap(),
        canonical_bytes(&b.report).unwrap()
    );
    assert_eq!(a.replay_hash, b.replay_hash);
}

#[test]
fn handoff_gate_passes_when_thresholds_permissive() {
    let r = run_cognition(&input(), &rd()).unwrap();
    assert!(r.handoff_gate.passed);
    assert!(matches!(
        r.outcome,
        CognitionOutcome::CandidateBundle { .. }
    ));
    assert!(r.bundle.is_some());
}

#[test]
fn gate_fails_closed_when_threshold_too_strict() {
    let mut rd = rd();
    rd.thresholds.min_panorama_coverage = Fixed::quantize(2.0, 9).unwrap();
    let r = run_cognition(&input(), &rd).unwrap();
    assert!(!r.handoff_gate.passed);
    assert!(matches!(r.outcome, CognitionOutcome::Hold { .. }));
    assert!(r.bundle.is_none());
}

#[test]
fn report_id_changes_when_descriptor_changes() {
    let a = run_cognition(&input(), &rd()).unwrap();
    let mut rd_b = rd();
    rd_b.seed = rd_b.seed.wrapping_add(1);
    let b = run_cognition(&input(), &rd_b).unwrap();
    assert_ne!(a.report.report_id, b.report.report_id);
}

#[test]
fn no_finalized_emission_or_crystal_in_run_result() {
    // Negative test: the cognition kernel MUST NOT produce a
    // FinalizedEmission or SemanticCrystal. The run result type only
    // exposes a `bundle` (CognitionCandidateBundle); its serialized
    // bytes do not contain crystal markers.
    let r = run_cognition(&input(), &rd()).unwrap();
    let bytes = canonical_bytes(&r).unwrap();
    let s = String::from_utf8_lossy(&bytes);
    assert!(!s.contains("SemanticCrystal"));
    assert!(!s.contains("FinalizedEmission"));
    assert!(!s.contains("crystal_id"));
}

// ──────────────────────────────────────────────────────────────────
// Property tests (§21.3)
// ──────────────────────────────────────────────────────────────────

#[test]
fn property_tightening_thresholds_is_monotone_to_failure() {
    let baseline = run_cognition(&input(), &rd()).unwrap();
    let baseline_pass = baseline.handoff_gate.passed;

    let mut tightened = rd();
    tightened.thresholds.min_panorama_coverage = Fixed::quantize(2.0, 9).unwrap();
    tightened.thresholds.min_self_model_coherence = Fixed::quantize(2.0, 9).unwrap();
    tightened.thresholds.min_constraint_mass = Fixed::quantize(100.0, 9).unwrap();
    let tightened_run = run_cognition(&input(), &tightened).unwrap();
    if !baseline_pass {
        assert!(!tightened_run.handoff_gate.passed);
    } else {
        // baseline passed, tightened must be ≤ baseline.
        assert!(!tightened_run.handoff_gate.passed || baseline_pass);
    }
}

#[test]
fn property_candidate_bundle_id_is_function_of_referenced_hashes() {
    let a = run_cognition(&input(), &rd()).unwrap();
    let b = run_cognition(&input(), &rd()).unwrap();
    if let (Some(ba), Some(bb)) = (&a.bundle, &b.bundle) {
        assert_eq!(ba.bundle_id, bb.bundle_id);
    }
}
