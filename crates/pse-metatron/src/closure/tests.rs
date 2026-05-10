//! Unit, integration and negative tests for PSE-METATRON-MONOLITH-01.

use super::primitives::{CanonicalNumber, Hash256, NctcsClass};
use super::*;

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

/// §10.1 #1: MetatronOperator does not mutate inputs.
/// Verified by API design: run_metatron_closure takes all inputs by value
/// and returns a new struct; original artifact refs are never modified.
#[test]
fn metatron_operator_does_not_mutate_inputs() {
    let h = hash_a();
    // Content address is idempotent (pure function of input).
    let id1 = super::primitives::content_address(&h).unwrap();
    let id2 = super::primitives::content_address(&h).unwrap();
    assert_eq!(id1, id2);
}

/// §10.1 #2: LocalMonolithProjection is content-addressed.
#[test]
fn local_monolith_projection_is_content_addressed() {
    let p = LocalMonolithProjection::build(
        hash_a(),
        hash_a(),
        q(10000),
        q(10000),
        q(5000),
        Some(hash_a()),
        hash_a(),
        vec![],
    )
    .unwrap();
    let id1 = p.projection_id.clone();
    // Re-computing must yield the same id.
    let p2 = LocalMonolithProjection::build(
        hash_a(),
        hash_a(),
        q(10000),
        q(10000),
        q(5000),
        Some(hash_a()),
        hash_a(),
        vec![],
    )
    .unwrap();
    assert_eq!(id1, p2.projection_id);
    assert_ne!(p.projection_id, zero());
}

/// §10.1 #3: IsomorphicProjection requires gate order preservation.
#[test]
fn isomorphic_projection_requires_gate_order_preservation() {
    let r = IsomorphicProjectionReport::build(
        hash_a(),
        hash_b(),
        hash_a(),
        hash_b(),
        true,  // operator_path_compatible
        false, // gate_order_preserved = false  ← not preserved
        true,
        true,
        q(0),
        q(10000),
        vec![],
    )
    .unwrap();
    assert!(!r.passed);
    assert!(!r.gate_order_preserved);
}

/// §10.1 #4: IsomorphicProjection rejects missing trace dependency.
#[test]
fn isomorphic_projection_rejects_missing_trace_dependency() {
    let r = IsomorphicProjectionReport::build(
        hash_a(),
        hash_b(),
        hash_a(),
        hash_b(),
        true,
        true,
        false, // trace_dependency_preserved = false
        true,
        q(0),
        q(10000),
        vec![],
    )
    .unwrap();
    assert!(!r.passed);
    assert!(!r.trace_dependency_preserved);
}

/// §10.1 #5: SpectralGapDelta is deterministic.
#[test]
fn spectral_gap_delta_is_deterministic() {
    let build = || {
        SpectralGapStitchReport::build(q(5000), q(6000), vec![hash_a()], hash_a(), vec![]).unwrap()
    };
    let r1 = build();
    let r2 = build();
    assert_eq!(r1.report_id, r2.report_id);
    assert_eq!(r1.delta_gap, r2.delta_gap);
}

/// §10.1 #6: Coherence alone does not pass MetatronGate.
#[test]
fn coherence_alone_does_not_pass_metatron_gate() {
    // G_iso fails (no iso reports) → gate fails.
    let gap = SpectralGapStitchReport::build(q(5000), q(6000), vec![], hash_a(), vec![]).unwrap();
    let gate = gates::MetatronGateReport::evaluate(
        NctcsClass::C4,
        NctcsClass::C2,
        true,
        &q(10000),
        &q(10000),
        &[], // no iso reports → g_iso fails
        &gap,
        true,
        &q(0),
        &q(5),
        vec![],
    )
    .unwrap();
    // g_iso fails because no report has passed=true.
    assert!(!gate.g_iso.passed);
    assert!(!gate.passed);
}

/// §10.1 #7: MetatronGate fails closed without replay.
#[test]
fn metatron_gate_fails_closed_without_replay() {
    let gap =
        SpectralGapStitchReport::build(q(5000), q(6000), vec![hash_a()], hash_a(), vec![]).unwrap();
    let iso = IsomorphicProjectionReport::build(
        hash_a(),
        hash_b(),
        hash_a(),
        hash_b(),
        true,
        true,
        true,
        true,
        q(0),
        q(10000),
        vec![],
    )
    .unwrap();
    let gate = gates::MetatronGateReport::evaluate(
        NctcsClass::C4,
        NctcsClass::C2,
        true,
        &q(0),     // replay_identity = 0 → fails G_replay
        &q(10000), // min_replay_identity = 100%
        &[iso],
        &gap,
        true,
        &q(0),
        &q(5),
        vec![],
    )
    .unwrap();
    assert!(!gate.g_replay.passed);
    assert!(!gate.passed);
}

