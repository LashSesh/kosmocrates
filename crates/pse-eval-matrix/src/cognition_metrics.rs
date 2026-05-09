//! Cognition-specific metrics (§11).
//!
//! Provides the `MetricSpec` set the spec mandates for cognition
//! evaluations. The actual values are produced by trial executors and
//! recorded as `MetricObservation`s; this module only declares the
//! schema.

use serde::{Deserialize, Serialize};

use crate::metrics::{
    AggregationPolicy, InvalidationRule, MetricDirection, MetricFamily, MetricSpec,
};

/// Cognition metric kinds (§11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CognitionMetricKind {
    /// `panorama_coverage` — current + counterfactual horizon coverage.
    PanoramaCoverage,
    /// `current_horizon_coverage`.
    CurrentHorizonCoverage,
    /// `counterfactual_coverage`.
    CounterfactualCoverage,
    /// `blocked_path_correctness`.
    BlockedPathCorrectness,
    /// `attractor_ranking_accuracy`.
    AttractorRankingAccuracy,
    /// `panorama_compression_ratio`.
    PanoramaCompressionRatio,
    /// `hit_precision`.
    HitPrecision,
    /// `hit_recall`.
    HitRecall,
    /// `reuse_gain`.
    ReuseGain,
    /// `false_resonance_rate`.
    FalseResonanceRate,
    /// `percolation_accuracy`.
    PercolationAccuracy,
    /// `entropy_collapse_predictiveness`.
    EntropyCollapsePredictiveness,
    /// `hidden_single_utility`.
    HiddenSingleUtility,
    /// `boundary_contract_violation_rate`.
    BoundaryContractViolationRate,
    /// `self_drift_bound_adherence`.
    SelfDriftBoundAdherence,
    /// `dual_trigger_precision`.
    DualTriggerPrecision,
    /// `calibration_acceptance_quality`.
    CalibrationAcceptanceQuality,
    /// `carrier_migration_benefit`.
    CarrierMigrationBenefit,
}

impl CognitionMetricKind {
    /// Stable metric id.
    pub fn metric_id(self) -> &'static str {
        match self {
            CognitionMetricKind::PanoramaCoverage => "panorama_coverage",
            CognitionMetricKind::CurrentHorizonCoverage => "current_horizon_coverage",
            CognitionMetricKind::CounterfactualCoverage => "counterfactual_coverage",
            CognitionMetricKind::BlockedPathCorrectness => "blocked_path_correctness",
            CognitionMetricKind::AttractorRankingAccuracy => "attractor_ranking_accuracy",
            CognitionMetricKind::PanoramaCompressionRatio => "panorama_compression_ratio",
            CognitionMetricKind::HitPrecision => "hit_precision",
            CognitionMetricKind::HitRecall => "hit_recall",
            CognitionMetricKind::ReuseGain => "reuse_gain",
            CognitionMetricKind::FalseResonanceRate => "false_resonance_rate",
            CognitionMetricKind::PercolationAccuracy => "percolation_accuracy",
            CognitionMetricKind::EntropyCollapsePredictiveness => "entropy_collapse_predictiveness",
            CognitionMetricKind::HiddenSingleUtility => "hidden_single_utility",
            CognitionMetricKind::BoundaryContractViolationRate => {
                "boundary_contract_violation_rate"
            }
            CognitionMetricKind::SelfDriftBoundAdherence => "self_drift_bound_adherence",
            CognitionMetricKind::DualTriggerPrecision => "dual_trigger_precision",
            CognitionMetricKind::CalibrationAcceptanceQuality => "calibration_acceptance_quality",
            CognitionMetricKind::CarrierMigrationBenefit => "carrier_migration_benefit",
        }
    }

    /// Direction.
    pub fn direction(self) -> MetricDirection {
        match self {
            CognitionMetricKind::FalseResonanceRate
            | CognitionMetricKind::BoundaryContractViolationRate => MetricDirection::LowerIsBetter,
            _ => MetricDirection::HigherIsBetter,
        }
    }

    /// MetricSpec for this kind.
    pub fn metric_spec(self) -> MetricSpec {
        MetricSpec {
            metric_id: self.metric_id().into(),
            family: MetricFamily::Cognition,
            direction: self.direction(),
            primary: matches!(
                self,
                CognitionMetricKind::PanoramaCoverage
                    | CognitionMetricKind::AttractorRankingAccuracy
            ),
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![InvalidationRule::RequireReplayPass],
        }
    }
}

/// Returns the canonical cognition-metric MetricSpec set.
pub fn cognition_metric_specs() -> Vec<MetricSpec> {
    [
        CognitionMetricKind::PanoramaCoverage,
        CognitionMetricKind::CurrentHorizonCoverage,
        CognitionMetricKind::CounterfactualCoverage,
        CognitionMetricKind::BlockedPathCorrectness,
        CognitionMetricKind::AttractorRankingAccuracy,
        CognitionMetricKind::PanoramaCompressionRatio,
        CognitionMetricKind::HitPrecision,
        CognitionMetricKind::HitRecall,
        CognitionMetricKind::ReuseGain,
        CognitionMetricKind::FalseResonanceRate,
        CognitionMetricKind::PercolationAccuracy,
        CognitionMetricKind::EntropyCollapsePredictiveness,
        CognitionMetricKind::HiddenSingleUtility,
        CognitionMetricKind::BoundaryContractViolationRate,
        CognitionMetricKind::SelfDriftBoundAdherence,
        CognitionMetricKind::DualTriggerPrecision,
        CognitionMetricKind::CalibrationAcceptanceQuality,
        CognitionMetricKind::CarrierMigrationBenefit,
    ]
    .into_iter()
    .map(|k| k.metric_spec())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cognition_metric_specs_cover_full_table() {
        // §11.1 (5) + §11.2 (4) + §11.3 (4) + §11.4 (4) + alias = 18.
        assert_eq!(cognition_metric_specs().len(), 18);
    }

    #[test]
    fn false_resonance_rate_is_lower_is_better() {
        assert_eq!(
            CognitionMetricKind::FalseResonanceRate.direction(),
            MetricDirection::LowerIsBetter
        );
    }
}
