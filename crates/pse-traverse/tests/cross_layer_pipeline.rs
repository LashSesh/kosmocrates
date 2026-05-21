//! Cross-layer integration tests — topology → cognition handoff.
//!
//! Verifies that the content-addressed link between the TPT-MTL topology
//! pipeline and the cognition kernel is real: changing topology input changes
//! the downstream cognition state_id (Inv: content-addressed handoff chain).
//!
//! These are the "maiden flight" checks — they exercise the actual data path
//! that connects both layers in production.

#![cfg(all(feature = "topology", feature = "cognition"))]

use std::collections::BTreeMap;

use pse_traverse::cognition::{
    pipeline::{run_cognition, CognitionInput},
    CognitionOutcome, CognitionPolicies, CognitionRunDescriptor, CognitionThresholds, Fixed,
};
use pse_traverse::dynamic_state::Hash256;
use pse_traverse::topology::{
    phase_window::{PointStatus, SamplePoint, TptMtlInput},
    pipeline::run_tpt_mtl,
    primitives::{Fixed as TFixed, TptEvidenceRef},
    run_descriptor::TptMtlRunDescriptor,
};

// ── Builders ──────────────────────────────────────────────────────────────

fn tpt_input(n: usize) -> TptMtlInput {
    use pse_traverse::topology::carrier::CarrierContext;
    let sample_points = (0..n)
        .map(|i| SamplePoint {
            coordinates: [
                TFixed::quantize(i as f64 * 0.1 + 0.05, 9).unwrap(),
                TFixed::quantize(i as f64 * 0.12, 9).unwrap(),
                TFixed::quantize(i as f64 * 0.08 + 0.1, 9).unwrap(),
                TFixed::quantize(i as f64 * 0.15, 9).unwrap(),
                TFixed::quantize(i as f64 * 0.09 + 0.02, 9).unwrap(),
            ],
            provenance: vec![TptEvidenceRef {
                evidence_id: Hash256::zero(),
                kind: "test".into(),
                source_digest: Hash256::zero(),
                trace_ref: Hash256::zero(),
            }],
            status: Some(PointStatus::Active),
        })
        .collect();
    TptMtlInput {
        state_digest: Hash256::from_hex(
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        )
        .unwrap(),
        carrier: Some(CarrierContext::new_minimal(1)),
        horizon_refs: vec![],
        constraint_refs: vec![],
        sample_points,
        boundary_ref: Hash256::zero(),
        trace_ref: Hash256::zero(),
    }
}

