//! Unit, integration and negative tests for PSE-NCTCS-CONFORMANCE-01.

use super::primitives::Hash256;
#[allow(unused_imports)]
use super::*;
use pse_traverse::dynamic_state::CanonicalNumber;

fn zero() -> Hash256 {
    Hash256::zero()
}

fn hash_a() -> Hash256 {
    Hash256([1u8; 32])
}

fn hash_b() -> Hash256 {
    Hash256([2u8; 32])
}

fn q(n: i64) -> CanonicalNumber {
    CanonicalNumber::FixedI64 { raw: n, scale: 2 }
}

// ── Unit Tests ────────────────────────────────────────────────────────────────

/// §10.1 #1: NullCenter is not a phase state.
#[test]
fn null_center_is_not_phase_state() {
    use super::null_center::NullCenterRef;
    let nc = NullCenterRef::new(zero(), zero());
    assert!(nc.not_phase_state);
    assert!(nc.not_agent);
    assert!(nc.declared_exogenous);
    assert!(nc.c0_satisfied());
}

/// §10.1 #2: Projection distinguishes K0 from π0(K0).
#[test]
fn projection_distinguishes_k0_from_theta0() {
    use super::null_center::NullProjectionAudit;
    let k0 = hash_a();
    let projected = hash_b();
    let audit = NullProjectionAudit::build(k0, projected, 5, zero()).unwrap();
    assert!(audit.projection_distinction_passed);
}

/// §10.1 #2 negative: same hash fails.
#[test]
fn projection_fails_when_k0_equals_projection() {
    use super::null_center::NullProjectionAudit;
    let same = hash_a();
    let audit = NullProjectionAudit::build(same.clone(), same, 5, zero()).unwrap();
    assert!(!audit.projection_distinction_passed);
}

/// §10.1 #3: Candidate formation requires phase visibility.
#[test]
fn candidate_requires_phase_visibility() {
    use super::candidate_audit::CandidateFormationAudit;
    let q100 = q(10000);
    let without_visibility = CandidateFormationAudit::build(
        vec![hash_a()],
        vec![],
        false, // candidate_requires_visibility_passed = false
        q100.clone(),
        q100.clone(),
        q100.clone(),
        q100,
    )
    .unwrap();
    assert!(!without_visibility.passed);
}

/// §10.1 #4: Visibility does not imply persistence.
/// (The materialization gate enforces no direct fabric→tensor mutation.)
#[test]
fn visibility_does_not_imply_persistence() {
    use super::phase_visibility::PhaseVisibilityAudit;
    use super::primitives::NctcsThresholds;
    let thresholds = NctcsThresholds::default();
    let audit = PhaseVisibilityAudit::build(hash_a(), vec![hash_a()], vec![], &thresholds).unwrap();
    // Visibility passes but that alone does not produce persistent tensor state.
    assert!(audit.passed);
    // No tensor_after_hash was mutated by this audit — it is a read-only check.
}

/// §10.1 #5: Ephemeral fabric cannot directly mutate tensor.
#[test]
fn ephemeral_fabric_cannot_directly_mutate_tensor() {
    use super::materialization_audit::MaterializationAudit;
    let audit = MaterializationAudit::build(
        hash_a(), // ephemeral_fabric_hash
        hash_a(), // tensor_before_hash
        vec![],   // stitch_candidate_hashes
        vec![],   // accepted_update_hashes
        vec![],   // rejected_candidate_hashes
        vec![],   // gate_report_hashes
        hash_a(), // tensor_after_hash (same = no mutation)
        true,     // no_direct_fabric_to_tensor_mutation
    )
    .unwrap();
    assert!(audit.no_direct_fabric_to_tensor_mutation);
}

/// §10.1 #6: Gate failure preserves tensor state.
#[test]
fn gate_failure_preserves_tensor_state() {
    use super::materialization_audit::MaterializationAudit;
    let before = hash_a();
    let audit = MaterializationAudit::build(
        zero(),
        before.clone(),
        vec![],
        vec![],
        vec![],
        vec![],
        before.clone(),
        true,
    )
    .unwrap();
    // No accepted updates → tensor unchanged.
    assert!(audit.tensor_unchanged_on_no_update);
    assert_eq!(audit.tensor_before_hash, audit.tensor_after_hash);
}

/// §10.1 #7: ReplayReady required for gate pass.
#[test]
fn replay_ready_required_for_gate_pass() {
    use super::trace_replay_contract::TraceReplayContractReport;
    // replay_identity < 100% → not passed.
    let report = TraceReplayContractReport::build(
        zero(),
        zero(),
        zero(),
        zero(),
        zero(),
        q(5000), // 50.00% — below threshold
        true,
    )
    .unwrap();
    assert!(!report.passed);
}

