//! `PhaseMatrixRunDescriptorV3` and the threshold/policy bundles
//! (§7).
//!
//! The descriptor is the single replay anchor: same descriptor + same
//! input MUST produce byte-identical reports.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::cluster_gate::{
    ClusterFormationPolicy, ClusterTracePolicy, DissolutionPolicy, IntentPolicy, MorphologyPolicy,
};
use super::matrix::MatrixGatePolicy;
use super::primitives::{EvidenceRef, Fixed, Hash256, PhaseError, StableId};

/// Threshold bundle (§7).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CellSubstrateThresholds {
    /// Minimum cell-level coherence (`D(t) · L_T(t)` floor).
    pub min_cell_coherence: Fixed,
    /// Minimum cluster-level coherence.
    pub min_cluster_coherence: Fixed,
    /// Minimum phase alignment for cluster formation.
    pub min_phase_alignment: Fixed,
    /// Minimum morphodynamic potential `H` for morphology events.
    pub min_morphodynamic_potential: Fixed,
    /// Maximum admissible cluster conflict score.
    pub max_cluster_conflict: Fixed,
    /// Minimum convergence-field score for intent admission.
    pub min_convergence_score: Fixed,
    /// Minimum field tension for intent admission.
    pub min_intent_tension: Fixed,
    /// Maximum logical ticks a working state may live before
    /// dissolution must run.
    pub max_working_state_retention_ticks: u64,
}

impl CellSubstrateThresholds {
    /// Permissive defaults — used by the synthetic fixtures.
    pub fn permissive() -> Self {
        let zero = Fixed::Rational { num: 0, den: 1 };
        let small = Fixed::Rational { num: 1, den: 100 };
        let _ = small.clone();
        CellSubstrateThresholds {
            min_cell_coherence: zero.clone(),
            min_cluster_coherence: zero.clone(),
            min_phase_alignment: zero.clone(),
            min_morphodynamic_potential: zero.clone(),
            max_cluster_conflict: Fixed::Rational { num: 1, den: 1 },
            min_convergence_score: zero.clone(),
            min_intent_tension: zero,
            max_working_state_retention_ticks: 64,
        }
    }
}

/// Policy bundle (§7).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CellSubstratePolicies {
    /// Cluster-formation policy.
    pub formation_policy: ClusterFormationPolicy,
    /// Morphology policy.
    pub morphology_policy: MorphologyPolicy,
    /// Dissolution policy.
    pub dissolution_policy: DissolutionPolicy,
    /// Cluster-trace policy.
    pub trace_policy: ClusterTracePolicy,
    /// Intent policy.
    pub intent_policy: IntentPolicy,
    /// Matrix-boundary gate policy.
    pub matrix_gate_policy: MatrixGatePolicy,
}

impl CellSubstratePolicies {
    /// Strict defaults.
    pub fn strict() -> Self {
        CellSubstratePolicies {
            formation_policy: ClusterFormationPolicy::Strict,
            morphology_policy: MorphologyPolicy::Strict,
            dissolution_policy: DissolutionPolicy::CompactPreserveTrace,
            trace_policy: ClusterTracePolicy::AllPhases,
            intent_policy: IntentPolicy::Conservative,
            matrix_gate_policy: MatrixGatePolicy::strict(),
        }
    }
}

/// `PhaseMatrixRunDescriptorV3` (§7).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PhaseMatrixRunDescriptorV3 {
    /// Stable run id.
    pub run_id: StableId,
    /// Anchoring matrix.
    pub matrix_id: Hash256,
    /// Optional parent profile hash for lineage.
    pub parent_profile_hash: Option<Hash256>,
    /// Operator versions (sorted by key).
    pub operator_versions: BTreeMap<String, String>,
    /// Canonicalisation version stamp.
    pub canonicalization_version: String,
    /// Threshold bundle.
    pub thresholds: CellSubstrateThresholds,
    /// Policy bundle.
    pub policies: CellSubstratePolicies,
    /// Active feature flags.
    pub feature_flags: BTreeSet<String>,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl PhaseMatrixRunDescriptorV3 {
    /// Validate descriptor invariants.
    pub fn validate(&self) -> Result<(), PhaseError> {
        if self.run_id.is_empty() {
            return Err(PhaseError::InvalidDescriptor {
                reason: "run_id is empty".into(),
            });
        }
        if self.canonicalization_version.is_empty() {
            return Err(PhaseError::InvalidDescriptor {
                reason: "canonicalization_version is empty".into(),
            });
        }
        if self.thresholds.max_working_state_retention_ticks == 0 {
            return Err(PhaseError::InvalidDescriptor {
                reason: "max_working_state_retention_ticks must be >= 1".into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rd() -> PhaseMatrixRunDescriptorV3 {
        PhaseMatrixRunDescriptorV3 {
            run_id: "phase.test".into(),
            matrix_id: Hash256::zero(),
            parent_profile_hash: None,
            operator_versions: BTreeMap::new(),
            canonicalization_version: "phase-matrix-v0.3".into(),
            thresholds: CellSubstrateThresholds::permissive(),
            policies: CellSubstratePolicies::strict(),
            feature_flags: BTreeSet::new(),
            evidence_refs: Vec::new(),
        }
    }

    #[test]
    fn descriptor_validates_when_well_formed() {
        assert!(rd().validate().is_ok());
    }

    #[test]
    fn empty_run_id_fails_validation() {
        let mut d = rd();
        d.run_id.clear();
        assert!(d.validate().is_err());
    }
}
