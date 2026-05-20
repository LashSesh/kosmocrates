//! Benchmark-matrix presets (§18).
//!
//! Five canonical presets that the CLI's `init` subcommand can stamp:
//!
//! 1. `agent_cognition` — variants B0/B2/B5/B6/B7 over CodeAgentPatch /
//!    DocSynthesis / TraversalPuzzle / MemoryReuse.
//! 2. `streaming_event_detection` — B0 (classical) / B1 (PSE Core) / B7
//!    (full calibrated) over StreamEvent / AnomalyRegime.
//! 3. `post_symbolic_ablation` — full B0..B7 ladder over every
//!    mandatory workload family with minimal fixtures.
//! 4. `phase_matrix_substrate` — B0 / B7 / B8 over the
//!    `MorphoCellSubstrate` workload (PHASEMATRIX-HIVEMIND-03), with
//!    the cell-substrate metric set added to the matrix.
//! 5. `dual_fabric_stitch` — B0 / B8 / B9 over the `DualFabricStitch`
//!    workload (PHASEMATRIX-HIVEMIND-03.1), scored against the full
//!    Dual-Fabric Stitch metric set.

use std::collections::BTreeMap;

use crate::cell_substrate_metrics::{b9_metric_specs, cell_substrate_metric_specs};
use crate::lpcm_metrics::lpcm_metric_specs;
use crate::datasets::DatasetManifest;
use crate::ground_truth::GroundTruthProfile;
use crate::metrics::MetricSpec;
use crate::primitives::EvalError;
use crate::spec::{
    DeterminismPolicy, EvaluationPurpose, EvaluationSpec, ExperimentDesign, OutputPolicy,
    StatisticalPlan,
};
use crate::variants::{SystemVariantSpec, VariantLadder};
use crate::workloads::{WorkloadFamily, WorkloadSpec};

/// Identifier of a built-in preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Preset {
    /// `agent-cognition` (§18.1).
    AgentCognition,
    /// `streaming-event-detection` (§18.2).
    StreamingEventDetection,
    /// `post-symbolic-ablation` (§18.3).
    PostSymbolicAblation,
    /// `phase-matrix-substrate` — PHASEMATRIX-HIVEMIND-03 closure
    /// preset.
    PhaseMatrixSubstrate,
    /// `dual-fabric-stitch` — PHASEMATRIX-HIVEMIND-03.1 Dual-Fabric
    /// Field-Tensor Stitch Layer closure preset.
    DualFabricStitch,
    /// `lpcm-fragment-collapse` — PSE-LPCM-IMPLEMENTATION-01 Fragmented
    /// 51% Condensation Layer closure preset: B0 / B7 / B10 over the
    /// `LpcmFragmentCollapse` workload.
    LpcmFragmentCollapse,
}

impl Preset {
    /// Parse a CLI tag.
    pub fn from_tag(s: &str) -> Option<Self> {
        match s {
            "agent-cognition" => Some(Preset::AgentCognition),
            "streaming-event-detection" => Some(Preset::StreamingEventDetection),
            "post-symbolic-ablation" => Some(Preset::PostSymbolicAblation),
            "phase-matrix-substrate" => Some(Preset::PhaseMatrixSubstrate),
            "dual-fabric-stitch" => Some(Preset::DualFabricStitch),
            "lpcm-fragment-collapse" => Some(Preset::LpcmFragmentCollapse),
            _ => None,
        }
    }

    /// Build the spec for this preset.
    pub fn build(self) -> Result<EvaluationSpec, EvalError> {
        match self {
            Preset::AgentCognition => Ok(preset_agent_cognition()?),
            Preset::StreamingEventDetection => Ok(preset_streaming_event_detection()?),
            Preset::PostSymbolicAblation => Ok(preset_post_symbolic_ablation()?),
            Preset::PhaseMatrixSubstrate => Ok(preset_phase_matrix_substrate()?),
            Preset::DualFabricStitch => Ok(preset_dual_fabric_stitch()?),
            Preset::LpcmFragmentCollapse => Ok(preset_lpcm_fragment_collapse()?),
        }
    }
}

