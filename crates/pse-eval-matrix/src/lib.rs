//! PSE-EVAL-MATRIX-01 — Empirical Benchmark Matrix for Post-Symbolic
//! Cognition Systems.
//!
//! See `PSE_EVAL_MATRIX_01.pdf` in the repo root for the normative
//! specification this crate realises.
//!
//! The eval matrix is **not** a benchmark harness. It is the formal
//! description of:
//!
//! * which scientific question is being asked
//!   ([`spec::EvaluationPurpose`]);
//! * which system variants are compared
//!   ([`variants::SystemVariantSpec`] over a B0 → B7 ladder);
//! * which workloads and domains are admissible
//!   ([`workloads::WorkloadSpec`] / [`datasets::DatasetManifest`]);
//! * which metrics count as primary, secondary or diagnostic
//!   ([`metrics::MetricSpec`]);
//! * which ablations are mandatory
//!   ([`ablation::AblationLadder`]);
//! * which gate / replay violations invalidate a result
//!   ([`failure_taxonomy::FailureKind`]);
//! * how runs are aggregated statistically
//!   ([`spec::StatisticalPlan`]);
//! * what counts as evidence sufficient for publication or product
//!   decisions ([`reports::EvaluationSummaryReport`] +
//!   [`scoring::CapabilityProfile`]).
//!
//! A run is the deterministic application of `(variant × workload ×
//! dataset × metric × run_descriptor)` to a `TrialExecutor`. The
//! resulting [`reports::TrialReport`] is content-addressed; an
//! [`ledger::EvaluationRunLedger`] is append-only and hash-chained;
//! replay is byte-identical or the run is invalid; scoring runs
//! exclusively from the ledger and the declared metric specs.
//!
//! ## Hard rules
//!
//! * No platform floats in score / metric / gate hashes — every
//!   gate-relevant scalar is a [`primitives::Fixed`]
//!   (`CanonicalNumber`).
//! * Every keyed collection is a `BTreeMap`; every unordered list is
//!   sorted before hashing.
//! * Wall-clock timestamps are forbidden in the audit pathway.
//! * A `ReplayMismatch` invalidates the run; an invalid run cannot
//!   contribute to any conclusion claim.
//! * `score` MUST NOT recompute metrics — only aggregate from
//!   recorded `MetricObservation`s.

#![deny(missing_docs)]

pub mod ablation;
pub mod calibration;
pub mod cell_substrate_metrics;
pub mod cognition_metrics;
pub mod datasets;
pub mod export;
pub mod failure_taxonomy;
pub mod ground_truth;
pub mod ledger;
pub mod metrics;
pub mod presets;
pub mod primitives;
pub mod replay;
pub mod reports;
pub mod runner;
pub mod scoring;
pub mod spec;
pub mod statistics;
pub mod variants;
pub mod workloads;
pub mod topology_metrics;

#[cfg(feature = "eval-agent")]
pub mod agent_tasks;

pub use ablation::{
    build_ablation_ladder, summarize_ablation, AblationConclusion, AblationLadder, AblationSummary,
    MetricDelta,
};
pub use calibration::{CalibrationLedgerEntry, CalibrationProfile, CalibrationReason};
pub use cell_substrate_metrics::{
    b9_metric_specs, cell_substrate_metric_specs, dual_fabric_stitch_metric_specs,
    CellSubstrateMetricKind,
};
pub use cognition_metrics::{cognition_metric_specs, CognitionMetricKind};
pub use topology_metrics::{topology_metric_specs, TopologyMetricKind};
pub use datasets::{DatasetManifest, DatasetSplit, DatasetSplitKind};
pub use export::{render_json_summary, render_markdown_summary};
pub use failure_taxonomy::{FailureKind, FailureRecord};
pub use ground_truth::{
    AdjudicationPolicy, GroundTruthProfile, LabelSource, LabelSpace, UncertaintyPolicy,
};
pub use ledger::{
    append_to_ledger, init_ledger, EvaluationRunEntry, EvaluationRunLedger, RunStatus,
};
pub use metrics::{
    AggregationPolicy, InvalidationRule, MetricDirection, MetricFamily, MetricObservation,
    MetricSpec,
};
pub use presets::{
    preset_agent_cognition, preset_dual_fabric_stitch, preset_phase_matrix_substrate,
    preset_post_symbolic_ablation, preset_streaming_event_detection, Preset,
};
pub use primitives::{
    fixed_abs_diff, fixed_ge, fixed_le, fixed_sub, EvalError, EvidenceRef, Fixed,
};
pub use replay::{verify_trial_replay, ReplayObservation, ReplayVerificationReport};
pub use reports::{
    ConclusionFlag, DiagnosticRecord, EvaluationSummaryReport, GateObservation, GateObservationSet,
    ReviewerReport, RubricDimension, StatisticalSummary, TrialOutputs, TrialReport, VariantSummary,
    WorkloadSummary,
};
pub use runner::{
    plan_runs, run_trial, EvaluationPlan, EvaluationPlanEntry, RunDescriptor,
    SyntheticTrialExecutor, TrialExecutor,
};
pub use scoring::{
    cognition_uplift, layer_marginal_utility, safety_adjusted_utility, score_capability_profile,
    score_ledger, CapabilityProfile,
};
pub use spec::{
    DeterminismPolicy, EvaluationPurpose, EvaluationSpec, ExperimentDesign, ExperimentKind,
    OutputPolicy, StatisticalPlan,
};
pub use statistics::{
    bootstrap_mean_ci, exact_binomial_ci, paired_mean_diff, BootstrapResult, ConfidenceInterval,
};
pub use variants::{LayerMask, SolverProfile, SystemVariantSpec, VariantLadder};
pub use workloads::{
    SuccessCriterion, TaskBudget, WorkloadFamily, WorkloadSpec, MANDATORY_WORKLOAD_FAMILIES,
};

#[cfg(feature = "eval-agent")]
pub use agent_tasks::{agent_metric_specs, AgentTaskClass};