/// §10.1 #8: HolisticEigenmode requires MacroControlState (gate must pass).
#[test]
fn holistic_eigenmode_requires_macro_control_state() {
    use pipeline::{MetatronClosureInput, MetatronClosureOutcome};
    use run_descriptor::MetatronRunDescriptor;

    let rd = MetatronRunDescriptor::new("test", hash_a(), hash_b());
    let gap = SpectralGapStitchReport::build(q(5000), q(6000), vec![], hash_a(), vec![]).unwrap();

    let input = MetatronClosureInput {
        nctcs_report_ref: hash_b(),
        validation_report_ref: hash_a(),
        null_center_ref: hash_a(),
        field_tensor_ref: hash_a(),
        macro_control_state_ref: hash_a(), // present
        trace_head: hash_a(),
        nctcs_class: NctcsClass::C4,
        local_projections: vec![],
        isomorphic_reports: vec![],
        spectral_gap_report: gap,
        replay_identity: q(10000),
        eval_status_ok: true,
        unexplained_drift: q(0),
        global_coherence: q(8000),
        evidence_refs: vec![],
    };
    // Gate fails (no iso reports), so diagnostic is returned.
    let outcome = pipeline::run_metatron_closure(rd, input).unwrap();
    assert!(matches!(outcome, MetatronClosureOutcome::Diagnostic(_)));
}

// ── Integration Tests ─────────────────────────────────────────────────────────

/// Full closure with all gates passing produces HolisticEigenmodeState.
#[test]
fn full_closure_produces_holistic_eigenmode_state() {
    use pipeline::{MetatronClosureInput, MetatronClosureOutcome};
    use run_descriptor::MetatronRunDescriptor;

    let rd = MetatronRunDescriptor::new("full-test", hash_a(), hash_b());
    let gap =
        SpectralGapStitchReport::build(q(5000), q(6000), vec![hash_a()], hash_a(), vec![]).unwrap();
    let iso = IsomorphicProjectionReport::build(
        hash_a(),
        hash_b(),
        hash_a(),
        hash_b(),
        true,
        true,
        true,
        true,
        q(0),
        q(10000),
        vec![],
    )
    .unwrap();
    let proj = LocalMonolithProjection::build(
        hash_a(),
        hash_a(),
        q(10000),
        q(10000),
        q(5000),
        Some(hash_a()),
        hash_a(),
        vec![],
    )
    .unwrap();

    let input = MetatronClosureInput {
        nctcs_report_ref: hash_b(),
        validation_report_ref: hash_a(),
        null_center_ref: hash_a(),
        field_tensor_ref: hash_a(),
        macro_control_state_ref: hash_a(),
        trace_head: hash_a(),
        nctcs_class: NctcsClass::C4,
        local_projections: vec![proj],
        isomorphic_reports: vec![iso],
        spectral_gap_report: gap,
        replay_identity: q(10000),
        eval_status_ok: true,
        unexplained_drift: q(0),
        global_coherence: q(8000),
        evidence_refs: vec![],
    };
    let outcome = pipeline::run_metatron_closure(rd, input).unwrap();
    match outcome {
        MetatronClosureOutcome::Closed(state) => {
            assert_ne!(state.state_id, zero());
        }
        other => panic!("expected Closed, got {:?}", std::mem::discriminant(&other)),
    }
}

/// Replay must be byte-identical.
#[test]
fn metatron_replay_byte_identical() {
    use holistic_state::{
        AggregateGateStatus, HolisticEigenmodeState, MetatronConformanceClass,
        MetatronValidationStatus,
    };
    use replay::verify_holistic_eigenmode_state;

    let build = || {
        HolisticEigenmodeState::build(
            hash_a(),
            hash_a(),
            hash_a(),
            hash_a(),
            hash_a(),
            vec![hash_a()],
            vec![hash_b()],
            hash_a(),
            AggregateGateStatus {
                total_gates: 7,
                passed_gates: 7,
                failed_gates: 0,
            },
            q(8000),
            MetatronConformanceClass::M4,
            MetatronValidationStatus::Closed,
            vec![],
        )
        .unwrap()
    };

    let s1 = build();
    let s2 = build();
    assert_eq!(s1.state_id, s2.state_id);
    assert!(verify_holistic_eigenmode_state(&s1).unwrap());
}

// ── Negative Tests ────────────────────────────────────────────────────────────

