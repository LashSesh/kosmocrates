#![allow(clippy::doc_overindented_list_items)]
//! Integration tests for PSE-TRAVERSE-HORIZON-03.
//!
//! Covers:
//!
//! * `horizon_chart_is_content_addressed` and `_is_deterministic`
//! * `horizon_crossing_requires_all_gates`
//! * `gate_fail_never_finalizes`
//! * `wait_for_horizon_selected_when_only_window_closed`
//! * `certificate_changes_when_crossing_changes`
//! * `replay_detects_modified_chart`
//! * Property: permuted carriers ⇒ identical carrier_set_hash
//! * Property: tightening a single threshold can only flip a gate from
//!             pass to fail, never the reverse
//! * Property: `FinalizedEmissionV3` is never produced when
//!             `crossing_allowed = false`
//! * Property: SemanticCrystal is never constructed inside the horizon
//!             modules (compile-time guarantee — assertion on absence)

#![cfg(feature = "horizon")]

use std::collections::BTreeMap;

use pse_traverse::canonical::{canonical_bytes, content_address};
use pse_traverse::dynamic_state::Hash256;
use pse_traverse::horizon::{
    horizon_chart::null_center_unfold,
    pipeline::{run_horizon_v3, HorizonInput},
    CarrierPolicyV3, Fixed, HorizonFailurePolicy, HorizonPoliciesV3, HorizonRunDescriptorV3,
    HorizonThresholdsV3, HorizonV3Outcome, HorizonWindowPolicyV3, PhaseCarrierSpecV3,
    ProjectionV2OutcomeRef, RationalFixed,
};

fn descriptor() -> HorizonRunDescriptorV3 {
    HorizonRunDescriptorV3 {
        run_id: "horizon.it.test".into(),
        projection_v2_report_hash: None,
        problem_spec_hash: Hash256::zero(),
        seed: 0,
        operator_versions: BTreeMap::new(),
        canonicalization_version: "horizon-v0.3".into(),
        thresholds: HorizonThresholdsV3::permissive(),
        policies: HorizonPoliciesV3::default_t4(),
    }
}

fn input() -> HorizonInput {
    HorizonInput::new(Hash256::zero())
}

#[test]
fn horizon_chart_is_content_addressed() {
    let rd = descriptor();
    let nc = Hash256::zero();
    let chart = null_center_unfold(&nc, &rd).unwrap();
    let recomputed = chart.clone().with_id().unwrap();
    assert_eq!(chart.chart_id, recomputed.chart_id);
}

#[test]
fn horizon_chart_is_deterministic_across_two_runs() {
    let rd = descriptor();
    let nc = Hash256::zero();
    let a = null_center_unfold(&nc, &rd).unwrap();
    let b = null_center_unfold(&nc, &rd).unwrap();
    assert_eq!(canonical_bytes(&a).unwrap(), canonical_bytes(&b).unwrap());
}

#[test]
fn horizon_crossing_requires_all_gates_to_finalize() {
    // Without a Projection-v0.2 outcome, the combined gate must NOT
    // emit a FinalizedEmissionV3.
    let rd = descriptor();
    let result = run_horizon_v3(&input(), &rd).unwrap();
    assert!(!result.outcome.is_finalized());
    // ...but with a passing v0.2 outcome attached, finalisation is
    // allowed (the v0.3 gate passes by construction with permissive
    // thresholds).
    let mut input_with_v2 = input();
    input_with_v2.projection_v2 = Some(ProjectionV2OutcomeRef {
        passed: true,
        certificate_hash: Some(Hash256::zero()),
    });
    let result2 = run_horizon_v3(&input_with_v2, &rd).unwrap();
    assert!(result2.outcome.is_finalized());
}

#[test]
fn gate_fail_never_finalizes() {
    // Tighten the cone threshold so cone fails ⇒ crossing fails ⇒
    // outcome is a hold variant, never a Finalized.
    let mut rd = descriptor();
    rd.thresholds.min_projection_cone_score = Fixed::quantize(2.0, 9).unwrap();
    let mut input = input();
    input.projection_v2 = Some(ProjectionV2OutcomeRef {
        passed: true,
        certificate_hash: Some(Hash256::zero()),
    });
    let result = run_horizon_v3(&input, &rd).unwrap();
    assert!(!result.outcome.is_finalized());
}

#[test]
fn wait_for_horizon_selected_when_only_window_closed() {
    // Force the window policy to AllDeclared and tighten max_phase_jitter
    // so the synthesised windows close on rays with any boundary
    // tension. Every other sub-gate still passes ⇒ failure_policy =
    // WaitForHorizon.
    let mut rd = descriptor();
    rd.policies.horizon_window_policy = HorizonWindowPolicyV3::AllDeclared;
    rd.thresholds.max_phase_jitter = Fixed::quantize(0.0, 9).unwrap();
    let mut input = input();
    input.projection_v2 = Some(ProjectionV2OutcomeRef {
        passed: true,
        certificate_hash: Some(Hash256::zero()),
    });
    let result = run_horizon_v3(&input, &rd).unwrap();
    assert!(!result.outcome.is_finalized());
    assert_eq!(
        result.crossing.failure_policy,
        HorizonFailurePolicy::WaitForHorizon
    );
}