fn workload_for(
    family: WorkloadFamily,
    id: &str,
    dataset: &DatasetManifest,
    gt: Option<&GroundTruthProfile>,
) -> WorkloadSpec {
    let gt_id = gt.map(|g| g.profile_id.clone());
    match family {
        WorkloadFamily::CodeAgentPatch => {
            WorkloadSpec::code_agent_patch(id, dataset.dataset_id.clone(), gt_id)
        }
        WorkloadFamily::HorizonFinalization => {
            WorkloadSpec::horizon_finalization(id, dataset.dataset_id.clone(), gt_id)
        }
        WorkloadFamily::CognitionPanorama => {
            WorkloadSpec::cognition_panorama(id, dataset.dataset_id.clone(), gt_id)
        }
        WorkloadFamily::MorphoCellSubstrate => {
            WorkloadSpec::morpho_cell_substrate(id, dataset.dataset_id.clone(), gt_id)
        }
        WorkloadFamily::DualFabricStitch => {
            WorkloadSpec::dual_fabric_stitch(id, dataset.dataset_id.clone(), gt_id)
        }
        WorkloadFamily::LpcmFragmentCollapse => {
            WorkloadSpec::lpcm_fragment_collapse(id, dataset.dataset_id.clone(), gt_id)
        }
        // The other families share the StreamEvent shape for the
        // built-in presets — production callers should author their own
        // workload specs for the subtleties of each family.
        _ => WorkloadSpec::stream_event(id, dataset.dataset_id.clone(), gt_id),
    }
}

/// `agent-cognition` preset (§18.1).
pub fn preset_agent_cognition() -> Result<EvaluationSpec, EvalError> {
    let dataset = DatasetManifest::synthetic("d.agent_cognition");
    let gt = GroundTruthProfile::unit_test_oracle();
    let workloads = vec![
        workload_for(
            WorkloadFamily::CodeAgentPatch,
            "w.code_agent_patch",
            &dataset,
            Some(&gt),
        ),
        workload_for(
            WorkloadFamily::DocSynthesis,
            "w.doc_synthesis",
            &dataset,
            Some(&gt),
        ),
        workload_for(
            WorkloadFamily::TraversalPuzzle,
            "w.traversal_puzzle",
            &dataset,
            Some(&gt),
        ),
        workload_for(
            WorkloadFamily::MemoryReuse,
            "w.memory_reuse",
            &dataset,
            Some(&gt),
        ),
    ];
    let variants = vec![
        SystemVariantSpec::baseline(),
        SystemVariantSpec::traversal(),
        SystemVariantSpec::horizon(),
        SystemVariantSpec::cognition(),
        SystemVariantSpec::full_stack(),
    ];
    let metrics = vec![
        MetricSpec::task_success(),
        MetricSpec::regression_rate(),
        MetricSpec::manual_intervention(),
        MetricSpec::replay_identity(),
        MetricSpec::false_commit_rate(),
        MetricSpec::hold_correctness(),
        MetricSpec::panorama_coverage(),
    ];
    EvaluationSpec {
        spec_id: crate::primitives::Hash256::zero(),
        schema_version: "eval-matrix-v0.1".into(),
        purpose: EvaluationPurpose::Composite,
        variants,
        workloads,
        datasets: vec![dataset],
        metrics,
        experiment_design: ExperimentDesign::default_design(),
        statistical_plan: StatisticalPlan::default_plan(),
        determinism_policy: DeterminismPolicy::default(),
        output_policy: OutputPolicy::default_policy(),
        tags: BTreeMap::from_iter([("preset".to_string(), "agent-cognition".to_string())]),
    }
    .with_id()
}

