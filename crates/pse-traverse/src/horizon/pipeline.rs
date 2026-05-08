//! Reference pipeline `run_horizon_v3`.
//!
//! Implements §8 of the spec: load → canonicalize → null-resolve → unfold
//! → ray-lift → window-evaluate → cone-check → causal-check → duality-check
//! → crossing-gate → projection-v0.2 merge → finalize-or-hold → certify
//! → replay.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::causal_admissibility::check_causal_admissibility;
use super::certificate::HorizonCertificate;
use super::collapse_emission_duality::{
    check_collapse_emission_duality, CollapseEmissionDualitySpec,
};
use super::gate::{merge_projection_and_horizon_gates, ProjectionV2OutcomeRef};
use super::horizon_chart::{null_center_unfold, HorizonChart};
use super::horizon_crossing::{horizon_crossing_gate, HorizonCrossingReport};
use super::horizon_window::evaluate_horizon_windows;
use super::outcome::{
    outcome_for_policy, FinalizedEmissionV3, HorizonHoldReport, HorizonV3Outcome,
};
use super::phase_ray::{attach_rays_to_chart, lift_phase_rays};
use super::primitives::{Fixed, HorizonError};
use super::projection_cone::check_projection_cone;
use super::replay::replay_hash_of;
use super::run_descriptor::HorizonRunDescriptorV3;

/// Caller-provided horizon input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorizonInput {
    /// State id under audit (typically a content hash of the upstream
    /// projection-v0.2 candidate or any external state).
    pub state_id: Hash256,
    /// Optional null-center id. When omitted the input state id is used
    /// as the canonical null center.
    pub null_center_id: Option<Hash256>,
    /// Optional duality spec. When omitted a canonical identity duality
    /// against the chart's null center is used.
    pub duality_spec: Option<CollapseEmissionDualitySpec>,
    /// Optional Projection-v0.2 outcome to merge into the combined gate.
    pub projection_v2: Option<ProjectionV2OutcomeRef>,
}

impl HorizonInput {
    pub fn new(state_id: Hash256) -> Self {
        HorizonInput {
            state_id,
            null_center_id: None,
            duality_spec: None,
            projection_v2: None,
        }
    }
}

/// One-shot result of `run_horizon_v3`: the outcome plus the certificate
/// and chart so the CLI can emit them without re-deriving. The input
/// is persisted so `replay` can re-run with byte-identical inputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorizonRunResult {
    pub input: HorizonInput,
    pub outcome: HorizonV3Outcome,
    pub chart: HorizonChart,
    pub crossing: HorizonCrossingReport,
    pub certificate: HorizonCertificate,
}