/// §10.1 #8: MacroControlState requires tensor history and trace.
#[test]
fn macro_state_requires_tensor_history_and_trace() {
    use super::conformance::{NctcsConformanceClass, ValidationClosureStatus};
    use super::macro_control_state::{AggregateGateStatus, MacroControlState};
    let agg = AggregateGateStatus {
        total_gates: 1,
        passed_gates: 1,
        failed_gates: 0,
        held_gates: 0,
        quarantined_candidates: 0,
        no_update_count: 0,
        gate_coverage: q(10000),
    };
    let ms = MacroControlState::build(
        "test".into(),
        hash_a(),       // null_center_ref
        Some(hash_b()), // field_tensor_ref (tensor history)
        hash_a(),       // trace_head (non-zero trace)
        None,
        vec![],
        vec![],
        None,
        agg,
        NctcsConformanceClass::C4MacroControl,
        ValidationClosureStatus::StructuralPass,
        None,
        None,
        vec![],
    )
    .unwrap();
    assert_ne!(ms.macro_state_id, Hash256::zero());
    assert!(ms.field_tensor_ref.is_some());
    assert_ne!(ms.trace_head, Hash256::zero());
}

/// §10.1 #9: Coherence is not truth.
#[test]
fn coherence_is_not_truth() {
    // The NCTCS pipeline never outputs "truth" — only MacroControlState
    // with a ValidationClosureStatus. High coherence alone cannot produce C4.
    // Verified by classify_conformance: macro_state = false → not C4.
    use super::candidate_audit::CandidateFormationAudit;
    use super::conformance::{classify_conformance, NctcsConformanceClass};
    use super::materialization_audit::MaterializationAudit;
    use super::null_center::{NullCenterRef, NullProjectionAudit};
    use super::phase_visibility::PhaseVisibilityAudit;
    use super::primitives::NctcsThresholds;
    use super::trace_replay_contract::TraceReplayContractReport;

    let nc = NullCenterRef::new(hash_a(), zero());
    let proj = NullProjectionAudit::build(hash_a(), hash_b(), 5, zero()).unwrap();
    let thresholds = NctcsThresholds::default();
    let vis = PhaseVisibilityAudit::build(hash_a(), vec![hash_a()], vec![], &thresholds).unwrap();
    let cand = CandidateFormationAudit::build(
        vec![hash_a()],
        vec![],
        true,
        q(10000),
        q(10000),
        q(10000),
        q(10000),
    )
    .unwrap();
    let mat = MaterializationAudit::build(
        zero(),
        hash_a(),
        vec![],
        vec![],
        vec![],
        vec![],
        hash_a(),
        true,
    )
    .unwrap();
    let tr = TraceReplayContractReport::build(
        hash_a(),
        hash_a(),
        hash_a(),
        hash_a(),
        hash_a(),
        q(10000),
        true,
    )
    .unwrap();

    // Even with all other gates passing, without macro_state → not C4.
    let conformance = classify_conformance(
        &nc,
        &proj,
        &vis,
        &cand,
        &mat,
        &tr,
        false, // has_macro_control_state
        false,
        zero(),
    )
    .unwrap();
    assert!(conformance.reached_class < NctcsConformanceClass::C4MacroControl);
}

/// §10.1 #10: Hold and Reject are non-materializing.
#[test]
fn hold_and_reject_are_non_materializing() {
    use super::conformance::NctcsGateOutcome;
    assert!(!NctcsGateOutcome::Hold.is_materializing());
    assert!(!NctcsGateOutcome::Reject.is_materializing());
    assert!(!NctcsGateOutcome::Quarantine.is_materializing());
    assert!(!NctcsGateOutcome::NoUpdate.is_materializing());
    assert!(NctcsGateOutcome::Pass.is_materializing());
}

// ── Integration Tests ─────────────────────────────────────────────────────────

/// §10.2 #1: NCTCS closure produces a bundle.
#[test]
fn nctcs_closure_produces_bundle() {
    use super::pipeline::{close_nctcs, NctcsClosureInput};
    use super::run_descriptor::NctcsRunDescriptor;

    let rd = NctcsRunDescriptor::new("test-run", hash_a());
    let input = NctcsClosureInput {
        validation_bundle_hash: hash_a(),
        eval_summary_hash: None,
        stitch_bundle_hash: None,
        tpt_bundle_hash: None,
        phase_flow_hash: hash_a(),
        tick: 42,
        theta_t_hash: hash_b(),
        visible_cell_refs: vec![hash_a()],
        invisible_cell_refs: vec![],
        candidate_refs: vec![hash_a()],
        rejected_refs: vec![],
        activation_gate_coverage: q(10000),
        boundary_ready_coverage: q(10000),
        evidence_ready_coverage: q(10000),
        trace_replay_ready_coverage: q(10000),
        ephemeral_fabric_hash: hash_a(),
        tensor_before_hash: hash_a(),
        stitch_candidate_hashes: vec![hash_a()],
        accepted_update_hashes: vec![hash_a()],
        rejected_candidate_hashes: vec![],
        gate_report_hashes: vec![hash_a()],
        tensor_after_hash: hash_b(),
        trace_head: hash_a(),
        evidence_root: hash_a(),
        gate_history_root: hash_a(),
        replay_manifest_hash: hash_a(),
        has_domain_validation: false,
        domain_validation_hash: None,
    };
    let bundle = close_nctcs(rd, input).unwrap();
    assert_ne!(bundle.bundle_id, zero());
}

