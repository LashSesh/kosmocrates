//! `MetricSpec`, `MetricObservation` and the metric families (§5).
//!
//! Metric direction is a hard part of the contract: a `HigherIsBetter`
//! metric's monotone improvement is positive evidence; a
//! `LowerIsBetter` metric's monotone improvement is *also* positive
//! evidence — but the two are NEVER comparable except via this
//! direction tag.

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, EvalError, EvidenceRef, Fixed, Hash256};

/// Metric family per §5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MetricFamily {
    /// Primary outcome metric (TaskSuccess, F1, …).
    PrimaryOutcome,
    /// Post-symbolic structural metric (ConstraintMass, EntropyReduction, …).
    PostSymbolicStructural,
    /// System metric (throughput, latency, memory).
    System,
    /// Cognition-specific metric.
    Cognition,
    /// Code/agent-specific metric.
    Agent,
}

/// Direction the metric should move in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MetricDirection {
    /// Higher is better (e.g. precision, F1, throughput).
    HigherIsBetter,
    /// Lower is better (e.g. latency, false-commit rate).
    LowerIsBetter,
    /// Direction is per-context — must not be compared in aggregate.
    Mixed,
}

/// How the metric is aggregated across trials.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AggregationPolicy {
    /// Arithmetic mean.
    Mean,
    /// Median (no float — implemented as the lower median for even
    /// counts).
    Median,
    /// Sum (only valid for additive metrics).
    Sum,
    /// Maximum (worst-case aggregation for `LowerIsBetter`).
    Max,
    /// Minimum.
    Min,
    /// Pass-through (for diagnostic metrics that must be reported but
    /// not aggregated).
    PassThrough,
}

/// Rule that invalidates a single metric observation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum InvalidationRule {
    /// Replay must succeed for the observation to be counted.
    RequireReplayPass,
    /// The trial must declare a ground-truth profile.
    RequireGroundTruth,
    /// Numerator/denominator must be non-zero.
    RequireNonZeroDenominator,
}

/// `MetricSpec` (§10.4).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MetricSpec {
    /// Stable metric id (e.g. `"task_success"`, `"f1"`).
    pub metric_id: String,
    /// Metric family.
    pub family: MetricFamily,
    /// Direction.
    pub direction: MetricDirection,
    /// Whether this metric is part of the *primary* set.
    pub primary: bool,
    /// Aggregation policy across trials.
    pub aggregation: AggregationPolicy,
    /// Invalidation rules.
    pub invalidation_rules: Vec<InvalidationRule>,
}

impl MetricSpec {
    /// `task_success` — primary, higher-is-better, mean-aggregated.
    pub fn task_success() -> Self {
        MetricSpec {
            metric_id: "task_success".into(),
            family: MetricFamily::PrimaryOutcome,
            direction: MetricDirection::HigherIsBetter,
            primary: true,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![InvalidationRule::RequireReplayPass],
        }
    }

    /// `f1`.
    pub fn f1_score() -> Self {
        MetricSpec {
            metric_id: "f1".into(),
            family: MetricFamily::PrimaryOutcome,
            direction: MetricDirection::HigherIsBetter,
            primary: true,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![
                InvalidationRule::RequireReplayPass,
                InvalidationRule::RequireGroundTruth,
            ],
        }
    }

    /// `precision`.
    pub fn precision() -> Self {
        MetricSpec {
            metric_id: "precision".into(),
            family: MetricFamily::PrimaryOutcome,
            direction: MetricDirection::HigherIsBetter,
            primary: false,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![
                InvalidationRule::RequireReplayPass,
                InvalidationRule::RequireGroundTruth,
                InvalidationRule::RequireNonZeroDenominator,
            ],
        }
    }

    /// `recall`.
    pub fn recall() -> Self {
        MetricSpec {
            metric_id: "recall".into(),
            family: MetricFamily::PrimaryOutcome,
            direction: MetricDirection::HigherIsBetter,
            primary: false,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![
                InvalidationRule::RequireReplayPass,
                InvalidationRule::RequireGroundTruth,
                InvalidationRule::RequireNonZeroDenominator,
            ],
        }
    }

