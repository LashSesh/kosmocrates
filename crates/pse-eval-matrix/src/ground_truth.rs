//! `GroundTruthProfile`, label sources, adjudication and uncertainty
//! policies (§8).

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, EvalError, EvidenceRef, Hash256};

/// Label space: what labels the dataset uses.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LabelSpace {
    /// Two-class binary labels.
    Binary,
    /// Multi-class with a fixed cardinality.
    Multiclass {
        /// Number of classes.
        cardinality: u32,
    },
    /// Structured labels (e.g. event windows, code-test outcomes).
    Structured {
        /// Free-form schema id.
        schema_id: String,
    },
}

/// Where the labels come from (§8.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LabelSource {
    /// `SyntheticExact` — exact known events / puzzles / constraints.
    SyntheticExact,
    /// `SemiSyntheticInjection` — real data with injected known events.
    SemiSyntheticInjection,
    /// `HistoricalOutcome` — real outcomes with timestamp + provenance.
    HistoricalOutcome,
    /// `UnitTestOracle` — software tasks with a test suite as oracle.
    UnitTestOracle,
    /// `HumanAdjudicated` — double-annotation with conflict policy.
    HumanAdjudicated,
}

/// How to resolve disagreements / conflicts in labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AdjudicationPolicy {
    /// No adjudication needed (single source of truth).
    None,
    /// Require unanimous agreement among annotators.
    Unanimous,
    /// Majority vote; ties resolve to "uncertain".
    MajorityVote,
    /// Senior reviewer breaks ties.
    SeniorReviewer,
}

/// How to handle uncertain / missing labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UncertaintyPolicy {
    /// Mark as a separate `MissingLabel` category (default per §7.2.6).
    SeparateCategory,
    /// Drop from the evaluation (must be reported).
    DropAndReport,
    /// Treat as the negative class.
    AssumeNegative,
}

/// `GroundTruthProfile` (§8.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundTruthProfile {
    /// Content-addressed profile id.
    pub profile_id: Hash256,
    /// Label space.
    pub label_space: LabelSpace,
    /// Label source.
    pub label_source: LabelSource,
    /// Adjudication policy.
    pub adjudication_policy: AdjudicationPolicy,
    /// Uncertainty policy.
    pub uncertainty_policy: UncertaintyPolicy,
    /// Evidence references (e.g. annotator pool, oracle tests).
    pub evidence_refs: Vec<EvidenceRef>,
}

impl GroundTruthProfile {
    /// Build a synthetic-exact profile (the most common test setup).
    pub fn synthetic_exact() -> Self {
        let p = GroundTruthProfile {
            profile_id: Hash256::zero(),
            label_space: LabelSpace::Binary,
            label_source: LabelSource::SyntheticExact,
            adjudication_policy: AdjudicationPolicy::None,
            uncertainty_policy: UncertaintyPolicy::SeparateCategory,
            evidence_refs: Vec::new(),
        };
        p.clone().with_id().unwrap_or(p)
    }

    /// Build a unit-test-oracle profile (for code-agent tasks).
    pub fn unit_test_oracle() -> Self {
        let p = GroundTruthProfile {
            profile_id: Hash256::zero(),
            label_space: LabelSpace::Structured {
                schema_id: "unit_test".into(),
            },
            label_source: LabelSource::UnitTestOracle,
            adjudication_policy: AdjudicationPolicy::None,
            uncertainty_policy: UncertaintyPolicy::SeparateCategory,
            evidence_refs: Vec::new(),
        };
        p.clone().with_id().unwrap_or(p)
    }

    /// Recompute `profile_id` after any field change.
    pub fn with_id(mut self) -> Result<Self, EvalError> {
        self.evidence_refs.sort();
        let mut probe = self.clone();
        probe.profile_id = Hash256::zero();
        self.profile_id = content_address(&probe)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_exact_is_content_addressed() {
        let a = GroundTruthProfile::synthetic_exact();
        let b = a.clone().with_id().unwrap();
        assert_eq!(a.profile_id, b.profile_id);
    }

    #[test]
    fn id_changes_with_uncertainty_policy() {
        let a = GroundTruthProfile::synthetic_exact();
        let mut b = a.clone();
        b.uncertainty_policy = UncertaintyPolicy::DropAndReport;
        let b = b.with_id().unwrap();
        assert_ne!(a.profile_id, b.profile_id);
    }
}
