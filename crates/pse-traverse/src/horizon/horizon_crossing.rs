//! `HorizonCrossingReport` and the `HorizonCrossingGate`.
//!
//! This is the conjunction of every per-axis sub-gate in §7. The
//! `crossing_score` is the deterministic min over the per-sub-gate
//! scores so a single failing sub-gate forces a fail-closed verdict.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::causal_admissibility::CausalAdmissibilityReport;
use super::collapse_emission_duality::DualityReport;
use super::horizon_window::HorizonWindowReportV3;
use super::primitives::{fixed_ge, fixed_le, Fixed, HorizonError, HorizonEvidenceRef};
use super::projection_cone::ProjectionConeReport;
use super::run_descriptor::{HorizonFailurePolicy, HorizonRunDescriptorV3};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorizonCrossingReport {
    pub report_id: Hash256,
    pub chart_id: Hash256,
    pub state_id: Hash256,
    pub horizon_window_reports: Vec<HorizonWindowReportV3>,
    pub cone_report: ProjectionConeReport,
    pub causal_report: CausalAdmissibilityReport,
    pub duality_report: DualityReport,
    pub crossing_score: Fixed,
    pub crossing_allowed: bool,
    pub failure_policy: HorizonFailurePolicy,
    pub max_attenuation_observed: Fixed,
    pub max_boundary_tension_observed: Fixed,
    pub evidence_refs: Vec<HorizonEvidenceRef>,
}

impl HorizonCrossingReport {
    pub fn with_id(mut self) -> Result<Self, HorizonError> {
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| HorizonError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.report_id = Hash256(id);
        Ok(self)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn horizon_crossing_gate(
    chart_id: &Hash256,
    state_id: &Hash256,
    window_reports: Vec<HorizonWindowReportV3>,
    cone_report: ProjectionConeReport,
    causal_report: CausalAdmissibilityReport,
    duality_report: DualityReport,
    visibility_passed: bool,
    max_attenuation: Fixed,
    max_boundary_tension: Fixed,
    rd: &HorizonRunDescriptorV3,
) -> Result<HorizonCrossingReport, HorizonError> {
    let g_visible = visibility_passed;
    let g_cone = cone_report.passed;
    let g_causal = causal_report.passed;
    let g_dual = duality_report.passed;
    let tension_ok = fixed_le(&max_boundary_tension, &rd.thresholds.max_boundary_tension);
    let attenuation_ok = fixed_le(&max_attenuation, &rd.thresholds.max_attenuation);

    let crossing_allowed =
        g_visible && g_cone && g_causal && g_dual && tension_ok && attenuation_ok;

    // Crossing score is the min of cone/focus/duality/causal scores —
    // saturated to one if every sub-gate is happy. We use the
    // canonical Ord-based `min` so it stays float-free.
    let crossing_score = min_fixed(&[
        cone_report.angle_score.clone(),
        cone_report.focus_score.clone(),
        causal_report.admissibility_score.clone(),
        duality_report.duality_score.clone(),
    ]);

    let crossing_allowed =
        crossing_allowed && fixed_ge(&crossing_score, &rd.thresholds.min_crossing_score);

    let failure_policy = select_failure_policy(
        g_visible,
        g_cone,
        g_causal,
        g_dual,
        tension_ok,
        attenuation_ok,
        rd,
    );

    let report = HorizonCrossingReport {
        report_id: Hash256::zero(),
        chart_id: chart_id.clone(),
        state_id: state_id.clone(),
        horizon_window_reports: window_reports,
        cone_report,
        causal_report,
        duality_report,
        crossing_score,
        crossing_allowed,
        failure_policy,
        max_attenuation_observed: max_attenuation,
        max_boundary_tension_observed: max_boundary_tension,
        evidence_refs: Vec::new(),
    };
    report.with_id()
}

fn min_fixed(values: &[Fixed]) -> Fixed {
    let mut iter = values.iter();
    let first = match iter.next() {
        Some(v) => v.clone(),
        None => return Fixed::zero(),
    };
    iter.fold(first, |acc, v| if v < &acc { v.clone() } else { acc })
}

/// Deterministic FailurePolicy selection in spec-table order:
/// window-only fail → WaitForHorizon; cone fail → RefineProjectionCone;
/// causal fail → MigrateCarrier; tension/attenuation fail → Recondense;
/// duality fail → Recondense; otherwise Hold.
fn select_failure_policy(
    g_visible: bool,
    g_cone: bool,
    g_causal: bool,
    g_dual: bool,
    tension_ok: bool,
    attenuation_ok: bool,
    rd: &HorizonRunDescriptorV3,
) -> HorizonFailurePolicy {
    if g_visible && g_cone && g_causal && g_dual && tension_ok && attenuation_ok {
        // No failure path — return the first declared policy as the
        // "default policy" placeholder. The crossing_allowed flag is
        // the source of truth.
        return rd
            .policies
            .failure_policy_order
            .first()
            .copied()
            .unwrap_or(HorizonFailurePolicy::Hold);
    }
    if !g_visible && g_cone && g_causal && g_dual && tension_ok && attenuation_ok {
        return HorizonFailurePolicy::WaitForHorizon;
    }
    if !g_cone {
        return HorizonFailurePolicy::RefineProjectionCone;
    }
    if !g_causal {
        return HorizonFailurePolicy::MigrateCarrier;
    }
    if !tension_ok || !attenuation_ok {
        return HorizonFailurePolicy::Recondense;
    }
    if !g_dual {
        return HorizonFailurePolicy::Recondense;
    }
    HorizonFailurePolicy::Hold
}
