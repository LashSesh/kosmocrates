//! `CausalAdmissibilityReport` and the `CausalOrderCheck` operator.
//!
//! The carrier set in the chart implies a declared total order
//! (lex-by-`carrier_id`). The observed order is the order in which the
//! pipeline actually emitted phase rays. `G_causal` succeeds iff
//! observed = declared, with a Fixed-valued admissibility score in
//! `[0, 1]` reflecting the fraction of carriers in the right place.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::horizon_chart::HorizonChart;
use super::phase_ray::PhaseRay;
use super::primitives::{fixed_ge, Fixed, HorizonError, StableId};
use super::run_descriptor::{CausalPolicyV3, HorizonRunDescriptorV3};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalViolation {
    pub violation_id: Hash256,
    pub carrier_id: StableId,
    pub expected_before: StableId,
    pub observed_before: StableId,
    pub severity: Fixed,
}

impl CausalViolation {
    pub fn with_id(mut self) -> Result<Self, HorizonError> {
        let mut probe = self.clone();
        probe.violation_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| HorizonError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.violation_id = Hash256(id);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalAdmissibilityReport {
    pub report_id: Hash256,
    pub chart_id: Hash256,
    pub state_id: Hash256,
    pub declared_order_hash: Hash256,
    pub observed_order_hash: Hash256,
    pub admissibility_score: Fixed,
    pub violations: Vec<CausalViolation>,
    pub passed: bool,
}

impl CausalAdmissibilityReport {
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

/// `G_causal`: declared order respected.
pub fn check_causal_admissibility(
    chart: &HorizonChart,
    rays: &[PhaseRay],
    state_id: &Hash256,
    rd: &HorizonRunDescriptorV3,
) -> Result<CausalAdmissibilityReport, HorizonError> {
    let declared: Vec<StableId> = chart
        .carriers
        .iter()
        .map(|c| c.carrier_id.clone())
        .collect();
    let observed = observed_carrier_order(rays);

    let declared_order_hash =
        Hash256(
            content_address(&declared).map_err(|e| HorizonError::Canonicalization {
                reason: e.to_string(),
            })?,
        );
    let observed_order_hash =
        Hash256(
            content_address(&observed).map_err(|e| HorizonError::Canonicalization {
                reason: e.to_string(),
            })?,
        );

    let mut violations = Vec::new();
    let mut matches = 0i128;
    let total = declared.len() as i128;

    match rd.policies.causal_policy {
        CausalPolicyV3::DeclaredOrderRequired => {
            for (i, expected) in declared.iter().enumerate() {
                let got = observed.get(i);
                if got == Some(expected) {
                    matches += 1;
                } else if let Some(got) = got {
                    let v = CausalViolation {
                        violation_id: Hash256::zero(),
                        carrier_id: got.clone(),
                        expected_before: expected.clone(),
                        observed_before: got.clone(),
                        severity: Fixed::Rational {
                            num: 1,
                            den: total.max(1),
                        },
                    };
                    violations.push(v.with_id()?);
                }
            }
        }
        CausalPolicyV3::AnyTotalOrder => {
            // Any total order over carriers is admissible — the score
            // is 1 iff the observed multiset matches the declared
            // multiset.
            let mut decl_sorted = declared.clone();
            decl_sorted.sort();
            let mut obs_sorted = observed.clone();
            obs_sorted.sort();
            if decl_sorted == obs_sorted {
                matches = total;
            }
        }
    }

    let admissibility_score = if total == 0 {
        Fixed::quantize(1.0, 9).map_err(|e| HorizonError::InvalidPrimitive {
            reason: e.to_string(),
        })?
    } else {
        Fixed::Rational {
            num: matches,
            den: total,
        }
    };

    let passed = violations.is_empty()
        && fixed_ge(
            &admissibility_score,
            &rd.thresholds.min_causal_admissibility,
        );

    let report = CausalAdmissibilityReport {
        report_id: Hash256::zero(),
        chart_id: chart.chart_id.clone(),
        state_id: state_id.clone(),
        declared_order_hash,
        observed_order_hash,
        admissibility_score,
        violations,
        passed,
    };
    report.with_id()
}

fn observed_carrier_order(rays: &[PhaseRay]) -> Vec<StableId> {
    // The observed order is the first-appearance sequence of carrier
    // ids in the ray vector — preserves duplicates only once.
    let mut seen: Vec<StableId> = Vec::new();
    for r in rays {
        if !seen.contains(&r.carrier_id) {
            seen.push(r.carrier_id.clone());
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::horizon_chart::null_center_unfold;
    use crate::horizon::phase_ray::lift_phase_rays;
    use crate::horizon::run_descriptor::{HorizonPoliciesV3, HorizonThresholdsV3};
    use std::collections::BTreeMap;

    fn descriptor() -> HorizonRunDescriptorV3 {
        HorizonRunDescriptorV3 {
            run_id: "horizon.causal".into(),
            projection_v2_report_hash: None,
            problem_spec_hash: Hash256::zero(),
            seed: 1,
            operator_versions: BTreeMap::new(),
            canonicalization_version: "horizon-v0.3".into(),
            thresholds: HorizonThresholdsV3::permissive(),
            policies: HorizonPoliciesV3::default_t4(),
        }
    }

    #[test]
    fn admissibility_passes_for_declared_order() {
        let rd = descriptor();
        let chart = null_center_unfold(&Hash256::zero(), &rd).unwrap();
        let rays = lift_phase_rays(&chart, &rd).unwrap();
        let report = check_causal_admissibility(&chart, &rays, &Hash256::zero(), &rd).unwrap();
        assert!(report.passed);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn admissibility_detects_order_violation() {
        let rd = descriptor();
        let chart = null_center_unfold(&Hash256::zero(), &rd).unwrap();
        let mut rays = lift_phase_rays(&chart, &rd).unwrap();
        // Reverse the ray vector so the observed carrier order is
        // reversed.
        rays.reverse();
        let report = check_causal_admissibility(&chart, &rays, &Hash256::zero(), &rd).unwrap();
        assert!(!report.passed);
        assert!(!report.violations.is_empty());
    }
}
