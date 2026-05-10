//! Cell-substrate metrics — bridge from the eval matrix to
//! PHASEMATRIX-HIVEMIND-03 / 03.1 evidence.
//!
//! These metrics extend the eval-matrix coverage to morphodynamic
//! cell substrates and the Dual-Fabric Stitch Layer so the system
//! stays "closed": every layer
//! (Traversal / Horizon / Cognition / **PhaseMatrix Cell Substrate** /
//! **Dual-Fabric Stitch**)
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
    // ── PHASEMATRIX-HIVEMIND-03.1 Dual-Fabric Stitch metrics ───────────
    /// StitcherGate pass-rate — fraction of stitch cycles where
    /// `G_stitch = 1` (all seven sub-gates fired).
    StitcherGatePassRate,
    /// Coupling-update trace coverage — fraction of accepted
    /// `CouplingUpdate`s that carry a non-empty `FieldTensorTrace`.
    CouplingUpdateTraceCoverage,
    /// Tensor-revision monotonicity — fraction of cycles where
    /// `tensor_after.revision == tensor_before.revision + 1` when
    /// updates were accepted, and unchanged otherwise.
    TensorRevisionMonotonicity,
    /// Mirror-consistency compliance — fraction of candidates where
    /// `G_mci` passed.
    MirrorConsistencyCompliance,
    /// Stitch-replay identity — fraction of stitch cycles that
    /// reproduced byte-identical `StitcherReport`s on replay.
    StitchReplayIdentity,
    /// Fabric-H isolation rate — confirms that no Fabric-H link ever
    /// directly mutated Fabric-T (should always be 1.0).
    FabricHIsolationRate,
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
            CellSubstrateMetricKind::StitcherGatePassRate => "stitcher_gate_pass_rate",
            CellSubstrateMetricKind::CouplingUpdateTraceCoverage => {
                "coupling_update_trace_coverage"
            }
            CellSubstrateMetricKind::TensorRevisionMonotonicity => "tensor_revision_monotonicity",
            CellSubstrateMetricKind::MirrorConsistencyCompliance => "mirror_consistency_compliance",
            CellSubstrateMetricKind::StitchReplayIdentity => "stitch_replay_identity",
            CellSubstrateMetricKind::FabricHIsolationRate => "fabric_h_isolation_rate",
        }
    }

    /// Direction.
    pub fn direction(self) -> MetricDirection {
        match self {
            CellSubstrateMetricKind::MatrixBoundaryViolationRate => MetricDirection::LowerIsBetter,
            _ => MetricDirection::HigherIsBetter,
        }
    }

    /// True iff this metric belongs to the Dual-Fabric Stitch sub-set
    /// (PHASEMATRIX-HIVEMIND-03.1).
    pub fn is_dual_fabric(self) -> bool {
        matches!(
            self,
            CellSubstrateMetricKind::StitcherGatePassRate
                | CellSubstrateMetricKind::CouplingUpdateTraceCoverage
                | CellSubstrateMetricKind::TensorRevisionMonotonicity
                | CellSubstrateMetricKind::MirrorConsistencyCompliance
                | CellSubstrateMetricKind::StitchReplayIdentity
                | CellSubstrateMetricKind::FabricHIsolationRate
        )
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
                    | CellSubstrateMetricKind::StitcherGatePassRate
                    | CellSubstrateMetricKind::StitchReplayIdentity
                    | CellSubstrateMetricKind::FabricHIsolationRate
            ),
            aggregation: AggregationPolicy::Mean,
            invalidation_rules: vec![InvalidationRule::RequireReplayPass],
        }
    }
}

/// Returns the canonical PHASEMATRIX-HIVEMIND-03 metric set (v0.3).
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

/// Returns the Dual-Fabric Stitch Layer metric set (PHASEMATRIX-HIVEMIND-03.1).
pub fn dual_fabric_stitch_metric_specs() -> Vec<MetricSpec> {
    [
        CellSubstrateMetricKind::StitcherGatePassRate,
        CellSubstrateMetricKind::CouplingUpdateTraceCoverage,
        CellSubstrateMetricKind::TensorRevisionMonotonicity,
        CellSubstrateMetricKind::MirrorConsistencyCompliance,
        CellSubstrateMetricKind::StitchReplayIdentity,
        CellSubstrateMetricKind::FabricHIsolationRate,
    ]
    .into_iter()
    .map(|k| k.metric_spec())
    .collect()
}

/// Returns the combined metric set for B9: all v0.3 metrics plus all
/// Dual-Fabric Stitch metrics.
pub fn b9_metric_specs() -> Vec<MetricSpec> {
    let mut specs = cell_substrate_metric_specs();
    specs.extend(dual_fabric_stitch_metric_specs());
    specs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_substrate_metric_set_covers_ten_dimensions() {
        assert_eq!(cell_substrate_metric_specs().len(), 10);
    }

    #[test]
    fn dual_fabric_stitch_metric_set_covers_six_dimensions() {
        assert_eq!(dual_fabric_stitch_metric_specs().len(), 6);
    }

    #[test]
    fn b9_metric_set_covers_sixteen_dimensions() {
        assert_eq!(b9_metric_specs().len(), 16);
    }

    #[test]
    fn stitch_replay_identity_is_primary() {
        let spec = CellSubstrateMetricKind::StitchReplayIdentity.metric_spec();
        assert!(spec.primary);
    }

    #[test]
    fn fabric_h_isolation_is_primary() {
        let spec = CellSubstrateMetricKind::FabricHIsolationRate.metric_spec();
        assert!(spec.primary);
    }

    #[test]
    fn boundary_violation_is_lower_is_better() {
        assert_eq!(
            CellSubstrateMetricKind::MatrixBoundaryViolationRate.direction(),
            MetricDirection::LowerIsBetter
        );
    }
}
