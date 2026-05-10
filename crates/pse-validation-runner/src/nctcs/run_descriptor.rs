//! NctcsRunDescriptor — the top-level input contract for a NCTCS closure run.

use serde::{Deserialize, Serialize};

use super::primitives::{
    content_address, EvidenceRef, Hash256, NctcsError, NctcsPolicies, NctcsThresholds,
    OperatorVersions, StableId,
};

/// Canonical input descriptor for one NCTCS closure run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NctcsRunDescriptor {
    pub run_id: StableId,
    /// Must be "PSE-NCTCS-CONFORMANCE-01".
    pub closure_profile: String,
    pub canonicalization_version: String,
    pub source_validation_bundle_hash: Hash256,
    pub source_eval_summary_hash: Option<Hash256>,
    pub source_stitch_bundle_hash: Option<Hash256>,
    pub source_tpt_bundle_hash: Option<Hash256>,
    pub thresholds: NctcsThresholds,
    pub policies: NctcsPolicies,
    pub operator_versions: OperatorVersions,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl NctcsRunDescriptor {
    /// Build a default descriptor pointing at a validation bundle.
    pub fn new(run_id: impl Into<String>, source_validation_bundle_hash: Hash256) -> Self {
        NctcsRunDescriptor {
            run_id: run_id.into(),
            closure_profile: "PSE-NCTCS-CONFORMANCE-01".into(),
            canonicalization_version: "v0.1".into(),
            source_validation_bundle_hash,
            source_eval_summary_hash: None,
            source_stitch_bundle_hash: None,
            source_tpt_bundle_hash: None,
            thresholds: NctcsThresholds::default(),
            policies: NctcsPolicies::default(),
            operator_versions: OperatorVersions::new(),
            evidence_refs: vec![],
        }
    }

    pub fn content_hash(&self) -> Result<Hash256, NctcsError> {
        content_address(self)
    }
}