/// `streaming-event-detection` preset (§18.2).
pub fn preset_streaming_event_detection() -> Result<EvaluationSpec, EvalError> {
    let dataset = DatasetManifest::synthetic("d.stream_minimal");
    let gt = GroundTruthProfile::synthetic_exact();
    let workloads = vec![
        workload_for(
            WorkloadFamily::StreamEvent,
            "w.stream_event",
            &dataset,
            Some(&gt),
        ),
        workload_for(
            WorkloadFamily::AnomalyRegime,
            "w.anomaly_regime",
            &dataset,
            Some(&gt),
        ),
    ];
    let variants = vec![
        SystemVariantSpec::baseline(),
        SystemVariantSpec::pse_core(),
        SystemVariantSpec::full_stack(),
    ];
    let metrics = vec![
        MetricSpec::precision(),
        MetricSpec::recall(),
        MetricSpec::f1_score(),
        MetricSpec::false_commit_rate(),
        MetricSpec::throughput(),
        MetricSpec::latency(),
        MetricSpec::replay_identity(),
        MetricSpec::task_success(),
    ];
    EvaluationSpec {
        spec_id: crate::primitives::Hash256::zero(),
        schema_version: "eval-matrix-v0.1".into(),
        purpose: EvaluationPurpose::BaselineComparison,
        variants,
        workloads,
        datasets: vec![dataset],
        metrics,
        experiment_design: ExperimentDesign::default_design(),
        statistical_plan: StatisticalPlan::default_plan(),
        determinism_policy: DeterminismPolicy::default(),
        output_policy: OutputPolicy::default_policy(),
        tags: BTreeMap::from_iter([(
            "preset".to_string(),
            "streaming-event-detection".to_string(),
        )]),
    }
    .with_id()
}

/// `post-symbolic-ablation` preset (§18.3).
pub fn preset_post_symbolic_ablation() -> Result<EvaluationSpec, EvalError> {
    let dataset = DatasetManifest::synthetic("d.ablation_minimal");
    let gt = GroundTruthProfile::synthetic_exact();
    let workloads = vec![
        workload_for(
            WorkloadFamily::StreamEvent,
            "w.stream_event",
            &dataset,
            Some(&gt),
        ),
        workload_for(
            WorkloadFamily::HorizonFinalization,
            "w.horizon_finalization",
            &dataset,
            Some(&gt),
        ),
        workload_for(
            WorkloadFamily::CognitionPanorama,
            "w.cognition_panorama",
            &dataset,
            Some(&gt),
        ),
    ];
    let variants = VariantLadder::full();
    let metrics = vec![
        MetricSpec::task_success(),
        MetricSpec::false_commit_rate(),
        MetricSpec::hold_correctness(),
        MetricSpec::replay_identity(),
        MetricSpec::regression_rate(),
        MetricSpec::manual_intervention(),
    ];
    EvaluationSpec {
        spec_id: crate::primitives::Hash256::zero(),
        schema_version: "eval-matrix-v0.1".into(),
        purpose: EvaluationPurpose::AblationStudy,
        variants,
        workloads,
        datasets: vec![dataset],
        metrics,
        experiment_design: ExperimentDesign::default_design(),
        statistical_plan: StatisticalPlan::default_plan(),
        determinism_policy: DeterminismPolicy::default(),
        output_policy: OutputPolicy::default_policy(),
        tags: BTreeMap::from_iter([("preset".to_string(), "post-symbolic-ablation".to_string())]),
    }
    .with_id()
}

/// `phase-matrix-substrate` preset.
///
/// Empirical closure for PHASEMATRIX-HIVEMIND-03: variants B0 / B7 / B8
/// over the `MorphoCellSubstrate` workload, scored against the full
/// cell-substrate metric set plus the standard task-success / replay /
/// hold-correctness anchors so the cell-substrate uplift is comparable
/// against the rest of the matrix.
pub fn preset_phase_matrix_substrate() -> Result<EvaluationSpec, EvalError> {
    let dataset = DatasetManifest::synthetic("d.phase_matrix_substrate");
    let gt = GroundTruthProfile::synthetic_exact();
    let workloads = vec![workload_for(
        WorkloadFamily::MorphoCellSubstrate,
        "w.morpho_cell_substrate",
        &dataset,
        Some(&gt),
    )];
    let variants = vec![
        SystemVariantSpec::baseline(),
        SystemVariantSpec::full_stack(),
        SystemVariantSpec::phase_matrix_substrate(),
    ];
    let mut metrics = vec![
        MetricSpec::task_success(),
        MetricSpec::replay_identity(),
        MetricSpec::hold_correctness(),
        MetricSpec::false_commit_rate(),
    ];
    metrics.extend(cell_substrate_metric_specs());
    EvaluationSpec {
        spec_id: crate::primitives::Hash256::zero(),
        schema_version: "eval-matrix-v0.1".into(),
        purpose: EvaluationPurpose::Composite,
        variants,
        workloads,
        datasets: vec![dataset],
        metrics,
        experiment_design: ExperimentDesign::default_design(),
        statistical_plan: StatisticalPlan::default_plan(),
        determinism_policy: DeterminismPolicy::default(),
        output_policy: OutputPolicy::default_policy(),
        tags: BTreeMap::from_iter([("preset".to_string(), "phase-matrix-substrate".to_string())]),
    }
    .with_id()
}

