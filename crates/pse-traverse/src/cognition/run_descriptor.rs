//! `CognitionRunDescriptor`, thresholds and policies for the cognition
//! kernel. The descriptor is the single replay anchor: same descriptor,
//! same input ⇒ same report bytes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dynamic_state::Hash256;

use super::primitives::{CognitionError, Fixed, StableId};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CognitionRunDescriptor {
    pub run_id: StableId,
    pub problem_spec_hash: Hash256,
    pub traversal_report_hash: Option<Hash256>,
    pub projection_v2_hash: Option<Hash256>,
    pub seed: u64,
    pub operator_versions: BTreeMap<String, String>,
    pub thresholds: CognitionThresholds,
    pub policies: CognitionPolicies,
    pub canonicalization_version: String,
}

impl CognitionRunDescriptor {
    pub fn validate(&self) -> Result<(), CognitionError> {
        if self.run_id.is_empty() {
            return Err(CognitionError::InvalidDescriptor {
                reason: "run_id is empty".into(),
            });
        }
        if self.canonicalization_version.is_empty() {
            return Err(CognitionError::InvalidDescriptor {
                reason: "canonicalization_version is empty".into(),
            });
        }
        Ok(())
    }
}

/// Per §5.3 — every gate-relevant scalar is a `Fixed`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CognitionThresholds {
    pub min_resonance: Fixed,
    pub max_entropy: Fixed,
    pub min_constraint_mass: Fixed,
    pub min_entropy_reduction: Fixed,
    pub min_feasible_uniqueness: Fixed,
    pub min_panorama_coverage: Fixed,
    pub min_self_model_coherence: Fixed,
    pub max_drift: Fixed,
    pub min_attractor_score: Fixed,
    pub min_trigger_score: Fixed,
}

impl CognitionThresholds {
    /// Permissive baseline used by tests and CLI smoke runs. Real
    /// callers MUST author their own thresholds.
    pub fn permissive() -> Self {
        let zero = Fixed::quantize(0.0, 9).unwrap();
        CognitionThresholds {
            min_resonance: zero.clone(),
            max_entropy: Fixed::quantize(2.0, 9).unwrap(),
            min_constraint_mass: zero.clone(),
            min_entropy_reduction: zero.clone(),
            min_feasible_uniqueness: zero.clone(),
            min_panorama_coverage: zero.clone(),
            min_self_model_coherence: zero.clone(),
            max_drift: Fixed::quantize(2.0, 9).unwrap(),
            min_attractor_score: zero.clone(),
            min_trigger_score: zero,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CognitionPolicies {
    pub spiral_dimension: u8,
    pub hypercube_dimensions: u8,
    pub panorama_arc_count: u16,
    pub wormhole_budget: Fixed,
    pub wormhole_max_ttl: u64,
    pub failure_policy_order: Vec<CognitionFailurePolicy>,
}

impl CognitionPolicies {
    pub fn default_policies() -> Self {
        CognitionPolicies {
            spiral_dimension: 5,
            hypercube_dimensions: 4,
            panorama_arc_count: 8,
            wormhole_budget: Fixed::quantize(1.0, 9).unwrap(),
            wormhole_max_ttl: 16,
            failure_policy_order: vec![
                CognitionFailurePolicy::WaitForPhaseWindow,
                CognitionFailurePolicy::RefineConstraints,
                CognitionFailurePolicy::ExpandPanorama,
                CognitionFailurePolicy::QuerySpiralMemory,
                CognitionFailurePolicy::SplitHypercube,
                CognitionFailurePolicy::AdmitWormhole,
                CognitionFailurePolicy::MigrateCarrier,
                CognitionFailurePolicy::CalibrateOperators,
                CognitionFailurePolicy::Hold,
            ],
        }
    }
}

/// Per §17.2 — recovery actions surfaced when the handoff gate fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CognitionFailurePolicy {
    Hold,
    RefineConstraints,
    ExpandPanorama,
    QuerySpiralMemory,
    SplitHypercube,
    AdmitWormhole,
    WaitForPhaseWindow,
    MigrateCarrier,
    CalibrateOperators,
    RequireHumanReview,
    Abort,
}

/// Identifies which gate failed when the cognition pipeline holds. Used
/// by `CognitionHoldReport` so the next iteration can target the right
/// recovery action deterministically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CognitionGateKind {
    Percolation,
    Resonance,
    Panorama,
    SelfModel,
    Trigger,
    Replay,
    Composite,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissive_thresholds_are_internally_consistent() {
        let t = CognitionThresholds::permissive();
        assert!(t.max_entropy.cmp(&Fixed::zero()) == std::cmp::Ordering::Greater);
    }
}
