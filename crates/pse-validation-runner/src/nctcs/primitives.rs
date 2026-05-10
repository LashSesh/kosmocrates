//! Shared primitives for the NCTCS closure layer.
//!
//! Re-exports Hash256 and CanonicalNumber from pse-traverse so the entire
//! workspace shares one numeric and addressing domain.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use pse_traverse::dynamic_state::{CanonicalNumber, Hash256};

/// Every gate/metric/score scalar is a CanonicalNumber (Fixed).
pub type Fixed = CanonicalNumber;

/// Stable run identifier — human-readable, not part of any audit hash.
pub type StableId = String;

/// Content-addressed evidence reference.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub kind: String,
    pub address: Hash256,
    pub relation: String,
}

/// Minimum coverage / identity thresholds for the NCTCS pipeline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NctcsThresholds {
    pub min_visibility_coverage: Fixed,
    pub min_gate_coverage: Fixed,
    pub min_trace_coverage: Fixed,
    pub min_replay_identity: Fixed,
    pub max_unmapped_artifact_rate: Fixed,
    pub max_coherence_without_truth_rate: Fixed,
}

impl Default for NctcsThresholds {
    fn default() -> Self {
        use pse_traverse::dynamic_state::CanonicalNumber;
        let q = |n: i64| CanonicalNumber::FixedI64 { raw: n, scale: 2 };
        NctcsThresholds {
            min_visibility_coverage: q(60),
            min_gate_coverage: q(80),
            min_trace_coverage: q(80),
            min_replay_identity: q(100),
            max_unmapped_artifact_rate: q(5),
            max_coherence_without_truth_rate: q(0),
        }
    }
}

/// Policy flags that govern closure behaviour.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NctcsPolicies {
    pub require_real_domain_for_complete_validation: bool,
    pub require_tensor_history_for_c4: bool,
    pub require_eval_matrix_for_macro_state: bool,
    pub require_sorted_artifacts_before_hash: bool,
    pub fail_on_unmapped_persistent_artifact: bool,
}

impl Default for NctcsPolicies {
    fn default() -> Self {
        NctcsPolicies {
            require_real_domain_for_complete_validation: true,
            require_tensor_history_for_c4: true,
            require_eval_matrix_for_macro_state: true,
            require_sorted_artifacts_before_hash: true,
            fail_on_unmapped_persistent_artifact: false,
        }
    }
}

/// Total error surface for the NCTCS layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NctcsError {
    Canonicalization { reason: String },
    MissingArtifact { artifact: String },
    InvariantViolation { rule: String },
    ReplayMismatch { reason: String },
    PolicyViolation { reason: String },
}

impl std::fmt::Display for NctcsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NctcsError::Canonicalization { reason } => write!(f, "canonicalization: {reason}"),
            NctcsError::MissingArtifact { artifact } => write!(f, "missing artifact: {artifact}"),
            NctcsError::InvariantViolation { rule } => write!(f, "invariant violation: {rule}"),
            NctcsError::ReplayMismatch { reason } => write!(f, "replay mismatch: {reason}"),
            NctcsError::PolicyViolation { reason } => write!(f, "policy violation: {reason}"),
        }
    }
}

impl std::error::Error for NctcsError {}

/// JCS + SHA-256 content addressing.
pub fn content_address<T: Serialize>(value: &T) -> Result<Hash256, NctcsError> {
    pse_traverse::canonical::content_address(value)
        .map(Hash256)
        .map_err(|e| NctcsError::Canonicalization {
            reason: e.to_string(),
        })
}

/// Operator versions map type alias.
pub type OperatorVersions = BTreeMap<String, String>;
