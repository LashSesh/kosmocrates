//! TPT-MTL topology-layer metrics (PSE-TRAVERSE-TPT-MTL-04 §17).
//!
//! Provides `MetricSpec`s for all TPT-MTL evaluation metrics declared
//! in the spec. Values are produced by trial executors and recorded as
//! `MetricObservation`s; this module only declares the schema.

use serde::{Deserialize, Serialize};

use crate::metrics::{
    AggregationPolicy, InvalidationRule, MetricDirection, MetricFamily, MetricSpec,
};

/// TPT-MTL topology metric kinds (§17).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TopologyMetricKind {
    /// Fraction of inputs that produce a valid Window or a typed failure.
    AdapterTotalityRate,
    /// Fraction of points with correctly declared axis origin.
    AxisBridgeValidity,
    /// Fraction of identical-input Mesh replays that match digest.
    MeshDeterminismIdentity,
    /// Stability of Betti/persistence signatures under allowed perturbations.
    TopologyRobustness,
    /// Fraction of validly lifted points (with subwindow + proof).
    MicroLiftCoverage,
    /// Fraction of valid SeamComponents.
    SeamConsistencyRate,
    /// Preservation of direction, trace and carrier separation.
    CarrierContinuity,
    /// Fraction of gate-passed candidates later falsified by evidence/replay.
    FalseCrystalRate,
    /// Fraction of all transitions with complete trace.
    TraceCompleteness,
    /// Byte- or digest-identity of complete bundles on replay.
    ReplayIdentity,
}

impl TopologyMetricKind {
    /// Stable metric id.
    pub fn metric_id(self) -> &'static str {
        match self {
            TopologyMetricKind::AdapterTotalityRate => "tpt_adapter_totality_rate",
            TopologyMetricKind::AxisBridgeValidity => "tpt_axis_bridge_validity",
            TopologyMetricKind::MeshDeterminismIdentity => "tpt_mesh_determinism_identity",
            TopologyMetricKind::TopologyRobustness => "tpt_topology_robustness",
            TopologyMetricKind::MicroLiftCoverage => "tpt_micro_lift_coverage",
            TopologyMetricKind::SeamConsistencyRate => "tpt_seam_consistency_rate",
            TopologyMetricKind::CarrierContinuity => "tpt_carrier_continuity",
            TopologyMetricKind::FalseCrystalRate => "tpt_false_crystal_rate",
            TopologyMetricKind::TraceCompleteness => "tpt_trace_completeness",
            TopologyMetricKind::ReplayIdentity => "tpt_replay_identity",
        }
    }
}

/// Build the complete set of TPT-MTL metric specs.
pub fn topology_metric_specs() -> Vec<MetricSpec> {
    use TopologyMetricKind::*;
    let all = [
        (
            AdapterTotalityRate,
            MetricDirection::HigherIsBetter,
            MetricFamily::PostSymbolicStructural,
            true,
        ),
        (
            AxisBridgeValidity,
            MetricDirection::HigherIsBetter,
            MetricFamily::PostSymbolicStructural,
            true,
        ),
        (
            MeshDeterminismIdentity,
            MetricDirection::HigherIsBetter,
            MetricFamily::PostSymbolicStructural,
            true,
        ),
        (
            TopologyRobustness,
            MetricDirection::HigherIsBetter,
            MetricFamily::PostSymbolicStructural,
            false,
        ),
        (
            MicroLiftCoverage,
            MetricDirection::HigherIsBetter,
            MetricFamily::PostSymbolicStructural,
            true,
        ),
        (
            SeamConsistencyRate,
            MetricDirection::HigherIsBetter,
            MetricFamily::PostSymbolicStructural,
            false,
        ),
        (
            CarrierContinuity,
            MetricDirection::HigherIsBetter,
            MetricFamily::PostSymbolicStructural,
            false,
        ),
        (
            FalseCrystalRate,
            MetricDirection::LowerIsBetter,
            MetricFamily::PostSymbolicStructural,
            true,
        ),
        (
            TraceCompleteness,
            MetricDirection::HigherIsBetter,
            MetricFamily::PostSymbolicStructural,
            false,
        ),
        (
            ReplayIdentity,
            MetricDirection::HigherIsBetter,
            MetricFamily::PostSymbolicStructural,
            true,
        ),
    ];

    all.iter()
        .map(|&(k, direction, family, primary)| MetricSpec {
            metric_id: k.metric_id().into(),
            family,
            direction,
            primary,
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![InvalidationRule::RequireReplayPass],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_metric_specs_are_non_empty() {
        let specs = topology_metric_specs();
        assert_eq!(specs.len(), 10);
    }

    #[test]
    fn topology_metric_ids_are_unique() {
        let specs = topology_metric_specs();
        let mut ids: Vec<&str> = specs.iter().map(|s| s.metric_id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), specs.len());
    }

    #[test]
    fn false_crystal_rate_is_lower_is_better() {
        let specs = topology_metric_specs();
        let fcr = specs
            .iter()
            .find(|s| s.metric_id == "tpt_false_crystal_rate")
            .unwrap();
        assert_eq!(fcr.direction, MetricDirection::LowerIsBetter);
    }
}