    /// `false_commit_rate` — primary safety metric.
    pub fn false_commit_rate() -> Self {
        MetricSpec {
            metric_id: "false_commit_rate".into(),
            family: MetricFamily::PrimaryOutcome,
            direction: MetricDirection::LowerIsBetter,
            primary: true,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![InvalidationRule::RequireReplayPass],
        }
    }

    /// `hold_correctness`.
    pub fn hold_correctness() -> Self {
        MetricSpec {
            metric_id: "hold_correctness".into(),
            family: MetricFamily::PrimaryOutcome,
            direction: MetricDirection::HigherIsBetter,
            primary: false,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![InvalidationRule::RequireReplayPass],
        }
    }

    /// `panorama_coverage` — mean horizon coverage across cognition phases.
    pub fn panorama_coverage() -> Self {
        MetricSpec {
            metric_id: "panorama_coverage".into(),
            family: MetricFamily::Cognition,
            direction: MetricDirection::HigherIsBetter,
            primary: false,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![],
        }
    }

    /// `replay_identity` — primary, higher-is-better (boolean
    /// 0/1, mean ⇒ pass rate).
    pub fn replay_identity() -> Self {
        MetricSpec {
            metric_id: "replay_identity".into(),
            family: MetricFamily::PrimaryOutcome,
            direction: MetricDirection::HigherIsBetter,
            primary: true,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![],
        }
    }

    /// `regression_rate` — for code-agent tasks.
    pub fn regression_rate() -> Self {
        MetricSpec {
            metric_id: "regression_rate".into(),
            family: MetricFamily::Agent,
            direction: MetricDirection::LowerIsBetter,
            primary: false,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![InvalidationRule::RequireReplayPass],
        }
    }

    /// `manual_intervention` — autonomy metric.
    pub fn manual_intervention() -> Self {
        MetricSpec {
            metric_id: "manual_intervention".into(),
            family: MetricFamily::Agent,
            direction: MetricDirection::LowerIsBetter,
            primary: false,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![],
        }
    }

    /// `throughput_obs_per_sec` — system metric.
    pub fn throughput() -> Self {
        MetricSpec {
            metric_id: "throughput_obs_per_sec".into(),
            family: MetricFamily::System,
            direction: MetricDirection::HigherIsBetter,
            primary: false,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![],
        }
    }

    /// `pipeline_latency_us` — system metric.
    pub fn latency() -> Self {
        MetricSpec {
            metric_id: "pipeline_latency_us".into(),
            family: MetricFamily::System,
            direction: MetricDirection::LowerIsBetter,
            primary: false,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![],
        }
    }
}

/// `MetricObservation` (§10.7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricObservation {
    /// Metric id.
    pub metric_id: String,
    /// The observed value (canonical, float-free).
    pub value: Fixed,
    /// Optional unit string.
    pub unit: Option<String>,
    /// Whether the observation is valid for aggregation.
    pub valid: bool,
    /// Reason if `valid == false`.
    pub invalid_reason: Option<String>,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl MetricObservation {
    /// Build a valid observation.
    pub fn ok(metric_id: impl Into<String>, value: Fixed) -> Self {
        MetricObservation {
            metric_id: metric_id.into(),
            value,
            unit: None,
            valid: true,
            invalid_reason: None,
            evidence_refs: Vec::new(),
        }
    }

    /// Build an invalid observation with a reason.
    pub fn invalid(metric_id: impl Into<String>, reason: impl Into<String>) -> Self {
        MetricObservation {
            metric_id: metric_id.into(),
            value: Fixed::zero(),
            unit: None,
            valid: false,
            invalid_reason: Some(reason.into()),
            evidence_refs: Vec::new(),
        }
    }

    /// Stable content hash of the observation.
    pub fn content_hash(&self) -> Result<Hash256, EvalError> {
        content_address(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_direction_round_trips() {
        let m = MetricSpec::false_commit_rate();
        assert_eq!(m.direction, MetricDirection::LowerIsBetter);
    }

    #[test]
    fn invalid_observation_carries_reason() {
        let o = MetricObservation::invalid("task_success", "replay mismatch");
        assert!(!o.valid);
        assert!(o.invalid_reason.is_some());
    }
}
