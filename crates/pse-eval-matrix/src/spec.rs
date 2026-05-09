//! `EvaluationSpec` and the surrounding policy types (§10.1).
//!
//! The spec is the single source of truth: which question is being
//! asked, which variants and workloads are admissible, which metrics
//! count, how runs are aggregated. Every downstream artefact references
//! the `spec_id` (its content address).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::datasets::DatasetManifest;
use crate::metrics::MetricSpec;
use crate::primitives::{content_address, EvalError, Hash256};
use crate::variants::SystemVariantSpec;
use crate::workloads::WorkloadSpec;

/// Why this evaluation exists. Determines which conclusion shapes are
/// admissible (per §1.2 / §6.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EvaluationPurpose {
    /// Compare against classical detectors / naive agents.
    BaselineComparison,
    /// Isolate the contribution of each post-symbolic layer.
    AblationStudy,
    /// Calibrate gates / thresholds against a labelled split.
    Calibration,
    /// Verify byte-identical replay.
    ReplayAudit,
    /// Quantify robustness under stress / drift / perturbation.
    StressRobustness,
    /// Track drift / memory utility over long runs.
    LongRunStability,
    /// Test transfer between domains.
    CrossDomainTransfer,
    /// Capture human review, manual repair, explainability quality.
    HumanReviewStudy,
    /// Mixed / preset-driven evaluation.
    Composite,
}

/// Which experiment kinds the spec authors expect to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ExperimentKind {
    /// `BaselineComparison` per §7.1.
    BaselineComparison,
    /// `AblationLadder` per §7.1.
    AblationLadder,
    /// `CalibrationCurve` per §7.1.
    CalibrationCurve,
    /// `ReplayAudit` per §7.1.
    ReplayAudit,
    /// `StressPerturbation` per §7.1.
    StressPerturbation,
    /// `LongRunStability` per §7.1.
    LongRunStability,
    /// `CrossDomainTransfer` per §7.1.
    CrossDomainTransfer,
    /// `HumanReviewStudy` per §7.1.
    HumanReviewStudy,
}

/// Pre-declared experiment design (§10.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentDesign {
    /// Mandatory experiment kinds for this spec.
    pub experiments: Vec<ExperimentKind>,
    /// Whether multiple comparisons must be flagged or corrected
    /// (§7.2 rule 4).
    pub multiple_comparison_correction: bool,
    /// Whether paired comparisons are mandatory when variants share
    /// inputs (§7.2 rule 2).
    pub paired_when_shared_input: bool,
}

impl ExperimentDesign {
    /// Default design used by the presets.
    pub fn default_design() -> Self {
        ExperimentDesign {
            experiments: vec![ExperimentKind::AblationLadder, ExperimentKind::ReplayAudit],
            multiple_comparison_correction: true,
            paired_when_shared_input: true,
        }
    }
}

/// Statistical aggregation policy (§7.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatisticalPlan {
    /// Bootstrap iteration count for confidence intervals (0 = exact /
    /// no bootstrap).
    pub bootstrap_iterations: u32,
    /// Confidence level numerator (e.g. 95 for 95 %).
    pub confidence_percent: u32,
    /// Whether confidence intervals are required for primary metrics.
    pub require_ci_for_primary: bool,
    /// RNG seed for the bootstrap (audit path is deterministic).
    pub bootstrap_seed: u64,
}

impl StatisticalPlan {
    /// Default plan: 1000 bootstrap iterations, 95 % CI, required for
    /// primary metrics.
    pub fn default_plan() -> Self {
        StatisticalPlan {
            bootstrap_iterations: 1000,
            confidence_percent: 95,
            require_ci_for_primary: true,
            bootstrap_seed: 0xE0A1_B2C3_D4E5_F607,
        }
    }
}

/// Determinism policy (§17).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterminismPolicy {
    /// `BTreeMap` is mandatory for keyed collections (always true; this
    /// flag exists so a future spec version can relax for diagnostic
    /// outputs only).
    pub btreemap_required: bool,
    /// Audit path forbids platform floats (always true).
    pub forbid_platform_floats: bool,
    /// Audit path forbids wall-clock timestamps (always true).
    pub forbid_wall_clock: bool,
    /// `ReplayMismatch` invalidates the run.
    pub invalidate_on_replay_mismatch: bool,
}

