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

// ─── BenchGtRecord ───────────────────────────────────────────────────────────

/// Metrics record mirroring `BenchGtJsonOutput::metrics_per_source` values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BenchGtMetrics {
    pub tp: u64,
    pub fp: u64,
    pub fn_: u64,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub auprc: Option<f64>,
}

/// Deserialized form of a `bench_gt --format json` output file.
///
/// Field names and JSON keys match `BenchGtJsonOutput` in `pse-bench-gt` so
/// that `serde_json::from_str` deserializes directly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BenchGtRecord {
    pub scenario: String,
    pub n_observations: u64,
    pub tolerance_ticks: u64,
    pub pse_metrics: Option<BenchGtMetrics>,
    pub stl_zscore_metrics: Option<BenchGtMetrics>,
    pub isoforest_metrics: Option<BenchGtMetrics>,
    pub aggregate_metrics: BenchGtMetrics,
    pub config_hash: String,
    pub data_hash: String,
}

// ─── BaselineComparisonReport ─────────────────────────────────────────────────

/// Per-scenario comparison of PSE vs baselines.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScenarioComparison {
    pub scenario: String,
    pub pse_f1: f64,
    pub stl_zscore_f1: f64,
    pub isoforest_f1: f64,
    /// PSE F1 > best baseline F1.
    pub pse_wins: bool,
}

/// Aggregate baseline comparison across all domain scenarios.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaselineComparisonReport {
    pub scenarios: Vec<ScenarioComparison>,
    /// PSE wins on ≥ 50 % of scenarios.
    pub pse_majority_wins: bool,
}

impl BaselineComparisonReport {
    pub fn build(scenarios: Vec<ScenarioComparison>) -> Self {
        let wins = scenarios.iter().filter(|s| s.pse_wins).count();
        let pse_majority_wins = !scenarios.is_empty() && wins * 2 >= scenarios.len();
        BaselineComparisonReport {
            scenarios,
            pse_majority_wins,
        }
    }
}

// ─── DomainValidationSummary ──────────────────────────────────────────────────

/// Summary of a domain validation run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomainValidationSummary {
    pub domain: String,
    pub manifest_hash: Hash256,
    pub calibration_completed: bool,
    pub validation_completed: bool,
    pub test_completed: bool,
    pub leakage_check_passed: bool,
    /// Per-scenario metrics from the real bench_gt runs.
    pub scenario_metrics: Vec<BenchGtRecord>,
    /// Aggregate P/R/F1 across the test scenario (binance).
    pub test_f1: Option<f64>,
    pub test_precision: Option<f64>,
    pub test_recall: Option<f64>,
    /// Comparison report (PSE vs STL-zscore vs IsoForest).
    pub baseline_comparison: Option<BaselineComparisonReport>,
}

