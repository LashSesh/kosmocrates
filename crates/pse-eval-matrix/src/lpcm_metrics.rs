//! LPCM metrics — PSE-LPCM-IMPLEMENTATION-01 Fragmented 51% Condensation Layer.
//!
//! These metrics close the eval matrix over the LPCM layer: every primary
//! observable from §12 (Local Gate), §13 (Seam Graph), §14 (Percolation),
//! §15 (Coarse-Graining) has a declared `MetricSpec` here so the scorer can
//! aggregate without recomputing anything.

use serde::{Deserialize, Serialize};

use crate::metrics::{
    AggregationPolicy, InvalidationRule, MetricDirection, MetricFamily, MetricSpec,
};

/// LPCM metric kinds (PSE-LPCM-IMPLEMENTATION-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LpcmMetricKind {
    /// Fraction of fragments where `LocalCondensationBit.active = true`
    /// (the 51% gate fired). Higher is better — low rate indicates that
    /// evidence is too sparse or provenance is failing.
    LocalCondensationRate,
    /// Fraction of candidate percolation paths rejected as
    /// `FalsePercolation` (replay-incompatible). Lower is better.
    FalsePercolationRate,
    /// Byte-identity of the `HierarchicalCollapseReport` on independent
    /// re-run with the same `LpcmRunDescriptor` + window. Must be 1.0.
    LpcmReplayIdentity,
    /// Fraction of windows that produced a `CollapseCandidateReady`
    /// outcome (at least one coarse-grained condensate active). Higher
    /// is better.
    CoarseGrainActivationRate,
    /// Count of accepted percolation paths per window (informational;
    /// higher is better for well-evidenced windows).
    PercolationPathCount,
    /// Fraction of topology patch sites that are triangulable (satisfy
    /// the topology-guard). Low rate → `NonTriangulablePatch` failures.
    TriangulablePatchRate,
    /// Rate of `OscillatoryFlip` events (a fragment's active bit
    /// flipping more than once per window). Lower is better.
    FalseLocalCollapseRate,
    /// Fraction of candidate seam edges that cleared the `seam_min`
    /// threshold and replay-compatibility check (informational).
    SeamAcceptanceRate,
}

impl LpcmMetricKind {
    /// Stable metric id.
    pub fn metric_id(self) -> &'static str {
        match self {
            LpcmMetricKind::LocalCondensationRate => "lpcm_local_condensation_rate",
            LpcmMetricKind::FalsePercolationRate => "lpcm_false_percolation_rate",
            LpcmMetricKind::LpcmReplayIdentity => "lpcm_replay_identity",
            LpcmMetricKind::CoarseGrainActivationRate => "lpcm_coarse_grain_activation_rate",
            LpcmMetricKind::PercolationPathCount => "lpcm_percolation_path_count",
            LpcmMetricKind::TriangulablePatchRate => "lpcm_triangulable_patch_rate",
            LpcmMetricKind::FalseLocalCollapseRate => "lpcm_false_local_collapse_rate",
            LpcmMetricKind::SeamAcceptanceRate => "lpcm_seam_acceptance_rate",
        }
    }

    /// Direction.
    pub fn direction(self) -> MetricDirection {
        match self {
            LpcmMetricKind::FalsePercolationRate | LpcmMetricKind::FalseLocalCollapseRate => {
                MetricDirection::LowerIsBetter
            }
            _ => MetricDirection::HigherIsBetter,
        }
    }

    /// `MetricSpec` for this kind.
    pub fn metric_spec(self) -> MetricSpec {
        MetricSpec {
            metric_id: self.metric_id().into(),
            family: MetricFamily::PostSymbolicStructural,
            direction: self.direction(),
            primary: matches!(
                self,
                LpcmMetricKind::LocalCondensationRate
                    | LpcmMetricKind::LpcmReplayIdentity
                    | LpcmMetricKind::FalsePercolationRate
                    | LpcmMetricKind::CoarseGrainActivationRate
            ),
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![InvalidationRule::RequireReplayPass],
        }
    }
}

/// Returns the canonical PSE-LPCM-IMPLEMENTATION-01 metric set.
pub fn lpcm_metric_specs() -> Vec<MetricSpec> {
    [
        LpcmMetricKind::LocalCondensationRate,
        LpcmMetricKind::FalsePercolationRate,
        LpcmMetricKind::LpcmReplayIdentity,
        LpcmMetricKind::CoarseGrainActivationRate,
        LpcmMetricKind::PercolationPathCount,
        LpcmMetricKind::TriangulablePatchRate,
        LpcmMetricKind::FalseLocalCollapseRate,
        LpcmMetricKind::SeamAcceptanceRate,
    ]
    .into_iter()
    .map(|k| k.metric_spec())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lpcm_metric_set_covers_eight_dimensions() {
        assert_eq!(lpcm_metric_specs().len(), 8);
    }

    #[test]
    fn local_condensation_rate_is_primary() {
        assert!(LpcmMetricKind::LocalCondensationRate.metric_spec().primary);
    }

    #[test]
    fn lpcm_replay_identity_is_primary() {
        assert!(LpcmMetricKind::LpcmReplayIdentity.metric_spec().primary);
    }

    #[test]
    fn false_percolation_rate_is_lower_is_better() {
        assert_eq!(
            LpcmMetricKind::FalsePercolationRate.direction(),
            MetricDirection::LowerIsBetter,
        );
    }

    #[test]
    fn false_local_collapse_rate_is_lower_is_better() {
        assert_eq!(
            LpcmMetricKind::FalseLocalCollapseRate.direction(),
            MetricDirection::LowerIsBetter,
        );
    }

    #[test]
    fn percolation_path_count_is_secondary() {
        assert!(!LpcmMetricKind::PercolationPathCount.metric_spec().primary);
    }
}