/// `dual-fabric-stitch` preset.
///
/// Empirical closure for PHASEMATRIX-HIVEMIND-03.1: variants B0 / B8 / B9
/// over the `DualFabricStitch` workload.  Scored against the full B9
/// metric set (all cell-substrate metrics + all Dual-Fabric Stitch metrics)
/// so the stitch-layer uplift is comparable against B8.
pub fn preset_dual_fabric_stitch() -> Result<EvaluationSpec, EvalError> {
    let dataset = DatasetManifest::synthetic("d.dual_fabric_stitch");
    let gt = GroundTruthProfile::synthetic_exact();
    let workloads = vec![workload_for(
        WorkloadFamily::DualFabricStitch,
        "w.dual_fabric_stitch",
        &dataset,
        Some(&gt),
    )];
    let variants = vec![
        SystemVariantSpec::baseline(),
        SystemVariantSpec::phase_matrix_substrate(),
        SystemVariantSpec::dual_fabric_stitch(),
    ];
    let mut metrics = vec![
        MetricSpec::task_success(),
        MetricSpec::replay_identity(),
        MetricSpec::hold_correctness(),
        MetricSpec::false_commit_rate(),
    ];
    metrics.extend(b9_metric_specs());
    EvaluationSpec {
        spec_id: crate::primitives::Hash256::zero(),
        schema_version: "eval-matrix-v0.1".into(),
        purpose: EvaluationPurpose::Composite,
        variants,
        workloads,
        datasets: vec![dataset],
        metrics,
        experiment_design: ExperimentDesign::default_design(),
        statistical_plan: StatisticalPlan::default_plan(),
        determinism_policy: DeterminismPolicy::default(),
        output_policy: OutputPolicy::default_policy(),
        tags: BTreeMap::from_iter([("preset".to_string(), "dual-fabric-stitch".to_string())]),
    }
    .with_id()
}

