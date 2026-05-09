//! Benchmark-matrix presets (§18).
//!
//! Three canonical presets that the CLI's `init` subcommand can stamp:
//!
//! 1. `agent_cognition` — variants B0/B2/B5/B6/B7 over CodeAgentPatch /
//!    DocSynthesis / TraversalPuzzle / MemoryReuse.
//! 2. `streaming_event_detection` — B0 (classical) / B1 (PSE Core) / B7
//!    (full calibrated) over StreamEvent / AnomalyRegime.
//! 3. `post_symbolic_ablation` — full B0..B7 ladder over every
//!    mandatory workload family with minimal fixtures.

use std::collections::BTreeMap;

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
}

impl Preset {
    /// Parse a CLI tag (`"agent-cognition"`, `"streaming-event-detection"`,
    /// `"post-symbolic-ablation"`).
    pub fn from_tag(s: &str) -> Option<Self> {
        match s {
            "agent-cognition" => Some(Preset::AgentCognition),
            "streaming-event-detection" => Some(Preset::StreamingEventDetection),
            "post-symbolic-ablation" => Some(Preset::PostSymbolicAblation),
            _ => None,
        }
    }

    /// Build the spec for this preset.
    pub fn build(self) -> Result<EvaluationSpec, EvalError> {
        match self {
            Preset::AgentCognition => Ok(preset_agent_cognition()?),
            Preset::StreamingEventDetection => Ok(preset_streaming_event_detection()?),
            Preset::PostSymbolicAblation => Ok(preset_post_symbolic_ablation()?),
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
        assert!(Preset::from_tag("nonsense").is_none());
    }
}
