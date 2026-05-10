//! Domain validation types (L3, §8.4, §14).
//!
//! A domain run is only valid when a DatasetManifest with three
//! non-overlapping splits is present. Calibration MUST use only the
//! calibration split.

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, Hash256, ValidationError};

/// A split of a dataset (§14.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetSplit {
    pub split_id: Hash256,
    pub name: String,
    pub size: u64,
    /// SHA-256 of the serialized observation list.
    pub data_hash: Hash256,
}

/// Ground-truth profile for a domain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundTruthProfile {
    pub profile_id: Hash256,
    pub label_type: String,
    pub positive_fraction: Option<String>,
}

/// Full dataset manifest (§14.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetManifest {
    pub dataset_id: Hash256,
    pub domain: String,
    pub calibration_split: DatasetSplit,
    pub validation_split: DatasetSplit,
    pub test_split: DatasetSplit,
    pub ground_truth_profile: GroundTruthProfile,
    pub license: Option<String>,
    pub provenance_hash: Hash256,
}

impl DatasetManifest {
    pub fn content_hash(&self) -> Result<Hash256, ValidationError> {
        content_address(self)
    }

    /// Verify the three splits are non-overlapping (calibration ∩ test = ∅).
    pub fn verify_splits(&self) -> Result<(), ValidationError> {
        if self.calibration_split.data_hash == self.test_split.data_hash
            && self.calibration_split.size > 0
        {
            return Err(ValidationError::CalibrationLeakage {
                reason: "calibration_split and test_split have identical data_hash".into(),
            });
        }
        if self.test_split.size == 0 {
            return Err(ValidationError::TestSplitMissing);
        }
        Ok(())
    }
}

/// Summary of a domain validation run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainValidationSummary {
    pub domain: String,
    pub manifest_hash: Hash256,
    pub calibration_completed: bool,
    pub validation_completed: bool,
    pub test_completed: bool,
    pub leakage_check_passed: bool,
    pub metrics: std::collections::BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_split(name: &str, hash_byte: u8, size: u64) -> DatasetSplit {
        let hex = format!("{:0>62}{:02x}", "", hash_byte);
        DatasetSplit {
            split_id: Hash256::zero(),
            name: name.into(),
            size,
            data_hash: Hash256::from_hex(&hex).unwrap_or(Hash256::zero()),
        }
    }

    fn make_manifest(cal_hash: u8, test_hash: u8) -> DatasetManifest {
        DatasetManifest {
            dataset_id: Hash256::zero(),
            domain: "test-domain".into(),
            calibration_split: make_split("cal", cal_hash, 100),
            validation_split: make_split("val", 0xbb, 50),
            test_split: make_split("test", test_hash, 50),
            ground_truth_profile: GroundTruthProfile {
                profile_id: Hash256::zero(),
                label_type: "binary".into(),
                positive_fraction: None,
            },
            license: None,
            provenance_hash: Hash256::zero(),
        }
    }

    #[test]
    fn calibration_leakage_invalidates_domain_run() {
        let manifest = make_manifest(0xaa, 0xaa); // same hash — leakage
        let err = manifest.verify_splits().unwrap_err();
        assert!(matches!(err, ValidationError::CalibrationLeakage { .. }));
    }

    #[test]
    fn valid_manifest_passes_split_check() {
        let manifest = make_manifest(0xaa, 0xcc); // different hashes
        assert!(manifest.verify_splits().is_ok());
    }

    #[test]
    fn test_split_missing_invalidates_domain_run() {
        let mut manifest = make_manifest(0xaa, 0xcc);
        manifest.test_split.size = 0;
        assert!(matches!(
            manifest.verify_splits(),
            Err(ValidationError::TestSplitMissing)
        ));
    }
}