impl DomainValidationSummary {
    /// Build a summary from loaded `BenchGtRecord`s.
    pub fn build_from_records(
        domain: String,
        manifest_hash: Hash256,
        records: Vec<BenchGtRecord>,
        manifest: &DatasetManifest,
    ) -> Result<Self, ValidationError> {
        let leakage_check_passed = manifest.verify_splits().is_ok();

        let calibration_completed = records.iter().any(|r| r.scenario.contains("seismo"));
        let validation_completed = records.iter().any(|r| r.scenario.contains("vitals"));
        let test_completed = records.iter().any(|r| r.scenario.contains("binance"));

        // Test F1 from the binance (test-split) scenario.
        let test_record = records.iter().find(|r| r.scenario.contains("binance"));
        let test_f1 = test_record
            .and_then(|r| r.pse_metrics.as_ref())
            .map(|m| m.f1);
        let test_precision = test_record
            .and_then(|r| r.pse_metrics.as_ref())
            .map(|m| m.precision);
        let test_recall = test_record
            .and_then(|r| r.pse_metrics.as_ref())
            .map(|m| m.recall);

        // Build baseline comparison.
        let scenario_comparisons: Vec<ScenarioComparison> = records
            .iter()
            .map(|r| {
                let pse_f1 = r.pse_metrics.as_ref().map(|m| m.f1).unwrap_or(0.0);
                let stl_f1 = r.stl_zscore_metrics.as_ref().map(|m| m.f1).unwrap_or(0.0);
                let iso_f1 = r.isoforest_metrics.as_ref().map(|m| m.f1).unwrap_or(0.0);
                let best_baseline = stl_f1.max(iso_f1);
                ScenarioComparison {
                    scenario: r.scenario.clone(),
                    pse_f1,
                    stl_zscore_f1: stl_f1,
                    isoforest_f1: iso_f1,
                    pse_wins: pse_f1 > best_baseline,
                }
            })
            .collect();
        let baseline_comparison = if scenario_comparisons.is_empty() {
            None
        } else {
            Some(BaselineComparisonReport::build(scenario_comparisons))
        };

        Ok(DomainValidationSummary {
            domain,
            manifest_hash,
            calibration_completed,
            validation_completed,
            test_completed,
            leakage_check_passed,
            scenario_metrics: records,
            test_f1,
            test_precision,
            test_recall,
            baseline_comparison,
        })
    }
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

    fn make_record(scenario: &str, pse_f1: f64, stl_f1: f64, iso_f1: f64) -> BenchGtRecord {
        let mk = |f1: f64| BenchGtMetrics {
            tp: 1,
            fp: 0,
            fn_: 0,
            precision: f1,
            recall: f1,
            f1,
            auprc: None,
        };
        BenchGtRecord {
            scenario: scenario.into(),
            n_observations: 100,
            tolerance_ticks: 5,
            pse_metrics: Some(mk(pse_f1)),
            stl_zscore_metrics: Some(mk(stl_f1)),
            isoforest_metrics: Some(mk(iso_f1)),
            aggregate_metrics: mk(pse_f1),
            config_hash: "aa".into(),
            data_hash: "bb".into(),
        }
    }

    #[test]
    fn domain_summary_detects_test_completed() {
        let manifest = make_manifest(0xaa, 0xcc);
        let records = vec![
            make_record("seismo_mainshock", 0.8, 0.5, 0.4),
            make_record("vitals_afib", 0.7, 0.6, 0.5),
            make_record("binance_regime", 0.9, 0.7, 0.6),
        ];
        let summary = DomainValidationSummary::build_from_records(
            "test".into(),
            Hash256::zero(),
            records,
            &manifest,
        )
        .unwrap();
        assert!(summary.calibration_completed);
        assert!(summary.validation_completed);
        assert!(summary.test_completed);
        assert!(summary.leakage_check_passed);
        assert!(summary.test_f1.is_some());
    }

    #[test]
    fn baseline_comparison_pse_wins_majority() {
        let manifest = make_manifest(0xaa, 0xcc);
        let records = vec![
            make_record("seismo_mainshock", 0.9, 0.5, 0.4),
            make_record("vitals_afib", 0.8, 0.6, 0.5),
            make_record("binance_regime", 0.3, 0.7, 0.6), // PSE loses here
        ];
        let summary = DomainValidationSummary::build_from_records(
            "test".into(),
            Hash256::zero(),
            records,
            &manifest,
        )
        .unwrap();
        let bc = summary.baseline_comparison.unwrap();
        // 2 of 3 wins → majority
        assert!(bc.pse_majority_wins);
    }

    #[test]
    fn no_domain_summary_without_records() {
        let manifest = make_manifest(0xaa, 0xcc);
        let summary = DomainValidationSummary::build_from_records(
            "test".into(),
            Hash256::zero(),
            vec![],
            &manifest,
        )
        .unwrap();
        assert!(!summary.calibration_completed);
        assert!(!summary.test_completed);
        assert!(summary.baseline_comparison.is_none());
    }
}
