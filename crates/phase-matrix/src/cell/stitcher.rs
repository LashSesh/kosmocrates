//! `StitchRunDescriptor`, `StitchThresholds`, `StitchPolicies` and
//! `StitcherOutcome` (PHASEMATRIX-HIVEMIND-03.1 §4, §11).
//!
//! The run descriptor is the single replay anchor for a stitch cycle:
//! same descriptor + same inputs → byte-identical `StitcherReport`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};
use super::run_descriptor::PhaseMatrixRunDescriptorV3;

/// Threshold bundle for the Stitch layer (§4.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StitchThresholds {
    /// Minimum convergence score required by `G_conv`.
    pub min_convergence_score: Fixed,
    /// Minimum mirror-consistency index required by `G_mci`.
    pub min_mirror_consistency_index: Fixed,
    /// Maximum tensor-delta norm allowed by `G_delta`.
    pub max_tensor_delta_norm: Fixed,
    /// Maximum per-edge delta allowed.
    pub max_coupling_delta_per_edge: Fixed,
    /// Maximum number of `CouplingUpdate`s per cycle (`G_budget`).
    pub max_updates_per_cycle: u64,
    /// Minimum trace-completeness score (0…1).
    pub min_trace_completeness: Fixed,
}

impl StitchThresholds {
    /// Permissive defaults used by fixtures and tests.
    pub fn permissive() -> Self {
        let zero = Fixed::Rational { num: 0, den: 1 };
        // Use a very large cap so that any realistic candidate set passes
        // without triggering the delta gate.  Tests that want to exercise
        // gate failures should use `strict()` or set the threshold manually.
        let large = Fixed::Rational { num: 1000, den: 1 };
        StitchThresholds {
            min_convergence_score: zero.clone(),
            min_mirror_consistency_index: zero.clone(),
            max_tensor_delta_norm: large.clone(),
            max_coupling_delta_per_edge: large,
            max_updates_per_cycle: 64,
            min_trace_completeness: zero,
        }
    }

    /// Strict defaults: require non-trivial coherence and bounded delta.
    pub fn strict() -> Self {
        StitchThresholds {
            min_convergence_score: Fixed::Rational { num: 1, den: 4 },
            min_mirror_consistency_index: Fixed::Rational { num: 1, den: 4 },
            max_tensor_delta_norm: Fixed::Rational { num: 1, den: 2 },
            max_coupling_delta_per_edge: Fixed::Rational { num: 1, den: 4 },
            max_updates_per_cycle: 16,
            min_trace_completeness: Fixed::Rational { num: 1, den: 2 },
        }
    }
}

/// Policy bundle for the Stitch layer (§4.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StitchPolicies {
    /// If `true`, the boundary report must pass for the gate to open.
    pub require_boundary_pass: bool,
    /// If `true`, the trace must be ready for the gate to open.
    pub require_trace_ready: bool,
    /// If `true`, evidence refs must be non-empty for a persistent
    /// coupling.
    pub require_evidence_refs: bool,
    /// If `true`, a cycle that produces zero accepted candidates must
    /// return `StitcherOutcome::Hold`.
    pub reject_empty_update_set: bool,
    /// If `true`, `CouplingUpdate`s are sorted by id before hashing the
    /// `StitcherReport` (Invariant 9 / SHOULD in spec).
    pub sort_updates_before_hash: bool,
}

impl StitchPolicies {
    /// Permissive defaults.
    pub fn permissive() -> Self {
        StitchPolicies {
            require_boundary_pass: false,
            require_trace_ready: false,
            require_evidence_refs: false,
            reject_empty_update_set: false,
            sort_updates_before_hash: true,
        }
    }

    /// Strict defaults aligned with the normative spec.
    pub fn strict() -> Self {
        StitchPolicies {
            require_boundary_pass: true,
            require_trace_ready: true,
            require_evidence_refs: true,
            reject_empty_update_set: false,
            sort_updates_before_hash: true,
        }
    }
}