/// `lpcm-fragment-collapse` preset.
///
/// Empirical closure for PSE-LPCM-IMPLEMENTATION-01: variants B0 / B7 / B10
/// over the `LpcmFragmentCollapse` workload. Scored against the full LPCM
/// metric set plus the standard task-success / replay / hold-correctness
/// anchors so the LPCM uplift is comparable against the rest of the matrix.
///
/// Primary invariant under test: `B10_LpcmCollapse` must outperform `B7_FullStack`
/// (which lacks the LPCM bit) on `lpcm_local_condensation_rate` and
/// `lpcm_coarse_grain_activation_rate`, and both must strictly dominate `B0_Baseline`.
pub fn preset_lpcm_fragment_collapse() -> Result<EvaluationSpec, EvalError> {
    let dataset = DatasetManifest::synthetic("d.lpcm_fragment_collapse");
    let gt = GroundTruthProfile::synthetic_exact();
    let workloads = vec![workload_for(
        WorkloadFamily::LpcmFragmentCollapse,
        "w.lpcm_fragment_collapse",
        &dataset,
        Some(&gt),
    )];
    let variants = VariantLadder::lpcm_ablation();
    let mut metrics = vec![
        MetricSpec::task_success(),
        MetricSpec::replay_identity(),
        MetricSpec::hold_correctness(),
        MetricSpec::false_commit_rate(),
    ];
    metrics.extend(lpcm_metric_specs());
    EvaluationSpec {
        spec_id: crate::primitives::Hash256::zero(),
        schema_version: "eval-matrix-v0.1".into(),
        purpose: EvaluationPurpose::AblationStudy,
        variants,
        workloads,
        datasets: vec![dataset],
        metrics,
        experiment_design: ExperimentDesign::default_design(),
        statistical_plan: StatisticalPlan::default_plan(),
        determinism_policy: DeterminismPolicy::default(),
        output_policy: OutputPolicy::default_policy(),
        tags: BTreeMap::from_iter([("preset".to_string(), "lpcm-fragment-collapse".to_string())]),
    }
    .with_id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_cognition_preset_validates() {
        let s = preset_agent_cognition().unwrap();
        assert!(s.validate().is_ok());
        assert_eq!(s.variants.len(), 5);
        assert_eq!(s.workloads.len(), 4);
    }

    #[test]
    fn streaming_preset_includes_baseline_and_full_stack() {
        let s = preset_streaming_event_detection().unwrap();
        let ids: Vec<_> = s.variants.iter().map(|v| v.variant_id.as_str()).collect();
        assert!(ids.contains(&"B0_Baseline"));
        assert!(ids.contains(&"B7_FullStack"));
    }

    #[test]
    fn ablation_preset_runs_full_ladder() {
        let s = preset_post_symbolic_ablation().unwrap();
        assert_eq!(s.variants.len(), 8);
    }

    #[test]
    fn preset_tag_round_trip() {
        assert_eq!(
            Preset::from_tag("agent-cognition"),
            Some(Preset::AgentCognition)
        );
        assert_eq!(
            Preset::from_tag("phase-matrix-substrate"),
            Some(Preset::PhaseMatrixSubstrate)
        );
        assert_eq!(
            Preset::from_tag("dual-fabric-stitch"),
            Some(Preset::DualFabricStitch)
        );
        assert_eq!(
            Preset::from_tag("lpcm-fragment-collapse"),
            Some(Preset::LpcmFragmentCollapse)
        );
        assert!(Preset::from_tag("nonsense").is_none());
    }

    #[test]
    fn lpcm_preset_validates_with_b0_b7_b10() {
        let s = preset_lpcm_fragment_collapse().unwrap();
        assert!(s.validate().is_ok());
        let ids: Vec<_> = s.variants.iter().map(|v| v.variant_id.as_str()).collect();
        assert!(ids.contains(&"B0_Baseline"));
        assert!(ids.contains(&"B7_FullStack"));
        assert!(ids.contains(&"B10_LpcmCollapse"));
        assert_eq!(s.variants.len(), 3);
    }

    #[test]
    fn lpcm_preset_includes_lpcm_metric_set() {
        let s = preset_lpcm_fragment_collapse().unwrap();
        let mids: Vec<_> = s.metrics.iter().map(|m| m.metric_id.as_str()).collect();
        assert!(mids.contains(&"lpcm_local_condensation_rate"));
        assert!(mids.contains(&"lpcm_false_percolation_rate"));
        assert!(mids.contains(&"lpcm_replay_identity"));
        assert!(mids.contains(&"lpcm_coarse_grain_activation_rate"));
    }

    #[test]
    fn dual_fabric_stitch_preset_includes_b9_and_stitch_metrics() {
        let s = preset_dual_fabric_stitch().unwrap();
        assert!(s.validate().is_ok());
        let ids: Vec<_> = s.variants.iter().map(|v| v.variant_id.as_str()).collect();
        assert!(ids.contains(&"B0_Baseline"));
        assert!(ids.contains(&"B8_PhaseMatrix"));
        assert!(ids.contains(&"B9_DualFabricStitch"));
        let mids: Vec<_> = s.metrics.iter().map(|m| m.metric_id.as_str()).collect();
        assert!(mids.contains(&"stitcher_gate_pass_rate"));
        assert!(mids.contains(&"stitch_replay_identity"));
        assert!(mids.contains(&"fabric_h_isolation_rate"));
        assert!(mids.contains(&"cluster_formation_rate"));
    }

    #[test]
    fn phase_matrix_substrate_preset_includes_cell_metrics_and_b8() {
        let s = preset_phase_matrix_substrate().unwrap();
        assert!(s.validate().is_ok());
        let ids: Vec<_> = s.variants.iter().map(|v| v.variant_id.as_str()).collect();
        assert!(ids.contains(&"B0_Baseline"));
        assert!(ids.contains(&"B7_FullStack"));
        assert!(ids.contains(&"B8_PhaseMatrix"));
        let mids: Vec<_> = s.metrics.iter().map(|m| m.metric_id.as_str()).collect();
        assert!(mids.contains(&"cluster_formation_rate"));
        assert!(mids.contains(&"matrix_boundary_violation_rate"));
        assert!(mids.contains(&"dissolution_trace_preservation"));
    }
}
