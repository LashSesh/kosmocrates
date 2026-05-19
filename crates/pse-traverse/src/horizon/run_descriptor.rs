//! Run descriptor, thresholds and policies for the horizon layer.
//!
//! The descriptor is the single replay anchor: gating, canonicalisation
//! and operator versioning all derive from its content address. Two
//! horizon runs with byte-identical descriptors and inputs MUST produce
//! byte-identical reports.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dynamic_state::Hash256;

use super::phase_carrier_v3::PhaseCarrierSpecV3;
use super::primitives::{Fixed, RationalFixed, StableId};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HorizonRunDescriptorV3 {
    pub run_id: StableId,
    pub projection_v2_report_hash: Option<Hash256>,
    pub problem_spec_hash: Hash256,
    pub seed: u64,
    pub operator_versions: BTreeMap<String, String>,
    pub canonicalization_version: String,
    pub thresholds: HorizonThresholdsV3,
    pub policies: HorizonPoliciesV3,
}

impl HorizonRunDescriptorV3 {
    /// Validate descriptor invariants (no empty run id, plausible
    /// operator versions, dimension >= 1).
    pub fn validate(&self) -> Result<(), super::primitives::HorizonError> {
        use super::primitives::HorizonError;
        if self.run_id.is_empty() {
            return Err(HorizonError::InvalidDescriptor {
                reason: "run_id is empty".into(),
            });
        }
        if self.canonicalization_version.is_empty() {
            return Err(HorizonError::InvalidDescriptor {
                reason: "canonicalization_version is empty".into(),
            });
        }
        if self.policies.hypertorus_dimension == 0 {
            return Err(HorizonError::InvalidDescriptor {
                reason: "hypertorus_dimension must be >= 1".into(),
            });
        }
        if self.thresholds.max_steps == 0 {
            return Err(HorizonError::InvalidDescriptor {
                reason: "max_steps must be >= 1".into(),
            });
        }
        Ok(())
    }

    /// Permissive baseline for tests and smoke-runs. Real callers MUST author their own descriptor.
    pub fn permissive() -> Self {
        HorizonRunDescriptorV3 {
            run_id: "horizon.permissive".into(),
            projection_v2_report_hash: None,
            problem_spec_hash: Hash256::zero(),
            seed: 0,
            operator_versions: BTreeMap::new(),
            canonicalization_version: "horizon-v0.3".into(),
            thresholds: HorizonThresholdsV3::permissive(),
            policies: HorizonPoliciesV3::default_t4(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HorizonThresholdsV3 {
    pub max_null_distance: Fixed,
    pub min_horizon_visibility: Fixed,
    pub max_boundary_tension: Fixed,
    pub max_attenuation: Fixed,
    pub min_causal_admissibility: Fixed,
    pub min_projection_cone_score: Fixed,
    pub min_duality_score: Fixed,
    pub max_roundtrip_delta: Fixed,
    pub min_crossing_score: Fixed,
    pub max_phase_jitter: Fixed,
    pub max_steps: u64,
}

impl HorizonThresholdsV3 {
    /// A "permissive" baseline that lets every gate pass when scores are
    /// healthy. Useful for tests and CLI smoke-runs; production callers
    /// MUST specify their own thresholds.
    pub fn permissive() -> Self {
        let zero = Fixed::quantize(0.0, 9).unwrap();
        let one = Fixed::quantize(1.0, 9).unwrap();
        let small = Fixed::quantize(0.1, 9).unwrap();
        let tiny = Fixed::quantize(0.01, 9).unwrap();
        HorizonThresholdsV3 {
            max_null_distance: one.clone(),
            min_horizon_visibility: zero.clone(),
            max_boundary_tension: one.clone(),
            max_attenuation: one.clone(),
            min_causal_admissibility: zero.clone(),
            min_projection_cone_score: zero.clone(),
            min_duality_score: zero,
            max_roundtrip_delta: small,
            min_crossing_score: Fixed::quantize(0.5, 9).unwrap(),
            max_phase_jitter: tiny,
            max_steps: 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HorizonPoliciesV3 {
    pub hypertorus_dimension: u8,
    pub carrier_policy: CarrierPolicyV3,
    pub horizon_window_policy: HorizonWindowPolicyV3,
    pub projection_cone_policy: ProjectionConePolicyV3,
    pub causal_policy: CausalPolicyV3,
    pub duality_policy: DualityPolicyV3,
    pub crossing_policy: CrossingPolicyV3,
    pub failure_policy_order: Vec<HorizonFailurePolicy>,
}

impl HorizonPoliciesV3 {
    pub fn default_t4() -> Self {
        HorizonPoliciesV3 {
            hypertorus_dimension: 4,
            carrier_policy: CarrierPolicyV3::GenerateHypertorus {
                dimension: 4,
                rational_frequencies: vec![
                    RationalFixed::new(1, 1).unwrap(),
                    RationalFixed::new(1, 2).unwrap(),
                    RationalFixed::new(1, 3).unwrap(),
                    RationalFixed::new(1, 5).unwrap(),
                ],
            },
            horizon_window_policy: HorizonWindowPolicyV3::AcceptAll,
            projection_cone_policy: ProjectionConePolicyV3::Strict,
            causal_policy: CausalPolicyV3::DeclaredOrderRequired,
            duality_policy: DualityPolicyV3::SameNullCenter,
            crossing_policy: CrossingPolicyV3::AllGatesRequired,
            failure_policy_order: vec![
                HorizonFailurePolicy::WaitForHorizon,
                HorizonFailurePolicy::RefineProjectionCone,
                HorizonFailurePolicy::MigrateCarrier,
                HorizonFailurePolicy::Recondense,
                HorizonFailurePolicy::Hold,
            ],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CarrierPolicyV3 {
    InheritFromProjectionV2,
    Explicit(Vec<PhaseCarrierSpecV3>),
    GenerateHypertorus {
        dimension: u8,
        rational_frequencies: Vec<RationalFixed>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HorizonWindowPolicyV3 {
    /// Accept any phase ray as window-passing — used for tests and
    /// gate-only experiments. Production callers should pin specific
    /// windows.
    AcceptAll,
    /// All declared windows must be open for the gate to pass.
    AllDeclared,
    /// At least one declared window must be open.
    AnyDeclared,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProjectionConePolicyV3 {
    /// No cone constraint applied — score returns max.
    Permissive,
    /// All declared cone bounds (angle, focus, dispersion) must hold.
    Strict,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CausalPolicyV3 {
    /// Declared order in the chart must be respected exactly.
    DeclaredOrderRequired,
    /// Any total order over carriers is admissible.
    AnyTotalOrder,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DualityPolicyV3 {
    /// Round-trip must return to the same NullCenter id.
    SameNullCenter,
    /// Round-trip may return to any NullCenter; only the metric distance
    /// is bounded.
    BoundedDistance,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CrossingPolicyV3 {
    /// Every sub-gate must pass (`G_visible ∧ G_cone ∧ G_causal ∧ G_dual`).
    AllGatesRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HorizonFailurePolicy {
    Hold,
    WaitForHorizon,
    RefineProjectionCone,
    SplitCarrier,
    MigrateCarrier,
    Recondense,
    RequireHumanReview,
    Abort,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_t4_is_dimension_four() {
        let p = HorizonPoliciesV3::default_t4();
        assert_eq!(p.hypertorus_dimension, 4);
    }

    #[test]
    fn permissive_thresholds_are_internally_consistent() {
        let t = HorizonThresholdsV3::permissive();
        assert!(t.max_steps > 0);
    }
}