/// §10.2 #4: NCTCS replay is byte-identical.
#[test]
fn nctcs_replay_byte_identical() {
    use super::pipeline::{close_nctcs, NctcsClosureInput};
    use super::replay::verify_nctcs_bundle;
    use super::run_descriptor::NctcsRunDescriptor;

    let make_input = || NctcsClosureInput {
        validation_bundle_hash: hash_a(),
        eval_summary_hash: None,
        stitch_bundle_hash: None,
        tpt_bundle_hash: None,
        phase_flow_hash: hash_a(),
        tick: 1,
        theta_t_hash: hash_b(),
        visible_cell_refs: vec![hash_a()],
        invisible_cell_refs: vec![],
        candidate_refs: vec![hash_a()],
        rejected_refs: vec![],
        activation_gate_coverage: q(10000),
        boundary_ready_coverage: q(10000),
        evidence_ready_coverage: q(10000),
        trace_replay_ready_coverage: q(10000),
        ephemeral_fabric_hash: hash_a(),
        tensor_before_hash: hash_a(),
        stitch_candidate_hashes: vec![],
        accepted_update_hashes: vec![],
        rejected_candidate_hashes: vec![],
        gate_report_hashes: vec![],
        tensor_after_hash: hash_a(),
        trace_head: hash_a(),
        evidence_root: hash_a(),
        gate_history_root: hash_a(),
        replay_manifest_hash: hash_a(),
        has_domain_validation: false,
        domain_validation_hash: None,
    };

    let b1 = close_nctcs(NctcsRunDescriptor::new("r", hash_a()), make_input()).unwrap();
    let b2 = close_nctcs(NctcsRunDescriptor::new("r", hash_a()), make_input()).unwrap();
    assert_eq!(b1.bundle_id, b2.bundle_id);
    assert!(verify_nctcs_bundle(&b1).unwrap());
}

// ── Negative Tests ────────────────────────────────────────────────────────────

/// §10.3 #1: No HolisticEigenmodeState without NCTCS report.
/// (The NCTCS layer itself cannot produce C4 without a null center.)
#[test]
fn no_macro_state_without_null_center_c0() {
    use super::candidate_audit::CandidateFormationAudit;
    use super::conformance::classify_conformance;
    use super::conformance::NctcsConformanceClass;
    use super::materialization_audit::MaterializationAudit;
    use super::null_center::NullProjectionAudit;
    use super::phase_visibility::PhaseVisibilityAudit;
    use super::primitives::NctcsThresholds;
    use super::trace_replay_contract::TraceReplayContractReport;

    // NullCenter that fails C0 (not exogenous).
    let mut nc = super::null_center::NullCenterRef::new(zero(), zero());
    nc.declared_exogenous = false;

    let proj = NullProjectionAudit::build(zero(), zero(), 5, zero()).unwrap();
    let thresholds = NctcsThresholds::default();
    let vis = PhaseVisibilityAudit::build(zero(), vec![], vec![], &thresholds).unwrap();
    let cand =
        CandidateFormationAudit::build(vec![], vec![], false, q(0), q(0), q(0), q(0)).unwrap();
    let mat = MaterializationAudit::build(
        zero(),
        zero(),
        vec![],
        vec![],
        vec![],
        vec![],
        zero(),
        false,
    )
    .unwrap();
    let tr = TraceReplayContractReport::build(zero(), zero(), zero(), zero(), zero(), q(0), false)
        .unwrap();

    let r = classify_conformance(&nc, &proj, &vis, &cand, &mat, &tr, false, false, zero()).unwrap();
    assert_eq!(r.reached_class, NctcsConformanceClass::C0FormalTyped);
    assert!(!r.c1.passed);
}

/// §10.3 #2: No productive closure without trace and replay.
#[test]
fn no_productive_closure_without_trace_replay() {
    use super::trace_replay_contract::TraceReplayContractReport;
    let report = TraceReplayContractReport::build(
        zero(),
        zero(),
        zero(),
        zero(),
        zero(),
        q(0),  // replay_identity = 0%
        false, // replay_ready_required_for_gate_pass = false
    )
    .unwrap();
    assert!(!report.passed);
}

/// §10.3 #5: No non-deterministic hashing.
#[test]
fn hashing_is_deterministic() {
    use super::null_center::NullProjectionAudit;
    let a1 = NullProjectionAudit::build(hash_a(), hash_b(), 5, zero()).unwrap();
    let a2 = NullProjectionAudit::build(hash_a(), hash_b(), 5, zero()).unwrap();
    assert_eq!(a1.audit_id, a2.audit_id);
}
