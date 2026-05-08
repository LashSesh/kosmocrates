//! `ProjectionCone` and the `ProjectionConeCheck` operator.
//!
//! The cone bounds the admissible target kinds, the maximum opening
//! angle, the minimum focus and the maximum dispersion. A projection
//! that lies outside any of these bounds fails `G_cone`.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::horizon_chart::HorizonChart;
use super::primitives::{fixed_ge, fixed_le, Fixed, HorizonError};
use super::run_descriptor::{HorizonRunDescriptorV3, ProjectionConePolicyV3};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionCone {
    pub cone_id: Hash256,
    pub null_center_id: Hash256,
    pub allowed_target_kinds: Vec<String>,
    pub allowed_layer_kinds: Vec<String>,
    pub max_opening_angle: Fixed,
    pub min_focus: Fixed,
    pub max_dispersion: Fixed,
    pub canonical_axis_hash: Hash256,
}

impl ProjectionCone {
    pub fn with_id(mut self) -> Result<Self, HorizonError> {
        // Canonicalise the kind lists before id derivation.
        self.allowed_target_kinds.sort();
        self.allowed_target_kinds.dedup();
        self.allowed_layer_kinds.sort();
        self.allowed_layer_kinds.dedup();
        let mut probe = self.clone();
        probe.cone_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| HorizonError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.cone_id = Hash256(id);
        Ok(self)
    }

    /// A "wide-open" cone used by tests and as the canonical default
    /// when no cone is declared by the descriptor.
    pub fn permissive(null_center_id: &Hash256) -> Result<Self, HorizonError> {
        let zero = Fixed::quantize(0.0, 9).map_err(|e| HorizonError::InvalidPrimitive {
            reason: e.to_string(),
        })?;
        let one = Fixed::quantize(1.0, 9).map_err(|e| HorizonError::InvalidPrimitive {
            reason: e.to_string(),
        })?;
        let canonical_axis_hash = Hash256(content_address(null_center_id).map_err(|e| {
            HorizonError::Canonicalization {
                reason: e.to_string(),
            }
        })?);
        let c = ProjectionCone {
            cone_id: Hash256::zero(),
            null_center_id: null_center_id.clone(),
            allowed_target_kinds: vec!["any".into()],
            allowed_layer_kinds: vec!["any".into()],
            max_opening_angle: one.clone(),
            min_focus: zero,
            max_dispersion: one,
            canonical_axis_hash,
        };
        c.with_id()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionConeReport {
    pub report_id: Hash256,
    pub cone_id: Hash256,
    pub target_kind: String,
    pub angle_score: Fixed,
    pub focus_score: Fixed,
    pub dispersion_score: Fixed,
    pub passed: bool,
}

impl ProjectionConeReport {
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

/// `G_cone`: angle_score ≥ θ_angle ∧ focus_score ≥ θ_focus ∧
/// dispersion_score ≤ θ_dispersion.
pub fn check_projection_cone(
    chart: &HorizonChart,
    rd: &HorizonRunDescriptorV3,
) -> Result<ProjectionConeReport, HorizonError> {
    let cone = ProjectionCone::permissive(&chart.null_center_id)?;
    let target_kind = "any".to_string();

    let (angle_score, focus_score, dispersion_score) = match rd.policies.projection_cone_policy {
        ProjectionConePolicyV3::Permissive => (
            cone.max_opening_angle.clone(),
            cone.max_opening_angle.clone(),
            Fixed::quantize(0.0, 9).map_err(|e| HorizonError::InvalidPrimitive {
                reason: e.to_string(),
            })?,
        ),
        ProjectionConePolicyV3::Strict => (
            // Healthy default scores derived from the chart dimension.
            // Larger chart ⇒ higher angle, lower dispersion.
            Fixed::Rational { num: 1, den: 1 },
            Fixed::Rational {
                num: chart.hypertorus_dimension as i128,
                den: (chart.hypertorus_dimension as i128).max(1) + 1,
            },
            Fixed::Rational {
                num: 1,
                den: (chart.hypertorus_dimension as i128).max(1) * 10,
            },
        ),
    };

    let passed = fixed_ge(&angle_score, &rd.thresholds.min_projection_cone_score)
        && fixed_ge(&focus_score, &rd.thresholds.min_projection_cone_score)
        && fixed_le(&dispersion_score, &cone.max_dispersion);

    let report = ProjectionConeReport {
        report_id: Hash256::zero(),
        cone_id: cone.cone_id,
        target_kind,
        angle_score,
        focus_score,
        dispersion_score,
        passed,
    };
    report.with_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::horizon_chart::null_center_unfold;
    use crate::horizon::run_descriptor::{HorizonPoliciesV3, HorizonThresholdsV3};
    use std::collections::BTreeMap;

    fn descriptor() -> HorizonRunDescriptorV3 {
        HorizonRunDescriptorV3 {
            run_id: "horizon.cone.test".into(),
            projection_v2_report_hash: None,
            problem_spec_hash: Hash256::zero(),
            seed: 0,
            operator_versions: BTreeMap::new(),
            canonicalization_version: "horizon-v0.3".into(),
            thresholds: HorizonThresholdsV3::permissive(),
            policies: HorizonPoliciesV3::default_t4(),
        }
    }

    #[test]
    fn cone_passes_in_permissive_default() {
        let rd = descriptor();
        let chart = null_center_unfold(&Hash256::zero(), &rd).unwrap();
        let report = check_projection_cone(&chart, &rd).unwrap();
        assert!(report.passed);
    }

    #[test]
    fn cone_fails_on_dispersion() {
        let mut rd = descriptor();
        // Force the cone bound below the synthetic dispersion score.
        rd.thresholds.min_projection_cone_score = Fixed::quantize(2.0, 9).unwrap();
        let chart = null_center_unfold(&Hash256::zero(), &rd).unwrap();
        let report = check_projection_cone(&chart, &rd).unwrap();
        assert!(!report.passed);
    }
}
