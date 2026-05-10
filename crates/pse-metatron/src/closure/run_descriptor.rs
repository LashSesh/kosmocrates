//! MetatronRunDescriptor — top-level input contract for a closure run.

use serde::{Deserialize, Serialize};

use super::primitives::{
    content_address, EvidenceRef, Hash256, MetatronError, MetatronPolicies, MetatronThresholds,
    OperatorVersions, StableId,
};

/// Canonical input descriptor for one Metatron closure run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetatronRunDescriptor {
    pub run_id: StableId,
    pub parent_validation_run_id: Hash256,
    pub nctcs_conformance_report_id: Hash256,
    pub canonicalization_version: String,
    pub operator_versions: OperatorVersions,
    pub thresholds: MetatronThresholds,
    pub policies: MetatronPolicies,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl MetatronRunDescriptor {
    pub fn new(
        run_id: impl Into<String>,
        parent_validation_run_id: Hash256,
        nctcs_conformance_report_id: Hash256,
    ) -> Self {
        MetatronRunDescriptor {
            run_id: run_id.into(),
            parent_validation_run_id,
            nctcs_conformance_report_id,
            canonicalization_version: "v0.1".into(),
            operator_versions: OperatorVersions::new(),
            thresholds: MetatronThresholds::default(),
            policies: MetatronPolicies::default(),
            evidence_refs: vec![],
        }
    }

    pub fn content_hash(&self) -> Result<Hash256, MetatronError> {
        content_address(self)
    }
}