impl Default for DeterminismPolicy {
    fn default() -> Self {
        DeterminismPolicy {
            btreemap_required: true,
            forbid_platform_floats: true,
            forbid_wall_clock: true,
            invalidate_on_replay_mismatch: true,
        }
    }
}

/// Output artefact policy (§17.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputPolicy {
    /// Which artefact families MUST be produced.
    pub required_artefacts: Vec<RequiredArtefact>,
    /// Whether human-readable markdown summaries are emitted.
    pub markdown_summary: bool,
}

/// Mandatory output artefacts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RequiredArtefact {
    /// `EvaluationSpec` itself.
    EvaluationSpec,
    /// `EvaluationPlan`.
    EvaluationPlan,
    /// One `DatasetManifest` per workload.
    DatasetManifest,
    /// `GroundTruthProfile` per workload that has labels.
    GroundTruthProfile,
    /// `EvaluationRunLedger`.
    EvaluationRunLedger,
    /// One `TrialReport` per run.
    TrialReport,
    /// Final `EvaluationSummaryReport`.
    EvaluationSummaryReport,
    /// `ReplayVerificationReport`.
    ReplayVerificationReport,
    /// `AblationReport` if ablations are declared.
    AblationReport,
}

impl OutputPolicy {
    /// Default policy: every artefact required, markdown summary on.
    pub fn default_policy() -> Self {
        OutputPolicy {
            required_artefacts: vec![
                RequiredArtefact::EvaluationSpec,
                RequiredArtefact::EvaluationPlan,
                RequiredArtefact::DatasetManifest,
                RequiredArtefact::GroundTruthProfile,
                RequiredArtefact::EvaluationRunLedger,
                RequiredArtefact::TrialReport,
                RequiredArtefact::EvaluationSummaryReport,
                RequiredArtefact::ReplayVerificationReport,
                RequiredArtefact::AblationReport,
            ],
            markdown_summary: true,
        }
    }
}

/// `EvaluationSpec` — the canonical, content-addressed declaration of
/// what is being evaluated and how (§10.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationSpec {
    /// Content-addressed id (filled by [`EvaluationSpec::with_id`]).
    pub spec_id: Hash256,
    /// Schema version stamp.
    pub schema_version: String,
    /// Evaluation purpose.
    pub purpose: EvaluationPurpose,
    /// System variants (variant ladder).
    pub variants: Vec<SystemVariantSpec>,
    /// Workload families.
    pub workloads: Vec<WorkloadSpec>,
    /// Datasets / domains.
    pub datasets: Vec<DatasetManifest>,
    /// Metric declarations (primary, secondary, diagnostic).
    pub metrics: Vec<MetricSpec>,
    /// Experiment design.
    pub experiment_design: ExperimentDesign,
    /// Statistical plan.
    pub statistical_plan: StatisticalPlan,
    /// Determinism policy.
    pub determinism_policy: DeterminismPolicy,
    /// Output policy.
    pub output_policy: OutputPolicy,
    /// Optional auxiliary tags (sortable BTreeMap so canonical bytes
    /// are stable).
    pub tags: BTreeMap<String, String>,
}

impl EvaluationSpec {
    /// Build a content-addressed spec id from the rest of the fields.
    /// Idempotent.
    pub fn with_id(mut self) -> Result<Self, EvalError> {
        // Sort lists whose semantics permit re-ordering before hashing.
        self.variants
            .sort_by(|a, b| a.variant_id.cmp(&b.variant_id));
        self.workloads
            .sort_by(|a, b| a.workload_id.cmp(&b.workload_id));
        self.metrics.sort_by(|a, b| a.metric_id.cmp(&b.metric_id));
        self.datasets
            .sort_by(|a, b| a.dataset_id.cmp(&b.dataset_id));
        let mut probe = self.clone();
        probe.spec_id = Hash256::zero();
        let id = content_address(&probe)?;
        self.spec_id = id;
        Ok(self)
    }

