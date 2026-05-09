//! `TrialReport`, `EvaluationSummaryReport`, and the supporting
//! observation / diagnostic types (§10.6, §14).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::failure_taxonomy::FailureRecord;
use crate::metrics::MetricObservation;
use crate::primitives::{content_address, EvalError, EvidenceRef, Fixed, Hash256};
use crate::replay::ReplayObservation;

/// One observed gate decision.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GateObservation {
    /// Gate name (e.g. `"horizon_crossing"`, `"cognition_handoff"`).
    pub gate_name: String,
    /// Whether the gate passed.
    pub passed: bool,
    /// Optional reason.
    pub reason: Option<String>,
}

/// Set of gate observations for one trial.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateObservationSet {
    /// Sorted observations.
    pub observations: Vec<GateObservation>,
}

impl GateObservationSet {
    /// Build from a vec; sorts to keep canonical bytes stable.
    pub fn from_vec(mut v: Vec<GateObservation>) -> Self {
        v.sort();
        GateObservationSet { observations: v }
    }
}

/// Outputs the variant produced for the trial. Only their content
/// hashes are stored; the actual artefacts live in upstream layers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialOutputs {
    /// Optional top-level outcome hash (e.g. CognitionReport id).
    pub outcome_hash: Option<Hash256>,
    /// Optional candidate-bundle hash (when the variant produced one).
    pub candidate_bundle_hash: Option<Hash256>,
    /// Optional finalized-emission hash (when projection-v0.2 ran).
    pub finalized_emission_hash: Option<Hash256>,
    /// Number of detected events / candidates / patches.
    pub detected_count: u64,
    /// Number of true-positive detections (against ground truth).
    pub true_positive_count: u64,
    /// Number of false-positive detections.
    pub false_positive_count: u64,
    /// Number of missed ground-truth events.
    pub false_negative_count: u64,
    /// Number of safety events (false commit / false handoff / under-hold).
    pub safety_event_count: u64,
    /// Number of holds the variant emitted that were correct.
    pub correct_hold_count: u64,
    /// Number of holds emitted in total.
    pub hold_count: u64,
    /// Manual interventions applied.
    pub manual_intervention_count: u64,
}

impl TrialOutputs {
    /// Build a zeroed outputs struct.
    pub fn empty() -> Self {
        TrialOutputs {
            outcome_hash: None,
            candidate_bundle_hash: None,
            finalized_emission_hash: None,
            detected_count: 0,
            true_positive_count: 0,
            false_positive_count: 0,
            false_negative_count: 0,
            safety_event_count: 0,
            correct_hold_count: 0,
            hold_count: 0,
            manual_intervention_count: 0,
        }
    }
}

/// Diagnostic record (free-form, sortable). Used for warnings,
/// calibration notes, etc. — never for primary metric data.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DiagnosticRecord {
    /// Tag (e.g. `"calibration_drift"`, `"split_leakage_warning"`).
    pub kind: String,
    /// Free-form message.
    pub message: String,
}

/// `TrialReport` (§10.6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialReport {
    /// Content-addressed trial id.
    pub trial_id: Hash256,
    /// Variant id.
    pub variant_id: String,
    /// Workload id.
    pub workload_id: String,
    /// Dataset id.
    pub dataset_id: Hash256,
    /// Run descriptor content hash.
    pub run_descriptor_hash: Hash256,
    /// Outputs.
    pub outputs: TrialOutputs,
    /// Metric observations.
    pub metrics: Vec<MetricObservation>,
    /// Gate observations.
    pub gates: GateObservationSet,
    /// Replay observation.
    pub replay: ReplayObservation,
    /// Diagnostics.
    pub diagnostics: Vec<DiagnosticRecord>,
    /// Failure records (categorical, taxonomy-backed).
    pub failures: Vec<FailureRecord>,
}

impl TrialReport {
    /// Recompute the content-addressed `trial_id`.
    pub fn with_id(mut self) -> Result<Self, EvalError> {
        self.metrics.sort_by(|a, b| a.metric_id.cmp(&b.metric_id));
        self.diagnostics.sort();
        self.failures.sort();
        let mut probe = self.clone();
        probe.trial_id = Hash256::zero();
        self.trial_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Per-variant summary used in the evaluation summary report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantSummary {
    /// Variant id.
    pub variant_id: String,
    /// Number of completed trials.
    pub completed_trials: u64,
    /// Number of invalid trials.
    pub invalid_trials: u64,
    /// Aggregated metric values keyed by `metric_id`.
    pub metric_values: BTreeMap<String, Fixed>,
    /// Total safety events across completed trials.
    pub safety_event_count: u64,
}

/// Per-workload summary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadSummary {
    /// Workload id.
    pub workload_id: String,
    /// Trials per variant.
    pub trial_counts: BTreeMap<String, u64>,
    /// Per-variant primary-metric mean.
    pub primary_metric_means: BTreeMap<String, Fixed>,
}

/// Statistical summary attached to one comparison.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatisticalSummary {
    /// Comparison label (e.g. `"B6_Cognition vs B5_Horizon | task_success"`).
    pub label: String,
    /// Confidence interval lower bound.
    pub ci_low: Fixed,
    /// Confidence interval upper bound.
    pub ci_high: Fixed,
    /// Confidence percent (e.g. 95).
    pub confidence_percent: u32,
    /// Sample size (number of paired observations).
    pub n: u64,
    /// Whether the CI excludes zero.
    pub significant: bool,
}

