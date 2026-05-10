//! Cell-substrate metrics — bridge from the eval matrix to
//! PHASEMATRIX-HIVEMIND-03 evidence.
//!
//! These metrics extend the eval-matrix coverage to morphodynamic
//! cell substrates so the system stays "closed": every layer
//! (Traversal / Horizon / Cognition / **PhaseMatrix Cell Substrate**)
//! has a declared metric set that the existing scorer can aggregate
//! without recomputing anything.

use serde::{Deserialize, Serialize};

use crate::metrics::{
    AggregationPolicy, InvalidationRule, MetricDirection, MetricFamily, MetricSpec,
};

/// Cell-substrate metric kinds (PHASEMATRIX-HIVEMIND-03 §10–§18).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CellSubstrateMetricKind {
    /// Cluster formation pass-rate (`G_cluster` fires).
    ClusterFormationRate,
    /// Morphology gate compliance — fraction of MorphologyEvents that
    /// passed `G_morph` rather than being rejected.
    MorphologyGateCompliance,
    /// Convergence-field stability — `score_conv ≥ θ_conv` rate.
    ConvergenceStability,
    /// Intent generation rate — fraction of cycles producing an
    /// IntentCandidate.
    IntentGenerationRate,
    /// Dissolution trace preservation — fraction of dissolutions that
    /// preserved trace + evidence + lifecycle history.
    DissolutionTracePreservation,
    /// FunnelGraph acyclicity rate — fraction of graphs that passed
    /// topology validation.
    FunnelAcyclicityRate,
    /// Boundary-violation rate — fraction of cycles that hit
    /// `MatrixBoundaryViolation` (lower is better).
    MatrixBoundaryViolationRate,
    /// Working-state-compaction efficiency — bytes saved per cycle.
    WorkingStateCompactionEfficiency,
    /// Handoff-candidate utility — fraction of intent candidates that
    /// downstream layers found admissible.
    HandoffCandidateUtility,
    /// Self-coherence — `Hlattice` analogue applied to the substrate
    /// (1 = fully phase-locked seam, 0 = drift).
    SubstrateSelfCoherence,
}

impl CellSubstrateMetricKind {
    /// Stable metric id.
    pub fn metric_id(self) -> &'static str {
        match self {
            CellSubstrateMetricKind::ClusterFormationRate => "cluster_formation_rate",
            CellSubstrateMetricKind::MorphologyGateCompliance => "morphology_gate_compliance",
            CellSubstrateMetricKind::ConvergenceStability => "convergence_stability",
            CellSubstrateMetricKind::IntentGenerationRate => "intent_generation_rate",
            CellSubstrateMetricKind::DissolutionTracePreservation => {
                "dissolution_trace_preservation"
            }
            CellSubstrateMetricKind::FunnelAcyclicityRate => "funnel_acyclicity_rate",
            CellSubstrateMetricKind::MatrixBoundaryViolationRate => {
                "matrix_boundary_violation_rate"
            }
            CellSubstrateMetricKind::WorkingStateCompactionEfficiency => {
                "working_state_compaction_efficiency"
            }
            CellSubstrateMetricKind::HandoffCandidateUtility => "handoff_candidate_utility",
            CellSubstrateMetricKind::SubstrateSelfCoherence => "substrate_self_coherence",
        }
    }

    /// Direction.
    pub fn direction(self) -> MetricDirection {
        match self {
            CellSubstrateMetricKind::MatrixBoundaryViolationRate => MetricDirection::LowerIsBetter,
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
                CellSubstrateMetricKind::ClusterFormationRate
                    | CellSubstrateMetricKind::DissolutionTracePreservation
                    | CellSubstrateMetricKind::MatrixBoundaryViolationRate
            ),
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![InvalidationRule::RequireReplayPass],
        }
    }
}

/// Returns the canonical PHASEMATRIX-HIVEMIND-03 metric set.
pub fn cell_substrate_metric_specs() -> Vec<MetricSpec> {
    [
        CellSubstrateMetricKind::ClusterFormationRate,
        CellSubstrateMetricKind::MorphologyGateCompliance,
        CellSubstrateMetricKind::ConvergenceStability,
        CellSubstrateMetricKind::IntentGenerationRate,
        CellSubstrateMetricKind::DissolutionTracePreservation,
        CellSubstrateMetricKind::FunnelAcyclicityRate,
        CellSubstrateMetricKind::MatrixBoundaryViolationRate,
        CellSubstrateMetricKind::WorkingStateCompactionEfficiency,
        CellSubstrateMetricKind::HandoffCandidateUtility,
        CellSubstrateMetricKind::SubstrateSelfCoherence,
    ]
    .into_iter()
    .map(|k| k.metric_spec())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_substrate_metric_set_covers_ten_dimensions() {
        assert_eq!(cell_substrate_metric_specs().len(), 10);
    }

    #[test]
    fn boundary_violation_is_lower_is_better() {
        assert_eq!(
            CellSubstrateMetricKind::MatrixBoundaryViolationRate.direction(),
            MetricDirection::LowerIsBetter
        );
    }
}