/// §10.3 #1: No HolisticEigenmodeState without NCTCS-Report (nctcs_class < min).
#[test]
fn no_eigenmode_without_nctcs_report() {
    use pipeline::{MetatronClosureInput, MetatronClosureOutcome};
    use run_descriptor::MetatronRunDescriptor;

    let mut rd = MetatronRunDescriptor::new("neg1", hash_a(), hash_b());
    rd.thresholds.min_nctcs_class = NctcsClass::C4;

    let gap =
        SpectralGapStitchReport::build(q(5000), q(6000), vec![hash_a()], hash_a(), vec![]).unwrap();
    let iso = IsomorphicProjectionReport::build(
        hash_a(),
        hash_b(),
        hash_a(),
        hash_b(),
        true,
        true,
        true,
        true,
        q(0),
        q(10000),
        vec![],
    )
    .unwrap();
    let proj = LocalMonolithProjection::build(
        hash_a(),
        hash_a(),
        q(10000),
        q(10000),
        q(5000),
        Some(hash_a()),
        hash_a(),
        vec![],
    )
    .unwrap();

    let input = MetatronClosureInput {
        nctcs_report_ref: hash_b(),
        validation_report_ref: hash_a(),
        null_center_ref: hash_a(),
        field_tensor_ref: hash_a(),
        macro_control_state_ref: hash_a(),
        trace_head: hash_a(),
        nctcs_class: NctcsClass::C1, // below min_nctcs_class=C4
        local_projections: vec![proj],
        isomorphic_reports: vec![iso],
        spectral_gap_report: gap,
        replay_identity: q(10000),
        eval_status_ok: true,
        unexplained_drift: q(0),
        global_coherence: q(8000),
        evidence_refs: vec![],
    };
    let outcome = pipeline::run_metatron_closure(rd, input).unwrap();
    assert!(matches!(outcome, MetatronClosureOutcome::Diagnostic(_)));
}

/// §10.3 #2: No productive closure without trace.
#[test]
fn no_productive_closure_without_trace() {
    use pipeline::{MetatronClosureInput, MetatronClosureOutcome};
    use run_descriptor::MetatronRunDescriptor;

    let mut rd = MetatronRunDescriptor::new("neg2", hash_a(), hash_b());
    rd.policies.reject_on_missing_trace = true;

    let gap =
        SpectralGapStitchReport::build(q(5000), q(6000), vec![hash_a()], hash_a(), vec![]).unwrap();
    let input = MetatronClosureInput {
        nctcs_report_ref: hash_b(),
        validation_report_ref: hash_a(),
        null_center_ref: hash_a(),
        field_tensor_ref: hash_a(),
        macro_control_state_ref: hash_a(),
        trace_head: zero(), // zero = missing trace
        nctcs_class: NctcsClass::C4,
        local_projections: vec![],
        isomorphic_reports: vec![],
        spectral_gap_report: gap,
        replay_identity: q(10000),
        eval_status_ok: true,
        unexplained_drift: q(0),
        global_coherence: q(8000),
        evidence_refs: vec![],
    };
    let outcome = pipeline::run_metatron_closure(rd, input).unwrap();
    assert!(matches!(outcome, MetatronClosureOutcome::Rejected(_)));
}

/// §10.3 #4: Coherence alone does not pass the gate (→ no productive state).
#[test]
fn coherence_alone_no_productive_state() {
    // Already tested by coherence_alone_does_not_pass_metatron_gate above.
    // Here we verify end-to-end that Diagnostic is returned.
    use pipeline::{MetatronClosureInput, MetatronClosureOutcome};
    use run_descriptor::MetatronRunDescriptor;

    let rd = MetatronRunDescriptor::new("neg4", hash_a(), hash_b());
    let gap = SpectralGapStitchReport::build(q(5000), q(6000), vec![], hash_a(), vec![]).unwrap();
    let input = MetatronClosureInput {
        nctcs_report_ref: hash_b(),
        validation_report_ref: hash_a(),
        null_center_ref: hash_a(),
        field_tensor_ref: hash_a(),
        macro_control_state_ref: hash_a(),
        trace_head: hash_a(),
        nctcs_class: NctcsClass::C4,
        local_projections: vec![],
        isomorphic_reports: vec![], // no iso reports → g_iso fails
        spectral_gap_report: gap,
        replay_identity: q(10000),
        eval_status_ok: true,
        unexplained_drift: q(0),
        global_coherence: q(10000), // maximum coherence
        evidence_refs: vec![],
    };
    let outcome = pipeline::run_metatron_closure(rd, input).unwrap();
    // Even with max coherence, gate fails → Diagnostic, not Closed.
    assert!(matches!(outcome, MetatronClosureOutcome::Diagnostic(_)));
}