fn cognition_rd() -> CognitionRunDescriptor {
    CognitionRunDescriptor {
        run_id: "cross_layer.test".into(),
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

fn cognition_input_with_traversal(traversal_hash: Hash256) -> CognitionInput {
    let mut input = CognitionInput::minimal(Hash256::zero());
    input.source_traversal_report_hash = Some(traversal_hash);
    input
}

// ── Tests ─────────────────────────────────────────────────────────────────

/// The topology bundle_id is a deterministic content address — running
/// the same input twice gives the same bundle_id.
#[test]
fn topology_bundle_id_is_deterministic() {
    let rd = TptMtlRunDescriptor::default_permissive();
    let input = tpt_input(3);
    let r1 = run_tpt_mtl(&input, &rd).unwrap();
    let r2 = run_tpt_mtl(&input, &rd).unwrap();
    assert_eq!(
        r1.outcome.bundle.bundle_id, r2.outcome.bundle.bundle_id,
        "topology must be deterministic"
    );
}

/// Passing a topology bundle_id as source_traversal_report_hash is accepted
/// by the cognition pipeline and produces a valid run result.
#[test]
fn cognition_accepts_topology_bundle_hash() {
    let trd = TptMtlRunDescriptor::default_permissive();
    let topology_result = run_tpt_mtl(&tpt_input(3), &trd).unwrap();
    let bundle_hash = topology_result.outcome.bundle.bundle_id;

    let cog_input = cognition_input_with_traversal(bundle_hash);
    let cog_result = run_cognition(&cog_input, &cognition_rd()).unwrap();

    assert_ne!(
        cog_result.canonical_state.state_id,
        Hash256::zero(),
        "cognition state_id must be non-zero"
    );
}

/// The cognition state_id is a function of the traversal hash:
/// same topology → same bundle_id → same cognition state_id.
#[test]
fn cognition_state_id_is_deterministic_given_same_topology() {
    let trd = TptMtlRunDescriptor::default_permissive();
    let bundle_hash = run_tpt_mtl(&tpt_input(3), &trd)
        .unwrap()
        .outcome
        .bundle
        .bundle_id;

    let r1 = run_cognition(
        &cognition_input_with_traversal(bundle_hash.clone()),
        &cognition_rd(),
    )
    .unwrap();
    let r2 = run_cognition(
        &cognition_input_with_traversal(bundle_hash),
        &cognition_rd(),
    )
    .unwrap();

    assert_eq!(
        r1.canonical_state.state_id, r2.canonical_state.state_id,
        "same topology input must yield same cognition state"
    );
}

/// Changing topology input changes the bundle_id which changes the cognition
/// state_id — the content-addressed handoff chain is real.
#[test]
fn different_topology_yields_different_cognition_state_id() {
    let trd = TptMtlRunDescriptor::default_permissive();
    let hash_a = run_tpt_mtl(&tpt_input(3), &trd)
        .unwrap()
        .outcome
        .bundle
        .bundle_id;
    let hash_b = run_tpt_mtl(&tpt_input(4), &trd)
        .unwrap()
        .outcome
        .bundle
        .bundle_id;

    assert_ne!(
        hash_a, hash_b,
        "different inputs must produce different bundles"
    );

    let r_a = run_cognition(&cognition_input_with_traversal(hash_a), &cognition_rd()).unwrap();
    let r_b = run_cognition(&cognition_input_with_traversal(hash_b), &cognition_rd()).unwrap();

    assert_ne!(
        r_a.canonical_state.state_id, r_b.canonical_state.state_id,
        "different topology must produce different cognition state — handoff chain must be real"
    );
}

/// The full cross-layer chain is fail-closed: cognition with permissive
/// thresholds produces CandidateBundle (gate passes), and the traversal hash
/// is included in the content-addressed state.
#[test]
fn cross_layer_chain_produces_candidate_bundle_on_permissive_thresholds() {
    let trd = TptMtlRunDescriptor::default_permissive();
    let bundle_hash = run_tpt_mtl(&tpt_input(3), &trd)
        .unwrap()
        .outcome
        .bundle
        .bundle_id;

    let result = run_cognition(
        &cognition_input_with_traversal(bundle_hash),
        &cognition_rd(),
    )
    .unwrap();

    assert!(
        result.handoff_gate.passed,
        "permissive thresholds must pass the handoff gate"
    );
    assert!(
        matches!(result.outcome, CognitionOutcome::CandidateBundle { .. }),
        "must emit CandidateBundle, not Hold"
    );
    assert!(
        result.bundle.is_some(),
        "bundle must be present when gate passes"
    );
}

/// Fail-closed property holds across layers: tightening cognition thresholds
/// to unreachable values produces Hold regardless of what topology produced.
#[test]
fn cross_layer_fail_closed_when_cognition_thresholds_unreachable() {
    let trd = TptMtlRunDescriptor::default_permissive();
    let bundle_hash = run_tpt_mtl(&tpt_input(3), &trd)
        .unwrap()
        .outcome
        .bundle
        .bundle_id;

    let mut strict_rd = cognition_rd();
    // Set all thresholds to unreachable values.
    strict_rd.thresholds.min_panorama_coverage = Fixed::quantize(2.0, 9).unwrap();
    strict_rd.thresholds.min_self_model_coherence = Fixed::quantize(2.0, 9).unwrap();
    strict_rd.thresholds.min_constraint_mass = Fixed::quantize(100.0, 9).unwrap();

    let result = run_cognition(&cognition_input_with_traversal(bundle_hash), &strict_rd).unwrap();

    assert!(
        !result.handoff_gate.passed,
        "unreachable thresholds must fail the handoff gate"
    );
    assert!(
        matches!(result.outcome, CognitionOutcome::Hold { .. }),
        "must be Hold, not CandidateBundle — fail-closed property must hold across layer boundary"
    );
    assert!(
        result.bundle.is_none(),
        "bundle must be absent when gate fails"
    );
}