/// Run descriptor for one stitch cycle (§4.1).
///
/// Replay anchor: same descriptor + same inputs → byte-identical report.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StitchRunDescriptor {
    /// Stable run identifier.
    pub run_id: String,
    /// Content address of the parent `PhaseMatrixRunDescriptorV3`.
    pub parent_phase_matrix_run: Hash256,
    /// Optional content address of the `CellSubstrateCycleReport` used
    /// as input to this stitch cycle.
    pub cell_substrate_report_hash: Option<Hash256>,
    /// Operator version tags (sorted by key for canonical form).
    pub operator_versions: BTreeMap<String, String>,
    /// Threshold bundle.
    pub thresholds: StitchThresholds,
    /// Policy bundle.
    pub policies: StitchPolicies,
    /// Canonicalization version stamp.
    pub canonicalization_version: String,
}

impl StitchRunDescriptor {
    /// Validate invariants.
    pub fn validate(&self) -> Result<(), PhaseError> {
        if self.run_id.is_empty() {
            return Err(PhaseError::InvalidDescriptor {
                reason: "stitch run_id is empty".into(),
            });
        }
        if self.canonicalization_version.is_empty() {
            return Err(PhaseError::InvalidDescriptor {
                reason: "canonicalization_version is empty".into(),
            });
        }
        if self.thresholds.max_updates_per_cycle == 0 {
            return Err(PhaseError::InvalidDescriptor {
                reason: "max_updates_per_cycle must be >= 1".into(),
            });
        }
        Ok(())
    }

    /// Build a default `StitchRunDescriptor` from a `PhaseMatrixRunDescriptorV3`.
    pub fn default_for(pm_rd: &PhaseMatrixRunDescriptorV3) -> Result<Self, PhaseError> {
        let parent_hash = content_address(pm_rd)?;
        let mut ops = BTreeMap::new();
        ops.insert(
            "dual_fabric_stitch".into(),
            "phasematrix-hivemind-03.1".into(),
        );
        Ok(StitchRunDescriptor {
            run_id: format!("{}.stitch", pm_rd.run_id),
            parent_phase_matrix_run: parent_hash,
            cell_substrate_report_hash: None,
            operator_versions: ops,
            thresholds: StitchThresholds::permissive(),
            policies: StitchPolicies::strict(),
            canonicalization_version: "phase-matrix-v0.3.1".into(),
        })
    }
}

/// Outcome of one stitch cycle (§11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum StitcherOutcome {
    /// Cycle completed; `FieldTensorState` may have been updated.
    Completed(super::report::StitcherReport),
    /// Gate passed but no candidates were accepted; tensor unchanged.
    Hold(super::report::StitcherReport),
    /// Gate failed; tensor unchanged.
    Rejected(super::report::StitcherReport),
}

impl StitcherOutcome {
    /// Return the embedded report regardless of outcome variant.
    pub fn report(&self) -> &super::report::StitcherReport {
        match self {
            StitcherOutcome::Completed(r)
            | StitcherOutcome::Hold(r)
            | StitcherOutcome::Rejected(r) => r,
        }
    }

    /// Content address of the embedded report.
    pub fn report_hash(&self) -> Hash256 {
        self.report().report_id.clone()
    }

    /// True iff tensor was updated (only `Completed` with ≥1 accepted update).
    pub fn tensor_updated(&self) -> bool {
        matches!(self, StitcherOutcome::Completed(r) if !r.accepted_updates.is_empty())
    }
}

/// Evidence reference helper: build a reference to a `StitchRunDescriptor`.
pub fn evidence_ref_for_rd(rd: &StitchRunDescriptor) -> Result<EvidenceRef, PhaseError> {
    Ok(EvidenceRef {
        kind: "stitch.run_descriptor".into(),
        address: content_address(rd)?,
        relation: "governed_by".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::run_descriptor::{CellSubstratePolicies, CellSubstrateThresholds};
    use std::collections::BTreeSet;

    fn pm_rd() -> PhaseMatrixRunDescriptorV3 {
        PhaseMatrixRunDescriptorV3 {
            run_id: "pm.test".into(),
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
    fn stitch_rd_validates_when_well_formed() {
        let rd = StitchRunDescriptor::default_for(&pm_rd()).unwrap();
        assert!(rd.validate().is_ok());
    }

    #[test]
    fn empty_run_id_fails_validation() {
        let mut rd = StitchRunDescriptor::default_for(&pm_rd()).unwrap();
        rd.run_id.clear();
        assert!(rd.validate().is_err());
    }

    #[test]
    fn zero_max_updates_fails_validation() {
        let mut rd = StitchRunDescriptor::default_for(&pm_rd()).unwrap();
        rd.thresholds.max_updates_per_cycle = 0;
        assert!(rd.validate().is_err());
    }
}