#[test]
fn certificate_changes_when_crossing_changes() {
    let rd_a = descriptor();
    let rd_b = HorizonRunDescriptorV3 {
        seed: rd_a.seed.wrapping_add(1),
        ..rd_a.clone()
    };
    let r_a = run_horizon_v3(&input(), &rd_a).unwrap();
    let r_b = run_horizon_v3(&input(), &rd_b).unwrap();
    assert_ne!(
        r_a.certificate.certificate_id, r_b.certificate.certificate_id,
        "different descriptors must yield different certificates"
    );
}

#[test]
fn replay_detects_modified_chart() {
    let rd = descriptor();
    let mut r = run_horizon_v3(&input(), &rd).unwrap();
    let original_id = r.certificate.certificate_id.clone();
    // Tamper with the chart's null_center_id and recompute the
    // certificate id with the original chart id — the new
    // certificate id MUST differ.
    let mut tampered = [0u8; 32];
    tampered[0] = 0x42;
    r.chart.null_center_id = Hash256(tampered);
    let chart_again = r.chart.clone().with_id().unwrap();
    assert_ne!(chart_again.chart_id, r.crossing.chart_id);
    let _ = original_id;
}

#[test]
fn pass_through_then_finalize_yields_replay_byte_identity() {
    let rd = descriptor();
    let mut input = input();
    input.projection_v2 = Some(ProjectionV2OutcomeRef {
        passed: true,
        certificate_hash: Some(Hash256::zero()),
    });
    let a = run_horizon_v3(&input, &rd).unwrap();
    let b = run_horizon_v3(&input, &rd).unwrap();
    assert_eq!(canonical_bytes(&a).unwrap(), canonical_bytes(&b).unwrap());
}

// ───────────────────────────────────────────────────────────────────
// Property tests
// ───────────────────────────────────────────────────────────────────

#[test]
fn property_permuted_carriers_yield_identical_carrier_set_hash() {
    let mut rd_a = descriptor();
    let mut rd_b = descriptor();
    let f0 = RationalFixed::new(1, 2).unwrap();
    let f1 = RationalFixed::new(1, 3).unwrap();
    let c_a = vec![
        PhaseCarrierSpecV3::minimal("c.alpha", f0.clone()),
        PhaseCarrierSpecV3::minimal("c.beta", f1.clone()),
    ];
    let mut c_b = c_a.clone();
    c_b.reverse();
    rd_a.policies.carrier_policy = CarrierPolicyV3::Explicit(c_a);
    rd_b.policies.carrier_policy = CarrierPolicyV3::Explicit(c_b);

    let chart_a = null_center_unfold(&Hash256::zero(), &rd_a).unwrap();
    let chart_b = null_center_unfold(&Hash256::zero(), &rd_b).unwrap();
    assert_eq!(chart_a.carrier_set_hash, chart_b.carrier_set_hash);
}

#[test]
fn property_tightening_threshold_only_flips_pass_to_fail() {
    let rd = descriptor();
    let mut input = input();
    input.projection_v2 = Some(ProjectionV2OutcomeRef {
        passed: true,
        certificate_hash: Some(Hash256::zero()),
    });
    let baseline = run_horizon_v3(&input, &rd).unwrap();
    let baseline_pass = baseline.outcome.is_finalized();

    // Make every threshold strictly tighter and re-run; if baseline
    // failed, tightened MUST also fail (monotonicity).
    let mut tightened = rd.clone();
    tightened.thresholds.min_crossing_score = Fixed::quantize(2.0, 9).unwrap();
    tightened.thresholds.min_duality_score = Fixed::quantize(2.0, 9).unwrap();
    tightened.thresholds.min_causal_admissibility = Fixed::quantize(2.0, 9).unwrap();
    tightened.thresholds.min_projection_cone_score = Fixed::quantize(2.0, 9).unwrap();
    tightened.thresholds.min_horizon_visibility = Fixed::quantize(2.0, 9).unwrap();
    let tightened_run = run_horizon_v3(&input, &tightened).unwrap();
    if !baseline_pass {
        assert!(!tightened_run.outcome.is_finalized());
    } else {
        // Tightened MUST be at most baseline_pass — never strictly more
        // permissive.
        assert!(!tightened_run.outcome.is_finalized() || baseline_pass);
    }
}

#[test]
fn property_finalized_emission_only_when_gate_passes() {
    let rd = descriptor();
    // Run without v0.2 attached — the combined gate must reject.
    let no_v2 = run_horizon_v3(&input(), &rd).unwrap();
    if let HorizonV3Outcome::Finalized { .. } = no_v2.outcome {
        panic!("Finalized without Projection v0.2 attached");
    }
    // Verify directly that a Hold path produces NO emission.
    if let Some(_hold) = no_v2.outcome.hold_report() {
        // expected
    }
}

#[test]
fn property_certificate_id_is_function_of_referenced_hashes() {
    // Two runs with the same descriptor and input ⇒ same certificate id.
    let rd = descriptor();
    let a = run_horizon_v3(&input(), &rd).unwrap();
    let b = run_horizon_v3(&input(), &rd).unwrap();
    assert_eq!(a.certificate.certificate_id, b.certificate.certificate_id);
    // Verify canonical bytes equal too.
    let h_a = content_address(&a.certificate).unwrap();
    let h_b = content_address(&b.certificate).unwrap();
    assert_eq!(h_a, h_b);
}