/// Conclusion flag — high-level call attached to the summary report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ConclusionFlag {
    /// `ΔU_task > 0 ∧ ΔU_safety ≥ 0 ∧ ReplayIdentity = 1` per §23.
    EmpiricalImprovement,
    /// One or more conditions of §23 not met → diagnostic finding.
    DiagnosticFinding,
    /// Replay invalidations exceeded the tolerance.
    InvalidatedByReplay,
    /// Calibration leakage detected.
    InvalidatedByLeakage,
}

/// `EvaluationSummaryReport` (§14.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationSummaryReport {
    /// Content-addressed report id.
    pub report_id: Hash256,
    /// Anchoring evaluation spec hash.
    pub evaluation_spec_hash: Hash256,
    /// Source ledger hash.
    pub generated_from_ledger: Hash256,
    /// Per-variant summaries.
    pub variant_summaries: Vec<VariantSummary>,
    /// Per-workload summaries.
    pub workload_summaries: Vec<WorkloadSummary>,
    /// Ablation summaries.
    pub ablation_summaries: Vec<crate::ablation::AblationSummary>,
    /// Capability profiles (one per variant).
    pub capability_profiles: Vec<crate::scoring::CapabilityProfile>,
    /// Statistical summaries.
    pub statistical_summaries: Vec<StatisticalSummary>,
    /// Trial ids of invalid runs (excluded from scoring).
    pub invalid_runs: Vec<Hash256>,
    /// High-level conclusion flags.
    pub conclusion_flags: Vec<ConclusionFlag>,
}

impl EvaluationSummaryReport {
    /// Recompute `report_id`.
    pub fn with_id(mut self) -> Result<Self, EvalError> {
        self.variant_summaries
            .sort_by(|a, b| a.variant_id.cmp(&b.variant_id));
        self.workload_summaries
            .sort_by(|a, b| a.workload_id.cmp(&b.workload_id));
        self.ablation_summaries.sort_by(|a, b| {
            (&a.base_variant, &a.ablated_variant).cmp(&(&b.base_variant, &b.ablated_variant))
        });
        self.capability_profiles
            .sort_by(|a, b| a.variant_id.cmp(&b.variant_id));
        self.statistical_summaries
            .sort_by(|a, b| a.label.cmp(&b.label));
        self.invalid_runs.sort();
        self.conclusion_flags.sort();
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        self.report_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Rubric dimension per §19.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RubricDimension {
    /// Were structural primitives extracted correctly?
    StructuralFidelity,
    /// Was domain semantic correctly removed?
    NeutralizationQuality,
    /// Can a coding agent work directly from this?
    ImplementationUsefulness,
    /// Are types, gates, reports, tests, DoD present?
    Completeness,
    /// Were duplications avoided?
    NonRedundancy,
    /// Are failure cases machine-readable / actionable?
    FailureClarity,
}

/// `ReviewerReport` (§19.3) — qualitative evaluation record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewerReport {
    /// Hash of the reviewer identity (no PII).
    pub reviewer_id_hash: Hash256,
    /// Hash of the artefact under review.
    pub artifact_hash: Hash256,
    /// Per-dimension scores in `0..=5`.
    pub rubric_scores: BTreeMap<String, u8>,
    /// Blocking issues (if any).
    pub blocking_issues: Vec<String>,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl ReviewerReport {
    /// Validate the score range `0..=5` for every dimension.
    pub fn validate(&self) -> Result<(), EvalError> {
        for (dim, score) in &self.rubric_scores {
            if *score > 5 {
                return Err(EvalError::Schema {
                    reason: format!("rubric score for {dim} out of range (0..=5)"),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trial_report_id_is_stable() {
        let mut r = TrialReport {
            trial_id: Hash256::zero(),
            variant_id: "B0".into(),
            workload_id: "w.0".into(),
            dataset_id: Hash256::zero(),
            run_descriptor_hash: Hash256::zero(),
            outputs: TrialOutputs::empty(),
            metrics: vec![],
            gates: GateObservationSet::from_vec(vec![]),
            replay: ReplayObservation {
                expected_hash: Hash256::zero(),
                observed_hash: Hash256::zero(),
                passed: true,
            },
            diagnostics: vec![],
            failures: vec![],
        };
        r = r.with_id().unwrap();
        let again = r.clone().with_id().unwrap();
        assert_eq!(r.trial_id, again.trial_id);
    }

    #[test]
    fn reviewer_report_rejects_score_above_five() {
        let mut scores = BTreeMap::new();
        scores.insert("Completeness".into(), 9u8);
        let r = ReviewerReport {
            reviewer_id_hash: Hash256::zero(),
            artifact_hash: Hash256::zero(),
            rubric_scores: scores,
            blocking_issues: vec![],
            evidence_refs: vec![],
        };
        assert!(r.validate().is_err());
    }
}
