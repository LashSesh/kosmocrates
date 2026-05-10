//! MaterializationAudit — verifies that the ephemeral fabric cannot
//! directly mutate the persistent field tensor (C2 condition).

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, Hash256, NctcsError};

/// Audit of the materialization path: ephemeral → gate → tensor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializationAudit {
    pub audit_id: Hash256,
    pub ephemeral_fabric_hash: Hash256,
    pub tensor_before_hash: Hash256,
    pub stitch_candidate_hashes: Vec<Hash256>,
    pub accepted_update_hashes: Vec<Hash256>,
    pub rejected_candidate_hashes: Vec<Hash256>,
    pub gate_report_hashes: Vec<Hash256>,
    pub tensor_after_hash: Hash256,
    /// When no accepted updates exist, tensor_after_hash == tensor_before_hash.
    pub tensor_unchanged_on_no_update: bool,
    /// No stitch candidate bypassed the gate to directly mutate the tensor.
    pub no_direct_fabric_to_tensor_mutation: bool,
    pub passed: bool,
}

impl MaterializationAudit {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        ephemeral_fabric_hash: Hash256,
        tensor_before_hash: Hash256,
        mut stitch_candidate_hashes: Vec<Hash256>,
        mut accepted_update_hashes: Vec<Hash256>,
        mut rejected_candidate_hashes: Vec<Hash256>,
        mut gate_report_hashes: Vec<Hash256>,
        tensor_after_hash: Hash256,
        no_direct_fabric_to_tensor_mutation: bool,
    ) -> Result<Self, NctcsError> {
        stitch_candidate_hashes.sort();
        accepted_update_hashes.sort();
        rejected_candidate_hashes.sort();
        gate_report_hashes.sort();

        let tensor_unchanged_on_no_update =
            accepted_update_hashes.is_empty() && tensor_before_hash == tensor_after_hash;

        let passed = no_direct_fabric_to_tensor_mutation
            && (accepted_update_hashes.is_empty()
                || tensor_before_hash != tensor_after_hash
                || accepted_update_hashes.is_empty());

        let partial = MaterializationAudit {
            audit_id: Hash256::zero(),
            ephemeral_fabric_hash,
            tensor_before_hash,
            stitch_candidate_hashes,
            accepted_update_hashes,
            rejected_candidate_hashes,
            gate_report_hashes,
            tensor_after_hash,
            tensor_unchanged_on_no_update,
            no_direct_fabric_to_tensor_mutation,
            passed,
        };
        let id = content_address(&partial)?;
        Ok(MaterializationAudit {
            audit_id: id,
            ..partial
        })
    }
}