/// Reference pipeline. Total: never panics; every fail-path lands in a
/// `HorizonV3Outcome::*` or returns a `HorizonError`.
pub fn run_horizon_v3(
    input: &HorizonInput,
    rd: &HorizonRunDescriptorV3,
) -> Result<HorizonRunResult, HorizonError> {
    rd.validate()?;

    // §8.1 step 2 — canonical null resolution.
    let null_center_id = input
        .null_center_id
        .clone()
        .unwrap_or_else(|| input.state_id.clone());

    // §8.1 step 3 — unfold.
    let chart = null_center_unfold(&null_center_id, rd)?;

    // §8.1 step 4 — phase-ray lift.
    let rays = lift_phase_rays(&chart, rd)?;
    let chart = attach_rays_to_chart(chart, &rays)?;

    // §8.1 step 5 — windows.
    let (window_reports, visibility_passed) = evaluate_horizon_windows(&rays, rd)?;

    // §8.1 step 6 — cone.
    let cone_report = check_projection_cone(&chart, rd)?;

    // §8.1 step 7 — causal.
    let causal_report = check_causal_admissibility(&chart, &rays, &input.state_id, rd)?;

    // §8.1 step 8 — duality.
    let duality_spec = input
        .duality_spec
        .clone()
        .unwrap_or_else(|| CollapseEmissionDualitySpec {
            duality_id: "duality.identity".into(),
            collapse_operator_id: "collapse.id".into(),
            emission_operator_id: "emit.id".into(),
            null_center_id: null_center_id.clone(),
            max_roundtrip_delta: rd.thresholds.max_roundtrip_delta.clone(),
        });
    let duality_report =
        check_collapse_emission_duality(&input.state_id, &null_center_id, &duality_spec, rd)?;

    // Aggregate per-ray observations for the gate.
    let max_attenuation = max_fixed(rays.iter().map(|r| &r.attenuation));
    let max_boundary_tension = max_fixed(rays.iter().map(|r| &r.boundary_tension));

    // §8.1 step 9 — crossing gate.
    let crossing = horizon_crossing_gate(
        &chart.chart_id,
        &input.state_id,
        window_reports,
        cone_report,
        causal_report,
        duality_report,
        visibility_passed,
        max_attenuation,
        max_boundary_tension,
        rd,
    )?;

    // §8.1 step 10 — merge with Projection v0.2.
    let combined =
        merge_projection_and_horizon_gates(input.projection_v2.as_ref(), &crossing, true)?;

    // §8.1 step 11/12 — finalise or hold.
    let crossing_report_hash =
        Hash256(
            content_address(&crossing).map_err(|e| HorizonError::Canonicalization {
                reason: e.to_string(),
            })?,
        );

    let outcome = if combined.passed {
        let payload_hash = Hash256(
            content_address(&("horizon-v0.3-emission", &chart.chart_id)).map_err(|e| {
                HorizonError::Canonicalization {
                    reason: e.to_string(),
                }
            })?,
        );
        let emission = FinalizedEmissionV3 {
            emission_id: Hash256::zero(),
            chart_id: chart.chart_id.clone(),
            crossing_report_hash: crossing_report_hash.clone(),
            projection_v2_certificate_hash: combined.projection_v2_certificate_hash.clone(),
            canonical_payload_hash: payload_hash,
        }
        .with_id()?;
        HorizonV3Outcome::Finalized {
            emission,
            hold: None,
        }
    } else if crossing.crossing_allowed && !combined.projection_v2_passed {
        // v0.3 alright, v0.2 missing or rejected ⇒ ProjectionOnly hold.
        let hold = HorizonHoldReport::from_crossing(&chart, &crossing)?;
        HorizonV3Outcome::ProjectionOnly {
            projection_v2_certificate_hash: combined.projection_v2_certificate_hash.clone(),
            hold,
        }
    } else {
        let hold = HorizonHoldReport::from_crossing(&chart, &crossing)?;
        outcome_for_policy(crossing.failure_policy, hold)
    };

    // §8.1 step 13 — certificate.
    let finalized_emission_hash = match &outcome {
        HorizonV3Outcome::Finalized { emission, .. } => {
            Some(Hash256(content_address(emission).map_err(|e| {
                HorizonError::Canonicalization {
                    reason: e.to_string(),
                }
            })?))
        }
        _ => None,
    };
    let replay_hash = replay_hash_of(
        &chart.rd_hash,
        &chart.chart_id,
        &crossing_report_hash,
        finalized_emission_hash.as_ref(),
    )?;
    let certificate = HorizonCertificate::build(
        chart.rd_hash.clone(),
        chart.chart_id.clone(),
        crossing_report_hash,
        combined.projection_v2_certificate_hash,
        finalized_emission_hash,
        replay_hash,
    )?;

    Ok(HorizonRunResult {
        input: input.clone(),
        outcome,
        chart,
        crossing,
        certificate,
    })
}

fn max_fixed<'a, I: IntoIterator<Item = &'a Fixed>>(iter: I) -> Fixed {
    let mut iter = iter.into_iter();
    let mut acc = match iter.next() {
        Some(v) => v.clone(),
        None => return Fixed::zero(),
    };
    for v in iter {
        if v > &acc {
            acc = v.clone();
        }
    }
    acc
}