    /// Validate the spec: at least one variant, at least one workload,
    /// at least one primary metric, valid splits.
    pub fn validate(&self) -> Result<(), EvalError> {
        if self.variants.is_empty() {
            return Err(EvalError::InvalidSpec {
                reason: "no system variants declared".into(),
            });
        }
        if self.workloads.is_empty() {
            return Err(EvalError::InvalidSpec {
                reason: "no workloads declared".into(),
            });
        }
        if self.metrics.is_empty() {
            return Err(EvalError::InvalidSpec {
                reason: "no metrics declared".into(),
            });
        }
        if !self.metrics.iter().any(|m| m.primary) {
            return Err(EvalError::InvalidSpec {
                reason: "no primary metric declared".into(),
            });
        }
        if self.schema_version.is_empty() {
            return Err(EvalError::InvalidSpec {
                reason: "schema_version is empty".into(),
            });
        }
        // Variant id collisions.
        let mut seen = std::collections::BTreeSet::new();
        for v in &self.variants {
            if !seen.insert(&v.variant_id) {
                return Err(EvalError::InvalidSpec {
                    reason: format!("duplicate variant_id: {}", v.variant_id),
                });
            }
        }
        // Workload → dataset cross-reference.
        let dataset_ids: std::collections::BTreeSet<&Hash256> =
            self.datasets.iter().map(|d| &d.dataset_id).collect();
        for w in &self.workloads {
            if !dataset_ids.contains(&w.input_manifest_hash) {
                return Err(EvalError::InvalidSpec {
                    reason: format!(
                        "workload {} references unknown dataset {}",
                        w.workload_id,
                        w.input_manifest_hash.hex()
                    ),
                });
            }
            // If a workload claims success, it MUST attach a ground
            // truth profile (per §4.2).
            if !w.success_criteria.is_empty() && w.ground_truth_profile.is_none() {
                return Err(EvalError::MissingLabels {
                    reason: format!(
                        "workload {} declares success criteria but no ground_truth_profile",
                        w.workload_id
                    ),
                });
            }
        }
        // Determinism policy MUST be tight (§17).
        if !self.determinism_policy.invalidate_on_replay_mismatch {
            return Err(EvalError::InvalidSpec {
                reason: "determinism_policy.invalidate_on_replay_mismatch must be true".into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variants::{LayerMask, SystemVariantSpec};

    fn minimal_spec() -> EvaluationSpec {
        let variant = SystemVariantSpec::baseline();
        let dataset = DatasetManifest::synthetic("d.test");
        let workload = WorkloadSpec::stream_event("w.stream", dataset.dataset_id.clone(), None);
        let metric = MetricSpec::task_success();
        EvaluationSpec {
            spec_id: Hash256::zero(),
            schema_version: "eval-matrix-v0.1".into(),
            purpose: EvaluationPurpose::Composite,
            variants: vec![variant],
            workloads: vec![workload],
            datasets: vec![dataset],
            metrics: vec![metric],
            experiment_design: ExperimentDesign::default_design(),
            statistical_plan: StatisticalPlan::default_plan(),
            determinism_policy: DeterminismPolicy::default(),
            output_policy: OutputPolicy::default_policy(),
            tags: BTreeMap::new(),
        }
        .with_id()
        .unwrap()
    }

    #[test]
    fn spec_is_content_addressed() {
        let a = minimal_spec();
        let b = a.clone().with_id().unwrap();
        assert_eq!(a.spec_id, b.spec_id);
    }

    #[test]
    fn validate_requires_primary_metric() {
        let mut s = minimal_spec();
        for m in &mut s.metrics {
            m.primary = false;
        }
        assert!(s.validate().is_err());
    }

    #[test]
    fn variant_layer_mask_round_trip() {
        let v = SystemVariantSpec::baseline();
        assert_eq!(v.layer_mask, LayerMask::B0_BASELINE);
    }
}
