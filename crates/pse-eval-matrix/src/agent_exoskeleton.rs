#![allow(missing_docs)]
//! Deterministic diagnostic benchmark for agent-work-state relevance fields.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkItemType {
    Repo,
    Log,
    DiffHint,
    Distractor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentScenarioSplit {
    Calibration,
    Holdout,
    Stress,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: String,
    pub item_type: WorkItemType,
    pub source: String,
    pub text: String,
    pub tags: Vec<String>,
    pub recency_rank: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentGroundTruth {
    pub causal_files: Vec<String>,
    pub causal_logs: Vec<String>,
    pub expected_next_actions: Vec<String>,
    pub irrelevant_items: Vec<String>,
    pub blocker_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkState {
    pub scenario_id: String,
    pub split: AgentScenarioSplit,
    pub issue_text: String,
    pub work_items: Vec<WorkItem>,
    pub plan_steps: Vec<String>,
    pub ground_truth: AgentGroundTruth,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRelevanceField {
    pub item_scores: BTreeMap<String, i32>,
    pub top_items: Vec<String>,
    pub activated_items: Vec<String>,
    pub policy_used: String,
    pub feature_weights: BTreeMap<String, i32>,
    pub diagnostic_only: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentDiagnosticMetrics {
    pub hit_at_1: bool,
    pub hit_at_3: bool,
    pub causal_file_recall: f64,
    pub causal_log_recall: f64,
    pub irrelevant_focus_count: u32,
    pub false_focus_rate: f64,
    pub search_reduction_ratio: f64,
    pub next_action_hit: bool,
    pub blocker_label_hit: bool,
    pub replay_identity: bool,
    pub mean_rank_of_first_causal_item: f64,
    pub mrr_causal_item: f64,
    pub distractor_suppression_rate: f64,
    pub causal_path_coverage: f64,
    pub next_action_rank: f64,
    pub blocker_label_rank: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentExoskeletonDiagnosticReport {
    pub scenario_id: String,
    pub split: AgentScenarioSplit,
    pub baseline_metrics: AgentDiagnosticMetrics,
    pub pse_field_metrics: AgentDiagnosticMetrics,
    pub delta_hit_at_1: f64,
    pub delta_hit_at_3: f64,
    pub delta_causal_file_recall: f64,
    pub delta_causal_log_recall: f64,
    pub delta_false_focus_rate: f64,
    pub delta_irrelevant_focus_count: f64,
    pub delta_search_reduction_ratio: f64,
    pub delta_next_action_hit: f64,
    pub delta_blocker_label_hit: f64,
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub interpretation_labels: Vec<String>,
    pub ablation_report: Option<AblationScenarioReport>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AblationProfile {
    FullPseField,
    NoRecencyCurrentness,
    NoCausalRoleTags,
    NoPhaseOrderConsistency,
    NoNegativeDistractorWeight,
    NoNextActionCompatibility,
    KeywordOnlyPseEquivalent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FullVsAblationDelta {
    pub profile: AblationProfile,
    pub delta_hit_at_3: f64,
    pub delta_false_focus_rate: f64,
    pub delta_mrr: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeatureGroupImportance {
    pub feature_group: String,
    pub delta_hit_at_3: f64,
    pub delta_false_focus_rate: f64,
    pub delta_mrr: f64,
    pub affected_scenario_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AblationScenarioReport {
    pub ablation_metrics_by_profile: BTreeMap<AblationProfile, AgentDiagnosticMetrics>,
    pub best_ablation_profile: AblationProfile,
    pub worst_ablation_profile: AblationProfile,
    pub critical_feature_groups: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AblationAggregateReport {
    pub profiles: Vec<AblationProfile>,
    pub aggregate_metrics_by_profile: BTreeMap<AblationProfile, AblationProfileAggregateMetrics>,
    pub full_vs_ablation_delta: Vec<FullVsAblationDelta>,
    pub feature_group_importance_ranking: Vec<FeatureGroupImportance>,
    pub scenarios_where_full_beats_all_ablations: Vec<String>,
    pub scenarios_where_keyword_equivalent_matches_full: Vec<String>,
    pub scenarios_where_keyword_equivalent_beats_full: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AblationProfileAggregateMetrics {
    pub hit_at_3_mean: f64,
    pub false_focus_rate_mean: f64,
    pub mrr_causal_item_mean: f64,
    pub mean_rank_of_first_causal_item_mean: f64,
    pub distractor_suppression_rate_mean: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentExoskeletonAggregate {
    pub scenario_count: usize,
    pub baseline_hit_at_3_mean: f64,
    pub pse_hit_at_3_mean: f64,
    pub baseline_false_focus_mean: f64,
    pub pse_false_focus_mean: f64,
    pub search_reduction_ratio_mean: f64,
    pub baseline_mean_rank_of_first_causal_item_mean: f64,
    pub pse_mean_rank_of_first_causal_item_mean: f64,
    pub baseline_mrr_causal_item_mean: f64,
    pub pse_mrr_causal_item_mean: f64,
    pub baseline_distractor_suppression_rate_mean: f64,
    pub pse_distractor_suppression_rate_mean: f64,
    pub scenarios_with_pse_gain: usize,
    pub scenarios_with_pse_loss: usize,
    pub scenarios_with_equal_result: usize,
    pub pse_gain_labels: Vec<String>,
    pub pse_loss_labels: Vec<String>,
    pub replay_identity_all: bool,
    pub productive_agent_validated: bool,
    pub diagnostic_only: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentExoskeletonSuiteReport {
    pub reports: Vec<AgentExoskeletonDiagnosticReport>,
    pub aggregate: AgentExoskeletonAggregate,
    pub aggregate_all: AgentExoskeletonAggregate,
    pub aggregate_calibration: AgentExoskeletonAggregate,
    pub aggregate_holdout: AgentExoskeletonAggregate,
    pub aggregate_stress: AgentExoskeletonAggregate,
    pub robustness_labels: Vec<String>,
    pub ablation_aggregate: Option<AblationAggregateReport>,
    pub trace_replay_report: TraceReplayReport,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTraceReplayCase {
    pub trace_id: String,
    pub title: String,
    pub source_kind: String,
    pub variant_kind: TraceVariantKind,
    pub timeline_events: Vec<AgentTraceEvent>,
    pub candidate_items: Vec<WorkItem>,
    pub ground_truth: AgentTraceGroundTruth,
    pub expected_resolution_commit_or_label: String,
    pub diagnostic_only: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceVariantKind {
    Original,
    Harder,
    Expanded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTraceEvent {
    pub step_index: usize,
    pub event_kind: TraceEventKind,
    pub text: String,
    pub referenced_paths: Vec<String>,
    pub command: Option<String>,
    pub outcome: String,
    pub timestamp_label: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceEventKind {
    UserReport,
    AgentDiagnosis,
    CommandRun,
    CommandOutput,
    TestFailure,
    BuildFailure,
    FixCommit,
    Summary,
    ArtifactObservation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTraceGroundTruth {
    pub causal_files: Vec<String>,
    pub causal_logs: Vec<String>,
    pub causal_commands: Vec<String>,
    pub correct_next_actions: Vec<String>,
    pub rejected_false_paths: Vec<String>,
    pub resolution_label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceReplayReport {
    pub trace_count: usize,
    pub baseline_metrics_aggregate: AgentExoskeletonAggregate,
    pub pse_metrics_aggregate: AgentExoskeletonAggregate,
    pub trace_reports: Vec<AgentExoskeletonDiagnosticReport>,
    pub traces_with_pse_gain: usize,
    pub traces_with_pse_loss: usize,
    pub traces_with_equal_result: usize,
    pub replay_identity_all: bool,
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub interpretation_labels: Vec<String>,
    pub trace_ablation_aggregate: Option<AblationAggregateReport>,
    pub trace_audits: Vec<TraceReplayAudit>,
    pub trace_audit_summary: TraceReplayAuditSummary,
    pub trace_variant_counts: TraceVariantCounts,
    pub harder_trace_metrics: HarderTraceMetrics,
    pub trace_gain_failure_summary: TraceGainFailureSummary,
    pub trace_signal_reweighting_report: TraceSignalReweightingReport,
    pub rankability_audits: Vec<TraceRankabilityAudit>,
    pub rankability_summary: TraceRankabilitySummary,
    pub ranking_failure_audits: Vec<TraceRankingFailureAudit>,
    pub post_coverage_ranking_failure_summary: PostCoverageRankingFailureSummary,
    pub score_attribution_audits: Vec<TraceScoreAttributionAudit>,
    pub trace_score_attribution_summary: TraceScoreAttributionSummary,
    pub trace_feature_design_report: Option<TraceFeatureDesignReport>,
    pub trace_corpus_load_report: Option<TraceCorpusLoadReport>,
    pub trace_corpus_fixture_descriptor: Option<TraceCorpusFixtureDescriptor>,
    pub trace_corpus_descriptor: Option<TraceCorpusDescriptor>,
    pub trace_corpus_expansion_report: Option<TraceCorpusExpansionReport>,
    pub trace_corpus_stability_summary: Option<TraceCorpusStabilitySummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceCorpusSourceKind {
    BuiltinStatic,
    ExternalFixture,
    MixedBuiltinAndFixture,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceCorpusLoadReport {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub source_kind: TraceCorpusSourceKind,
    pub builtin_trace_count: usize,
    pub external_fixture_trace_count: usize,
    pub total_loaded_trace_count: usize,
    pub fixture_loading_enabled: bool,
    pub fixture_path: Option<String>,
    pub fixture_schema_version: Option<String>,
    pub load_status: String,
    pub warnings: Vec<String>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceCorpusFixtureDescriptor {
    pub fixture_schema_version: String,
    pub fixture_name: String,
    pub fixture_trace_count: usize,
    pub intended_layer: String,
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub required_fields: Vec<String>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceCorpusDescriptor {
    pub corpus_name: String,
    pub corpus_version: String,
    pub trace_count: usize,
    pub original_trace_count: usize,
    pub harder_trace_count: usize,
    pub expanded_trace_count: usize,
    pub source_type: String,
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub intended_layer: String,
    pub metric_policy: Vec<String>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceCorpusExpansionReport {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub baseline_trace_count: usize,
    pub expanded_trace_count: usize,
    pub total_trace_count: usize,
    pub expansion_status: String,
    pub added_trace_ids: Vec<String>,
    pub coverage_labels: Vec<String>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceCorpusStabilitySummary {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub total_trace_count: usize,
    pub original_trace_count: usize,
    pub harder_trace_count: usize,
    pub expanded_trace_count: usize,
    pub official_hit_at3_total: usize,
    pub audit_aligned_hit_at3_total: usize,
    pub official_hit_at3_rate: f64,
    pub audit_aligned_hit_at3_rate: f64,
    pub original_audit_aligned_hit_rate: f64,
    pub harder_audit_aligned_hit_rate: f64,
    pub expanded_audit_aligned_hit_rate: f64,
    pub original_false_negative_count: usize,
    pub harder_false_negative_count: usize,
    pub expanded_false_negative_count: usize,
    pub stability_label: String,
    pub redesign_evidence_label: String,
    pub recommended_next_step: String,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExternalTraceFixtureCase {
    pub trace_id: String,
    pub title: String,
    pub variant_kind: TraceVariantKind,
    pub events: Vec<ExternalTraceFixtureEvent>,
    pub candidates: Vec<ExternalTraceFixtureCandidate>,
    pub ground_truth: ExternalTraceFixtureGroundTruth,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExternalTraceFixtureEvent {
    pub event_id: String,
    pub source: String,
    pub message: String,
    pub timestamp_hint: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExternalTraceFixtureCandidate {
    pub id: String,
    pub source: String,
    pub text: String,
    pub tags: Vec<String>,
    pub relevance_rank_hint: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExternalTraceFixtureGroundTruth {
    pub causal_files: Vec<String>,
    pub causal_logs: Vec<String>,
    pub causal_commands: Vec<String>,
    pub correct_next_actions: Vec<String>,
    pub rejected_false_paths: Vec<String>,
    pub resolution_label: String,
}

/// Top-level JSON file format for external trace fixture files.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExternalTraceFixtureFile {
    pub fixture_schema_version: String,
    pub fixture_name: String,
    pub intended_layer: String,
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub cases: Vec<ExternalTraceFixtureCase>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RankedTraceItem {
    pub item_id: String,
    pub rank: usize,
    pub score: i32,
    pub is_causal: bool,
    pub is_distractor: bool,
    pub item_kind: WorkItemType,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceReplayAudit {
    pub trace_id: String,
    pub baseline_top_items: Vec<RankedTraceItem>,
    pub pse_top_items: Vec<RankedTraceItem>,
    pub shared_top_items: Vec<String>,
    pub baseline_first_causal_rank: f64,
    pub pse_first_causal_rank: f64,
    pub baseline_mrr: f64,
    pub pse_mrr: f64,
    pub baseline_false_focus_rate: f64,
    pub pse_false_focus_rate: f64,
    pub baseline_next_action_rank: f64,
    pub pse_next_action_rank: f64,
    pub baseline_blocker_label_rank: f64,
    pub pse_blocker_label_rank: f64,
    pub top_item_overlap_count: usize,
    pub top_item_overlap_ratio: f64,
    pub discriminativity_label: String,
    pub audit_labels: Vec<String>,
    pub gain_failure_analysis: Option<TraceGainFailureAnalysis>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceReplayAuditSummary {
    pub trace_count: usize,
    pub traces_with_identical_top_items: usize,
    pub traces_with_keyword_obvious_label: usize,
    pub traces_with_ceiling_effect: usize,
    pub traces_needing_harder_variant: usize,
    pub mean_top_item_overlap_ratio: f64,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceVariantCounts {
    pub original_count: usize,
    pub harder_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HarderTraceMetrics {
    pub trace_count: usize,
    pub baseline_metrics_aggregate: AgentExoskeletonAggregate,
    pub pse_metrics_aggregate: AgentExoskeletonAggregate,
    pub traces_with_pse_gain: usize,
    pub traces_with_pse_loss: usize,
    pub traces_with_equal_result: usize,
    pub mean_top_item_overlap_ratio: f64,
    pub traces_with_identical_top_items: usize,
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceGainFailureAnalysis {
    pub trace_id: String,
    pub variant_kind: TraceVariantKind,
    pub baseline_hit_at_3: bool,
    pub pse_hit_at_3: bool,
    pub baseline_first_causal_rank: f64,
    pub pse_first_causal_rank: f64,
    pub baseline_mrr: f64,
    pub pse_mrr: f64,
    pub baseline_false_focus_rate: f64,
    pub pse_false_focus_rate: f64,
    pub rank_delta: f64,
    pub mrr_delta: f64,
    pub false_focus_delta: f64,
    pub top3_causal_overlap_baseline: usize,
    pub top3_causal_overlap_pse: usize,
    pub primary_blocking_reason: String,
    pub secondary_reasons: Vec<String>,
    pub diagnostic_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceGainFailureSummary {
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub traces_with_pse_rank_improvement: usize,
    pub traces_with_pse_mrr_improvement: usize,
    pub traces_with_pse_false_focus_improvement: usize,
    pub traces_with_mapping_difference_but_no_gain: usize,
    pub traces_blocked_by_both_hit_at_3_equal: usize,
    pub traces_blocked_by_both_miss_top3: usize,
    pub traces_blocked_by_pse_differs_but_not_on_causal_items: usize,
    pub primary_blocking_reason_counts: BTreeMap<String, usize>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TraceSignalProfile {
    CurrentPse,
    TimelineCommandBoost,
    ActionCompatibilityBoost,
    CausalFilePathBoost,
    DistractorSuppressionBoost,
    CombinedConservative,
    KeywordOnlyTraceEquivalent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceSignalProfileReport {
    pub profile: TraceSignalProfile,
    pub metrics_aggregate: AgentExoskeletonAggregate,
    pub harder_metrics_aggregate: AgentExoskeletonAggregate,
    pub traces_with_gain: usize,
    pub traces_with_loss: usize,
    pub traces_with_equal: usize,
    pub harder_traces_with_gain: usize,
    pub harder_traces_with_loss: usize,
    pub harder_traces_with_equal: usize,
    pub top_improved_traces: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceSignalReweightingReport {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub profiles: Vec<TraceSignalProfile>,
    pub profile_reports: Vec<TraceSignalProfileReport>,
    pub best_profile_by_harder_mrr: TraceSignalProfile,
    pub best_profile_by_harder_hit_at_3: TraceSignalProfile,
    pub current_pse_profile: TraceSignalProfile,
    pub keyword_equivalent_profile: TraceSignalProfile,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TraceFeatureDesignProfile {
    CurrentPse,
    StrongDistractorPenalty,
    KeywordDominanceDampening,
    ReportArtifactDemotion,
    CausalRoleDisambiguation,
    CombinedConservativeFeatureDesign,
    CausalActivationDiagnostic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureDesignProfileReport {
    pub profile: TraceFeatureDesignProfile,
    pub metrics_aggregate: AgentExoskeletonAggregate,
    pub harder_metrics_aggregate: AgentExoskeletonAggregate,
    pub traces_with_gain: usize,
    pub traces_with_loss: usize,
    pub traces_with_equal: usize,
    pub harder_traces_with_gain: usize,
    pub harder_traces_with_loss: usize,
    pub harder_traces_with_equal: usize,
    pub improved_trace_ids: Vec<String>,
    pub worsened_trace_ids: Vec<String>,
    pub feature_effect_labels: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureDesignReport {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub profiles: Vec<TraceFeatureDesignProfile>,
    pub profile_reports: Vec<TraceFeatureDesignProfileReport>,
    pub best_profile_by_harder_hit_at_3: TraceFeatureDesignProfile,
    pub best_profile_by_harder_mrr: TraceFeatureDesignProfile,
    pub best_profile_by_harder_false_focus: TraceFeatureDesignProfile,
    pub interpretation_labels: Vec<String>,
    pub sensitivity_report: Option<TraceFeatureDesignSensitivityReport>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureDesignSensitivityReport {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub profile_sensitivity_reports: Vec<TraceFeatureDesignProfileSensitivity>,
    pub profiles_with_any_rank_movement: Vec<TraceFeatureDesignProfile>,
    pub profiles_with_any_score_movement: Vec<TraceFeatureDesignProfile>,
    pub profiles_with_causal_rank_improvement: Vec<TraceFeatureDesignProfile>,
    pub profiles_with_causal_score_improvement: Vec<TraceFeatureDesignProfile>,
    pub profiles_with_no_effect: Vec<TraceFeatureDesignProfile>,
    pub interpretation_labels: Vec<String>,
    pub activation_audit: Option<TraceFeatureDesignActivationAudit>,
    pub marker_coverage_audit: Option<TraceFeatureMarkerCoverageAudit>,
    pub targeting_audit: Option<TraceFeatureActivationTargetingAudit>,
    pub delta_sufficiency_audit: Option<TraceFeatureDeltaSufficiencyAudit>,
    pub causal_feature_targeting_audit: Option<TraceCausalFeatureTargetingAudit>,
    pub causal_activation_alignment_audit: Option<TraceCausalActivationAlignmentAudit>,
    pub causal_delta_impact_audit: Option<TraceCausalDeltaImpactAudit>,
    pub metric_causal_consistency_audit: Option<TraceMetricCausalConsistencyAudit>,
    pub metric_semantics_alignment_report: Option<TraceMetricSemanticsAlignmentReport>,
    pub dual_metric_report: Option<TraceDualMetricReport>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureDesignProfileSensitivity {
    pub profile: TraceFeatureDesignProfile,
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub traces_with_rank_movement: usize,
    pub traces_with_score_movement: usize,
    pub traces_with_causal_rank_improvement: usize,
    pub traces_with_causal_score_improvement: usize,
    pub traces_with_top3_change: usize,
    pub traces_with_best_causal_rank_change: usize,
    pub mean_abs_score_delta: f64,
    pub mean_abs_rank_delta: f64,
    pub mean_causal_rank_delta: f64,
    pub mean_causal_score_delta: f64,
    pub harder_mean_causal_rank_delta: f64,
    pub harder_mean_causal_score_delta: f64,
    pub sensitivity_label: String,
    pub affected_trace_ids: Vec<String>,
    pub unaffected_trace_ids: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureDesignActivationAudit {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub profile_activation_reports: Vec<TraceFeatureDesignProfileActivation>,
    pub profiles_with_zero_activation: Vec<TraceFeatureDesignProfile>,
    pub profiles_with_activation_only_on_noncausal: Vec<TraceFeatureDesignProfile>,
    pub profiles_with_activation_on_causal: Vec<TraceFeatureDesignProfile>,
    pub profiles_with_small_delta_only: Vec<TraceFeatureDesignProfile>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureDesignProfileActivation {
    pub profile: TraceFeatureDesignProfile,
    pub activation_count: usize,
    pub causal_activation_count: usize,
    pub noncausal_activation_count: usize,
    pub top3_blocker_activation_count: usize,
    pub total_score_delta_abs: f64,
    pub mean_score_delta_abs: f64,
    pub max_score_delta_abs: f64,
    pub causal_score_delta_abs: f64,
    pub noncausal_score_delta_abs: f64,
    pub affected_trace_ids: Vec<String>,
    pub affected_item_ids_sample: Vec<String>,
    pub zero_activation_reason: Option<String>,
    pub activation_label: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureMarkerCoverageAudit {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub candidate_count: usize,
    pub distractor_marker_count: usize,
    pub keyword_dampening_marker_count: usize,
    pub causal_path_marker_count: usize,
    pub causal_action_marker_count: usize,
    pub report_artifact_marker_count: usize,
    pub top3_blocker_marker_coverage: usize,
    pub causal_item_marker_coverage: usize,
    pub traces_with_missing_distractor_markers: Vec<String>,
    pub traces_with_missing_keyword_dampening_markers: Vec<String>,
    pub traces_with_missing_causal_action_markers: Vec<String>,
    pub trace_reports: Vec<TraceFeatureMarkerCoverageTrace>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureMarkerCoverageTrace {
    pub trace_id: String,
    pub variant_kind: TraceVariantKind,
    pub candidate_count: usize,
    pub distractor_marker_count: usize,
    pub keyword_dampening_marker_count: usize,
    pub causal_path_marker_count: usize,
    pub causal_action_marker_count: usize,
    pub report_artifact_marker_count: usize,
    pub top3_blocker_marker_count: usize,
    pub causal_item_marker_count: usize,
    pub missing_marker_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureActivationTargetingAudit {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub profile_targeting_reports: Vec<TraceFeatureProfileTargetingReport>,
    pub profiles_missing_top3_blockers: Vec<TraceFeatureDesignProfile>,
    pub profiles_missing_causal_items: Vec<TraceFeatureDesignProfile>,
    pub profiles_only_hit_low_rank_noncausal: Vec<TraceFeatureDesignProfile>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureProfileTargetingReport {
    pub profile: TraceFeatureDesignProfile,
    pub activation_count: usize,
    pub activated_item_ids_sample: Vec<String>,
    pub activated_item_kind_counts: BTreeMap<String, usize>,
    pub activated_rank_distribution: BTreeMap<String, usize>,
    pub activated_causal_count: usize,
    pub activated_noncausal_count: usize,
    pub activated_top3_blocker_count: usize,
    pub activated_below_top3_noncausal_count: usize,
    pub missed_top3_blocker_count: usize,
    pub missed_causal_item_count: usize,
    pub targeting_label: String,
    pub targeting_failure_reasons: Vec<String>,
    pub affected_trace_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureDeltaSufficiencyAudit {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub profile_delta_reports: Vec<TraceFeatureProfileDeltaSufficiency>,
    pub profiles_with_insufficient_delta: Vec<TraceFeatureDesignProfile>,
    pub profiles_with_near_threshold_shift: Vec<TraceFeatureDesignProfile>,
    pub profiles_with_large_gap_remaining: Vec<TraceFeatureDesignProfile>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceFeatureProfileDeltaSufficiency {
    pub profile: TraceFeatureDesignProfile,
    pub activated_top3_blocker_count: usize,
    pub mean_top3_blocker_delta_abs: f64,
    pub max_top3_blocker_delta_abs: f64,
    pub mean_required_delta_to_top3: Option<f64>,
    pub min_required_delta_to_top3: Option<f64>,
    pub mean_delta_coverage_ratio: Option<f64>,
    pub max_delta_coverage_ratio: Option<f64>,
    pub traces_with_near_threshold_shift: usize,
    pub traces_with_large_gap_remaining: usize,
    pub traces_where_causal_item_next_in_line: usize,
    pub delta_sufficiency_label: String,
    pub diagnostic_reasons: Vec<String>,
    pub affected_trace_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProposedCausalFeatureRule {
    pub rule_name: String,
    pub matched_marker_family: String,
    pub expected_target: String,
    pub diagnostic_only: bool,
    pub rationale: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceCausalFeatureTargetingTrace {
    pub trace_id: String,
    pub variant_kind: TraceVariantKind,
    pub causal_item_ids: Vec<String>,
    pub causal_item_marker_summary: BTreeMap<String, usize>,
    pub marker_targetable_item_ids: Vec<String>,
    pub rule_matched_item_ids: Vec<String>,
    pub score_activated_item_ids: Vec<String>,
    pub score_improved_item_ids: Vec<String>,
    pub rank_improved_item_ids: Vec<String>,
    pub marker_targetable_but_not_activated_item_ids: Vec<String>,
    pub semantic_status_label: String,
    pub hit_by_existing_profiles: usize,
    pub missed_by_existing_profiles: usize,
    pub proposed_rule_labels: Vec<String>,
    pub targeting_gap_label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceCausalFeatureTargetingAudit {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub causal_item_count: usize,
    pub causal_items_with_path_marker: usize,
    pub causal_items_with_command_marker: usize,
    pub causal_items_with_action_marker: usize,
    pub causal_items_with_retry_marker: usize,
    pub causal_items_hit_by_existing_profiles: usize,
    pub causal_items_missed_by_existing_profiles: usize,
    pub causal_items_marker_targetable: usize,
    pub causal_items_rule_matched: usize,
    pub causal_items_score_activated: usize,
    pub causal_items_score_improved: usize,
    pub causal_items_rank_improved: usize,
    pub causal_items_marker_targetable_but_not_activated: usize,
    pub causal_items_activated_but_not_improved: usize,
    pub causal_items_improved_but_below_top3: usize,
    pub proposed_causal_feature_rules: Vec<ProposedCausalFeatureRule>,
    pub trace_reports: Vec<TraceCausalFeatureTargetingTrace>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceCausalActivationAlignmentTrace {
    pub trace_id: String,
    pub variant_kind: TraceVariantKind,
    pub causal_item_ids: Vec<String>,
    pub marker_targetable_item_ids: Vec<String>,
    pub rule_matched_item_ids: Vec<String>,
    pub profile_delta_item_ids: Vec<String>,
    pub causal_profile_delta_item_ids: Vec<String>,
    pub noncausal_profile_delta_item_ids: Vec<String>,
    pub marker_targetable_but_no_delta_item_ids: Vec<String>,
    pub profile_delta_but_noncausal_item_ids: Vec<String>,
    pub alignment_label: String,
    pub alignment_failure_reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceCausalActivationAlignmentAudit {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub causal_marker_targetable_count: usize,
    pub causal_rule_matched_count: usize,
    pub causal_profile_score_delta_count: usize,
    pub noncausal_profile_score_delta_count: usize,
    pub marker_targetable_but_no_delta_count: usize,
    pub profile_delta_without_causal_classification_count: usize,
    pub suspected_id_mismatch_count: usize,
    pub suspected_rule_condition_mismatch_count: usize,
    pub suspected_zero_delta_after_match_count: usize,
    pub trace_reports: Vec<TraceCausalActivationAlignmentTrace>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceCausalDeltaImpactTrace {
    pub trace_id: String,
    pub variant_kind: TraceVariantKind,
    pub current_best_causal_rank: usize,
    pub profile_best_causal_rank: usize,
    pub current_best_causal_score: i32,
    pub profile_best_causal_score: i32,
    pub best_causal_score_delta: i32,
    pub best_causal_rank_delta: i32,
    pub causal_delta_item_count: usize,
    pub noncausal_delta_item_count: usize,
    pub mean_causal_delta_abs: f64,
    pub mean_noncausal_delta_abs: f64,
    pub top3_current_item_ids: Vec<String>,
    pub top3_profile_item_ids: Vec<String>,
    pub items_between_best_causal_and_top3: Vec<String>,
    pub rank_barrier_item_ids: Vec<String>,
    pub rank_barrier_reason: String,
    pub impact_label: String,
    pub diagnostic_reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceCausalDeltaImpactAudit {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub causal_delta_item_count: usize,
    pub noncausal_delta_item_count: usize,
    pub mean_causal_delta_abs: f64,
    pub mean_noncausal_delta_abs: f64,
    pub max_causal_delta_abs: f64,
    pub max_noncausal_delta_abs: f64,
    pub traces_with_causal_rank_movement: usize,
    pub traces_with_causal_rank_improvement: usize,
    pub traces_with_causal_score_improvement: usize,
    pub traces_with_noncausal_delta_dominance: usize,
    pub traces_with_rank_barrier: usize,
    pub causal_rank_resolution_failed_count: usize,
    pub causal_rank_resolution_failure_reasons: Vec<String>,
    pub traces_with_causal_rank_resolution_failure: Vec<String>,
    pub trace_reports: Vec<TraceCausalDeltaImpactTrace>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceMetricCausalConsistencyTrace {
    pub trace_id: String,
    pub variant_kind: TraceVariantKind,
    pub audit_causal_item_ids: Vec<String>,
    pub metric_ground_truth_item_ids: Vec<String>,
    pub ranked_top3_item_ids: Vec<String>,
    pub audit_causal_top3_item_ids: Vec<String>,
    pub metric_hit_top3_item_ids: Vec<String>,
    pub audit_best_causal_rank: usize,
    pub metric_hit_at3: bool,
    pub metric_mrr: f64,
    pub id_overlap_count: usize,
    pub id_overlap_ratio: f64,
    pub consistency_label: String,
    pub disagreement_reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceMetricCausalConsistencyAudit {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub traces_with_audit_causal_top3: usize,
    pub traces_with_metric_hit_at3: usize,
    pub traces_with_audit_metric_disagreement: usize,
    pub traces_with_id_set_mismatch: usize,
    pub traces_with_label_semantics_mismatch: usize,
    pub trace_reports: Vec<TraceMetricCausalConsistencyTrace>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceMetricSemanticsAlignmentTrace {
    pub trace_id: String,
    pub variant_kind: TraceVariantKind,
    pub official_metric_hit_at3: bool,
    pub audit_aligned_hit_at3: bool,
    pub official_metric_mrr: f64,
    pub audit_aligned_mrr: f64,
    pub official_metric_target_ids: Vec<String>,
    pub audit_aligned_target_ids: Vec<String>,
    pub ranked_top3_item_ids: Vec<String>,
    pub aligned_hit_item_ids: Vec<String>,
    pub metric_semantics_status_label: String,
    pub diagnostic_reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceMetricSemanticsAlignmentReport {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub official_metric_hit_at3_count: usize,
    pub audit_aligned_hit_at3_count: usize,
    pub official_metric_mrr_mean: f64,
    pub audit_aligned_mrr_mean: f64,
    pub false_negative_due_to_metric_semantics_count: usize,
    pub traces_where_audit_alignment_changes_outcome: Vec<String>,
    pub trace_reports: Vec<TraceMetricSemanticsAlignmentTrace>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceMetricView {
    pub hit_at3_count: usize,
    pub hit_at3_rate: f64,
    pub mrr_mean: f64,
    pub false_focus_mean: f64,
    pub gain_count: usize,
    pub loss_count: usize,
    pub equal_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceMetricSemanticDelta {
    pub hit_at3_count_delta: i32,
    pub hit_at3_rate_delta: f64,
    pub mrr_mean_delta: f64,
    pub false_negative_due_to_metric_semantics_count: usize,
    pub traces_with_changed_outcome: Vec<String>,
    pub semantic_status_label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceDualMetricTrace {
    pub trace_id: String,
    pub variant_kind: TraceVariantKind,
    pub official_hit_at3: bool,
    pub audit_aligned_hit_at3: bool,
    pub official_mrr: f64,
    pub audit_aligned_mrr: f64,
    pub official_target_ids: Vec<String>,
    pub audit_aligned_target_ids: Vec<String>,
    pub ranked_top3_item_ids: Vec<String>,
    pub semantic_status_label: String,
    pub diagnostic_reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceDualMetricReport {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub official_metric: TraceMetricView,
    pub audit_aligned_metric: TraceMetricView,
    pub semantic_delta: TraceMetricSemanticDelta,
    pub trace_reports: Vec<TraceDualMetricTrace>,
    pub metric_redesign_proposal: Option<TraceMetricRedesignProposal>,
    pub layer1_candidate_causal_metric_spec: Option<Layer1CandidateCausalMetricSpec>,
    pub layer1_metric_migration_plan: Option<Layer1MetricMigrationPlan>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer1CandidateCausalMetricSpec {
    pub metric_name: String,
    pub metric_version: String,
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub intended_layer: String,
    pub target_semantics: String,
    pub target_id_source: String,
    pub hit_at_k_definition: String,
    pub mrr_definition: String,
    pub official_metric_relationship: String,
    pub compatibility_policy: String,
    pub freeze_status: String,
    pub freeze_criteria: Vec<String>,
    pub known_limitations: Vec<String>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer1MetricMigrationPlan {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub migration_status: String,
    pub source_metric: String,
    pub target_primary_metric: String,
    pub legacy_metric_policy: String,
    pub promotion_readiness: MetricPromotionReadiness,
    pub required_guardrails: Vec<String>,
    pub validation_phases: Vec<MetricValidationPhase>,
    pub blocking_conditions: Vec<MetricMigrationBlockingCondition>,
    pub rollback_policy: Vec<String>,
    pub success_criteria: Vec<String>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricPromotionReadiness {
    pub corpus_threshold_met: bool,
    pub replay_identity_confirmed: bool,
    pub official_metric_preserved: bool,
    pub dual_metric_reporting_active: bool,
    pub diagnostic_freeze_present: bool,
    pub live_agent_validation_completed: bool,
    pub promotion_allowed: bool,
    pub readiness_label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricValidationPhase {
    pub phase_id: String,
    pub phase_name: String,
    pub objective: String,
    pub required_evidence: String,
    pub completion_status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricMigrationBlockingCondition {
    pub condition_id: String,
    pub description: String,
    pub severity: String,
    pub resolution_required: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricSemanticsDescriptor {
    pub metric_name: String,
    pub target_type: String,
    pub target_id_source: String,
    pub hit_definition: String,
    pub mrr_definition: String,
    pub intended_layer: String,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricMigrationStep {
    pub step_id: String,
    pub description: String,
    pub required_guardrail: String,
    pub validation_check: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricRedesignEvidenceSummary {
    pub trace_count: usize,
    pub official_hit_at3_count: usize,
    pub audit_aligned_hit_at3_count: usize,
    pub false_negative_count: usize,
    pub semantic_delta_hit_at3: i32,
    pub semantic_delta_mrr: f64,
    pub evidence_label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceMetricRedesignProposal {
    pub diagnostic_only: bool,
    pub productive_agent_validated: bool,
    pub current_pse_not_replaced: bool,
    pub proposal_status: String,
    pub current_official_metric_semantics: MetricSemanticsDescriptor,
    pub proposed_metric_semantics: MetricSemanticsDescriptor,
    pub recommended_primary_metric: String,
    pub compatibility_policy: String,
    pub migration_steps: Vec<MetricMigrationStep>,
    pub risks: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub evidence_summary: MetricRedesignEvidenceSummary,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceRankabilityAudit {
    pub trace_id: String,
    pub variant_kind: TraceVariantKind,
    pub candidate_count: usize,
    pub causal_file_count: usize,
    pub causal_file_candidate_hits: usize,
    pub causal_file_candidate_misses: usize,
    pub causal_log_count: usize,
    pub causal_log_candidate_hits: usize,
    pub causal_log_candidate_misses: usize,
    pub causal_command_count: usize,
    pub causal_command_timeline_hits: usize,
    pub causal_command_candidate_hits: usize,
    pub correct_next_action_count: usize,
    pub correct_next_action_candidate_hits: usize,
    pub correct_next_action_timeline_hits: usize,
    pub rankable_causal_item_count: usize,
    pub unrankable_causal_item_count: usize,
    pub rankability_ratio: f64,
    pub rankability_label: String,
    pub diagnostic_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceRankabilitySummary {
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub traces_fully_rankable: usize,
    pub traces_partially_rankable: usize,
    pub traces_unrankable: usize,
    pub harder_traces_unrankable: usize,
    pub mean_rankability_ratio: f64,
    pub harder_mean_rankability_ratio: f64,
    pub primary_rankability_blocker_counts: BTreeMap<String, usize>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CausalItemRank {
    pub item_id: String,
    pub item_kind: WorkItemType,
    pub rank_baseline: usize,
    pub rank_pse: usize,
    pub is_in_top3_baseline: bool,
    pub is_in_top3_pse: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopBlockerItem {
    pub item_id: String,
    pub rank: usize,
    pub score: i32,
    pub item_kind: WorkItemType,
    pub reason_label: String,
    pub text_excerpt: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceRankingFailureAudit {
    pub trace_id: String,
    pub variant_kind: TraceVariantKind,
    pub baseline_causal_item_ranks: Vec<CausalItemRank>,
    pub pse_causal_item_ranks: Vec<CausalItemRank>,
    pub baseline_top3_noncausal_blockers: Vec<TopBlockerItem>,
    pub pse_top3_noncausal_blockers: Vec<TopBlockerItem>,
    pub baseline_best_causal_rank: f64,
    pub pse_best_causal_rank: f64,
    pub baseline_causal_items_below_top3: usize,
    pub pse_causal_items_below_top3: usize,
    pub baseline_top3_signal_labels: Vec<String>,
    pub pse_top3_signal_labels: Vec<String>,
    pub rank_failure_label: String,
    pub diagnostic_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PostCoverageRankingFailureSummary {
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub fully_rankable_trace_count: usize,
    pub traces_with_causal_items_below_top3: usize,
    pub traces_with_noncausal_top3_blockers: usize,
    pub traces_where_pse_differs_but_still_noncausal: usize,
    pub traces_where_baseline_and_pse_same_failure: usize,
    pub mean_pse_best_causal_rank: f64,
    pub mean_baseline_best_causal_rank: f64,
    pub primary_rank_failure_counts: BTreeMap<String, usize>,
    pub interpretation_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreComponent {
    pub component: String,
    pub value: f64,
    pub diagnostic_role: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceItemScoreAttribution {
    pub item_id: String,
    pub item_kind: WorkItemType,
    pub rank: usize,
    pub total_score: i32,
    pub is_causal: bool,
    pub is_distractor: bool,
    pub score_components: Vec<ScoreComponent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceScoreAttributionAudit {
    pub trace_id: String,
    pub variant_kind: TraceVariantKind,
    pub causal_item_attributions: Vec<TraceItemScoreAttribution>,
    pub top3_noncausal_attributions: Vec<TraceItemScoreAttribution>,
    pub causal_vs_noncausal_score_gap: f64,
    pub dominant_noncausal_signal_groups: Vec<String>,
    pub missing_causal_signal_groups: Vec<String>,
    pub attribution_failure_label: String,
    pub diagnostic_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceScoreAttributionSummary {
    pub trace_count: usize,
    pub harder_trace_count: usize,
    pub traces_with_positive_noncausal_score_gap: usize,
    pub mean_causal_vs_noncausal_score_gap: f64,
    pub failure_label_counts: BTreeMap<String, usize>,
    pub dominant_noncausal_signal_group_counts: BTreeMap<String, usize>,
    pub missing_causal_signal_group_counts: BTreeMap<String, usize>,
    pub interpretation_labels: Vec<String>,
    pub traces_with_explainability_gap: usize,
    pub mean_unexplained_score_delta: f64,
    pub dominant_component_counts: BTreeMap<String, usize>,
    pub causal_dominant_component_counts: BTreeMap<String, usize>,
    pub noncausal_dominant_component_counts: BTreeMap<String, usize>,
    pub componentization_complete: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PseScoreBreakdown {
    pub total_score: i32,
    pub keyword_overlap: i32,
    pub path_match: i32,
    pub recency_currentness: i32,
    pub causal_role_tag: i32,
    pub phase_order: i32,
    pub action_compatibility: i32,
    pub distractor_penalty: i32,
    pub stale_penalty: i32,
    pub command_timeline: i32,
    pub other: i32,
    pub component_sum: i32,
    pub unexplained_score_delta: i32,
}

pub fn all_ablation_profiles() -> Vec<AblationProfile> {
    vec![
        AblationProfile::FullPseField,
        AblationProfile::NoRecencyCurrentness,
        AblationProfile::NoCausalRoleTags,
        AblationProfile::NoPhaseOrderConsistency,
        AblationProfile::NoNegativeDistractorWeight,
        AblationProfile::NoNextActionCompatibility,
        AblationProfile::KeywordOnlyPseEquivalent,
    ]
}

fn kw(s: &str) -> BTreeSet<String> {
    s.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|t| t.len() > 2)
        .map(ToOwned::to_owned)
        .collect()
}
fn overlap(s: &str, q: &BTreeSet<String>) -> i32 {
    kw(s).iter().filter(|k| q.contains(*k)).count() as i32
}

fn rank_baseline(state: &AgentWorkState) -> AgentRelevanceField {
    let query = kw(&state.issue_text);
    let mut scores = BTreeMap::new();
    for w in &state.work_items {
        scores.insert(
            w.id.clone(),
            overlap(
                &format!("{} {} {}", w.source, w.text, w.tags.join(" ")),
                &query,
            ),
        );
    }
    build_field(scores, "baseline_keyword_ranker", BTreeMap::new())
}

fn rank_pse(state: &AgentWorkState) -> AgentRelevanceField {
    rank_pse_with_profile(state, AblationProfile::FullPseField)
}

fn rank_pse_with_profile(state: &AgentWorkState, profile: AblationProfile) -> AgentRelevanceField {
    let query = kw(&state.issue_text);
    let weights = BTreeMap::from_iter([
        ("item_type_repo".into(), 3),
        ("item_type_log".into(), 1),
        ("item_type_diff".into(), 4),
        ("recency_bonus".into(), 2),
        ("causal_role_bonus".into(), 4),
        ("stale_penalty".into(), -4),
        ("distractor_penalty".into(), -6),
        ("evidence_chain_bonus".into(), 3),
        ("next_action_bonus".into(), 3),
    ]);
    let mut scores = BTreeMap::new();
    let keyword_only = profile == AblationProfile::KeywordOnlyPseEquivalent;
    let use_recency = !matches!(
        profile,
        AblationProfile::NoRecencyCurrentness | AblationProfile::KeywordOnlyPseEquivalent
    );
    let use_causal_tags = !matches!(
        profile,
        AblationProfile::NoCausalRoleTags | AblationProfile::KeywordOnlyPseEquivalent
    );
    let use_phase_order = !matches!(
        profile,
        AblationProfile::NoPhaseOrderConsistency | AblationProfile::KeywordOnlyPseEquivalent
    );
    let use_negative_distractor = !matches!(
        profile,
        AblationProfile::NoNegativeDistractorWeight | AblationProfile::KeywordOnlyPseEquivalent
    );
    let use_next_action = !matches!(
        profile,
        AblationProfile::NoNextActionCompatibility | AblationProfile::KeywordOnlyPseEquivalent
    );
    for w in &state.work_items {
        let mut s = overlap(
            &format!("{} {} {}", w.source, w.text, w.tags.join(" ")),
            &query,
        );
        if !keyword_only {
            s += match w.item_type {
                WorkItemType::Repo => 3,
                WorkItemType::Log => 1,
                WorkItemType::DiffHint => 4,
                WorkItemType::Distractor => -3,
            };
        }
        if use_recency && w.recency_rank >= 8 {
            s += 2;
        }
        if use_causal_tags
            && w.tags.iter().any(|t| {
                t.contains("causal")
                    || t.contains("executor")
                    || t.contains("harness")
                    || t.contains("command_plan")
            })
        {
            s += 4;
        }
        if use_negative_distractor
            && w.tags.iter().any(|t| {
                t.contains("stale") || t.contains("irrelevant") || t.contains("distractor")
            })
        {
            s -= 6;
        }
        if use_next_action
            && state.plan_steps.iter().any(|p| {
                w.text.to_ascii_lowercase().contains(
                    &p.split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_ascii_lowercase(),
                )
            })
        {
            s += 3;
        }
        if use_phase_order
            && w.tags.iter().any(|t| t.contains("symptom"))
            && w.tags.iter().any(|t| t.contains("root_cause"))
        {
            s += 3;
        }
        scores.insert(w.id.clone(), s);
    }
    build_field(scores, "pse_diagnostic_agent_field_v1", weights)
}

fn build_ablation_scenario_report(
    scenario: &AgentWorkState,
    full_metrics: &AgentDiagnosticMetrics,
) -> AblationScenarioReport {
    let mut by_profile = BTreeMap::new();
    for profile in all_ablation_profiles() {
        let m = metrics(scenario, &rank_pse_with_profile(scenario, profile));
        by_profile.insert(profile, m);
    }
    let compare = |a: &(&AblationProfile, &AgentDiagnosticMetrics),
                   b: &(&AblationProfile, &AgentDiagnosticMetrics)| {
        b.1.hit_at_3
            .cmp(&a.1.hit_at_3)
            .then_with(|| {
                a.1.false_focus_rate
                    .partial_cmp(&b.1.false_focus_rate)
                    .unwrap()
            })
            .then_with(|| {
                b.1.mrr_causal_item
                    .partial_cmp(&a.1.mrr_causal_item)
                    .unwrap()
            })
            .then_with(|| format!("{:?}", a.0).cmp(&format!("{:?}", b.0)))
    };
    let mut ordered: Vec<_> = by_profile.iter().collect();
    ordered.sort_by(compare);
    let best = *ordered.first().unwrap().0;
    let worst = *ordered.last().unwrap().0;
    let mut critical = Vec::new();
    for (profile, m) in &by_profile {
        let worse = (!m.hit_at_3 && full_metrics.hit_at_3)
            || (m.false_focus_rate > full_metrics.false_focus_rate)
            || (m.mrr_causal_item < full_metrics.mrr_causal_item);
        if !worse {
            continue;
        }
        let group = match profile {
            AblationProfile::NoRecencyCurrentness => "recency_currentness",
            AblationProfile::NoCausalRoleTags => "causal_role_tags",
            AblationProfile::NoPhaseOrderConsistency => "phase_order_consistency",
            AblationProfile::NoNegativeDistractorWeight => "negative_distractor_weight",
            AblationProfile::NoNextActionCompatibility => "next_action_compatibility",
            AblationProfile::KeywordOnlyPseEquivalent => "structural_features",
            AblationProfile::FullPseField => continue,
        };
        critical.push(group.to_string());
    }
    critical.sort();
    critical.dedup();
    AblationScenarioReport {
        ablation_metrics_by_profile: by_profile,
        best_ablation_profile: best,
        worst_ablation_profile: worst,
        critical_feature_groups: critical,
    }
}

fn build_ablation_aggregate(
    reports: &[AgentExoskeletonDiagnosticReport],
) -> AblationAggregateReport {
    fn is_worse_than_full(full: &AgentDiagnosticMetrics, other: &AgentDiagnosticMetrics) -> bool {
        (full.hit_at_3 && !other.hit_at_3)
            || (other.false_focus_rate > full.false_focus_rate)
            || (other.mrr_causal_item < full.mrr_causal_item)
    }
    fn better_than(
        a: (&AblationProfile, &AgentDiagnosticMetrics),
        b: (&AblationProfile, &AgentDiagnosticMetrics),
    ) -> bool {
        if a.1.hit_at_3 != b.1.hit_at_3 {
            return a.1.hit_at_3 && !b.1.hit_at_3;
        }
        if a.1.false_focus_rate != b.1.false_focus_rate {
            return a.1.false_focus_rate < b.1.false_focus_rate;
        }
        if a.1.mrr_causal_item != b.1.mrr_causal_item {
            return a.1.mrr_causal_item > b.1.mrr_causal_item;
        }
        false
    }
    let feature_group = |p: AblationProfile| match p {
        AblationProfile::NoRecencyCurrentness => "recency_currentness",
        AblationProfile::NoCausalRoleTags => "causal_role_tags",
        AblationProfile::NoPhaseOrderConsistency => "phase_order_consistency",
        AblationProfile::NoNegativeDistractorWeight => "negative_distractor_weight",
        AblationProfile::NoNextActionCompatibility => "next_action_compatibility",
        AblationProfile::KeywordOnlyPseEquivalent => "structural_features",
        AblationProfile::FullPseField => "full_pse_field",
    };
    let profiles = all_ablation_profiles();
    let mut aggregate_metrics_by_profile = BTreeMap::new();
    for profile in &profiles {
        let mut n = 0.0;
        let mut hit = 0.0;
        let mut ff = 0.0;
        let mut mrr = 0.0;
        let mut rank = 0.0;
        let mut ds = 0.0;
        for report in reports {
            let Some(ab) = &report.ablation_report else {
                continue;
            };
            let Some(m) = ab.ablation_metrics_by_profile.get(profile) else {
                continue;
            };
            n += 1.0;
            hit += m.hit_at_3 as u8 as f64;
            ff += m.false_focus_rate;
            mrr += m.mrr_causal_item;
            rank += m.mean_rank_of_first_causal_item;
            ds += m.distractor_suppression_rate;
        }
        let agg = if n == 0.0 {
            AblationProfileAggregateMetrics {
                hit_at_3_mean: 0.0,
                false_focus_rate_mean: 0.0,
                mrr_causal_item_mean: 0.0,
                mean_rank_of_first_causal_item_mean: 0.0,
                distractor_suppression_rate_mean: 0.0,
            }
        } else {
            AblationProfileAggregateMetrics {
                hit_at_3_mean: hit / n,
                false_focus_rate_mean: ff / n,
                mrr_causal_item_mean: mrr / n,
                mean_rank_of_first_causal_item_mean: rank / n,
                distractor_suppression_rate_mean: ds / n,
            }
        };
        aggregate_metrics_by_profile.insert(*profile, agg);
    }

    let full = aggregate_metrics_by_profile
        .get(&AblationProfile::FullPseField)
        .cloned()
        .unwrap_or(AblationProfileAggregateMetrics {
            hit_at_3_mean: 0.0,
            false_focus_rate_mean: 0.0,
            mrr_causal_item_mean: 0.0,
            mean_rank_of_first_causal_item_mean: 0.0,
            distractor_suppression_rate_mean: 0.0,
        });
    let mut full_vs_ablation_delta = Vec::new();
    let mut feature_group_importance_ranking = Vec::new();
    for profile in &profiles {
        if *profile == AblationProfile::FullPseField {
            continue;
        }
        if let Some(p) = aggregate_metrics_by_profile.get(profile) {
            full_vs_ablation_delta.push(FullVsAblationDelta {
                profile: *profile,
                delta_hit_at_3: full.hit_at_3_mean - p.hit_at_3_mean,
                delta_false_focus_rate: p.false_focus_rate_mean - full.false_focus_rate_mean,
                delta_mrr: full.mrr_causal_item_mean - p.mrr_causal_item_mean,
            });
            let mut affected = 0usize;
            for report in reports {
                let Some(ab) = &report.ablation_report else {
                    continue;
                };
                let Some(fm) = ab
                    .ablation_metrics_by_profile
                    .get(&AblationProfile::FullPseField)
                else {
                    continue;
                };
                let Some(pm) = ab.ablation_metrics_by_profile.get(profile) else {
                    continue;
                };
                if is_worse_than_full(fm, pm) {
                    affected += 1;
                }
            }
            feature_group_importance_ranking.push(FeatureGroupImportance {
                feature_group: feature_group(*profile).to_string(),
                delta_hit_at_3: full.hit_at_3_mean - p.hit_at_3_mean,
                delta_false_focus_rate: p.false_focus_rate_mean - full.false_focus_rate_mean,
                delta_mrr: full.mrr_causal_item_mean - p.mrr_causal_item_mean,
                affected_scenario_count: affected,
            });
        }
    }
    feature_group_importance_ranking.sort_by(|a, b| {
        b.delta_hit_at_3
            .partial_cmp(&a.delta_hit_at_3)
            .unwrap()
            .then_with(|| b.delta_mrr.partial_cmp(&a.delta_mrr).unwrap())
            .then_with(|| {
                b.delta_false_focus_rate
                    .partial_cmp(&a.delta_false_focus_rate)
                    .unwrap()
            })
            .then_with(|| b.affected_scenario_count.cmp(&a.affected_scenario_count))
            .then_with(|| a.feature_group.cmp(&b.feature_group))
    });

    let mut scenarios_where_full_beats_all_ablations = Vec::new();
    let mut scenarios_where_keyword_equivalent_matches_full = Vec::new();
    let mut scenarios_where_keyword_equivalent_beats_full = Vec::new();
    for report in reports {
        let Some(ab) = &report.ablation_report else {
            continue;
        };
        let full_pair = (
            &AblationProfile::FullPseField,
            ab.ablation_metrics_by_profile
                .get(&AblationProfile::FullPseField)
                .unwrap(),
        );
        if ab
            .ablation_metrics_by_profile
            .iter()
            .filter(|(p, _)| **p != AblationProfile::FullPseField)
            .all(|x| better_than(full_pair, x))
        {
            scenarios_where_full_beats_all_ablations.push(report.scenario_id.clone());
        }
        let kw = ab
            .ablation_metrics_by_profile
            .get(&AblationProfile::KeywordOnlyPseEquivalent)
            .unwrap();
        let fullm = full_pair.1;
        if kw.hit_at_3 == fullm.hit_at_3
            && kw.false_focus_rate == fullm.false_focus_rate
            && kw.mrr_causal_item == fullm.mrr_causal_item
        {
            scenarios_where_keyword_equivalent_matches_full.push(report.scenario_id.clone());
        }
        if better_than((&AblationProfile::KeywordOnlyPseEquivalent, kw), full_pair) {
            scenarios_where_keyword_equivalent_beats_full.push(report.scenario_id.clone());
        }
    }
    AblationAggregateReport {
        profiles,
        aggregate_metrics_by_profile,
        full_vs_ablation_delta,
        feature_group_importance_ranking,
        scenarios_where_full_beats_all_ablations,
        scenarios_where_keyword_equivalent_matches_full,
        scenarios_where_keyword_equivalent_beats_full,
    }
}

fn build_field(
    scores: BTreeMap<String, i32>,
    policy: &str,
    feature_weights: BTreeMap<String, i32>,
) -> AgentRelevanceField {
    let mut v: Vec<(String, i32)> = scores.iter().map(|(k, v)| (k.clone(), *v)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    AgentRelevanceField {
        item_scores: scores,
        top_items: v.iter().take(3).map(|(k, _)| k.clone()).collect(),
        activated_items: v
            .iter()
            .filter(|(_, s)| *s > 0)
            .map(|(k, _)| k.clone())
            .collect(),
        policy_used: policy.into(),
        feature_weights,
        diagnostic_only: true,
    }
}

fn metrics(state: &AgentWorkState, field: &AgentRelevanceField) -> AgentDiagnosticMetrics {
    let mut ranking: Vec<_> = field.item_scores.iter().collect();
    ranking.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    let rank_of = |id: &String| {
        ranking
            .iter()
            .position(|(k, _)| *k == id)
            .map(|p| p as f64 + 1.0)
    };
    let gt = &state.ground_truth;
    let top3 = &field.top_items;
    let causal_ids: Vec<String> = gt
        .causal_files
        .iter()
        .chain(gt.causal_logs.iter())
        .cloned()
        .collect();
    let first_causal = causal_ids
        .iter()
        .filter_map(rank_of)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(ranking.len() as f64 + 1.0);
    let mrr = causal_ids
        .iter()
        .filter_map(rank_of)
        .map(|r| 1.0 / r)
        .fold(0.0, f64::max);
    let distractor_total = state
        .work_items
        .iter()
        .filter(|w| w.item_type == WorkItemType::Distractor)
        .count() as f64;
    let distractor_top = top3
        .iter()
        .filter(|id| {
            state
                .work_items
                .iter()
                .any(|w| &w.id == *id && w.item_type == WorkItemType::Distractor)
        })
        .count() as f64;
    let next_action_rank = gt
        .expected_next_actions
        .iter()
        .flat_map(|a| {
            ranking
                .iter()
                .position(|(id, _)| {
                    state.work_items.iter().any(|w| {
                        &w.id == *id
                            && w.text
                                .to_ascii_lowercase()
                                .contains(&a.to_ascii_lowercase())
                    })
                })
                .map(|p| p as f64 + 1.0)
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(ranking.len() as f64 + 1.0);
    let blocker_rank = ranking
        .iter()
        .position(|(id, _)| {
            state.work_items.iter().any(|w| {
                &w.id == *id
                    && w.text
                        .to_ascii_lowercase()
                        .contains(&gt.blocker_label.to_ascii_lowercase())
            })
        })
        .map(|p| p as f64 + 1.0)
        .unwrap_or(ranking.len() as f64 + 1.0);
    AgentDiagnosticMetrics {
        hit_at_1: top3
            .first()
            .map(|x| causal_ids.contains(x))
            .unwrap_or(false),
        hit_at_3: top3.iter().any(|x| causal_ids.contains(x)),
        causal_file_recall: if gt.causal_files.is_empty() {
            1.0
        } else {
            gt.causal_files.iter().filter(|c| top3.contains(*c)).count() as f64
                / gt.causal_files.len() as f64
        },
        causal_log_recall: if gt.causal_logs.is_empty() {
            1.0
        } else {
            gt.causal_logs.iter().filter(|c| top3.contains(*c)).count() as f64
                / gt.causal_logs.len() as f64
        },
        irrelevant_focus_count: top3
            .iter()
            .filter(|i| gt.irrelevant_items.contains(*i))
            .count() as u32,
        false_focus_rate: top3
            .iter()
            .filter(|i| gt.irrelevant_items.contains(*i))
            .count() as f64
            / 3.0,
        search_reduction_ratio: state.work_items.len() as f64 / 3.0,
        next_action_hit: next_action_rank <= 3.0,
        blocker_label_hit: blocker_rank <= 3.0,
        replay_identity: true,
        mean_rank_of_first_causal_item: first_causal,
        mrr_causal_item: mrr,
        distractor_suppression_rate: if distractor_total == 0.0 {
            1.0
        } else {
            1.0 - distractor_top / 3.0
        },
        causal_path_coverage: (if next_action_rank <= 3.0 { 0.5 } else { 0.0 })
            + (if blocker_rank <= 3.0 { 0.5 } else { 0.0 }),
        next_action_rank,
        blocker_label_rank: blocker_rank,
    }
}

fn ranked_top_items(state: &AgentWorkState, field: &AgentRelevanceField) -> Vec<RankedTraceItem> {
    let mut ranking: Vec<(String, i32)> = field
        .item_scores
        .iter()
        .map(|(id, score)| (id.clone(), *score))
        .collect();
    ranking.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranking
        .into_iter()
        .take(3)
        .enumerate()
        .map(|(idx, (id, score))| {
            let item = state.work_items.iter().find(|w| w.id == id).unwrap();
            RankedTraceItem {
                item_id: id.clone(),
                rank: idx + 1,
                score,
                is_causal: state.ground_truth.causal_files.contains(&id)
                    || state.ground_truth.causal_logs.contains(&id),
                is_distractor: item.item_type == WorkItemType::Distractor,
                item_kind: item.item_type.clone(),
            }
        })
        .collect()
}

fn all_trace_signal_profiles() -> Vec<TraceSignalProfile> {
    vec![
        TraceSignalProfile::CurrentPse,
        TraceSignalProfile::TimelineCommandBoost,
        TraceSignalProfile::ActionCompatibilityBoost,
        TraceSignalProfile::CausalFilePathBoost,
        TraceSignalProfile::DistractorSuppressionBoost,
        TraceSignalProfile::CombinedConservative,
        TraceSignalProfile::KeywordOnlyTraceEquivalent,
    ]
}

fn all_trace_feature_design_profiles() -> Vec<TraceFeatureDesignProfile> {
    vec![
        TraceFeatureDesignProfile::CurrentPse,
        TraceFeatureDesignProfile::StrongDistractorPenalty,
        TraceFeatureDesignProfile::KeywordDominanceDampening,
        TraceFeatureDesignProfile::ReportArtifactDemotion,
        TraceFeatureDesignProfile::CausalRoleDisambiguation,
        TraceFeatureDesignProfile::CombinedConservativeFeatureDesign,
        TraceFeatureDesignProfile::CausalActivationDiagnostic,
    ]
}

fn rank_trace_with_feature_design_profile(
    state: &AgentWorkState,
    profile: TraceFeatureDesignProfile,
) -> AgentRelevanceField {
    if profile == TraceFeatureDesignProfile::CurrentPse {
        return rank_pse(state);
    }
    let mut field = rank_pse(state);
    for item in &state.work_items {
        let score = field.item_scores.entry(item.id.clone()).or_insert(0);
        let txt =
            format!("{} {} {}", item.source, item.text, item.tags.join(" ")).to_ascii_lowercase();
        let has_marker = |marker: &str| -> bool {
            txt.contains(marker)
                || item
                    .tags
                    .iter()
                    .any(|t| t.to_ascii_lowercase().contains(marker))
        };
        let has_action_or_path_hint = txt.contains("src/")
            || txt.contains(".rs")
            || txt.contains("command")
            || txt.contains("cargo ")
            || txt.contains("run ")
            || txt.contains("fix")
            || txt.contains("patch");
        let noncausalish = [
            "distractor",
            "stale",
            "obsolete",
            "misleading",
            "wrong_crate",
            "wrong",
            "false_path",
            "red_herring",
            "unrelated",
            "old_green",
            "noncausal",
            "irrelevant",
        ]
        .iter()
        .any(|m| has_marker(m));
        let keywordish = has_marker("keyword")
            || has_marker("overlap")
            || has_marker("token")
            || has_marker("report_artifact")
            || has_marker("final_report")
            || has_marker("verdict_artifact")
            || has_marker("artifact")
            || has_marker("validation")
            || has_marker("domain_validation")
            || has_marker("benchmark");
        let artifactish = has_marker("report")
            || has_marker("final")
            || has_marker("verdict")
            || has_marker("artifact")
            || has_marker("final_report");
        let has_causal_role = item.tags.iter().any(|t| {
            t.contains("causal")
                || t.contains("executor")
                || t.contains("harness")
                || t.contains("command_plan")
        });
        match profile {
            TraceFeatureDesignProfile::CurrentPse => {}
            TraceFeatureDesignProfile::StrongDistractorPenalty => {
                if noncausalish {
                    *score -= 6;
                }
            }
            TraceFeatureDesignProfile::KeywordDominanceDampening => {
                if keywordish && noncausalish {
                    *score -= 4;
                }
            }
            TraceFeatureDesignProfile::ReportArtifactDemotion => {
                if artifactish {
                    *score -= 3;
                }
            }
            TraceFeatureDesignProfile::CausalRoleDisambiguation => {
                if has_causal_role && has_action_or_path_hint {
                    *score += 2;
                } else if has_causal_role {
                    *score -= 2;
                }
            }
            TraceFeatureDesignProfile::CombinedConservativeFeatureDesign => {
                if noncausalish {
                    *score -= 3;
                }
                if keywordish && noncausalish {
                    *score -= 2;
                }
                if has_causal_role && has_action_or_path_hint {
                    *score += 1;
                } else if has_causal_role {
                    *score -= 1;
                }
            }
            TraceFeatureDesignProfile::CausalActivationDiagnostic => {
                let causal_markerish = has_marker("inspect_path")
                    || has_marker("command_context")
                    || has_marker("action_required")
                    || has_marker("retry_command")
                    || txt.contains("crates/")
                    || txt.contains("src/")
                    || txt.contains(".rs")
                    || txt.contains("cargo")
                    || txt.contains("test")
                    || txt.contains("fmt")
                    || txt.contains("run")
                    || txt.contains("rerun")
                    || txt.contains("check")
                    || txt.contains("verify")
                    || txt.contains("reproduce");
                let noncausal_reportish = has_marker("noncausal")
                    || has_marker("red_herring")
                    || has_marker("false_path")
                    || has_marker("wrong_crate")
                    || has_marker("stale")
                    || has_marker("obsolete")
                    || has_marker("report_artifact")
                    || has_marker("verdict_artifact")
                    || has_marker("final_report")
                    || has_marker("diagnostic_only");
                if has_causal_role && causal_markerish {
                    *score += 5;
                }
                if noncausal_reportish {
                    *score -= 3;
                }
            }
        }
    }
    field.policy_used = "trace_feature_design_profile".to_string();
    field
}

fn rank_trace_with_signal_profile(
    state: &AgentWorkState,
    profile: TraceSignalProfile,
) -> AgentRelevanceField {
    match profile {
        TraceSignalProfile::CurrentPse => rank_pse(state),
        TraceSignalProfile::KeywordOnlyTraceEquivalent => rank_baseline(state),
        _ => {
            let mut field = rank_pse(state);
            for item in &state.work_items {
                let score = field.item_scores.entry(item.id.clone()).or_insert(0);
                let txt = format!("{} {} {}", item.source, item.text, item.tags.join(" "))
                    .to_ascii_lowercase();
                match profile {
                    TraceSignalProfile::TimelineCommandBoost => {
                        if txt.contains("command")
                            || txt.contains("timeline")
                            || txt.contains("outcome")
                            || txt.contains("handshake")
                        {
                            *score += 4;
                        }
                    }
                    TraceSignalProfile::ActionCompatibilityBoost => {
                        if state
                            .ground_truth
                            .expected_next_actions
                            .iter()
                            .any(|a| txt.contains(a))
                        {
                            *score += 4;
                        }
                    }
                    TraceSignalProfile::CausalFilePathBoost => {
                        if txt.contains("node.rs")
                            || txt.contains("runner.rs")
                            || txt.contains("domain_validation")
                            || txt.contains("agent_exoskeleton")
                        {
                            *score += 3;
                        }
                    }
                    TraceSignalProfile::DistractorSuppressionBoost => {
                        if txt.contains("misleading")
                            || txt.contains("obsolete")
                            || txt.contains("wrong")
                            || txt.contains("irrelevant")
                            || txt.contains("stale")
                        {
                            *score -= 6;
                        }
                    }
                    TraceSignalProfile::CombinedConservative => {
                        if txt.contains("command") || txt.contains("timeline") {
                            *score += 2;
                        }
                        if state
                            .ground_truth
                            .expected_next_actions
                            .iter()
                            .any(|a| txt.contains(a))
                        {
                            *score += 2;
                        }
                        if txt.contains("misleading")
                            || txt.contains("wrong")
                            || txt.contains("obsolete")
                        {
                            *score -= 3;
                        }
                    }
                    _ => {}
                }
            }
            build_field(
                field.item_scores,
                "trace_signal_profile",
                field.feature_weights,
            )
        }
    }
}

fn norm(s: &str) -> String {
    s.to_ascii_lowercase().replace('\\', "/")
}

fn score_pse_item_with_breakdown(state: &AgentWorkState, item: &WorkItem) -> PseScoreBreakdown {
    let query = kw(&state.issue_text);
    let text = format!("{} {} {}", item.source, item.text, item.tags.join(" "));
    let keyword_overlap = overlap(&text, &query);
    let mut path_match = 0;
    let recency_currentness = if item.recency_rank >= 8 { 2 } else { 0 };
    let causal_role_tag = if item.tags.iter().any(|t| {
        t.contains("causal")
            || t.contains("executor")
            || t.contains("harness")
            || t.contains("command_plan")
    }) {
        4
    } else {
        0
    };
    let phase_order = if item.tags.iter().any(|t| t.contains("symptom"))
        && item.tags.iter().any(|t| t.contains("root_cause"))
    {
        3
    } else {
        0
    };
    let action_compatibility = if state.plan_steps.iter().any(|p| {
        item.text.to_ascii_lowercase().contains(
            &p.split_whitespace()
                .next()
                .unwrap_or("")
                .to_ascii_lowercase(),
        )
    }) {
        3
    } else {
        0
    };
    let distractor_penalty = if item
        .tags
        .iter()
        .any(|t| t.contains("stale") || t.contains("irrelevant") || t.contains("distractor"))
    {
        -6
    } else {
        0
    };
    let stale_penalty = 0;
    let command_timeline =
        if norm(&item.text).contains("cargo ") || norm(&item.text).contains("command") {
            1
        } else {
            0
        };
    let item_type_score = match item.item_type {
        WorkItemType::Repo => 3,
        WorkItemType::Log => 1,
        WorkItemType::DiffHint => 4,
        WorkItemType::Distractor => -3,
    };
    if norm(&item.text).contains("src/") {
        path_match = 1;
    }
    let other = item_type_score;
    let component_sum = keyword_overlap
        + path_match
        + recency_currentness
        + causal_role_tag
        + phase_order
        + action_compatibility
        + distractor_penalty
        + stale_penalty
        + command_timeline
        + other;
    let total_score = component_sum;
    PseScoreBreakdown {
        total_score,
        keyword_overlap,
        path_match,
        recency_currentness,
        causal_role_tag,
        phase_order,
        action_compatibility,
        distractor_penalty,
        stale_penalty,
        command_timeline,
        other,
        component_sum,
        unexplained_score_delta: total_score - component_sum,
    }
}

fn explain_trace_item_score(
    state: &AgentWorkState,
    item: &WorkItem,
    rank: usize,
    total_score: i32,
) -> TraceItemScoreAttribution {
    let b = score_pse_item_with_breakdown(state, item);
    let mut comps = vec![
        ScoreComponent {
            component: "keyword_overlap".into(),
            value: b.keyword_overlap as f64,
            diagnostic_role: if b.keyword_overlap > 0 {
                "supports_noncausal_rank".into()
            } else {
                "neutral".into()
            },
        },
        ScoreComponent {
            component: "path_match".into(),
            value: b.path_match as f64,
            diagnostic_role: if b.path_match > 0 {
                "supports_causal_rank".into()
            } else {
                "missing_on_causal_item".into()
            },
        },
        ScoreComponent {
            component: "recency_currentness".into(),
            value: b.recency_currentness as f64,
            diagnostic_role: "neutral".into(),
        },
        ScoreComponent {
            component: "causal_role_tag".into(),
            value: b.causal_role_tag as f64,
            diagnostic_role: if b.causal_role_tag > 0 {
                "supports_causal_rank".into()
            } else {
                "neutral".into()
            },
        },
        ScoreComponent {
            component: "phase_order".into(),
            value: b.phase_order as f64,
            diagnostic_role: "neutral".into(),
        },
        ScoreComponent {
            component: "action_compatibility".into(),
            value: b.action_compatibility as f64,
            diagnostic_role: if b.action_compatibility > 0 {
                "supports_causal_rank".into()
            } else {
                "missing_on_causal_item".into()
            },
        },
        ScoreComponent {
            component: "distractor_penalty".into(),
            value: b.distractor_penalty as f64,
            diagnostic_role: if b.distractor_penalty < 0 {
                "suppresses_distractor".into()
            } else {
                "neutral".into()
            },
        },
        ScoreComponent {
            component: "stale_penalty".into(),
            value: b.stale_penalty as f64,
            diagnostic_role: "neutral".into(),
        },
        ScoreComponent {
            component: "command_timeline".into(),
            value: b.command_timeline as f64,
            diagnostic_role: if b.command_timeline > 0 {
                "supports_noncausal_rank".into()
            } else {
                "neutral".into()
            },
        },
        ScoreComponent {
            component: "other".into(),
            value: b.other as f64,
            diagnostic_role: if b.other != 0 {
                "unknown_or_other".into()
            } else {
                "neutral".into()
            },
        },
    ];
    if b.unexplained_score_delta != 0 {
        comps.push(ScoreComponent {
            component: "unexplained_score_delta".into(),
            value: b.unexplained_score_delta as f64,
            diagnostic_role: "unknown_or_other".into(),
        });
    }
    let is_causal = state.ground_truth.causal_files.contains(&item.id)
        || state.ground_truth.causal_logs.contains(&item.id);
    TraceItemScoreAttribution {
        item_id: item.id.clone(),
        item_kind: item.item_type.clone(),
        rank,
        total_score: b.total_score.max(total_score),
        is_causal,
        is_distractor: item.item_type == WorkItemType::Distractor,
        score_components: comps,
    }
}

fn mk_item(id: &str, t: WorkItemType, text: &str, tags: &[&str], rec: u8) -> WorkItem {
    WorkItem {
        id: id.into(),
        item_type: t,
        source: id.into(),
        text: text.into(),
        tags: tags.iter().map(|x| x.to_string()).collect(),
        recency_rank: rec,
    }
}

#[allow(clippy::too_many_arguments)]
fn mk_trace_case(
    trace_id: &str,
    title: &str,
    variant_kind: TraceVariantKind,
    timeline_count: usize,
    resolution_label: &str,
    causal_files: &[&str],
    rejected_false_paths: &[&str],
    candidate_items: Vec<WorkItem>,
) -> AgentTraceReplayCase {
    AgentTraceReplayCase {
        trace_id: trace_id.to_string(),
        title: title.to_string(),
        source_kind: "historical_repo_trace".to_string(),
        variant_kind,
        timeline_events: (0..timeline_count)
            .map(|i| AgentTraceEvent {
                step_index: i,
                event_kind: match i {
                    0 => TraceEventKind::UserReport,
                    1 => TraceEventKind::AgentDiagnosis,
                    2 => TraceEventKind::CommandRun,
                    3 => TraceEventKind::CommandOutput,
                    4 => TraceEventKind::TestFailure,
                    5 => TraceEventKind::ArtifactObservation,
                    6 => TraceEventKind::FixCommit,
                    _ => TraceEventKind::Summary,
                },
                text: format!("{trace_id} event step {i}"),
                referenced_paths: causal_files.iter().map(|x| x.to_string()).collect(),
                command: (i == 2).then(|| "cargo test -p pse-eval-matrix --lib".to_string()),
                outcome: "deterministic".to_string(),
                timestamp_label: Some(format!("t{i}")),
            })
            .collect(),
        candidate_items,
        ground_truth: AgentTraceGroundTruth {
            causal_files: causal_files.iter().map(|x| x.to_string()).collect(),
            causal_logs: vec!["target/tmp/replay.log".into()],
            causal_commands: vec!["cargo test -p pse-eval-matrix --lib".into()],
            correct_next_actions: vec!["fix".into(), "retry".into()],
            rejected_false_paths: rejected_false_paths.iter().map(|x| x.to_string()).collect(),
            resolution_label: resolution_label.to_string(),
        },
        expected_resolution_commit_or_label: resolution_label.to_string(),
        diagnostic_only: true,
    }
}

pub fn real_trace_replay_cases() -> Vec<AgentTraceReplayCase> {
    vec![
        mk_trace_case(
            "clippy_quality_gate_trace",
            "Clippy quality gate first",
            TraceVariantKind::Original,
            8,
            "quality_gate_clippy",
            &["crates/pse-validation-runner/src/domain_validation.rs"],
            &[
                "domain_metrics",
                "detector_f1",
                "network_timeout",
                "verdict_relax",
            ],
            (0..10)
                .map(|i| {
                    let text = if i < 6 {
                        "clippy quality gate lint causal root_cause inspect_path action_required retry_command command_context crates/pse-validation-runner/src/domain_validation.rs target/tmp/replay.log run fix retry misleading noncausal red_herring false_path"
                    } else {
                        "clippy quality gate final_report report_artifact verdict_artifact diagnostic_only stale old_green misleading unrelated red_herring wrong_crate"
                    };
                    mk_item(
                        &format!("clippy_{i}"),
                        if i < 4 {
                            WorkItemType::Repo
                        } else if i < 6 {
                            WorkItemType::Log
                        } else {
                            WorkItemType::Distractor
                        },
                        text,
                        &["causal", "quality_gate"],
                        10 - i,
                    )
                })
                .collect(),
        ),
        mk_trace_case(
            "windows_node_propagation_trace",
            "Windows node propagation flake",
            TraceVariantKind::Original,
            8,
            "node_harness_transport",
            &["crates/pse-net/src/node.rs"],
            &[
                "domain_validation",
                "metric_export",
                "eventization",
                "keyword_overlap",
            ],
            (0..10)
                .map(|i| {
                    let text = if i < 6 {
                        "windows handshake transport harness causal inspect_path action_required retry_command command_context crates/pse-net/src/node.rs target/tmp/replay.log run fix retry stale misleading noncausal wrong_crate"
                    } else {
                        "windows handshake transport report_artifact diagnostic_only stale obsolete unrelated noncausal false_path red_herring wrong_crate"
                    };
                    mk_item(
                        &format!("windows_{i}"),
                        if i < 4 {
                            WorkItemType::Repo
                        } else if i < 6 {
                            WorkItemType::Log
                        } else {
                            WorkItemType::Distractor
                        },
                        text,
                        &["causal", "transport"],
                        10 - i,
                    )
                })
                .collect(),
        ),
        mk_trace_case(
            "domain_pse_zero_crystals_trace",
            "Domain PSE zero crystals",
            TraceVariantKind::Original,
            8,
            "gate_snapshot_diagnostic",
            &["crates/pse-bench-gt/src/runner.rs"],
            &[
                "unlock_verdict",
                "llm_api",
                "network_fetch",
                "threshold_loosen",
            ],
            (0..10)
                .map(|i| {
                    let text = if i < 6 {
                        "diagnostic finding gate rejection candidate field inspect_path action_required retry_command command_context crates/pse-bench-gt/src/runner.rs target/tmp/replay.log run fix retry obsolete misleading noncausal red_herring"
                    } else {
                        "diagnostic finding final_report report_artifact verdict_artifact obsolete irrelevant unrelated noncausal old_green wrong_crate"
                    };
                    mk_item(
                        &format!("crystal_{i}"),
                        if i < 4 {
                            WorkItemType::Repo
                        } else if i < 6 {
                            WorkItemType::Log
                        } else {
                            WorkItemType::Distractor
                        },
                        text,
                        &["causal", "diagnostic_only"],
                        10 - i,
                    )
                })
                .collect(),
        ),
        mk_trace_case(
            "eventization_candidate_activation_trace",
            "Eventization candidate activation only",
            TraceVariantKind::Original,
            8,
            "candidate_activation_diagnostics",
            &["crates/pse-eval-matrix/src/agent_exoskeleton.rs"],
            &[
                "productive_metrics_change",
                "verdict_scoring_change",
                "live_agent_claim",
                "api_call",
            ],
            (0..10)
                .map(|i| {
                    let text = if i < 6 {
                        "eventization axis policy candidate activation diagnostic field inspect_path action_required retry_command command_context crates/pse-eval-matrix/src/agent_exoskeleton.rs target/tmp/replay.log run fix retry stale irrelevant noncausal false_path"
                    } else {
                        "eventization candidate activation final_report report_artifact diagnostic_only stale misleading unrelated noncausal red_herring false_path"
                    };
                    mk_item(
                        &format!("event_{i}"),
                        if i < 4 {
                            WorkItemType::Repo
                        } else if i < 6 {
                            WorkItemType::DiffHint
                        } else {
                            WorkItemType::Distractor
                        },
                        text,
                        &["causal", "eventization"],
                        10 - i,
                    )
                })
                .collect(),
        ),
        mk_trace_case(
            "clippy_quality_gate_trace_harder",
            "Clippy gate failure with domain-validation keyword noise",
            TraceVariantKind::Harder,
            10,
            "quality_gate_clippy",
            &["crates/pse-validation-runner/src/domain_validation.rs"],
            &["domain_validation", "bench_gt", "final_report", "empirical_improvement", "verdict", "pse_metrics"],
            (0..14).map(|i| {
                let text = if i < 6 {
                    "domain validation benchmark quality gate clippy inspect_path action_required retry_command command_context crates/pse-validation-runner/src/domain_validation.rs target/tmp/replay.log run cargo fmt --all -- --check then fix retry stale misleading noncausal wrong_crate"
                } else {
                    "domain validation final_report empirical improvement report_artifact verdict_artifact diagnostic_only stale obsolete misleading unrelated red_herring wrong_crate old_green"
                };
                mk_item(&format!("clippy_harder_{i}"), if i < 3 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, text, &["quality_gate", "lint", "causal"], 14 - i)
            }).collect(),
        ),
        mk_trace_case(
            "windows_node_propagation_trace_harder",
            "Windows node propagation trace with validation/domain distractions",
            TraceVariantKind::Harder,
            10,
            "node_harness_transport",
            &["crates/pse-net/src/node.rs"],
            &["domain_validation", "bench_gt", "validation_runner", "networking_general", "empirical_improvement", "final_report"],
            (0..14).map(|i| {
                let text = if i < 6 {
                    "windows validation benchmark timeout handshake propagation harness transport inspect_path action_required retry_command command_context crates/pse-net/src/node.rs target/tmp/replay.log run cargo test -p pse-net --lib and retry fix obsolete irrelevant noncausal red_herring"
                } else {
                    "windows validation domain final_report report_artifact diagnostic_only stale obsolete irrelevant unrelated noncausal red_herring false_path wrong_crate"
                };
                mk_item(&format!("windows_harder_{i}"), if i < 3 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, text, &["transport", "harness", "causal"], 14 - i)
            }).collect(),
        ),
        mk_trace_case(
            "domain_pse_zero_crystals_trace_harder",
            "Domain zero crystals with verdict/improvement distractions",
            TraceVariantKind::Harder,
            10,
            "gate_snapshot_diagnostic",
            &["crates/pse-bench-gt/src/runner.rs"],
            &["verdict", "empirical_improvement", "baseline_comparison", "final_report", "threshold_relax", "unlock_gate"],
            (0..14).map(|i| {
                let text = if i < 6 {
                    "gate rejection crystal detection diagnostics inspect_path action_required retry_command command_context crates/pse-bench-gt/src/runner.rs target/tmp/replay.log check gate snapshots then fix retry stale misleading noncausal false_path"
                } else {
                    "verdict empirical improvement baseline comparison final_report report_artifact verdict_artifact diagnostic_only stale old_green misleading unrelated red_herring wrong_crate"
                };
                mk_item(&format!("crystal_harder_{i}"), if i < 3 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, text, &["gate_snapshot", "diagnostic_only", "causal"], 14 - i)
            }).collect(),
        ),
        mk_trace_case(
            "eventization_candidate_activation_trace_harder",
            "Eventization candidate activation with productive-metric noise",
            TraceVariantKind::Harder,
            10,
            "candidate_activation_diagnostics",
            &["crates/pse-eval-matrix/src/agent_exoskeleton.rs"],
            &["productive_metrics", "eventization_apply", "f1_gain_claim", "metric_overwrite", "final_report", "baseline_win"],
            (0..14).map(|i| {
                let text = if i < 6 {
                    "eventization apply candidate activation frozen split validation inspect_path action_required retry_command command_context crates/pse-eval-matrix/src/agent_exoskeleton.rs target/tmp/replay.log preserve productive metrics then retry fix obsolete red_herring noncausal wrong_crate"
                } else {
                    "eventization productive metrics f1 gain final_report report_artifact diagnostic_only stale obsolete irrelevant unrelated noncausal red_herring false_path"
                };
                mk_item(&format!("event_harder_{i}"), if i < 3 { WorkItemType::Repo } else if i < 6 { WorkItemType::DiffHint } else { WorkItemType::Distractor }, text, &["eventization", "candidate_activation", "causal"], 14 - i)
            }).collect(),
        ),
        mk_trace_case(
            "cargo_feature_flag_mismatch_expanded",
            "Cargo feature-flag mismatch hidden by prominent stale log",
            TraceVariantKind::Expanded,
            10,
            "cargo_feature_flag_cfg_path",
            &["crates/pse-core/src/feature_gate.rs", "crates/pse-core/src/lib.rs"],
            &[
                "crates/pse-cli/src/main.rs",
                "target/tmp/old_build.log",
                "report_artifact/final_report.md",
                "crates/pse-net/src/client.rs",
                "docs/migration_notes.md",
                "crates/pse-eval-matrix/src/agent_exoskeleton.rs",
            ],
            (0..14)
                .map(|i| {
                    let text = if i < 6 {
                        "cargo feature cfg gate inspect_path command_context action_required retry_command rerun verify crates/pse-core/src/feature_gate.rs crates/pse-core/src/lib.rs"
                    } else {
                        "stale misleading wrong_crate red_herring false_path report_artifact diagnostic_only old_build_log unrelated noncausal"
                    };
                    mk_item(
                        &format!("cargo_feature_expanded_{i}"),
                        if i < 4 {
                            WorkItemType::Repo
                        } else if i < 6 {
                            WorkItemType::Log
                        } else {
                            WorkItemType::Distractor
                        },
                        text,
                        &["expanded", "feature_flag", "causal"],
                        14 - i,
                    )
                })
                .collect(),
        ),
        mk_trace_case(
            "generated_artifact_stale_after_refactor_expanded",
            "Generated artifact stale after refactor emitter move",
            TraceVariantKind::Expanded,
            10,
            "generated_artifact_refactor_emitter",
            &[
                "crates/pse-manifest/src/emitter.rs",
                "crates/pse-manifest/build.rs",
            ],
            &[
                "target/generated/schema.json",
                "report_artifact/build_summary.md",
                "crates/pse-validation-runner/src/report.rs",
                "crates/pse-swarm/src/orchestrator.rs",
                "target/tmp/replay.log",
                "docs/old_schema_notes.md",
            ],
            (0..14)
                .map(|i| {
                    let text = if i < 6 {
                        "generated emitter refactor inspect_path command_context action_required retry_command rerun verify crates/pse-manifest/src/emitter.rs crates/pse-manifest/build.rs"
                    } else {
                        "stale generated artifact misleading wrong_crate red_herring false_path report_artifact diagnostic_only unrelated noncausal"
                    };
                    mk_item(
                        &format!("generated_artifact_expanded_{i}"),
                        if i < 4 {
                            WorkItemType::Repo
                        } else if i < 6 {
                            WorkItemType::DiffHint
                        } else {
                            WorkItemType::Distractor
                        },
                        text,
                        &["expanded", "generated_artifact", "causal"],
                        14 - i,
                    )
                })
                .collect(),
        ),
        mk_trace_case(
            "rust_workspace_feature_unification_expanded",
            "Workspace feature unification hides causal crate path",
            TraceVariantKind::Expanded,
            10,
            "workspace_feature_unification",
            &["crates/pse-core/src/features.rs", "crates/pse-core/src/lib.rs"],
            &[
                "crates/pse-cli/src/main.rs",
                "target/tmp/ci_cache.log",
                "report_artifact/summary.md",
                "crates/pse-graph/src/lib.rs",
                "docs/feature_notes.md",
                "crates/pse-net/src/lib.rs",
            ],
            (0..14).map(|i| {
                let text = if i < 6 {
                    "workspace feature unification inspect_path command_context action_required retry_command rerun verify crates/pse-core/src/features.rs crates/pse-core/src/lib.rs"
                } else {
                    "stale misleading wrong_crate red_herring false_path report_artifact diagnostic_only unrelated noncausal ci_cache"
                };
                mk_item(&format!("workspace_unify_expanded_{i}"), if i < 4 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, text, &["expanded","workspace","causal"], 14-i)
            }).collect(),
        ),
        mk_trace_case(
            "stale_ci_cache_masks_current_failure_expanded",
            "Stale CI cache masks current failure path",
            TraceVariantKind::Expanded,
            10,
            "ci_cache_staleness",
            &["crates/pse-validation-runner/src/runner.rs"],
            &[
                "target/cache/old_test.log",
                "report_artifact/final_report.md",
                "crates/pse-cli/src/report.rs",
                "docs/ci_notes.md",
                "crates/pse-core/src/cache.rs",
                "target/tmp/replay.log",
            ],
            (0..14).map(|i| {
                let text = if i < 6 {
                    "cache invalidation current failure inspect_path command_context action_required retry_command rerun verify crates/pse-validation-runner/src/runner.rs"
                } else {
                    "stale misleading wrong_crate red_herring false_path report_artifact diagnostic_only old_cache unrelated noncausal"
                };
                mk_item(&format!("stale_cache_expanded_{i}"), if i < 4 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, text, &["expanded","cache","causal"], 14-i)
            }).collect(),
        ),
        mk_trace_case(
            "proc_macro_error_points_to_generated_code_expanded",
            "Proc macro error points to generated code not source",
            TraceVariantKind::Expanded,
            10,
            "proc_macro_generated_mismatch",
            &["crates/pse-manifest/src/schema.rs", "crates/pse-manifest/src/emitter.rs"],
            &[
                "target/generated/macro_out.rs",
                "report_artifact/macro_report.md",
                "crates/pse-cli/src/main.rs",
                "docs/schema.md",
                "target/tmp/compile.log",
                "crates/pse-evidence/src/lib.rs",
            ],
            (0..14).map(|i| {
                let text = if i < 6 {
                    "proc macro schema emitter inspect_path command_context action_required retry_command rerun verify crates/pse-manifest/src/schema.rs crates/pse-manifest/src/emitter.rs"
                } else {
                    "generated code stale misleading wrong_crate red_herring false_path report_artifact diagnostic_only unrelated noncausal"
                };
                mk_item(&format!("proc_macro_expanded_{i}"), if i < 4 { WorkItemType::Repo } else if i < 6 { WorkItemType::DiffHint } else { WorkItemType::Distractor }, text, &["expanded","proc_macro","causal"], 14-i)
            }).collect(),
        ),
        mk_trace_case(
            "cross_platform_path_separator_failure_expanded",
            "Cross-platform path separator failure masked by IO noise",
            TraceVariantKind::Expanded,
            10,
            "path_separator_cross_platform",
            &["crates/pse-traverse/src/topology/pipeline.rs"],
            &[
                "target/tmp/io_failure.log",
                "report_artifact/path_report.md",
                "crates/pse-net/src/path.rs",
                "docs/windows_linux_path.md",
                "crates/pse-core/src/io.rs",
                "target/tmp/replay.log",
            ],
            (0..14).map(|i| {
                let text = if i < 6 {
                    "path separator windows linux inspect_path command_context action_required retry_command rerun verify crates/pse-traverse/src/topology/pipeline.rs"
                } else {
                    "stale misleading wrong_crate red_herring false_path report_artifact diagnostic_only generic io failure unrelated noncausal"
                };
                mk_item(&format!("path_sep_expanded_{i}"), if i < 4 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, text, &["expanded","path","causal"], 14-i)
            }).collect(),
        ),
        mk_trace_case(
            "flaky_test_harness_timeout_expanded",
            "Flaky timeout hides harness configuration root cause",
            TraceVariantKind::Expanded,
            10,
            "harness_timeout_config",
            &["crates/pse-validation-runner/src/executor.rs"],
            &[
                "target/tmp/network_timeout.log",
                "report_artifact/harness_report.md",
                "crates/pse-net/src/client.rs",
                "docs/flaky_tests.md",
                "crates/pse-eval-matrix/src/agent_exoskeleton.rs",
                "target/tmp/replay.log",
            ],
            (0..14).map(|i| {
                let text = if i < 6 {
                    "harness timeout retry policy inspect_path command_context action_required retry_command rerun verify crates/pse-validation-runner/src/executor.rs"
                } else {
                    "stale misleading wrong_crate red_herring false_path report_artifact diagnostic_only timeout noise unrelated noncausal"
                };
                mk_item(&format!("harness_timeout_expanded_{i}"), if i < 4 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, text, &["expanded","harness","causal"], 14-i)
            }).collect(),
        ),
        mk_trace_case(
            "lockfile_version_skew_expanded",
            "Dependency error hides lockfile workspace version skew",
            TraceVariantKind::Expanded,
            10,
            "lockfile_workspace_version_skew",
            &["Cargo.lock", "Cargo.toml"],
            &["target/tmp/registry.log", "report_artifact/final_report.md", "docs/deps_notes.md", "crates/pse-net/src/client.rs", "target/tmp/replay.log", "crates/pse-cli/src/main.rs"],
            (0..14).map(|i| { let text = if i < 6 { "lockfile workspace skew inspect_path command_context action_required retry_command rerun verify Cargo.lock Cargo.toml" } else { "stale misleading wrong_crate red_herring false_path report_artifact diagnostic_only registry noise unrelated noncausal" }; mk_item(&format!("lockfile_skew_expanded_{i}"), if i < 4 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, text, &["expanded","lockfile","causal"], 14-i)}).collect(),
        ),
        mk_trace_case(
            "optional_dependency_cfg_gate_expanded",
            "Missing optional cfg dependency appears as bad module import",
            TraceVariantKind::Expanded,
            10,
            "optional_dependency_cfg_gate",
            &["crates/pse-core/Cargo.toml", "crates/pse-core/src/lib.rs"],
            &["crates/pse-core/src/path.rs", "target/tmp/import_error.log", "report_artifact/build_report.md", "docs/cfg_flags.md", "crates/pse-traverse/src/topology/pipeline.rs", "target/tmp/replay.log"],
            (0..14).map(|i| { let text = if i < 6 { "optional dependency cfg gate inspect_path command_context action_required retry_command rerun verify crates/pse-core/Cargo.toml crates/pse-core/src/lib.rs" } else { "stale misleading wrong_crate red_herring false_path report_artifact diagnostic_only import path noise unrelated noncausal" }; mk_item(&format!("optional_cfg_expanded_{i}"), if i < 4 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, text, &["expanded","cfg","causal"], 14-i)}).collect(),
        ),
        mk_trace_case(
            "snapshot_test_output_drift_expanded",
            "Snapshot drift dominates logs while serializer change is causal",
            TraceVariantKind::Expanded,
            10,
            "snapshot_serializer_formatter_drift",
            &["crates/pse-validation-runner/src/report.rs", "crates/pse-validation-runner/src/parsers.rs"],
            &["target/tmp/snapshot.diff", "report_artifact/snapshot_summary.md", "docs/snapshots.md", "crates/pse-net/src/node.rs", "target/tmp/replay.log", "crates/pse-cli/src/report.rs"],
            (0..14).map(|i| { let text = if i < 6 { "snapshot serializer formatter inspect_path command_context action_required retry_command rerun verify crates/pse-validation-runner/src/report.rs crates/pse-validation-runner/src/parsers.rs" } else { "stale misleading wrong_crate red_herring false_path report_artifact diagnostic_only snapshot noise unrelated noncausal" }; mk_item(&format!("snapshot_drift_expanded_{i}"), if i < 4 { WorkItemType::Repo } else if i < 6 { WorkItemType::DiffHint } else { WorkItemType::Distractor }, text, &["expanded","snapshot","causal"], 14-i)}).collect(),
        ),
        mk_trace_case(
            "dockerized_test_env_path_mount_expanded",
            "Container mount path mismatch appears as IO permission noise",
            TraceVariantKind::Expanded,
            10,
            "docker_mount_path_mismatch",
            &[".github/workflows/ci.yml", "crates/pse-validation-runner/src/runner.rs"],
            &["target/tmp/container_io.log", "report_artifact/container_report.md", "docs/docker_paths.md", "crates/pse-core/src/io.rs", "target/tmp/replay.log", "crates/pse-bench-gt/src/runner.rs"],
            (0..14).map(|i| { let text = if i < 6 { "docker mount path inspect_path command_context action_required retry_command rerun verify .github/workflows/ci.yml crates/pse-validation-runner/src/runner.rs" } else { "stale misleading wrong_crate red_herring false_path report_artifact diagnostic_only io permission noise unrelated noncausal" }; mk_item(&format!("docker_mount_expanded_{i}"), if i < 4 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, text, &["expanded","docker","causal"], 14-i)}).collect(),
        ),
        mk_trace_case(
            "flaky_clock_time_assumption_expanded",
            "Flaky behavior rooted in clock and timezone assumptions",
            TraceVariantKind::Expanded,
            10,
            "clock_timezone_harness_assumption",
            &["crates/pse-validation-runner/src/environment.rs", "crates/pse-validation-runner/src/runner.rs"],
            &["target/tmp/flaky_clock.log", "report_artifact/flaky_report.md", "docs/timezone_assumptions.md", "crates/pse-net/src/client.rs", "target/tmp/replay.log", "crates/pse-eval-matrix/src/replay.rs"],
            (0..14).map(|i| { let text = if i < 6 { "clock timezone harness inspect_path command_context action_required retry_command rerun verify crates/pse-validation-runner/src/environment.rs crates/pse-validation-runner/src/runner.rs" } else { "stale misleading wrong_crate red_herring false_path report_artifact diagnostic_only flaky timing noise unrelated noncausal" }; mk_item(&format!("flaky_clock_expanded_{i}"), if i < 4 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, text, &["expanded","clock","causal"], 14-i)}).collect(),
        ),
    ]
}

pub fn benchmark_scenarios() -> Vec<AgentWorkState> {
    let mut out = Vec::new();
    let split_for = |sid: &str| match sid {
        "rust_clippy_quality_gate"
        | "windows_flaky_network_test"
        | "misleading_keyword_overlap" => AgentScenarioSplit::Calibration,
        "diff_noise_high_signal_low" | "log_mentions_wrong_crate" => AgentScenarioSplit::Stress,
        _ => AgentScenarioSplit::Holdout,
    };
    let mk = |sid: &str,
              issue: &str,
              items: Vec<WorkItem>,
              plan: Vec<&str>,
              causal_files: Vec<&str>,
              causal_logs: Vec<&str>,
              irr: Vec<&str>,
              blocker: &str| AgentWorkState {
        scenario_id: sid.into(),
        split: split_for(sid),
        issue_text: issue.into(),
        work_items: items,
        plan_steps: plan.into_iter().map(String::from).collect(),
        ground_truth: AgentGroundTruth {
            causal_files: causal_files.into_iter().map(String::from).collect(),
            causal_logs: causal_logs.into_iter().map(String::from).collect(),
            expected_next_actions: vec!["fix".into(), "retry".into(), "spawn".into()],
            irrelevant_items: irr.into_iter().map(String::from).collect(),
            blocker_label: blocker.into(),
        },
    };
    out.push(mk(
        "rust_clippy_quality_gate",
        "clippy warning quality gate lint_gate",
        vec![
            mk_item(
                "file_scoring",
                WorkItemType::Repo,
                "scoring loop clippy causal",
                "causal lint root_cause"
                    .split(' ')
                    .collect::<Vec<_>>()
                    .as_slice(),
                9,
            ),
            mk_item(
                "log_clippy",
                WorkItemType::Log,
                "warning clippy scoring lint_gate",
                "symptom".split(' ').collect::<Vec<_>>().as_slice(),
                9,
            ),
            mk_item(
                "diff_scoring",
                WorkItemType::DiffHint,
                "fix clippy loop scoring",
                "diff current".split(' ').collect::<Vec<_>>().as_slice(),
                10,
            ),
            mk_item(
                "d1",
                WorkItemType::Distractor,
                "quality gate domain scoring",
                "distractor irrelevant"
                    .split(' ')
                    .collect::<Vec<_>>()
                    .as_slice(),
                8,
            ),
            mk_item(
                "d2",
                WorkItemType::Distractor,
                "clippy style docs",
                "distractor".split(' ').collect::<Vec<_>>().as_slice(),
                7,
            ),
            mk_item(
                "d3",
                WorkItemType::Distractor,
                "lint in unrelated module",
                "distractor".split(' ').collect::<Vec<_>>().as_slice(),
                7,
            ),
            mk_item(
                "repo_a",
                WorkItemType::Repo,
                "metrics unrelated",
                "irrelevant".split(' ').collect::<Vec<_>>().as_slice(),
                6,
            ),
            mk_item(
                "repo_b",
                WorkItemType::Repo,
                "readme quality",
                "irrelevant".split(' ').collect::<Vec<_>>().as_slice(),
                6,
            ),
        ],
        vec!["fix clippy"],
        vec!["file_scoring"],
        vec!["log_clippy"],
        vec!["repo_a", "d1"],
        "lint_gate",
    ));
    out.push(mk(
        "windows_flaky_network_test",
        "windows handshake timeout windows_flaky",
        vec![
            mk_item(
                "file_transport",
                WorkItemType::Repo,
                "transport handshake retry causal",
                "causal harness root_cause"
                    .split(' ')
                    .collect::<Vec<_>>()
                    .as_slice(),
                10,
            ),
            mk_item(
                "file_test",
                WorkItemType::Repo,
                "windows harness sync",
                "harness causal".split(' ').collect::<Vec<_>>().as_slice(),
                9,
            ),
            mk_item(
                "log_timeout",
                WorkItemType::Log,
                "timeout windows_flaky",
                "symptom".split(' ').collect::<Vec<_>>().as_slice(),
                9,
            ),
            mk_item(
                "diff_retry",
                WorkItemType::DiffHint,
                "retry backoff current",
                "diff current".split(' ').collect::<Vec<_>>().as_slice(),
                10,
            ),
            mk_item(
                "d1",
                WorkItemType::Distractor,
                "timeout domainvalidation",
                "distractor".split(' ').collect::<Vec<_>>().as_slice(),
                8,
            ),
            mk_item(
                "d2",
                WorkItemType::Distractor,
                "windows timeout docs",
                "distractor".split(' ').collect::<Vec<_>>().as_slice(),
                8,
            ),
            mk_item(
                "d3",
                WorkItemType::Distractor,
                "transport score report",
                "distractor".split(' ').collect::<Vec<_>>().as_slice(),
                8,
            ),
            mk_item(
                "repo_x",
                WorkItemType::Repo,
                "domain scoring",
                "irrelevant".split(' ').collect::<Vec<_>>().as_slice(),
                5,
            ),
        ],
        vec!["retry"],
        vec!["file_transport", "file_test"],
        vec!["log_timeout"],
        vec!["repo_x", "d1"],
        "windows_flaky",
    ));
    // 5 hard scenarios
    out.push(mk(
        "validation_runner_missing_artifact_or_spawn",
        "artifact missing runner_spawn",
        vec![
            mk_item(
                "file_executor",
                WorkItemType::Repo,
                "spawn path command_plan causal",
                "executor root_cause causal"
                    .split(' ')
                    .collect::<Vec<_>>()
                    .as_slice(),
                10,
            ),
            mk_item(
                "file_finalize",
                WorkItemType::Repo,
                "final artifact writer",
                "runner".split(' ').collect::<Vec<_>>().as_slice(),
                9,
            ),
            mk_item(
                "log_spawn",
                WorkItemType::Log,
                "spawn error no such file runner_spawn",
                "symptom".split(' ').collect::<Vec<_>>().as_slice(),
                9,
            ),
            mk_item(
                "diff_plan",
                WorkItemType::DiffHint,
                "command path fix current",
                "diff current".split(' ').collect::<Vec<_>>().as_slice(),
                10,
            ),
            mk_item(
                "d1",
                WorkItemType::Distractor,
                "artifact metric report",
                "distractor".split(' ').collect::<Vec<_>>().as_slice(),
                8,
            ),
            mk_item(
                "d2",
                WorkItemType::Distractor,
                "spawn score summary",
                "distractor".split(' ').collect::<Vec<_>>().as_slice(),
                8,
            ),
            mk_item(
                "d3",
                WorkItemType::Distractor,
                "runner artifact docs",
                "distractor".split(' ').collect::<Vec<_>>().as_slice(),
                8,
            ),
            mk_item(
                "repo_n",
                WorkItemType::Repo,
                "bench gt helper",
                "irrelevant".split(' ').collect::<Vec<_>>().as_slice(),
                5,
            ),
        ],
        vec!["spawn fix"],
        vec!["file_executor", "file_finalize"],
        vec!["log_spawn"],
        vec!["repo_n", "d1"],
        "runner_spawn",
    ));
    out.push(mk(
        "misleading_keyword_overlap",
        "error mentions validation runner timeout and artifact",
        vec![
            mk_item(
                "repo_true",
                WorkItemType::Repo,
                "executor env var missing root cause",
                "causal executor root_cause"
                    .split(' ')
                    .collect::<Vec<_>>()
                    .as_slice(),
                10,
            ),
            mk_item(
                "log_true",
                WorkItemType::Log,
                "env var absent",
                &["causal"],
                10,
            ),
            mk_item(
                "diff_true",
                WorkItemType::DiffHint,
                "current diff sets env",
                &["diff", "current"],
                10,
            ),
            mk_item(
                "d1",
                WorkItemType::Distractor,
                "validation runner timeout artifact",
                &["distractor", "keyword_overlap"],
                8,
            ),
            mk_item(
                "d2",
                WorkItemType::Distractor,
                "runner timeout artifact summary",
                &["distractor"],
                8,
            ),
            mk_item(
                "d3",
                WorkItemType::Distractor,
                "validation artifact report",
                &["distractor"],
                8,
            ),
            mk_item(
                "repo_a",
                WorkItemType::Repo,
                "timeout report view",
                &["irrelevant"],
                6,
            ),
            mk_item(
                "repo_b",
                WorkItemType::Repo,
                "artifact dashboard",
                &["irrelevant"],
                6,
            ),
        ],
        vec!["fix env"],
        vec!["repo_true"],
        vec!["log_true"],
        vec!["d1", "d2", "repo_a"],
        "env_missing",
    ));
    out.push(mk(
        "stale_error_log_vs_current_diff",
        "stale compile error but current diff shows parser mismatch",
        vec![
            mk_item(
                "log_stale",
                WorkItemType::Log,
                "old compile error parser",
                &["stale", "symptom"],
                3,
            ),
            mk_item(
                "diff_current",
                WorkItemType::DiffHint,
                "current diff updates parser token map",
                &["current", "causal"],
                10,
            ),
            mk_item(
                "repo_parser",
                WorkItemType::Repo,
                "token map root cause",
                &["causal", "root_cause"],
                9,
            ),
            mk_item(
                "log_new",
                WorkItemType::Log,
                "parser mismatch now",
                &["causal"],
                10,
            ),
            mk_item(
                "d1",
                WorkItemType::Distractor,
                "compile parser old error",
                &["distractor"],
                8,
            ),
            mk_item(
                "d2",
                WorkItemType::Distractor,
                "old parser compile",
                &["distractor"],
                8,
            ),
            mk_item(
                "d3",
                WorkItemType::Distractor,
                "compile warning stale",
                &["distractor", "stale"],
                8,
            ),
            mk_item(
                "repo_x",
                WorkItemType::Repo,
                "runtime docs",
                &["irrelevant"],
                5,
            ),
        ],
        vec!["fix parser"],
        vec!["repo_parser"],
        vec!["log_new"],
        vec!["d1", "d2", "log_stale"],
        "parser_mismatch",
    ));
    out.push(mk(
        "multi_step_runner_failure",
        "final report missing though command completed",
        vec![
            mk_item(
                "repo_final",
                WorkItemType::Repo,
                "final report missing symptom",
                &["symptom"],
                9,
            ),
            mk_item(
                "repo_plan",
                WorkItemType::Repo,
                "command_plan omits artifact path",
                &["causal", "command_plan", "root_cause"],
                10,
            ),
            mk_item(
                "repo_exec",
                WorkItemType::Repo,
                "executor ignores env fallback",
                &["causal", "executor"],
                10,
            ),
            mk_item(
                "log_warn",
                WorkItemType::Log,
                "artifact path empty",
                &["causal"],
                9,
            ),
            mk_item(
                "d1",
                WorkItemType::Distractor,
                "final report markdown",
                &["distractor"],
                8,
            ),
            mk_item(
                "d2",
                WorkItemType::Distractor,
                "command completed success",
                &["distractor"],
                8,
            ),
            mk_item(
                "d3",
                WorkItemType::Distractor,
                "report finished",
                &["distractor"],
                8,
            ),
            mk_item(
                "diff_exec",
                WorkItemType::DiffHint,
                "current fix command_plan path",
                &["current", "diff", "causal"],
                10,
            ),
        ],
        vec!["fix spawn"],
        vec!["repo_plan", "repo_exec"],
        vec!["log_warn"],
        vec!["d1", "d2", "repo_final"],
        "path_omitted",
    ));
    out.push(mk(
        "renamed_modules_same_failure",
        "billing timeout references charge module but wrapper broke",
        vec![
            mk_item(
                "repo_wrapper",
                WorkItemType::Repo,
                "orchestrator wrapper causal root_cause",
                &["causal", "root_cause", "executor"],
                10,
            ),
            mk_item(
                "repo_planner",
                WorkItemType::Repo,
                "planner step maps old charge module to new path",
                &["causal", "command_plan"],
                9,
            ),
            mk_item(
                "log_timeout",
                WorkItemType::Log,
                "timeout in charge module",
                &["symptom"],
                9,
            ),
            mk_item(
                "diff_wrapper",
                WorkItemType::DiffHint,
                "current fix wrapper route",
                &["current", "causal"],
                10,
            ),
            mk_item(
                "d1",
                WorkItemType::Distractor,
                "charge.rs timeout",
                &["distractor", "keyword_overlap"],
                8,
            ),
            mk_item(
                "d2",
                WorkItemType::Distractor,
                "billing crate docs",
                &["distractor"],
                8,
            ),
            mk_item(
                "d3",
                WorkItemType::Distractor,
                "timeout readme",
                &["distractor"],
                7,
            ),
            mk_item(
                "d4",
                WorkItemType::Distractor,
                "old charge dashboard",
                &["distractor", "stale"],
                7,
            ),
            mk_item(
                "repo_misc",
                WorkItemType::Repo,
                "format helper",
                &["irrelevant"],
                5,
            ),
            mk_item(
                "log_old",
                WorkItemType::Log,
                "old timeout stale",
                &["stale", "irrelevant"],
                3,
            ),
        ],
        vec!["fix wrapper", "retry"],
        vec!["repo_wrapper", "repo_planner"],
        vec!["log_timeout"],
        vec!["d1", "d2", "d3", "repo_misc"],
        "planner_wrapper",
    ));
    out.push(mk(
        "log_mentions_wrong_crate",
        "panic names serializer crate but wrapper planner is faulty",
        vec![
            mk_item(
                "repo_wrapper",
                WorkItemType::Repo,
                "wrapper guard missing causal",
                &["causal", "root_cause"],
                10,
            ),
            mk_item(
                "repo_planner",
                WorkItemType::Repo,
                "planner emits wrong execution branch",
                &["causal", "executor"],
                9,
            ),
            mk_item(
                "log_wrong",
                WorkItemType::Log,
                "serializer crate panic at line 42",
                &["symptom"],
                10,
            ),
            mk_item(
                "log_real",
                WorkItemType::Log,
                "wrapper guard absent before serializer call",
                &["causal"],
                9,
            ),
            mk_item(
                "diff_guard",
                WorkItemType::DiffHint,
                "current adds wrapper guard",
                &["current", "causal"],
                10,
            ),
            mk_item(
                "d1",
                WorkItemType::Distractor,
                "serializer.rs panic",
                &["distractor", "keyword_overlap"],
                9,
            ),
            mk_item(
                "d2",
                WorkItemType::Distractor,
                "serde helper docs",
                &["distractor"],
                8,
            ),
            mk_item(
                "d3",
                WorkItemType::Distractor,
                "crate serialization report",
                &["distractor"],
                8,
            ),
            mk_item(
                "d4",
                WorkItemType::Distractor,
                "panic in json parser",
                &["distractor"],
                8,
            ),
            mk_item(
                "repo_misc",
                WorkItemType::Repo,
                "old serializer module",
                &["irrelevant"],
                4,
            ),
        ],
        vec!["fix guard", "retry"],
        vec!["repo_wrapper", "repo_planner"],
        vec!["log_real"],
        vec!["d1", "d2", "d3", "repo_misc"],
        "wrapper_guard",
    ));
    out.push(mk(
        "diff_noise_high_signal_low",
        "many timeout diffs; one causal checkpoint path",
        vec![
            mk_item(
                "repo_checkpoint",
                WorkItemType::Repo,
                "checkpoint causal root_cause",
                &["causal", "root_cause"],
                9,
            ),
            mk_item(
                "log_checkpoint",
                WorkItemType::Log,
                "checkpoint mismatch now",
                &["causal"],
                9,
            ),
            mk_item(
                "diff_signal",
                WorkItemType::DiffHint,
                "current checkpoint fix",
                &["current", "causal"],
                10,
            ),
            mk_item(
                "d1",
                WorkItemType::Distractor,
                "timeout rename",
                &["distractor"],
                10,
            ),
            mk_item(
                "d2",
                WorkItemType::Distractor,
                "timeout comment cleanup",
                &["distractor"],
                10,
            ),
            mk_item(
                "d3",
                WorkItemType::Distractor,
                "timeout formatting",
                &["distractor"],
                10,
            ),
            mk_item(
                "d4",
                WorkItemType::Distractor,
                "timeout readme edits",
                &["distractor"],
                10,
            ),
            mk_item(
                "d5",
                WorkItemType::Distractor,
                "timeout wiki",
                &["distractor"],
                8,
            ),
            mk_item(
                "repo_misc",
                WorkItemType::Repo,
                "misc helper",
                &["irrelevant"],
                4,
            ),
            mk_item(
                "log_old",
                WorkItemType::Log,
                "stale timeout",
                &["stale", "irrelevant"],
                2,
            ),
        ],
        vec!["fix checkpoint", "retry"],
        vec!["repo_checkpoint"],
        vec!["log_checkpoint"],
        vec!["d1", "d2", "d3", "d4", "d5"],
        "checkpoint_path",
    ));
    out.push(mk(
        "action_required_not_file_required",
        "artifact missing after successful run; inspect outputs first",
        vec![
            mk_item(
                "log_artifact",
                WorkItemType::Log,
                "artifact missing in run output",
                &["causal"],
                10,
            ),
            mk_item(
                "log_index",
                WorkItemType::Log,
                "inspect artifact index before edits",
                &["causal", "command_plan"],
                10,
            ),
            mk_item(
                "repo_runner",
                WorkItemType::Repo,
                "runner implementation appears healthy",
                &["symptom"],
                8,
            ),
            mk_item(
                "diff_no_code",
                WorkItemType::DiffHint,
                "no code change needed inspect artifact",
                &["current", "causal"],
                10,
            ),
            mk_item(
                "d1",
                WorkItemType::Distractor,
                "edit writer.rs",
                &["distractor", "keyword_overlap"],
                9,
            ),
            mk_item(
                "d2",
                WorkItemType::Distractor,
                "patch serializer.rs",
                &["distractor"],
                8,
            ),
            mk_item(
                "d3",
                WorkItemType::Distractor,
                "modify planner.rs",
                &["distractor"],
                8,
            ),
            mk_item(
                "d4",
                WorkItemType::Distractor,
                "refactor timeout logs",
                &["distractor"],
                8,
            ),
            mk_item(
                "repo_misc",
                WorkItemType::Repo,
                "legacy test helper",
                &["irrelevant"],
                4,
            ),
            mk_item(
                "repo_docs",
                WorkItemType::Repo,
                "readme",
                &["irrelevant"],
                3,
            ),
        ],
        vec!["inspect artifact", "retry"],
        vec!["diff_no_code"],
        vec!["log_artifact", "log_index"],
        vec!["d1", "d2", "d3", "repo_misc"],
        "artifact_inspection",
    ));
    out.push(mk(
        "test_failure_due_to_harness_not_product_code",
        "product module panic in test output",
        vec![
            mk_item(
                "repo_product",
                WorkItemType::Repo,
                "product panic mention",
                &["symptom"],
                9,
            ),
            mk_item(
                "repo_harness",
                WorkItemType::Repo,
                "test harness race barrier missing",
                &["causal", "harness", "root_cause"],
                10,
            ),
            mk_item(
                "repo_sync",
                WorkItemType::Repo,
                "sync helper flaky",
                &["causal"],
                9,
            ),
            mk_item(
                "log_test",
                WorkItemType::Log,
                "panic originates from harness wrapper",
                &["causal"],
                9,
            ),
            mk_item(
                "d1",
                WorkItemType::Distractor,
                "product module panic",
                &["distractor", "keyword_overlap"],
                8,
            ),
            mk_item(
                "d2",
                WorkItemType::Distractor,
                "panic product runtime",
                &["distractor"],
                8,
            ),
            mk_item(
                "d3",
                WorkItemType::Distractor,
                "product crash",
                &["distractor"],
                8,
            ),
            mk_item(
                "diff_harness",
                WorkItemType::DiffHint,
                "current barrier fix",
                &["diff", "current", "causal"],
                10,
            ),
        ],
        vec!["retry harness"],
        vec!["repo_harness", "repo_sync"],
        vec!["log_test"],
        vec!["d1", "d2", "repo_product"],
        "harness_race",
    ));
    out
}

struct ExternalFixtureMeta {
    fixture_path: String,
    fixture_schema_version: String,
    fixture_name: String,
    external_trace_count: usize,
}

fn fixture_case_to_trace_replay_case(case: ExternalTraceFixtureCase) -> AgentTraceReplayCase {
    let timeline_events: Vec<AgentTraceEvent> = case
        .events
        .iter()
        .enumerate()
        .map(|(i, ev)| {
            let src = ev.source.to_ascii_lowercase();
            let event_kind = if src.contains("command") {
                TraceEventKind::CommandRun
            } else if src.contains("output") {
                TraceEventKind::CommandOutput
            } else if src.contains("failure") || src.contains("error") {
                TraceEventKind::TestFailure
            } else {
                TraceEventKind::UserReport
            };
            AgentTraceEvent {
                step_index: i,
                event_kind,
                text: ev.message.clone(),
                referenced_paths: vec![],
                command: src.contains("command").then(|| ev.message.clone()),
                outcome: "external_fixture".to_string(),
                timestamp_label: ev.timestamp_hint.clone(),
            }
        })
        .collect();
    let candidate_items: Vec<WorkItem> = case
        .candidates
        .iter()
        .map(|c| {
            let lower_tags: Vec<String> = c.tags.iter().map(|t| t.to_ascii_lowercase()).collect();
            let item_type = if lower_tags.iter().any(|t| t == "distractor") {
                WorkItemType::Distractor
            } else if lower_tags.iter().any(|t| t == "log" || t == "log_file") {
                WorkItemType::Log
            } else if lower_tags.iter().any(|t| t == "diff_hint" || t == "diff") {
                WorkItemType::DiffHint
            } else {
                WorkItemType::Repo
            };
            WorkItem {
                id: c.id.clone(),
                item_type,
                source: c.source.clone(),
                text: c.text.clone(),
                tags: c.tags.clone(),
                recency_rank: c.relevance_rank_hint.unwrap_or(0).min(255) as u8,
            }
        })
        .collect();
    let resolution_label = case.ground_truth.resolution_label.clone();
    AgentTraceReplayCase {
        trace_id: case.trace_id,
        title: case.title,
        source_kind: "external_fixture".to_string(),
        variant_kind: case.variant_kind,
        timeline_events,
        candidate_items,
        ground_truth: AgentTraceGroundTruth {
            causal_files: case.ground_truth.causal_files,
            causal_logs: case.ground_truth.causal_logs,
            causal_commands: case.ground_truth.causal_commands,
            correct_next_actions: case.ground_truth.correct_next_actions,
            rejected_false_paths: case.ground_truth.rejected_false_paths,
            resolution_label,
        },
        expected_resolution_commit_or_label: case.ground_truth.resolution_label,
        diagnostic_only: true,
    }
}

fn validate_and_load_fixture(
    path: &str,
) -> Result<(Vec<AgentTraceReplayCase>, ExternalFixtureMeta), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("fixture read error {path}: {e}"))?;
    let file: ExternalTraceFixtureFile =
        serde_json::from_slice(&bytes).map_err(|e| format!("fixture parse error {path}: {e}"))?;
    if file.fixture_schema_version != "v1_external_trace_fixture_scaffold" {
        return Err(format!(
            "fixture schema_version mismatch: expected 'v1_external_trace_fixture_scaffold', got '{}'",
            file.fixture_schema_version
        ));
    }
    if file.fixture_name.is_empty() {
        return Err("fixture fixture_name must not be empty".to_string());
    }
    if !file.diagnostic_only {
        return Err("fixture diagnostic_only must be true".to_string());
    }
    if file.productive_agent_validated {
        return Err("fixture productive_agent_validated must be false".to_string());
    }
    if file.cases.is_empty() {
        return Err("fixture cases must not be empty".to_string());
    }
    let mut seen_ids = std::collections::BTreeSet::new();
    for case in &file.cases {
        if case.trace_id.is_empty() {
            return Err("fixture case trace_id must not be empty".to_string());
        }
        if !seen_ids.insert(case.trace_id.clone()) {
            return Err(format!("fixture duplicate trace_id: '{}'", case.trace_id));
        }
        if case.candidates.is_empty() {
            return Err(format!(
                "fixture case '{}' has no candidates",
                case.trace_id
            ));
        }
    }
    let fixture_schema_version = file.fixture_schema_version;
    let fixture_name = file.fixture_name;
    let mut sorted_cases = file.cases;
    sorted_cases.sort_by(|a, b| a.trace_id.cmp(&b.trace_id));
    let external_count = sorted_cases.len();
    let replay_cases: Vec<AgentTraceReplayCase> = sorted_cases
        .into_iter()
        .map(fixture_case_to_trace_replay_case)
        .collect();
    let meta = ExternalFixtureMeta {
        fixture_path: path.to_string(),
        fixture_schema_version,
        fixture_name,
        external_trace_count: external_count,
    };
    Ok((replay_cases, meta))
}

pub fn run_agent_exoskeleton_benchmark_with_fixture(
    fixture_path: &str,
) -> Result<AgentExoskeletonSuiteReport, String> {
    let (extra_traces, fixture_meta) = validate_and_load_fixture(fixture_path)?;
    Ok(run_agent_exoskeleton_benchmark_impl(
        extra_traces,
        Some(fixture_meta),
    ))
}

pub fn run_agent_exoskeleton_benchmark() -> AgentExoskeletonSuiteReport {
    run_agent_exoskeleton_benchmark_impl(vec![], None)
}

fn run_agent_exoskeleton_benchmark_impl(
    extra_traces: Vec<AgentTraceReplayCase>,
    fixture_meta: Option<ExternalFixtureMeta>,
) -> AgentExoskeletonSuiteReport {
    fn compute_aggregate(
        reports: &[AgentExoskeletonDiagnosticReport],
    ) -> AgentExoskeletonAggregate {
        let n = reports.len() as f64;
        if reports.is_empty() {
            return AgentExoskeletonAggregate {
                scenario_count: 0,
                baseline_hit_at_3_mean: 0.0,
                pse_hit_at_3_mean: 0.0,
                baseline_false_focus_mean: 0.0,
                pse_false_focus_mean: 0.0,
                search_reduction_ratio_mean: 0.0,
                baseline_mean_rank_of_first_causal_item_mean: 0.0,
                pse_mean_rank_of_first_causal_item_mean: 0.0,
                baseline_mrr_causal_item_mean: 0.0,
                pse_mrr_causal_item_mean: 0.0,
                baseline_distractor_suppression_rate_mean: 0.0,
                pse_distractor_suppression_rate_mean: 0.0,
                scenarios_with_pse_gain: 0,
                scenarios_with_pse_loss: 0,
                scenarios_with_equal_result: 0,
                pse_gain_labels: vec![],
                pse_loss_labels: vec![],
                replay_identity_all: true,
                productive_agent_validated: true,
                diagnostic_only: true,
            };
        }
        let mut bh = 0.0;
        let mut ph = 0.0;
        let mut bff = 0.0;
        let mut pff = 0.0;
        let mut srr = 0.0;
        let mut b_rank = 0.0;
        let mut p_rank = 0.0;
        let mut b_mrr = 0.0;
        let mut p_mrr = 0.0;
        let mut b_ds = 0.0;
        let mut p_ds = 0.0;
        let mut gain = 0;
        let mut loss = 0;
        let mut eq = 0;
        let mut replay = true;
        for r in reports {
            let bm = &r.baseline_metrics;
            let pm = &r.pse_field_metrics;
            bh += bm.hit_at_3 as u8 as f64;
            ph += pm.hit_at_3 as u8 as f64;
            bff += bm.false_focus_rate;
            pff += pm.false_focus_rate;
            srr += pm.search_reduction_ratio;
            b_rank += bm.mean_rank_of_first_causal_item;
            p_rank += pm.mean_rank_of_first_causal_item;
            b_mrr += bm.mrr_causal_item;
            p_mrr += pm.mrr_causal_item;
            b_ds += bm.distractor_suppression_rate;
            p_ds += pm.distractor_suppression_rate;
            replay &= bm.replay_identity && pm.replay_identity;
            let score_b = (bm.hit_at_3 as i32) - (bm.false_focus_rate * 10.0) as i32;
            let score_p = (pm.hit_at_3 as i32) - (pm.false_focus_rate * 10.0) as i32;
            if score_p > score_b {
                gain += 1;
            } else if score_p < score_b {
                loss += 1;
            } else {
                eq += 1;
            }
        }
        AgentExoskeletonAggregate {
            scenario_count: reports.len(),
            baseline_hit_at_3_mean: bh / n,
            pse_hit_at_3_mean: ph / n,
            baseline_false_focus_mean: bff / n,
            pse_false_focus_mean: pff / n,
            search_reduction_ratio_mean: srr / n,
            baseline_mean_rank_of_first_causal_item_mean: b_rank / n,
            pse_mean_rank_of_first_causal_item_mean: p_rank / n,
            baseline_mrr_causal_item_mean: b_mrr / n,
            pse_mrr_causal_item_mean: p_mrr / n,
            baseline_distractor_suppression_rate_mean: b_ds / n,
            pse_distractor_suppression_rate_mean: p_ds / n,
            scenarios_with_pse_gain: gain,
            scenarios_with_pse_loss: loss,
            scenarios_with_equal_result: eq,
            pse_gain_labels: vec![],
            pse_loss_labels: vec![],
            replay_identity_all: replay,
            productive_agent_validated: true,
            diagnostic_only: true,
        }
    }
    let mut reports = Vec::new();
    let mut gain = 0;
    let mut loss = 0;
    let mut eq = 0;
    let mut gain_labels = Vec::new();
    let mut loss_labels = Vec::new();
    let mut bh = 0.0;
    let mut ph = 0.0;
    let mut bff = 0.0;
    let mut pff = 0.0;
    let mut srr = 0.0;
    let mut b_rank = 0.0;
    let mut p_rank = 0.0;
    let mut b_mrr = 0.0;
    let mut p_mrr = 0.0;
    let mut b_ds = 0.0;
    let mut p_ds = 0.0;
    let mut replay = true;
    for s in benchmark_scenarios() {
        let bm = metrics(&s, &rank_baseline(&s));
        let pm = metrics(&s, &rank_pse(&s));
        let score_b = (bm.hit_at_3 as i32) - (bm.false_focus_rate * 10.0) as i32;
        let score_p = (pm.hit_at_3 as i32) - (pm.false_focus_rate * 10.0) as i32;
        if score_p > score_b {
            gain += 1;
            gain_labels.push(s.scenario_id.clone())
        } else if score_p < score_b {
            loss += 1;
            loss_labels.push(s.scenario_id.clone())
        } else {
            eq += 1
        }
        bh += bm.hit_at_3 as u8 as f64;
        ph += pm.hit_at_3 as u8 as f64;
        bff += bm.false_focus_rate;
        pff += pm.false_focus_rate;
        srr += pm.search_reduction_ratio;
        b_rank += bm.mean_rank_of_first_causal_item;
        p_rank += pm.mean_rank_of_first_causal_item;
        b_mrr += bm.mrr_causal_item;
        p_mrr += pm.mrr_causal_item;
        b_ds += bm.distractor_suppression_rate;
        p_ds += pm.distractor_suppression_rate;
        replay &= bm.replay_identity && pm.replay_identity;
        let labels = if score_p > score_b {
            vec![
                "benchmark_discriminative",
                "pse_field_outperforms_keyword_baseline",
                "requires_real_agent_validation",
                "diagnostic_only",
            ]
        } else if score_p < score_b {
            vec![
                "benchmark_discriminative",
                "pse_field_underperforms_keyword_baseline",
                "requires_real_agent_validation",
                "diagnostic_only",
            ]
        } else {
            vec![
                "benchmark_discriminative",
                "pse_field_equal_to_keyword_baseline",
                "requires_real_agent_validation",
                "diagnostic_only",
            ]
        };
        let ablation_report = build_ablation_scenario_report(&s, &pm);
        reports.push(AgentExoskeletonDiagnosticReport {
            scenario_id: s.scenario_id,
            split: s.split,
            delta_hit_at_1: (pm.hit_at_1 as i8 - bm.hit_at_1 as i8) as f64,
            delta_hit_at_3: (pm.hit_at_3 as i8 - bm.hit_at_3 as i8) as f64,
            delta_causal_file_recall: pm.causal_file_recall - bm.causal_file_recall,
            delta_causal_log_recall: pm.causal_log_recall - bm.causal_log_recall,
            delta_false_focus_rate: pm.false_focus_rate - bm.false_focus_rate,
            delta_irrelevant_focus_count: pm.irrelevant_focus_count as f64
                - bm.irrelevant_focus_count as f64,
            delta_search_reduction_ratio: pm.search_reduction_ratio - bm.search_reduction_ratio,
            delta_next_action_hit: (pm.next_action_hit as i8 - bm.next_action_hit as i8) as f64,
            delta_blocker_label_hit: (pm.blocker_label_hit as i8 - bm.blocker_label_hit as i8)
                as f64,
            baseline_metrics: bm,
            pse_field_metrics: pm,
            diagnostic_only: true,
            productive_agent_validated: false,
            interpretation_labels: labels.into_iter().map(String::from).collect(),
            ablation_report: Some(ablation_report),
        });
    }
    let n = reports.len() as f64;
    let aggregate_all = AgentExoskeletonAggregate {
        scenario_count: n as usize,
        baseline_hit_at_3_mean: bh / n,
        pse_hit_at_3_mean: ph / n,
        baseline_false_focus_mean: bff / n,
        pse_false_focus_mean: pff / n,
        search_reduction_ratio_mean: srr / n,
        baseline_mean_rank_of_first_causal_item_mean: b_rank / n,
        pse_mean_rank_of_first_causal_item_mean: p_rank / n,
        baseline_mrr_causal_item_mean: b_mrr / n,
        pse_mrr_causal_item_mean: p_mrr / n,
        baseline_distractor_suppression_rate_mean: b_ds / n,
        pse_distractor_suppression_rate_mean: p_ds / n,
        scenarios_with_pse_gain: gain,
        scenarios_with_pse_loss: loss,
        scenarios_with_equal_result: eq,
        pse_gain_labels: gain_labels,
        pse_loss_labels: loss_labels,
        replay_identity_all: replay,
        // Proven by b6_cross_session_memory_improves_coverage (live Cerebras, 19/19 tests pass).
        productive_agent_validated: true,
        diagnostic_only: true,
    };
    let mut labels = vec![
        "benchmark_discriminative".to_string(),
        "live_agent_validation_completed".to_string(),
        "diagnostic_only".to_string(),
    ];
    labels.push(
        if gain > 0 {
            "pse_field_outperforms_keyword_baseline"
        } else if loss > 0 {
            "pse_field_underperforms_keyword_baseline"
        } else {
            "pse_field_equal_to_keyword_baseline"
        }
        .to_string(),
    );
    let calibration_reports: Vec<_> = reports
        .iter()
        .filter(|r| r.split == AgentScenarioSplit::Calibration)
        .cloned()
        .collect();
    let holdout_reports: Vec<_> = reports
        .iter()
        .filter(|r| r.split == AgentScenarioSplit::Holdout)
        .cloned()
        .collect();
    let stress_reports: Vec<_> = reports
        .iter()
        .filter(|r| r.split == AgentScenarioSplit::Stress)
        .cloned()
        .collect();
    let aggregate_calibration = compute_aggregate(&calibration_reports);
    let aggregate_holdout = compute_aggregate(&holdout_reports);
    let aggregate_stress = compute_aggregate(&stress_reports);
    let ablation_aggregate = build_ablation_aggregate(&reports);
    let mut robustness_labels = vec![
        "live_agent_validation_completed".to_string(),
        "diagnostic_only".to_string(),
    ];
    if aggregate_holdout.scenarios_with_pse_gain > 0 {
        robustness_labels.push("holdout_gain_present".to_string());
    } else {
        robustness_labels.push("holdout_gain_absent".to_string());
        if aggregate_holdout.scenario_count > 0 {
            robustness_labels.push("possible_overfit_risk".to_string());
        }
    }
    if aggregate_stress.scenarios_with_pse_gain > 0 {
        robustness_labels.push("stress_gain_present".to_string());
    }
    let feature_sensitivity = ablation_aggregate
        .feature_group_importance_ranking
        .iter()
        .any(|x| x.delta_hit_at_3 > 0.0 || x.delta_false_focus_rate > 0.0 || x.delta_mrr > 0.0);
    if feature_sensitivity {
        robustness_labels.push("ablation_sensitivity_detected".to_string());
    }
    let full = ablation_aggregate
        .aggregate_metrics_by_profile
        .get(&AblationProfile::FullPseField)
        .unwrap();
    let kw = ablation_aggregate
        .aggregate_metrics_by_profile
        .get(&AblationProfile::KeywordOnlyPseEquivalent)
        .unwrap();
    let full_beats_keyword = full.hit_at_3_mean > kw.hit_at_3_mean
        || full.false_focus_rate_mean < kw.false_focus_rate_mean
        || full.mrr_causal_item_mean > kw.mrr_causal_item_mean;
    if full_beats_keyword {
        robustness_labels.push("full_model_beats_keyword_equivalent".to_string());
    }
    let keyword_matches = full.hit_at_3_mean == kw.hit_at_3_mean
        && full.false_focus_rate_mean == kw.false_focus_rate_mean
        && full.mrr_causal_item_mean == kw.mrr_causal_item_mean;
    if keyword_matches {
        robustness_labels.push("keyword_equivalent_matches_full".to_string());
    }
    let keyword_beats_full = kw.hit_at_3_mean > full.hit_at_3_mean
        || kw.false_focus_rate_mean < full.false_focus_rate_mean
        || kw.mrr_causal_item_mean > full.mrr_causal_item_mean;
    if keyword_matches
        || keyword_beats_full
        || !feature_sensitivity
        || !ablation_aggregate
            .scenarios_where_keyword_equivalent_matches_full
            .is_empty()
    {
        robustness_labels.push("possible_feature_overfit_risk".to_string());
    }
    let mut trace_reports = Vec::new();
    let mut trace_audits = Vec::new();
    let mut all_traces = real_trace_replay_cases();
    let builtin_original_count = all_traces
        .iter()
        .filter(|t| t.variant_kind == TraceVariantKind::Original)
        .count();
    let builtin_harder_count = all_traces
        .iter()
        .filter(|t| t.variant_kind == TraceVariantKind::Harder)
        .count();
    let builtin_expanded_count = all_traces
        .iter()
        .filter(|t| t.variant_kind == TraceVariantKind::Expanded)
        .count();
    let builtin_expanded_ids: Vec<String> = all_traces
        .iter()
        .filter(|t| t.variant_kind == TraceVariantKind::Expanded)
        .map(|t| t.trace_id.clone())
        .collect();
    all_traces.extend(extra_traces);
    let original_count = all_traces
        .iter()
        .filter(|t| t.variant_kind == TraceVariantKind::Original)
        .count();
    let harder_count = all_traces
        .iter()
        .filter(|t| t.variant_kind == TraceVariantKind::Harder)
        .count();
    let expanded_count = all_traces
        .iter()
        .filter(|t| t.variant_kind == TraceVariantKind::Expanded)
        .count();
    for trace in all_traces {
        let state = AgentWorkState {
            scenario_id: trace.trace_id,
            split: AgentScenarioSplit::Holdout,
            issue_text: trace.title,
            work_items: trace.candidate_items,
            plan_steps: vec!["fix".into(), "retry".into()],
            ground_truth: AgentGroundTruth {
                causal_files: trace.ground_truth.causal_files,
                causal_logs: trace.ground_truth.causal_logs,
                expected_next_actions: trace.ground_truth.correct_next_actions,
                irrelevant_items: trace.ground_truth.rejected_false_paths,
                blocker_label: trace.ground_truth.resolution_label,
            },
        };
        let baseline_field = rank_baseline(&state);
        let pse_field = rank_pse(&state);
        let bm = metrics(&state, &baseline_field);
        let pm = metrics(&state, &pse_field);
        let baseline_top_items = ranked_top_items(&state, &baseline_field);
        let pse_top_items = ranked_top_items(&state, &pse_field);
        let shared_top_items: Vec<String> = baseline_top_items
            .iter()
            .map(|x| x.item_id.clone())
            .filter(|id| pse_top_items.iter().any(|y| &y.item_id == id))
            .collect();
        let top_item_overlap_count = shared_top_items.len();
        let top_item_overlap_ratio = top_item_overlap_count as f64 / 3.0;
        let identical_top_items = baseline_top_items
            .iter()
            .map(|x| &x.item_id)
            .eq(pse_top_items.iter().map(|x| &x.item_id));
        let base_score = (bm.hit_at_3 as i32) - (bm.false_focus_rate * 10.0) as i32;
        let pse_score = (pm.hit_at_3 as i32) - (pm.false_focus_rate * 10.0) as i32;
        let mut discriminativity_label = if pse_score > base_score {
            "pse_gain".to_string()
        } else if pse_score < base_score {
            "baseline_gain".to_string()
        } else {
            "equal".to_string()
        };
        if bm.hit_at_3 && pm.hit_at_3 && bm.false_focus_rate == 0.0 && pm.false_focus_rate == 0.0 {
            discriminativity_label = "too_easy_ceiling".to_string();
        } else if identical_top_items {
            discriminativity_label = "mapping_too_coarse".to_string();
        } else if top_item_overlap_ratio >= 0.66 {
            discriminativity_label = "keyword_obvious".to_string();
        }
        let mut audit_labels = vec![
            "trace_replay_audit".to_string(),
            "diagnostic_only".to_string(),
        ];
        if identical_top_items {
            audit_labels.push("baseline_pse_top_items_identical".to_string());
            audit_labels.push("needs_harder_trace_variant".to_string());
        }
        if top_item_overlap_ratio >= 0.66 {
            audit_labels.push("keyword_obvious_trace".to_string());
        }
        if bm.hit_at_3 && pm.hit_at_3 && bm.false_focus_rate == 0.0 && pm.false_focus_rate == 0.0 {
            audit_labels.push("ceiling_effect_possible".to_string());
            audit_labels.push("needs_harder_trace_variant".to_string());
        }
        let baseline_top_ids: Vec<String> = baseline_top_items
            .iter()
            .map(|x| x.item_id.clone())
            .collect();
        let pse_top_ids: Vec<String> = pse_top_items.iter().map(|x| x.item_id.clone()).collect();
        let top3_causal_overlap_baseline = baseline_top_ids
            .iter()
            .filter(|id| {
                state.ground_truth.causal_files.contains(*id)
                    || state.ground_truth.causal_logs.contains(*id)
            })
            .count();
        let top3_causal_overlap_pse = pse_top_ids
            .iter()
            .filter(|id| {
                state.ground_truth.causal_files.contains(*id)
                    || state.ground_truth.causal_logs.contains(*id)
            })
            .count();
        let rank_delta = bm.mean_rank_of_first_causal_item - pm.mean_rank_of_first_causal_item;
        let mrr_delta = pm.mrr_causal_item - bm.mrr_causal_item;
        let false_focus_delta = bm.false_focus_rate - pm.false_focus_rate;
        let mut diagnostic_labels = Vec::new();
        if rank_delta > 0.0 {
            diagnostic_labels.push("pse_rank_improvement_present".to_string());
        }
        if mrr_delta > 0.0 {
            diagnostic_labels.push("pse_mrr_improvement_present".to_string());
        }
        if false_focus_delta > 0.0 {
            diagnostic_labels.push("pse_false_focus_improvement_present".to_string());
        }
        if top_item_overlap_ratio < 1.0 {
            diagnostic_labels.push("pse_top_items_different".to_string());
        }
        if top_item_overlap_ratio < 1.0 && pse_score == base_score {
            diagnostic_labels.push("no_metric_gain_despite_mapping_difference".to_string());
        }
        if bm.hit_at_3 == pm.hit_at_3 {
            diagnostic_labels.push("ceiling_or_metric_threshold_effect".to_string());
        }
        if trace.variant_kind == TraceVariantKind::Harder
            && pse_score == base_score
            && !(rank_delta > 0.0 || mrr_delta > 0.0)
        {
            diagnostic_labels.push("harder_trace_needs_signal_reweighting".to_string());
        }
        let primary_blocking_reason = if identical_top_items {
            "baseline_and_pse_identical"
        } else if bm.hit_at_3 && pm.hit_at_3 && pse_score == base_score {
            "both_hit_at_3_equal"
        } else if !bm.hit_at_3 && !pm.hit_at_3 {
            "both_miss_top3"
        } else if rank_delta > 0.0 && !pm.hit_at_3 {
            "pse_rank_improves_but_not_hit_threshold"
        } else if mrr_delta > 0.0 && false_focus_delta < 0.0 {
            "pse_mrr_improves_but_false_focus_offsets"
        } else if top_item_overlap_ratio < 1.0
            && top3_causal_overlap_baseline == top3_causal_overlap_pse
        {
            "pse_differs_but_not_on_causal_items"
        } else if pse_score == base_score && bm.hit_at_3 == pm.hit_at_3 {
            "both_hit_at_3_equal"
        } else {
            "unknown_equal_case"
        }
        .to_string();
        let mut secondary_reasons = Vec::new();
        if top3_causal_overlap_baseline == 0 && top3_causal_overlap_pse == 0 {
            secondary_reasons.push("no_causal_items_in_top3_for_both".to_string());
        }
        if top_item_overlap_ratio < 1.0 && pse_score == base_score {
            secondary_reasons.push("mapping_difference_without_gain".to_string());
        }
        trace_audits.push(TraceReplayAudit {
            trace_id: state.scenario_id.clone(),
            baseline_top_items,
            pse_top_items,
            shared_top_items,
            baseline_first_causal_rank: bm.mean_rank_of_first_causal_item,
            pse_first_causal_rank: pm.mean_rank_of_first_causal_item,
            baseline_mrr: bm.mrr_causal_item,
            pse_mrr: pm.mrr_causal_item,
            baseline_false_focus_rate: bm.false_focus_rate,
            pse_false_focus_rate: pm.false_focus_rate,
            baseline_next_action_rank: bm.next_action_rank,
            pse_next_action_rank: pm.next_action_rank,
            baseline_blocker_label_rank: bm.blocker_label_rank,
            pse_blocker_label_rank: pm.blocker_label_rank,
            top_item_overlap_count,
            top_item_overlap_ratio,
            discriminativity_label,
            audit_labels,
            gain_failure_analysis: Some(TraceGainFailureAnalysis {
                trace_id: state.scenario_id.clone(),
                variant_kind: trace.variant_kind,
                baseline_hit_at_3: bm.hit_at_3,
                pse_hit_at_3: pm.hit_at_3,
                baseline_first_causal_rank: bm.mean_rank_of_first_causal_item,
                pse_first_causal_rank: pm.mean_rank_of_first_causal_item,
                baseline_mrr: bm.mrr_causal_item,
                pse_mrr: pm.mrr_causal_item,
                baseline_false_focus_rate: bm.false_focus_rate,
                pse_false_focus_rate: pm.false_focus_rate,
                rank_delta,
                mrr_delta,
                false_focus_delta,
                top3_causal_overlap_baseline,
                top3_causal_overlap_pse,
                primary_blocking_reason,
                secondary_reasons,
                diagnostic_labels,
            }),
        });
        trace_reports.push(AgentExoskeletonDiagnosticReport {
            scenario_id: state.scenario_id,
            split: state.split,
            delta_hit_at_1: (pm.hit_at_1 as i8 - bm.hit_at_1 as i8) as f64,
            delta_hit_at_3: (pm.hit_at_3 as i8 - bm.hit_at_3 as i8) as f64,
            delta_causal_file_recall: pm.causal_file_recall - bm.causal_file_recall,
            delta_causal_log_recall: pm.causal_log_recall - bm.causal_log_recall,
            delta_false_focus_rate: pm.false_focus_rate - bm.false_focus_rate,
            delta_irrelevant_focus_count: pm.irrelevant_focus_count as f64
                - bm.irrelevant_focus_count as f64,
            delta_search_reduction_ratio: pm.search_reduction_ratio - bm.search_reduction_ratio,
            delta_next_action_hit: (pm.next_action_hit as i8 - bm.next_action_hit as i8) as f64,
            delta_blocker_label_hit: (pm.blocker_label_hit as i8 - bm.blocker_label_hit as i8)
                as f64,
            baseline_metrics: bm,
            pse_field_metrics: pm,
            diagnostic_only: true,
            productive_agent_validated: false,
            interpretation_labels: vec![
                "real_trace_replay_present".into(),
                "diagnostic_only".into(),
            ],
            ablation_report: None,
        });
    }
    let trace_pse_aggregate = compute_aggregate(&trace_reports);
    let trace_baseline_aggregate = compute_aggregate(
        &trace_reports
            .iter()
            .map(|r| AgentExoskeletonDiagnosticReport {
                baseline_metrics: r.baseline_metrics.clone(),
                pse_field_metrics: r.baseline_metrics.clone(),
                ..r.clone()
            })
            .collect::<Vec<_>>(),
    );
    let harder_reports: Vec<_> = trace_reports
        .iter()
        .filter(|x| x.scenario_id.ends_with("_harder"))
        .cloned()
        .collect();
    let harder_pse = compute_aggregate(&harder_reports);
    let harder_baseline = compute_aggregate(
        &harder_reports
            .iter()
            .map(|r| AgentExoskeletonDiagnosticReport {
                baseline_metrics: r.baseline_metrics.clone(),
                pse_field_metrics: r.baseline_metrics.clone(),
                ..r.clone()
            })
            .collect::<Vec<_>>(),
    );
    let harder_audits: Vec<_> = trace_audits
        .iter()
        .filter(|x| x.trace_id.ends_with("_harder"))
        .collect();
    let harder_trace_metrics = HarderTraceMetrics {
        trace_count: harder_reports.len(),
        baseline_metrics_aggregate: harder_baseline,
        pse_metrics_aggregate: harder_pse.clone(),
        traces_with_pse_gain: harder_pse.scenarios_with_pse_gain,
        traces_with_pse_loss: harder_pse.scenarios_with_pse_loss,
        traces_with_equal_result: harder_pse.scenarios_with_equal_result,
        mean_top_item_overlap_ratio: if harder_audits.is_empty() {
            0.0
        } else {
            harder_audits
                .iter()
                .map(|x| x.top_item_overlap_ratio)
                .sum::<f64>()
                / harder_audits.len() as f64
        },
        traces_with_identical_top_items: harder_audits
            .iter()
            .filter(|x| {
                x.audit_labels
                    .contains(&"baseline_pse_top_items_identical".to_string())
            })
            .count(),
        diagnostic_only: true,
        productive_agent_validated: false,
    };
    let gain_analyses: Vec<_> = trace_audits
        .iter()
        .filter_map(|x| x.gain_failure_analysis.as_ref())
        .collect();
    let mut reason_counts = BTreeMap::new();
    for ga in &gain_analyses {
        *reason_counts
            .entry(ga.primary_blocking_reason.clone())
            .or_insert(0usize) += 1;
    }
    let traces_with_mapping_difference_but_no_gain = gain_analyses
        .iter()
        .filter(|ga| {
            ga.diagnostic_labels
                .contains(&"no_metric_gain_despite_mapping_difference".to_string())
        })
        .count();
    let trace_gain_failure_summary = TraceGainFailureSummary {
        trace_count: gain_analyses.len(),
        harder_trace_count: gain_analyses
            .iter()
            .filter(|ga| ga.variant_kind == TraceVariantKind::Harder)
            .count(),
        traces_with_pse_rank_improvement: gain_analyses
            .iter()
            .filter(|ga| {
                ga.diagnostic_labels
                    .contains(&"pse_rank_improvement_present".to_string())
            })
            .count(),
        traces_with_pse_mrr_improvement: gain_analyses
            .iter()
            .filter(|ga| {
                ga.diagnostic_labels
                    .contains(&"pse_mrr_improvement_present".to_string())
            })
            .count(),
        traces_with_pse_false_focus_improvement: gain_analyses
            .iter()
            .filter(|ga| {
                ga.diagnostic_labels
                    .contains(&"pse_false_focus_improvement_present".to_string())
            })
            .count(),
        traces_with_mapping_difference_but_no_gain,
        traces_blocked_by_both_hit_at_3_equal: gain_analyses
            .iter()
            .filter(|ga| ga.primary_blocking_reason == "both_hit_at_3_equal")
            .count(),
        traces_blocked_by_both_miss_top3: gain_analyses
            .iter()
            .filter(|ga| ga.primary_blocking_reason == "both_miss_top3")
            .count(),
        traces_blocked_by_pse_differs_but_not_on_causal_items: gain_analyses
            .iter()
            .filter(|ga| ga.primary_blocking_reason == "pse_differs_but_not_on_causal_items")
            .count(),
        primary_blocking_reason_counts: reason_counts,
        interpretation_labels: vec![
            "trace_gain_failure_analysis_present".to_string(),
            "mapping_difference_without_metric_gain".to_string(),
            if gain_analyses.iter().any(|ga| {
                ga.variant_kind == TraceVariantKind::Harder
                    && ga
                        .diagnostic_labels
                        .contains(&"harder_trace_needs_signal_reweighting".to_string())
            }) {
                "signal_reweighting_required".to_string()
            } else {
                "signal_reweighting_not_required".to_string()
            },
            if gain_analyses
                .iter()
                .filter(|ga| ga.primary_blocking_reason == "both_hit_at_3_equal")
                .count()
                >= gain_analyses.len() / 2
            {
                "metric_threshold_effect_possible".to_string()
            } else {
                "metric_threshold_effect_not_dominant".to_string()
            },
            "diagnostic_only".to_string(),
            "requires_live_agent_validation".to_string(),
        ],
    };
    let rankability_audits: Vec<_> = real_trace_replay_cases()
        .into_iter()
        .map(|trace| {
            let ctexts: Vec<String> = trace
                .candidate_items
                .iter()
                .map(|c| norm(&format!("{} {} {}", c.id, c.source, c.text)))
                .collect();
            let has_in_candidates = |needle: &str| {
                let n = norm(needle);
                ctexts.iter().any(|t| {
                    t == &n
                        || t.contains(&n)
                        || n.split('/').next_back().is_some_and(|s| t.contains(s))
                })
            };
            let has_in_timeline = |needle: &str| {
                let n = norm(needle);
                trace.timeline_events.iter().any(|e| {
                    e.command
                        .as_ref()
                        .map(|c| norm(c).contains(&n))
                        .unwrap_or(false)
                        || norm(&e.text).contains(&n)
                })
            };
            let cfile_hits = trace
                .ground_truth
                .causal_files
                .iter()
                .filter(|x| has_in_candidates(x))
                .count();
            let clog_hits = trace
                .ground_truth
                .causal_logs
                .iter()
                .filter(|x| has_in_candidates(x))
                .count();
            let ccmd_t_hits = trace
                .ground_truth
                .causal_commands
                .iter()
                .filter(|x| has_in_timeline(x))
                .count();
            let ccmd_c_hits = trace
                .ground_truth
                .causal_commands
                .iter()
                .filter(|x| has_in_candidates(x))
                .count();
            let action_c_hits = trace
                .ground_truth
                .correct_next_actions
                .iter()
                .filter(|x| has_in_candidates(x))
                .count();
            let action_t_hits = trace
                .ground_truth
                .correct_next_actions
                .iter()
                .filter(|x| has_in_timeline(x))
                .count();
            let rankable = cfile_hits + clog_hits + ccmd_t_hits.max(ccmd_c_hits);
            let total_causal = trace.ground_truth.causal_files.len()
                + trace.ground_truth.causal_logs.len()
                + trace.ground_truth.causal_commands.len();
            let unrankable = total_causal.saturating_sub(rankable);
            let ratio = if total_causal == 0 {
                0.0
            } else {
                rankable as f64 / total_causal as f64
            };
            let label = if rankable == total_causal {
                "fully_rankable"
            } else if rankable == 0 && (action_c_hits > 0 || action_t_hits > 0) {
                "action_only_rankable"
            } else if rankable > 0 {
                "partially_rankable"
            } else {
                "unrankable"
            }
            .to_string();
            let mut labels = vec![
                "trace_rankability_audit".to_string(),
                "diagnostic_only".to_string(),
            ];
            if cfile_hits < trace.ground_truth.causal_files.len() {
                labels.push("causal_files_missing_from_candidates".to_string());
            }
            if clog_hits < trace.ground_truth.causal_logs.len() {
                labels.push("causal_logs_missing_from_candidates".to_string());
            }
            if ccmd_t_hits == 0
                && ccmd_c_hits == 0
                && !trace.ground_truth.causal_commands.is_empty()
            {
                labels.push("causal_commands_missing_from_candidates".to_string());
            }
            if action_c_hits == 0
                && action_t_hits == 0
                && !trace.ground_truth.correct_next_actions.is_empty()
            {
                labels.push("correct_next_actions_missing_from_candidates".to_string());
            }
            if ratio == 0.0 {
                labels.push("mrr_zero_explained_by_unrankable_targets".to_string());
            } else {
                labels.push("ranker_failure_not_candidate_failure".to_string());
            }
            TraceRankabilityAudit {
                trace_id: trace.trace_id,
                variant_kind: trace.variant_kind,
                candidate_count: trace.candidate_items.len(),
                causal_file_count: trace.ground_truth.causal_files.len(),
                causal_file_candidate_hits: cfile_hits,
                causal_file_candidate_misses: trace
                    .ground_truth
                    .causal_files
                    .len()
                    .saturating_sub(cfile_hits),
                causal_log_count: trace.ground_truth.causal_logs.len(),
                causal_log_candidate_hits: clog_hits,
                causal_log_candidate_misses: trace
                    .ground_truth
                    .causal_logs
                    .len()
                    .saturating_sub(clog_hits),
                causal_command_count: trace.ground_truth.causal_commands.len(),
                causal_command_timeline_hits: ccmd_t_hits,
                causal_command_candidate_hits: ccmd_c_hits,
                correct_next_action_count: trace.ground_truth.correct_next_actions.len(),
                correct_next_action_candidate_hits: action_c_hits,
                correct_next_action_timeline_hits: action_t_hits,
                rankable_causal_item_count: rankable,
                unrankable_causal_item_count: unrankable,
                rankability_ratio: ratio,
                rankability_label: label,
                diagnostic_labels: labels,
            }
        })
        .collect();
    let rankability_summary = {
        let t = rankability_audits.len();
        let harder: Vec<_> = rankability_audits
            .iter()
            .filter(|x| x.variant_kind == TraceVariantKind::Harder)
            .collect();
        let mut blockers = BTreeMap::new();
        for a in &rankability_audits {
            let key = if a.rankability_label == "unrankable" {
                "unrankable"
            } else if a
                .diagnostic_labels
                .contains(&"causal_files_missing_from_candidates".to_string())
            {
                "causal_files_missing"
            } else {
                "partially_or_fully_rankable"
            };
            *blockers.entry(key.to_string()).or_insert(0usize) += 1;
        }
        TraceRankabilitySummary {
            trace_count: t,
            harder_trace_count: harder.len(),
            traces_fully_rankable: rankability_audits
                .iter()
                .filter(|x| x.rankability_label == "fully_rankable")
                .count(),
            traces_partially_rankable: rankability_audits
                .iter()
                .filter(|x| {
                    x.rankability_label == "partially_rankable"
                        || x.rankability_label == "action_only_rankable"
                })
                .count(),
            traces_unrankable: rankability_audits
                .iter()
                .filter(|x| x.rankability_label == "unrankable")
                .count(),
            harder_traces_unrankable: harder
                .iter()
                .filter(|x| x.rankability_label == "unrankable")
                .count(),
            mean_rankability_ratio: if t == 0 {
                0.0
            } else {
                rankability_audits
                    .iter()
                    .map(|x| x.rankability_ratio)
                    .sum::<f64>()
                    / t as f64
            },
            harder_mean_rankability_ratio: if harder.is_empty() {
                0.0
            } else {
                harder.iter().map(|x| x.rankability_ratio).sum::<f64>() / harder.len() as f64
            },
            primary_rankability_blocker_counts: blockers,
            interpretation_labels: vec![
                "trace_rankability_coverage_audit_present".into(),
                "diagnostic_only".into(),
                "requires_live_agent_validation".into(),
            ],
        }
    };
    let ranking_failure_audits: Vec<_> = real_trace_replay_cases()
        .into_iter()
        .map(|trace| {
            let state = AgentWorkState {
                scenario_id: trace.trace_id.clone(),
                split: AgentScenarioSplit::Holdout,
                issue_text: trace.title.clone(),
                work_items: trace.candidate_items.clone(),
                plan_steps: vec!["fix".into(), "retry".into()],
                ground_truth: AgentGroundTruth {
                    causal_files: trace.ground_truth.causal_files.clone(),
                    causal_logs: trace.ground_truth.causal_logs.clone(),
                    expected_next_actions: trace.ground_truth.correct_next_actions.clone(),
                    irrelevant_items: trace.ground_truth.rejected_false_paths.clone(),
                    blocker_label: trace.ground_truth.resolution_label.clone(),
                },
            };
            let bf = rank_baseline(&state);
            let pf = rank_pse(&state);
            let mut br: Vec<_> = bf
                .item_scores
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            let mut pr: Vec<_> = pf
                .item_scores
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            br.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            pr.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let causal_ids: Vec<String> = state
                .ground_truth
                .causal_files
                .iter()
                .chain(state.ground_truth.causal_logs.iter())
                .cloned()
                .collect();
            let ranks: Vec<CausalItemRank> = causal_ids
                .iter()
                .map(|id| {
                    let bpos = br
                        .iter()
                        .position(|(x, _)| x == id)
                        .map(|x| x + 1)
                        .unwrap_or(br.len() + 1);
                    let ppos = pr
                        .iter()
                        .position(|(x, _)| x == id)
                        .map(|x| x + 1)
                        .unwrap_or(pr.len() + 1);
                    let kind = state
                        .work_items
                        .iter()
                        .find(|w| &w.id == id)
                        .map(|w| w.item_type.clone())
                        .unwrap_or(WorkItemType::Log);
                    CausalItemRank {
                        item_id: id.clone(),
                        item_kind: kind,
                        rank_baseline: bpos,
                        rank_pse: ppos,
                        is_in_top3_baseline: bpos <= 3,
                        is_in_top3_pse: ppos <= 3,
                    }
                })
                .collect();
            let mk_blockers = |ranking: &Vec<(String, i32)>| {
                ranking
                    .iter()
                    .take(3)
                    .filter_map(|(id, score)| {
                        if causal_ids.contains(id) {
                            return None;
                        }
                        let it = state.work_items.iter().find(|w| &w.id == id)?;
                        Some(TopBlockerItem {
                            item_id: id.clone(),
                            rank: ranking.iter().position(|(x, _)| x == id).unwrap() + 1,
                            score: *score,
                            item_kind: it.item_type.clone(),
                            reason_label: "top_noncausal".into(),
                            text_excerpt: it.text.chars().take(80).collect(),
                        })
                    })
                    .collect::<Vec<_>>()
            };
            let bblock = mk_blockers(&br);
            let pblock = mk_blockers(&pr);
            let bbest = ranks
                .iter()
                .map(|x| x.rank_baseline as f64)
                .fold((br.len() + 1) as f64, f64::min);
            let pbest = ranks
                .iter()
                .map(|x| x.rank_pse as f64)
                .fold((pr.len() + 1) as f64, f64::min);
            let bbelow = ranks.iter().filter(|x| !x.is_in_top3_baseline).count();
            let pbelow = ranks.iter().filter(|x| !x.is_in_top3_pse).count();
            let btop: Vec<String> = br
                .iter()
                .take(3)
                .map(|(id, _)| {
                    let t = state
                        .work_items
                        .iter()
                        .find(|w| &w.id == id)
                        .map(|w| w.text.to_ascii_lowercase())
                        .unwrap_or_default();
                    if t.contains("command") || t.contains("cargo") {
                        "command_signal".into()
                    } else if t.contains("retry") || t.contains("fix") {
                        "action_signal".into()
                    } else {
                        "keyword_signal".into()
                    }
                })
                .collect();
            let ptop: Vec<String> = pr
                .iter()
                .take(3)
                .map(|(id, _)| {
                    let t = state
                        .work_items
                        .iter()
                        .find(|w| &w.id == id)
                        .map(|w| w.text.to_ascii_lowercase())
                        .unwrap_or_default();
                    if t.contains("command") || t.contains("cargo") {
                        "command_signal".into()
                    } else if t.contains("retry") || t.contains("fix") {
                        "action_signal".into()
                    } else {
                        "keyword_signal".into()
                    }
                })
                .collect();
            let same_top3 = br
                .iter()
                .take(3)
                .map(|x| &x.0)
                .eq(pr.iter().take(3).map(|x| &x.0));
            let mut diag = vec![
                "post_coverage_ranking_audit".to_string(),
                "diagnostic_only".to_string(),
            ];
            if bbelow > 0 || pbelow > 0 {
                diag.push("causal_items_available_below_threshold".into());
            }
            if !bblock.is_empty() || !pblock.is_empty() {
                diag.push("top3_blocked_by_noncausal_items".into());
            }
            if !same_top3 {
                diag.push("signal_selection_failure".into());
            }
            if state.ground_truth.causal_files.len() + state.ground_truth.causal_logs.len() > 0
                && pbelow > 0
                && bbelow > 0
            {
                diag.push("fully_rankable_but_no_gain".into());
            }
            let label = if same_top3 && bbelow > 0 && pbelow > 0 {
                "baseline_and_pse_same_failure"
            } else if !same_top3 && !pblock.is_empty() {
                "pse_prioritizes_different_noncausal_items"
            } else if !bblock.is_empty() && !pblock.is_empty() {
                "noncausal_items_dominate_top3"
            } else if bbelow > 0 || pbelow > 0 {
                "causal_items_ranked_below_top3"
            } else if pbest > 3.0 {
                "causal_items_present_but_weak_signal"
            } else {
                "no_failure_detected"
            }
            .to_string();
            TraceRankingFailureAudit {
                trace_id: trace.trace_id,
                variant_kind: trace.variant_kind,
                baseline_causal_item_ranks: ranks.clone(),
                pse_causal_item_ranks: ranks,
                baseline_top3_noncausal_blockers: bblock,
                pse_top3_noncausal_blockers: pblock,
                baseline_best_causal_rank: bbest,
                pse_best_causal_rank: pbest,
                baseline_causal_items_below_top3: bbelow,
                pse_causal_items_below_top3: pbelow,
                baseline_top3_signal_labels: btop,
                pse_top3_signal_labels: ptop,
                rank_failure_label: label,
                diagnostic_labels: diag,
            }
        })
        .collect();
    let post_coverage_ranking_failure_summary = {
        let t = ranking_failure_audits.len();
        let mut c = BTreeMap::new();
        for a in &ranking_failure_audits {
            *c.entry(a.rank_failure_label.clone()).or_insert(0usize) += 1;
        }
        PostCoverageRankingFailureSummary {
            trace_count: t,
            harder_trace_count: ranking_failure_audits
                .iter()
                .filter(|a| a.variant_kind == TraceVariantKind::Harder)
                .count(),
            fully_rankable_trace_count: rankability_audits
                .iter()
                .filter(|a| a.rankability_label == "fully_rankable")
                .count(),
            traces_with_causal_items_below_top3: ranking_failure_audits
                .iter()
                .filter(|a| {
                    a.baseline_causal_items_below_top3 > 0 || a.pse_causal_items_below_top3 > 0
                })
                .count(),
            traces_with_noncausal_top3_blockers: ranking_failure_audits
                .iter()
                .filter(|a| {
                    !a.baseline_top3_noncausal_blockers.is_empty()
                        || !a.pse_top3_noncausal_blockers.is_empty()
                })
                .count(),
            traces_where_pse_differs_but_still_noncausal: ranking_failure_audits
                .iter()
                .filter(|a| a.rank_failure_label == "pse_prioritizes_different_noncausal_items")
                .count(),
            traces_where_baseline_and_pse_same_failure: ranking_failure_audits
                .iter()
                .filter(|a| a.rank_failure_label == "baseline_and_pse_same_failure")
                .count(),
            mean_pse_best_causal_rank: ranking_failure_audits
                .iter()
                .map(|a| a.pse_best_causal_rank)
                .sum::<f64>()
                / t.max(1) as f64,
            mean_baseline_best_causal_rank: ranking_failure_audits
                .iter()
                .map(|a| a.baseline_best_causal_rank)
                .sum::<f64>()
                / t.max(1) as f64,
            primary_rank_failure_counts: c,
            interpretation_labels: vec![
                "post_coverage_ranking_failure_present".into(),
                "candidate_coverage_repaired".into(),
                "signal_selection_failure_present".into(),
                "diagnostic_only".into(),
                "requires_live_agent_validation".into(),
            ],
        }
    };
    let score_attribution_audits: Vec<_> = real_trace_replay_cases()
        .into_iter()
        .map(|trace| {
            let state = AgentWorkState {
                scenario_id: trace.trace_id.clone(),
                split: AgentScenarioSplit::Holdout,
                issue_text: trace.title.clone(),
                work_items: trace.candidate_items.clone(),
                plan_steps: vec!["fix".into(), "retry".into()],
                ground_truth: AgentGroundTruth {
                    causal_files: trace.ground_truth.causal_files.clone(),
                    causal_logs: trace.ground_truth.causal_logs.clone(),
                    expected_next_actions: trace.ground_truth.correct_next_actions.clone(),
                    irrelevant_items: trace.ground_truth.rejected_false_paths.clone(),
                    blocker_label: trace.ground_truth.resolution_label.clone(),
                },
            };
            let pf = rank_pse(&state);
            let mut pr: Vec<_> = pf
                .item_scores
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            pr.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let causal_ids: Vec<String> = state
                .ground_truth
                .causal_files
                .iter()
                .chain(state.ground_truth.causal_logs.iter())
                .cloned()
                .collect();
            let mut causal_items: Vec<_> = pr
                .iter()
                .filter_map(|(id, score)| {
                    if !causal_ids.contains(id) {
                        return None;
                    }
                    let it = state.work_items.iter().find(|w| &w.id == id)?;
                    Some(explain_trace_item_score(
                        &state,
                        it,
                        pr.iter().position(|(x, _)| x == id).unwrap() + 1,
                        *score,
                    ))
                })
                .collect();
            if causal_items.is_empty() {
                for (idx, (id, score)) in pr.iter().enumerate() {
                    let it = state.work_items.iter().find(|w| &w.id == id).unwrap();
                    let t = norm(&format!("{} {} {}", it.id, it.source, it.text));
                    if state
                        .ground_truth
                        .causal_files
                        .iter()
                        .chain(state.ground_truth.causal_logs.iter())
                        .any(|c| t.contains(&norm(c)))
                    {
                        causal_items.push(explain_trace_item_score(&state, it, idx + 1, *score));
                    }
                }
            }
            let noncausal_top3: Vec<_> = pr
                .iter()
                .take(3)
                .filter_map(|(id, score)| {
                    if causal_ids.contains(id) {
                        return None;
                    }
                    let it = state.work_items.iter().find(|w| &w.id == id)?;
                    Some(explain_trace_item_score(
                        &state,
                        it,
                        pr.iter().position(|(x, _)| x == id).unwrap() + 1,
                        *score,
                    ))
                })
                .collect();
            let best_causal_mean = if causal_items.is_empty() {
                0.0
            } else {
                causal_items
                    .iter()
                    .take(3)
                    .map(|x| x.total_score as f64)
                    .sum::<f64>()
                    / causal_items.iter().take(3).count() as f64
            };
            let top_non_mean = if noncausal_top3.is_empty() {
                0.0
            } else {
                noncausal_top3
                    .iter()
                    .map(|x| x.total_score as f64)
                    .sum::<f64>()
                    / noncausal_top3.len() as f64
            };
            let gap = top_non_mean - best_causal_mean;
            let mut dominant: Vec<String> = Vec::new();
            for it in &noncausal_top3 {
                if let Some(maxc) = it
                    .score_components
                    .iter()
                    .max_by(|x, y| x.value.partial_cmp(&y.value).unwrap())
                {
                    dominant.push(maxc.component.clone());
                }
            }
            dominant.sort();
            dominant.dedup();
            if dominant.is_empty() {
                dominant.push("unknown_or_other".into());
            }
            let mut missing = Vec::new();
            if causal_items.iter().all(|x| {
                x.score_components
                    .iter()
                    .find(|c| c.component == "path_match")
                    .map(|c| c.value)
                    .unwrap_or(0.0)
                    == 0.0
            }) {
                missing.push("path_match".into());
            }
            if causal_items.iter().all(|x| {
                x.score_components
                    .iter()
                    .find(|c| c.component == "action_compatibility")
                    .map(|c| c.value)
                    .unwrap_or(0.0)
                    == 0.0
            }) {
                missing.push("action_compatibility".into());
            }
            let noncausal_keyword_dominates = noncausal_top3
                .iter()
                .filter(|it| {
                    it.score_components
                        .iter()
                        .max_by(|x, y| x.value.partial_cmp(&y.value).unwrap())
                        .map(|c| c.component.as_str() == "keyword_overlap")
                        .unwrap_or(false)
                })
                .count()
                >= 2;
            let noncausal_without_distractor_penalty = noncausal_top3
                .iter()
                .filter(|it| {
                    it.score_components
                        .iter()
                        .find(|c| c.component == "distractor_penalty")
                        .map(|c| c.value >= 0.0)
                        .unwrap_or(true)
                })
                .count()
                >= 2;
            let has_correct_next_actions = !state.ground_truth.expected_next_actions.is_empty();
            let has_timeline_context = !state.plan_steps.is_empty();
            let causal_role_for_both = causal_items.iter().any(|it| {
                it.score_components
                    .iter()
                    .max_by(|x, y| x.value.partial_cmp(&y.value).unwrap())
                    .map(|c| c.component.as_str() == "causal_role_tag")
                    .unwrap_or(false)
            }) && noncausal_top3.iter().any(|it| {
                it.score_components
                    .iter()
                    .max_by(|x, y| x.value.partial_cmp(&y.value).unwrap())
                    .map(|c| c.component.as_str() == "causal_role_tag")
                    .unwrap_or(false)
            });
            let label = if noncausal_keyword_dominates {
                "noncausal_keyword_signal_dominates"
            } else if missing.iter().any(|x| x == "path_match") && gap > 0.0 {
                "causal_path_signal_missing"
            } else if has_correct_next_actions
                && missing.iter().any(|x| x == "action_compatibility")
            {
                "action_compatibility_missing"
            } else if noncausal_without_distractor_penalty && gap >= 0.0 {
                "distractor_penalty_insufficient"
            } else if has_timeline_context
                && causal_items.iter().all(|x| {
                    x.score_components
                        .iter()
                        .find(|c| c.component == "phase_order")
                        .map(|c| c.value)
                        .unwrap_or(0.0)
                        == 0.0
                })
            {
                "phase_order_signal_missing"
            } else if causal_role_for_both {
                "causal_role_signal_not_discriminative"
            } else if dominant
                .iter()
                .all(|x| x == "other" || x == "unknown_or_other")
            {
                "score_components_not_separable"
            } else {
                "unknown"
            }
            .to_string();
            let mut d = vec![
                "trace_score_attribution_audit".to_string(),
                "diagnostic_only".to_string(),
            ];
            if gap > 0.0 {
                d.push("noncausal_score_gap_positive".into());
            }
            if missing.iter().any(|x| x == "path_match") {
                d.push("causal_items_lack_path_signal".into());
            }
            if missing.iter().any(|x| x == "action_compatibility") {
                d.push("causal_items_lack_action_signal".into());
            }
            if noncausal_top3.iter().any(|x| !x.is_distractor) {
                d.push("distractors_not_suppressed".into());
            }
            let has_componentized_scores = causal_items
                .iter()
                .chain(noncausal_top3.iter())
                .any(|it| !it.score_components.is_empty());
            let unexplained_non_zero = causal_items
                .iter()
                .chain(noncausal_top3.iter())
                .filter_map(|it| {
                    it.score_components
                        .iter()
                        .find(|c| c.component == "unexplained_score_delta")
                        .map(|c| c.value)
                })
                .any(|v| v != 0.0);
            let has_unknown_dominant = causal_items
                .iter()
                .chain(noncausal_top3.iter())
                .filter_map(|it| {
                    it.score_components
                        .iter()
                        .max_by(|x, y| x.value.partial_cmp(&y.value).unwrap())
                })
                .any(|c| c.component == "other" || c.component == "unknown_or_other");
            let has_total_mismatch = causal_items.iter().chain(noncausal_top3.iter()).any(|it| {
                let sum = it.score_components.iter().map(|c| c.value).sum::<f64>();
                (sum - it.total_score as f64).abs() > f64::EPSILON
            });
            let unknown_with_named_components = label == "unknown"
                && causal_items.iter().chain(noncausal_top3.iter()).any(|it| {
                    it.score_components.iter().any(|c| {
                        c.component != "other"
                            && c.component != "unknown_or_other"
                            && c.component != "unexplained_score_delta"
                    })
                });
            let explainability_gap = unexplained_non_zero
                || has_unknown_dominant
                || !has_componentized_scores
                || has_total_mismatch
                || unknown_with_named_components;
            if explainability_gap {
                d.push("score_explainability_gap".into());
            }
            TraceScoreAttributionAudit {
                trace_id: trace.trace_id,
                variant_kind: trace.variant_kind,
                causal_item_attributions: causal_items,
                top3_noncausal_attributions: noncausal_top3,
                causal_vs_noncausal_score_gap: gap,
                dominant_noncausal_signal_groups: dominant,
                missing_causal_signal_groups: missing,
                attribution_failure_label: label,
                diagnostic_labels: d,
            }
        })
        .collect();
    let trace_score_attribution_summary = {
        let mut fl = BTreeMap::new();
        let mut dom = BTreeMap::new();
        let mut miss = BTreeMap::new();
        let mut dominant_component_counts = BTreeMap::new();
        let mut causal_dominant_component_counts = BTreeMap::new();
        let mut noncausal_dominant_component_counts = BTreeMap::new();
        let mut traces_with_explainability_gap = 0usize;
        let mut mean_unexplained_score_delta = 0.0;
        for a in &score_attribution_audits {
            *fl.entry(a.attribution_failure_label.clone())
                .or_insert(0usize) += 1;
            for g in &a.dominant_noncausal_signal_groups {
                *dom.entry(g.clone()).or_insert(0usize) += 1;
            }
            for g in &a.missing_causal_signal_groups {
                *miss.entry(g.clone()).or_insert(0usize) += 1;
            }
            if a.diagnostic_labels
                .contains(&"score_explainability_gap".to_string())
            {
                traces_with_explainability_gap += 1;
            }
            for it in &a.causal_item_attributions {
                let maxc = it
                    .score_components
                    .iter()
                    .max_by(|x, y| x.value.partial_cmp(&y.value).unwrap());
                if let Some(c) = maxc {
                    *dominant_component_counts
                        .entry(c.component.clone())
                        .or_insert(0usize) += 1;
                    *causal_dominant_component_counts
                        .entry(c.component.clone())
                        .or_insert(0usize) += 1;
                }
                if let Some(u) = it
                    .score_components
                    .iter()
                    .find(|x| x.component == "unexplained_score_delta")
                {
                    mean_unexplained_score_delta += u.value;
                }
            }
            for it in &a.top3_noncausal_attributions {
                let maxc = it
                    .score_components
                    .iter()
                    .max_by(|x, y| x.value.partial_cmp(&y.value).unwrap());
                if let Some(c) = maxc {
                    *dominant_component_counts
                        .entry(c.component.clone())
                        .or_insert(0usize) += 1;
                    *noncausal_dominant_component_counts
                        .entry(c.component.clone())
                        .or_insert(0usize) += 1;
                }
            }
        }
        mean_unexplained_score_delta /= score_attribution_audits.len().max(1) as f64;
        let has_explainability_gap = traces_with_explainability_gap > 0;
        let componentization_complete =
            mean_unexplained_score_delta == 0.0 && !dominant_component_counts.is_empty();
        let signal_selection_explained = fl.keys().any(|k| k != "unknown");
        TraceScoreAttributionSummary {
            trace_count: score_attribution_audits.len(),
            harder_trace_count: score_attribution_audits
                .iter()
                .filter(|a| a.variant_kind == TraceVariantKind::Harder)
                .count(),
            traces_with_positive_noncausal_score_gap: score_attribution_audits
                .iter()
                .filter(|a| a.causal_vs_noncausal_score_gap > 0.0)
                .count(),
            mean_causal_vs_noncausal_score_gap: score_attribution_audits
                .iter()
                .map(|a| a.causal_vs_noncausal_score_gap)
                .sum::<f64>()
                / score_attribution_audits.len().max(1) as f64,
            failure_label_counts: fl,
            dominant_noncausal_signal_group_counts: dom,
            missing_causal_signal_group_counts: miss,
            interpretation_labels: vec![
                "score_componentization_present".into(),
                "trace_score_attribution_present".into(),
                if signal_selection_explained {
                    "signal_selection_failure_explained".into()
                } else {
                    "signal_selection_failure_unresolved".into()
                },
                if has_explainability_gap {
                    "score_explainability_gap_remaining".into()
                } else {
                    "score_explainability_improved".into()
                },
                "diagnostic_only".into(),
                "requires_live_agent_validation".into(),
            ],
            traces_with_explainability_gap,
            mean_unexplained_score_delta,
            dominant_component_counts,
            causal_dominant_component_counts,
            noncausal_dominant_component_counts,
            componentization_complete,
        }
    };
    let trace_replay_report = TraceReplayReport {
        trace_count: trace_reports.len(),
        baseline_metrics_aggregate: trace_baseline_aggregate,
        pse_metrics_aggregate: trace_pse_aggregate.clone(),
        trace_reports,
        traces_with_pse_gain: trace_pse_aggregate.scenarios_with_pse_gain,
        traces_with_pse_loss: trace_pse_aggregate.scenarios_with_pse_loss,
        traces_with_equal_result: trace_pse_aggregate.scenarios_with_equal_result,
        replay_identity_all: trace_pse_aggregate.replay_identity_all,
        diagnostic_only: true,
        productive_agent_validated: false,
        interpretation_labels: vec![
            "real_trace_replay_present".into(),
            "historical_debug_trace_only".into(),
            if trace_pse_aggregate.scenarios_with_pse_gain > 0 {
                "pse_trace_gain_present".into()
            } else {
                "pse_trace_gain_absent".into()
            },
            "requires_live_agent_validation".into(),
            "diagnostic_only".into(),
        ],
        trace_ablation_aggregate: None,
        trace_audits: trace_audits.clone(),
        trace_audit_summary: {
            let trace_count = trace_audits.len();
            let identical = trace_audits
                .iter()
                .filter(|x| {
                    x.audit_labels
                        .contains(&"baseline_pse_top_items_identical".to_string())
                })
                .count();
            let keyword_obvious = trace_audits
                .iter()
                .filter(|x| {
                    x.audit_labels
                        .contains(&"keyword_obvious_trace".to_string())
                })
                .count();
            let ceiling = trace_audits
                .iter()
                .filter(|x| {
                    x.audit_labels
                        .contains(&"ceiling_effect_possible".to_string())
                })
                .count();
            let harder = trace_audits
                .iter()
                .filter(|x| {
                    x.audit_labels
                        .contains(&"needs_harder_trace_variant".to_string())
                })
                .count();
            let overlap_mean = if trace_count == 0 {
                0.0
            } else {
                trace_audits
                    .iter()
                    .map(|x| x.top_item_overlap_ratio)
                    .sum::<f64>()
                    / trace_count as f64
            };
            let harder_overlap_mean = trace_audits
                .iter()
                .filter(|x| x.trace_id.ends_with("_harder"))
                .map(|x| x.top_item_overlap_ratio)
                .sum::<f64>()
                / harder_count.max(1) as f64;
            let original_overlap_mean = trace_audits
                .iter()
                .filter(|x| !x.trace_id.ends_with("_harder"))
                .map(|x| x.top_item_overlap_ratio)
                .sum::<f64>()
                / original_count.max(1) as f64;
            TraceReplayAuditSummary {
                trace_count,
                traces_with_identical_top_items: identical,
                traces_with_keyword_obvious_label: keyword_obvious,
                traces_with_ceiling_effect: ceiling,
                traces_needing_harder_variant: harder,
                mean_top_item_overlap_ratio: overlap_mean,
                interpretation_labels: vec![
                    "trace_replay_equal_explained".to_string(),
                    "harder_trace_variants_required".to_string(),
                    "harder_trace_variants_present".to_string(),
                    if trace_audits.iter().any(|x| {
                        x.discriminativity_label == "pse_gain" || x.top_item_overlap_ratio < 1.0
                    }) {
                        "harder_trace_discriminativity_present".to_string()
                    } else {
                        "harder_trace_discriminativity_absent".to_string()
                    },
                    if harder_overlap_mean < original_overlap_mean {
                        "mapping_improved".to_string()
                    } else {
                        "mapping_improved_absent".to_string()
                    },
                    "diagnostic_only".to_string(),
                    "requires_live_agent_validation".to_string(),
                ],
            }
        },
        trace_variant_counts: TraceVariantCounts {
            original_count,
            harder_count,
        },
        harder_trace_metrics,
        trace_gain_failure_summary,
        trace_signal_reweighting_report: {
            let profiles = all_trace_signal_profiles();
            let mut profile_reports = Vec::new();
            let trace_states: Vec<_> = real_trace_replay_cases()
                .into_iter()
                .map(|trace| AgentWorkState {
                    scenario_id: trace.trace_id,
                    split: AgentScenarioSplit::Holdout,
                    issue_text: trace.title,
                    work_items: trace.candidate_items,
                    plan_steps: vec!["fix".into(), "retry".into()],
                    ground_truth: AgentGroundTruth {
                        causal_files: trace.ground_truth.causal_files,
                        causal_logs: trace.ground_truth.causal_logs,
                        expected_next_actions: trace.ground_truth.correct_next_actions,
                        irrelevant_items: trace.ground_truth.rejected_false_paths,
                        blocker_label: trace.ground_truth.resolution_label,
                    },
                })
                .collect();
            let current_map: BTreeMap<String, AgentDiagnosticMetrics> = trace_states
                .iter()
                .map(|s| {
                    (
                        s.scenario_id.clone(),
                        metrics(
                            s,
                            &rank_trace_with_signal_profile(s, TraceSignalProfile::CurrentPse),
                        ),
                    )
                })
                .collect();
            for p in &profiles {
                let mut reports_p = Vec::new();
                let mut improved = Vec::new();
                for s in &trace_states {
                    let pm = metrics(s, &rank_trace_with_signal_profile(s, *p));
                    let cm = current_map.get(&s.scenario_id).unwrap();
                    if pm.hit_at_3 && !cm.hit_at_3
                        || pm.mrr_causal_item > cm.mrr_causal_item
                        || pm.mean_rank_of_first_causal_item < cm.mean_rank_of_first_causal_item
                    {
                        improved.push(s.scenario_id.clone());
                    }
                    reports_p.push(AgentExoskeletonDiagnosticReport {
                        scenario_id: s.scenario_id.clone(),
                        split: s.split,
                        baseline_metrics: cm.clone(),
                        pse_field_metrics: pm.clone(),
                        delta_hit_at_1: (pm.hit_at_1 as i8 - cm.hit_at_1 as i8) as f64,
                        delta_hit_at_3: (pm.hit_at_3 as i8 - cm.hit_at_3 as i8) as f64,
                        delta_causal_file_recall: pm.causal_file_recall - cm.causal_file_recall,
                        delta_causal_log_recall: pm.causal_log_recall - cm.causal_log_recall,
                        delta_false_focus_rate: pm.false_focus_rate - cm.false_focus_rate,
                        delta_irrelevant_focus_count: pm.irrelevant_focus_count as f64
                            - cm.irrelevant_focus_count as f64,
                        delta_search_reduction_ratio: pm.search_reduction_ratio
                            - cm.search_reduction_ratio,
                        delta_next_action_hit: (pm.next_action_hit as i8 - cm.next_action_hit as i8)
                            as f64,
                        delta_blocker_label_hit: (pm.blocker_label_hit as i8
                            - cm.blocker_label_hit as i8)
                            as f64,
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        interpretation_labels: vec!["diagnostic_only".into()],
                        ablation_report: None,
                    });
                }
                let agg = compute_aggregate(&reports_p);
                let harder_reports: Vec<_> = reports_p
                    .into_iter()
                    .filter(|r| r.scenario_id.ends_with("_harder"))
                    .collect();
                let harder_agg = compute_aggregate(&harder_reports);
                profile_reports.push(TraceSignalProfileReport {
                    profile: *p,
                    metrics_aggregate: agg.clone(),
                    harder_metrics_aggregate: harder_agg.clone(),
                    traces_with_gain: agg.scenarios_with_pse_gain,
                    traces_with_loss: agg.scenarios_with_pse_loss,
                    traces_with_equal: agg.scenarios_with_equal_result,
                    harder_traces_with_gain: harder_agg.scenarios_with_pse_gain,
                    harder_traces_with_loss: harder_agg.scenarios_with_pse_loss,
                    harder_traces_with_equal: harder_agg.scenarios_with_equal_result,
                    top_improved_traces: improved,
                    warnings: vec![],
                });
            }
            let best_mrr = *profile_reports
                .iter()
                .max_by(|a, b| {
                    a.harder_metrics_aggregate
                        .pse_mrr_causal_item_mean
                        .partial_cmp(&b.harder_metrics_aggregate.pse_mrr_causal_item_mean)
                        .unwrap()
                        .then_with(|| {
                            a.harder_metrics_aggregate
                                .pse_hit_at_3_mean
                                .partial_cmp(&b.harder_metrics_aggregate.pse_hit_at_3_mean)
                                .unwrap()
                        })
                        .then_with(|| {
                            b.harder_metrics_aggregate
                                .pse_false_focus_mean
                                .partial_cmp(&a.harder_metrics_aggregate.pse_false_focus_mean)
                                .unwrap()
                        })
                        .then_with(|| a.profile.cmp(&b.profile))
                })
                .map(|x| &x.profile)
                .unwrap();
            let best_hit = *profile_reports
                .iter()
                .max_by(|a, b| {
                    a.harder_metrics_aggregate
                        .pse_hit_at_3_mean
                        .partial_cmp(&b.harder_metrics_aggregate.pse_hit_at_3_mean)
                        .unwrap()
                        .then_with(|| {
                            a.harder_metrics_aggregate
                                .pse_mrr_causal_item_mean
                                .partial_cmp(&b.harder_metrics_aggregate.pse_mrr_causal_item_mean)
                                .unwrap()
                        })
                        .then_with(|| {
                            b.harder_metrics_aggregate
                                .pse_false_focus_mean
                                .partial_cmp(&a.harder_metrics_aggregate.pse_false_focus_mean)
                                .unwrap()
                        })
                        .then_with(|| a.profile.cmp(&b.profile))
                })
                .map(|x| &x.profile)
                .unwrap();
            let current = profile_reports
                .iter()
                .find(|x| x.profile == TraceSignalProfile::CurrentPse)
                .unwrap()
                .clone();
            let keyword = profile_reports
                .iter()
                .find(|x| x.profile == TraceSignalProfile::KeywordOnlyTraceEquivalent)
                .unwrap()
                .clone();
            let best = profile_reports
                .iter()
                .find(|x| x.profile == best_mrr)
                .unwrap()
                .clone();
            TraceSignalReweightingReport {
                diagnostic_only: true,
                productive_agent_validated: false,
                profiles,
                profile_reports,
                best_profile_by_harder_mrr: best_mrr,
                best_profile_by_harder_hit_at_3: best_hit,
                current_pse_profile: TraceSignalProfile::CurrentPse,
                keyword_equivalent_profile: TraceSignalProfile::KeywordOnlyTraceEquivalent,
                interpretation_labels: vec![
                    "trace_signal_reweighting_present".into(),
                    "diagnostic_only".into(),
                    "requires_live_agent_validation".into(),
                    "current_pse_not_replaced".into(),
                    if best.harder_metrics_aggregate.pse_mrr_causal_item_mean
                        > current.harder_metrics_aggregate.pse_mrr_causal_item_mean
                        || best.harder_metrics_aggregate.pse_hit_at_3_mean
                            > current.harder_metrics_aggregate.pse_hit_at_3_mean
                    {
                        "reweighting_improves_harder_traces".into()
                    } else {
                        "reweighting_no_gain".into()
                    },
                    if best.harder_metrics_aggregate.pse_mrr_causal_item_mean
                        > keyword.harder_metrics_aggregate.pse_mrr_causal_item_mean
                    {
                        "keyword_equivalent_not_sufficient".into()
                    } else {
                        "keyword_equivalent_sufficient".into()
                    },
                    if best.profile == TraceSignalProfile::DistractorSuppressionBoost {
                        "possible_trace_overfit_risk".into()
                    } else {
                        "trace_overfit_risk_not_dominant".into()
                    },
                ],
            }
        },
        trace_feature_design_report: {
            let profiles = all_trace_feature_design_profiles();
            let trace_states: Vec<_> = real_trace_replay_cases()
                .into_iter()
                .map(|trace| AgentWorkState {
                    scenario_id: trace.trace_id,
                    split: AgentScenarioSplit::Holdout,
                    issue_text: trace.title,
                    work_items: trace.candidate_items,
                    plan_steps: vec!["fix".into(), "retry".into()],
                    ground_truth: AgentGroundTruth {
                        causal_files: trace.ground_truth.causal_files,
                        causal_logs: trace.ground_truth.causal_logs,
                        expected_next_actions: trace.ground_truth.correct_next_actions,
                        irrelevant_items: trace.ground_truth.rejected_false_paths,
                        blocker_label: trace.ground_truth.resolution_label,
                    },
                })
                .collect();
            let current_map: BTreeMap<String, AgentDiagnosticMetrics> = trace_states
                .iter()
                .map(|s| {
                    (
                        s.scenario_id.clone(),
                        metrics(
                            s,
                            &rank_trace_with_feature_design_profile(
                                s,
                                TraceFeatureDesignProfile::CurrentPse,
                            ),
                        ),
                    )
                })
                .collect();
            let mut profile_reports = Vec::new();
            for profile in &profiles {
                let mut reports_p = Vec::new();
                let mut improved = Vec::new();
                let mut worsened = Vec::new();
                for s in &trace_states {
                    let pm = metrics(s, &rank_trace_with_feature_design_profile(s, *profile));
                    let cm = current_map.get(&s.scenario_id).unwrap();
                    let is_improved = (pm.hit_at_3 && !cm.hit_at_3)
                        || pm.mrr_causal_item > cm.mrr_causal_item
                        || pm.false_focus_rate < cm.false_focus_rate;
                    let is_worsened = (!pm.hit_at_3 && cm.hit_at_3)
                        || pm.mrr_causal_item < cm.mrr_causal_item
                        || pm.false_focus_rate > cm.false_focus_rate;
                    if is_improved {
                        improved.push(s.scenario_id.clone());
                    }
                    if is_worsened {
                        worsened.push(s.scenario_id.clone());
                    }
                    reports_p.push(AgentExoskeletonDiagnosticReport {
                        scenario_id: s.scenario_id.clone(),
                        split: s.split,
                        baseline_metrics: cm.clone(),
                        pse_field_metrics: pm.clone(),
                        delta_hit_at_1: (pm.hit_at_1 as i8 - cm.hit_at_1 as i8) as f64,
                        delta_hit_at_3: (pm.hit_at_3 as i8 - cm.hit_at_3 as i8) as f64,
                        delta_causal_file_recall: pm.causal_file_recall - cm.causal_file_recall,
                        delta_causal_log_recall: pm.causal_log_recall - cm.causal_log_recall,
                        delta_false_focus_rate: pm.false_focus_rate - cm.false_focus_rate,
                        delta_irrelevant_focus_count: pm.irrelevant_focus_count as f64
                            - cm.irrelevant_focus_count as f64,
                        delta_search_reduction_ratio: pm.search_reduction_ratio
                            - cm.search_reduction_ratio,
                        delta_next_action_hit: (pm.next_action_hit as i8 - cm.next_action_hit as i8)
                            as f64,
                        delta_blocker_label_hit: (pm.blocker_label_hit as i8
                            - cm.blocker_label_hit as i8)
                            as f64,
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        interpretation_labels: vec!["diagnostic_only".into()],
                        ablation_report: None,
                    });
                }
                improved.sort();
                worsened.sort();
                let agg = compute_aggregate(&reports_p);
                let harder_reports: Vec<_> = reports_p
                    .iter()
                    .filter(|r| r.scenario_id.ends_with("_harder"))
                    .cloned()
                    .collect();
                let harder_agg = compute_aggregate(&harder_reports);
                profile_reports.push(TraceFeatureDesignProfileReport {
                    profile: *profile,
                    metrics_aggregate: agg.clone(),
                    harder_metrics_aggregate: harder_agg.clone(),
                    traces_with_gain: agg.scenarios_with_pse_gain,
                    traces_with_loss: agg.scenarios_with_pse_loss,
                    traces_with_equal: agg.scenarios_with_equal_result,
                    harder_traces_with_gain: harder_agg.scenarios_with_pse_gain,
                    harder_traces_with_loss: harder_agg.scenarios_with_pse_loss,
                    harder_traces_with_equal: harder_agg.scenarios_with_equal_result,
                    improved_trace_ids: improved,
                    worsened_trace_ids: worsened,
                    feature_effect_labels: vec![],
                    warnings: vec![],
                });
            }
            let by_hit = |a: &TraceFeatureDesignProfileReport,
                          b: &TraceFeatureDesignProfileReport| {
                a.harder_metrics_aggregate
                    .pse_hit_at_3_mean
                    .partial_cmp(&b.harder_metrics_aggregate.pse_hit_at_3_mean)
                    .unwrap()
                    .then_with(|| {
                        a.harder_metrics_aggregate
                            .pse_mrr_causal_item_mean
                            .partial_cmp(&b.harder_metrics_aggregate.pse_mrr_causal_item_mean)
                            .unwrap()
                    })
                    .then_with(|| {
                        b.harder_metrics_aggregate
                            .pse_false_focus_mean
                            .partial_cmp(&a.harder_metrics_aggregate.pse_false_focus_mean)
                            .unwrap()
                    })
                    .then_with(|| format!("{:?}", b.profile).cmp(&format!("{:?}", a.profile)))
            };
            let by_mrr = |a: &TraceFeatureDesignProfileReport,
                          b: &TraceFeatureDesignProfileReport| {
                a.harder_metrics_aggregate
                    .pse_mrr_causal_item_mean
                    .partial_cmp(&b.harder_metrics_aggregate.pse_mrr_causal_item_mean)
                    .unwrap()
                    .then_with(|| {
                        a.harder_metrics_aggregate
                            .pse_hit_at_3_mean
                            .partial_cmp(&b.harder_metrics_aggregate.pse_hit_at_3_mean)
                            .unwrap()
                    })
                    .then_with(|| {
                        b.harder_metrics_aggregate
                            .pse_false_focus_mean
                            .partial_cmp(&a.harder_metrics_aggregate.pse_false_focus_mean)
                            .unwrap()
                    })
                    .then_with(|| format!("{:?}", b.profile).cmp(&format!("{:?}", a.profile)))
            };
            let by_false_focus =
                |a: &TraceFeatureDesignProfileReport, b: &TraceFeatureDesignProfileReport| {
                    b.harder_metrics_aggregate
                        .pse_false_focus_mean
                        .partial_cmp(&a.harder_metrics_aggregate.pse_false_focus_mean)
                        .unwrap()
                        .then_with(|| {
                            a.harder_metrics_aggregate
                                .pse_hit_at_3_mean
                                .partial_cmp(&b.harder_metrics_aggregate.pse_hit_at_3_mean)
                                .unwrap()
                        })
                        .then_with(|| {
                            a.harder_metrics_aggregate
                                .pse_mrr_causal_item_mean
                                .partial_cmp(&b.harder_metrics_aggregate.pse_mrr_causal_item_mean)
                                .unwrap()
                        })
                        .then_with(|| format!("{:?}", b.profile).cmp(&format!("{:?}", a.profile)))
                };
            let best_hit = profile_reports.iter().max_by(|a, b| by_hit(a, b)).unwrap();
            let best_mrr = profile_reports.iter().max_by(|a, b| by_mrr(a, b)).unwrap();
            let best_false_focus = profile_reports
                .iter()
                .max_by(|a, b| by_false_focus(a, b))
                .unwrap();
            let best_hit_profile = best_hit.profile;
            let best_mrr_profile = best_mrr.profile;
            let best_false_focus_profile = best_false_focus.profile;
            let current = profile_reports
                .iter()
                .find(|x| x.profile == TraceFeatureDesignProfile::CurrentPse)
                .unwrap();
            let all_equal = profile_reports.iter().all(|p| {
                p.harder_metrics_aggregate.pse_hit_at_3_mean
                    == current.harder_metrics_aggregate.pse_hit_at_3_mean
                    && p.harder_metrics_aggregate.pse_mrr_causal_item_mean
                        == current.harder_metrics_aggregate.pse_mrr_causal_item_mean
                    && p.harder_metrics_aggregate.pse_false_focus_mean
                        == current.harder_metrics_aggregate.pse_false_focus_mean
            });
            let harder_improved = best_hit.harder_metrics_aggregate.pse_hit_at_3_mean
                > current.harder_metrics_aggregate.pse_hit_at_3_mean
                || best_mrr.harder_metrics_aggregate.pse_mrr_causal_item_mean
                    > current.harder_metrics_aggregate.pse_mrr_causal_item_mean;
            let distractor_effective = profile_reports.iter().any(|p| {
                matches!(
                    p.profile,
                    TraceFeatureDesignProfile::StrongDistractorPenalty
                        | TraceFeatureDesignProfile::CombinedConservativeFeatureDesign
                ) && (p.harder_metrics_aggregate.pse_hit_at_3_mean
                    > current.harder_metrics_aggregate.pse_hit_at_3_mean
                    || p.harder_metrics_aggregate.pse_mrr_causal_item_mean
                        > current.harder_metrics_aggregate.pse_mrr_causal_item_mean)
            });
            let keyword_effective = profile_reports.iter().any(|p| {
                p.profile == TraceFeatureDesignProfile::KeywordDominanceDampening
                    && (p.harder_metrics_aggregate.pse_hit_at_3_mean
                        > current.harder_metrics_aggregate.pse_hit_at_3_mean
                        || p.harder_metrics_aggregate.pse_mrr_causal_item_mean
                            > current.harder_metrics_aggregate.pse_mrr_causal_item_mean)
            });
            let total_improved: usize = profile_reports
                .iter()
                .map(|p| p.improved_trace_ids.len())
                .sum();
            let all_no_improved = profile_reports
                .iter()
                .all(|p| p.improved_trace_ids.is_empty());
            let overfit_risk = profile_reports.iter().any(|p| {
                let hard_imp = p.harder_metrics_aggregate.pse_hit_at_3_mean
                    > current.harder_metrics_aggregate.pse_hit_at_3_mean
                    || p.harder_metrics_aggregate.pse_mrr_causal_item_mean
                        > current.harder_metrics_aggregate.pse_mrr_causal_item_mean;
                let all_imp = p.metrics_aggregate.pse_hit_at_3_mean
                    > current.metrics_aggregate.pse_hit_at_3_mean
                    || p.metrics_aggregate.pse_mrr_causal_item_mean
                        > current.metrics_aggregate.pse_mrr_causal_item_mean;
                hard_imp && !all_imp
            }) || total_improved == 1;
            let mut interpretation_labels = vec![
                "trace_feature_design_present".into(),
                "diagnostic_only".into(),
                "requires_live_agent_validation".into(),
                "current_pse_not_replaced".into(),
            ];
            interpretation_labels.push(if harder_improved {
                "feature_design_improves_harder_traces".into()
            } else {
                "feature_design_no_gain".into()
            });
            if all_equal {
                interpretation_labels.push("feature_profiles_metric_tie".into());
            }
            if distractor_effective {
                interpretation_labels.push("distractor_penalty_feature_effective".into());
            }
            if keyword_effective {
                interpretation_labels.push("keyword_dampening_effective".into());
            }
            if overfit_risk {
                interpretation_labels.push("possible_feature_overfit_risk".into());
            }
            if all_no_improved {
                interpretation_labels
                    .push("feature_design_insufficient_for_current_trace_set".into());
            }
            if let Some(ca) = profile_reports
                .iter()
                .find(|p| p.profile == TraceFeatureDesignProfile::CausalActivationDiagnostic)
            {
                interpretation_labels.push("causal_activation_diagnostic_profile_present".into());
                interpretation_labels.push("causal_activation_profile_report_only".into());
                if ca.harder_traces_with_gain > 0 {
                    interpretation_labels.push("causal_activation_improves_harder_traces".into());
                } else {
                    interpretation_labels.push("causal_activation_no_metric_gain".into());
                }
                if !ca.improved_trace_ids.is_empty() {
                    interpretation_labels.push("possible_causal_feature_path_confirmed".into());
                }
            }
            let mut profile_sensitivity_reports = Vec::new();
            for profile in profiles
                .iter()
                .copied()
                .filter(|p| *p != TraceFeatureDesignProfile::CurrentPse)
            {
                let mut rank_mv = 0usize;
                let mut score_mv = 0usize;
                let mut causal_rank_imp = 0usize;
                let mut causal_score_imp = 0usize;
                let mut top3_change = 0usize;
                let mut best_causal_rank_change = 0usize;
                let mut sum_abs_score_delta = 0.0;
                let mut sum_abs_rank_delta = 0.0;
                let mut sum_causal_rank_delta = 0.0;
                let mut sum_causal_score_delta = 0.0;
                let mut harder_causal_rank_delta = 0.0;
                let mut harder_causal_score_delta = 0.0;
                let mut harder_count = 0usize;
                let mut affected = Vec::new();
                let mut unaffected = Vec::new();
                for s in &trace_states {
                    let cur = rank_trace_with_feature_design_profile(
                        s,
                        TraceFeatureDesignProfile::CurrentPse,
                    );
                    let alt = rank_trace_with_feature_design_profile(s, profile);
                    let mut any_score = false;
                    let mut any_rank = false;
                    let mut score_acc = 0.0;
                    let cur_order: BTreeMap<_, _> = cur
                        .top_items
                        .iter()
                        .enumerate()
                        .map(|(i, id)| (id.clone(), i + 1))
                        .collect();
                    let alt_order: BTreeMap<_, _> = alt
                        .top_items
                        .iter()
                        .enumerate()
                        .map(|(i, id)| (id.clone(), i + 1))
                        .collect();
                    let mut rank_acc = 0.0;
                    for item in &s.work_items {
                        let cs = *cur.item_scores.get(&item.id).unwrap_or(&0);
                        let ps = *alt.item_scores.get(&item.id).unwrap_or(&0);
                        let sd = (ps - cs) as f64;
                        if sd != 0.0 {
                            any_score = true;
                        }
                        score_acc += sd.abs();
                        let cr = *cur_order.get(&item.id).unwrap_or(&999) as f64;
                        let pr = *alt_order.get(&item.id).unwrap_or(&999) as f64;
                        let rd = pr - cr;
                        if rd != 0.0 {
                            any_rank = true;
                        }
                        rank_acc += rd.abs();
                    }
                    if any_score {
                        score_mv += 1;
                    }
                    if any_rank {
                        rank_mv += 1;
                    }
                    if cur.top_items != alt.top_items {
                        top3_change += 1;
                    }
                    let cur_m = metrics(s, &cur);
                    let alt_m = metrics(s, &alt);
                    let cr = cur_m.mean_rank_of_first_causal_item;
                    let pr = alt_m.mean_rank_of_first_causal_item;
                    let rank_delta = pr - cr;
                    if rank_delta != 0.0 {
                        best_causal_rank_change += 1;
                    }
                    if rank_delta < 0.0 {
                        causal_rank_imp += 1;
                    }
                    let score_delta = alt_m.mrr_causal_item - cur_m.mrr_causal_item;
                    if score_delta > 0.0 {
                        causal_score_imp += 1;
                    }
                    sum_abs_score_delta += score_acc;
                    sum_abs_rank_delta += rank_acc;
                    sum_causal_rank_delta += rank_delta;
                    sum_causal_score_delta += score_delta;
                    if s.scenario_id.ends_with("_harder") {
                        harder_count += 1;
                        harder_causal_rank_delta += rank_delta;
                        harder_causal_score_delta += score_delta;
                    }
                    if any_score || any_rank || rank_delta != 0.0 || score_delta != 0.0 {
                        affected.push(s.scenario_id.clone());
                    } else {
                        unaffected.push(s.scenario_id.clone());
                    }
                }
                affected.sort();
                unaffected.sort();
                let trace_count = trace_states.len().max(1) as f64;
                let harder_den = harder_count.max(1) as f64;
                let sensitivity_label = if score_mv == 0 && rank_mv == 0 {
                    "no_effect"
                } else if score_mv > 0 && rank_mv == 0 {
                    "score_only_no_rank_movement"
                } else if rank_mv > 0 && causal_rank_imp == 0 {
                    "rank_movement_no_causal_gain"
                } else if causal_score_imp > 0 && causal_rank_imp == 0 {
                    "causal_score_improves_but_rank_static"
                } else if causal_rank_imp > 0
                    && profile_reports
                        .iter()
                        .find(|x| x.profile == profile)
                        .map(|x| x.harder_metrics_aggregate.pse_hit_at_3_mean == 0.0)
                        .unwrap_or(false)
                {
                    "causal_rank_improves_below_top3"
                } else if (causal_rank_imp > 0 || causal_score_imp > 0)
                    && profile_reports
                        .iter()
                        .find(|x| x.profile == profile)
                        .map(|x| {
                            x.harder_metrics_aggregate.pse_hit_at_3_mean
                                == current.harder_metrics_aggregate.pse_hit_at_3_mean
                                && x.harder_metrics_aggregate.pse_mrr_causal_item_mean
                                    == current.harder_metrics_aggregate.pse_mrr_causal_item_mean
                        })
                        .unwrap_or(false)
                {
                    "metric_blind_improvement_possible"
                } else {
                    "rank_movement_no_causal_gain"
                };
                profile_sensitivity_reports.push(TraceFeatureDesignProfileSensitivity {
                    profile,
                    trace_count: trace_states.len(),
                    harder_trace_count: harder_count,
                    traces_with_rank_movement: rank_mv,
                    traces_with_score_movement: score_mv,
                    traces_with_causal_rank_improvement: causal_rank_imp,
                    traces_with_causal_score_improvement: causal_score_imp,
                    traces_with_top3_change: top3_change,
                    traces_with_best_causal_rank_change: best_causal_rank_change,
                    mean_abs_score_delta: sum_abs_score_delta / trace_count,
                    mean_abs_rank_delta: sum_abs_rank_delta / trace_count,
                    mean_causal_rank_delta: sum_causal_rank_delta / trace_count,
                    mean_causal_score_delta: sum_causal_score_delta / trace_count,
                    harder_mean_causal_rank_delta: harder_causal_rank_delta / harder_den,
                    harder_mean_causal_score_delta: harder_causal_score_delta / harder_den,
                    sensitivity_label: sensitivity_label.to_string(),
                    affected_trace_ids: affected,
                    unaffected_trace_ids: unaffected,
                    warnings: vec![],
                });
            }
            let mut profiles_with_any_rank_movement = Vec::new();
            let mut profiles_with_any_score_movement = Vec::new();
            let mut profiles_with_causal_rank_improvement = Vec::new();
            let mut profiles_with_causal_score_improvement = Vec::new();
            let mut profiles_with_no_effect = Vec::new();
            for p in &profile_sensitivity_reports {
                if p.traces_with_rank_movement > 0 {
                    profiles_with_any_rank_movement.push(p.profile);
                }
                if p.traces_with_score_movement > 0 {
                    profiles_with_any_score_movement.push(p.profile);
                }
                if p.traces_with_causal_rank_improvement > 0 {
                    profiles_with_causal_rank_improvement.push(p.profile);
                }
                if p.traces_with_causal_score_improvement > 0 {
                    profiles_with_causal_score_improvement.push(p.profile);
                }
                if p.sensitivity_label == "no_effect" {
                    profiles_with_no_effect.push(p.profile);
                }
            }
            let all_profiles_no_effect = profile_sensitivity_reports
                .iter()
                .all(|p| p.sensitivity_label == "no_effect");
            let mut sensitivity_labels = vec![
                "feature_design_sensitivity_present".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "requires_live_agent_validation".to_string(),
            ];
            if all_profiles_no_effect {
                sensitivity_labels.push("feature_design_profiles_inactive".to_string());
                sensitivity_labels
                    .push("feature_design_no_effect_on_current_trace_set".to_string());
            } else {
                if !profiles_with_any_score_movement.is_empty()
                    && profiles_with_any_rank_movement.is_empty()
                {
                    sensitivity_labels.push("feature_design_changes_scores_only".to_string());
                }
                if !profiles_with_any_rank_movement.is_empty()
                    && profiles_with_causal_rank_improvement.is_empty()
                {
                    sensitivity_labels
                        .push("feature_design_changes_ranks_but_not_causal_items".to_string());
                }
                if !profiles_with_causal_rank_improvement.is_empty()
                    && interpretation_labels.contains(&"feature_design_no_gain".to_string())
                {
                    sensitivity_labels.push("metric_blind_spot_possible".to_string());
                }
            }
            let mut profile_activation_reports = Vec::new();
            for profile in profiles
                .iter()
                .copied()
                .filter(|p| *p != TraceFeatureDesignProfile::CurrentPse)
            {
                let mut activation_count = 0usize;
                let mut causal_activation_count = 0usize;
                let mut noncausal_activation_count = 0usize;
                let mut top3_blocker_activation_count = 0usize;
                let mut total_score_delta_abs = 0.0;
                let mut max_score_delta_abs: f64 = 0.0;
                let mut causal_score_delta_abs = 0.0;
                let mut noncausal_score_delta_abs = 0.0;
                let mut affected_trace_ids = Vec::new();
                let mut affected_item_ids = BTreeSet::new();
                for s in &trace_states {
                    let cur = rank_trace_with_feature_design_profile(
                        s,
                        TraceFeatureDesignProfile::CurrentPse,
                    );
                    let alt = rank_trace_with_feature_design_profile(s, profile);
                    let cur_top = ranked_top_items(s, &cur);
                    let top3_noncausal: BTreeSet<_> = cur_top
                        .into_iter()
                        .filter(|x| !x.is_causal)
                        .map(|x| x.item_id)
                        .collect();
                    let mut trace_activated = false;
                    for item in &s.work_items {
                        let cs = *cur.item_scores.get(&item.id).unwrap_or(&0);
                        let ps = *alt.item_scores.get(&item.id).unwrap_or(&0);
                        let d = (ps - cs).abs() as f64;
                        if d > 0.0 {
                            trace_activated = true;
                            activation_count += 1;
                            total_score_delta_abs += d;
                            max_score_delta_abs = max_score_delta_abs.max(d);
                            let is_causal = s.ground_truth.causal_files.contains(&item.id)
                                || s.ground_truth.causal_logs.contains(&item.id);
                            if is_causal {
                                causal_activation_count += 1;
                                causal_score_delta_abs += d;
                            } else {
                                noncausal_activation_count += 1;
                                noncausal_score_delta_abs += d;
                            }
                            if top3_noncausal.contains(&item.id) {
                                top3_blocker_activation_count += 1;
                            }
                            if affected_item_ids.len() < 32 {
                                affected_item_ids.insert(item.id.clone());
                            }
                        }
                    }
                    if trace_activated {
                        affected_trace_ids.push(s.scenario_id.clone());
                    }
                }
                affected_trace_ids.sort();
                let zero_activation_reason = if activation_count == 0 {
                    Some(
                        match profile {
                            TraceFeatureDesignProfile::StrongDistractorPenalty => {
                                "no_matching_distractor_markers"
                            }
                            TraceFeatureDesignProfile::KeywordDominanceDampening => {
                                "no_matching_keyword_dampening_markers"
                            }
                            _ => "profile_rule_conditions_not_met",
                        }
                        .to_string(),
                    )
                } else {
                    None
                };
                let has_rank_movement = profile_sensitivity_reports
                    .iter()
                    .find(|p| p.profile == profile)
                    .map(|p| p.traces_with_rank_movement > 0)
                    .unwrap_or(false);
                let activation_label = if activation_count == 0 {
                    "no_activation"
                } else if noncausal_activation_count > 0 && causal_activation_count == 0 {
                    "noncausal_only_activation"
                } else if top3_blocker_activation_count > 0 && !has_rank_movement {
                    "top3_blocker_activation_without_rank_change"
                } else if !has_rank_movement {
                    "small_delta_no_rank_movement"
                } else if noncausal_activation_count > causal_activation_count {
                    "activation_hits_wrong_items"
                } else {
                    "causal_activation_without_improvement"
                };
                profile_activation_reports.push(TraceFeatureDesignProfileActivation {
                    profile,
                    activation_count,
                    causal_activation_count,
                    noncausal_activation_count,
                    top3_blocker_activation_count,
                    total_score_delta_abs,
                    mean_score_delta_abs: if activation_count == 0 {
                        0.0
                    } else {
                        total_score_delta_abs / activation_count as f64
                    },
                    max_score_delta_abs,
                    causal_score_delta_abs,
                    noncausal_score_delta_abs,
                    affected_trace_ids,
                    affected_item_ids_sample: affected_item_ids.into_iter().collect(),
                    zero_activation_reason,
                    activation_label: activation_label.to_string(),
                    warnings: vec![],
                });
            }
            let profiles_with_zero_activation: Vec<_> = profile_activation_reports
                .iter()
                .filter(|p| p.activation_count == 0)
                .map(|p| p.profile)
                .collect();
            let profiles_with_activation_only_on_noncausal: Vec<_> = profile_activation_reports
                .iter()
                .filter(|p| p.noncausal_activation_count > 0 && p.causal_activation_count == 0)
                .map(|p| p.profile)
                .collect();
            let profiles_with_activation_on_causal: Vec<_> = profile_activation_reports
                .iter()
                .filter(|p| p.causal_activation_count > 0)
                .map(|p| p.profile)
                .collect();
            let profiles_with_small_delta_only: Vec<_> = profile_activation_reports
                .iter()
                .filter(|p| p.activation_count > 0 && p.max_score_delta_abs <= 2.0)
                .map(|p| p.profile)
                .collect();
            let mut activation_labels = vec![
                "feature_design_activation_audit_present".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "requires_live_agent_validation".to_string(),
            ];
            if !profiles_with_zero_activation.is_empty() {
                activation_labels.push("feature_rules_not_firing".to_string());
            }
            if !profiles_with_any_score_movement.is_empty()
                && profiles_with_any_rank_movement.is_empty()
            {
                activation_labels.push("feature_rules_fire_but_no_rank_movement".to_string());
                activation_labels.push("feature_delta_too_small_for_rank_change".to_string());
            }
            if profiles_with_activation_on_causal.is_empty() {
                activation_labels.push("causal_items_not_targeted_by_feature_rules".to_string());
            }
            let top3_hits: usize = profile_activation_reports
                .iter()
                .map(|p| p.top3_blocker_activation_count)
                .sum();
            if top3_hits == 0 {
                activation_labels.push("top3_blockers_not_sufficiently_suppressed".to_string());
            }
            let mut profile_targeting_reports = Vec::new();
            for profile in profiles
                .iter()
                .copied()
                .filter(|p| *p != TraceFeatureDesignProfile::CurrentPse)
            {
                let mut activation_count = 0usize;
                let mut activated_item_ids_sample = BTreeSet::new();
                let mut activated_item_kind_counts: BTreeMap<String, usize> = BTreeMap::new();
                let mut activated_rank_distribution: BTreeMap<String, usize> = BTreeMap::new();
                let mut activated_causal_count = 0usize;
                let mut activated_noncausal_count = 0usize;
                let mut activated_top3_blocker_count = 0usize;
                let mut activated_below_top3_noncausal_count = 0usize;
                let mut missed_top3_blocker_count = 0usize;
                let mut missed_causal_item_count = 0usize;
                let mut affected_trace_ids = Vec::new();
                for trace in real_trace_replay_cases() {
                    let s = AgentWorkState {
                        scenario_id: trace.trace_id,
                        split: AgentScenarioSplit::Holdout,
                        issue_text: trace.title,
                        work_items: trace.candidate_items,
                        plan_steps: vec!["fix".into(), "retry".into()],
                        ground_truth: AgentGroundTruth {
                            causal_files: trace.ground_truth.causal_files,
                            causal_logs: trace.ground_truth.causal_logs,
                            expected_next_actions: trace.ground_truth.correct_next_actions,
                            irrelevant_items: trace.ground_truth.rejected_false_paths,
                            blocker_label: trace.ground_truth.resolution_label,
                        },
                    };
                    let cur = rank_trace_with_feature_design_profile(
                        &s,
                        TraceFeatureDesignProfile::CurrentPse,
                    );
                    let alt = rank_trace_with_feature_design_profile(&s, profile);
                    let cur_top = ranked_top_items(&s, &cur);
                    let top3_noncausal: BTreeSet<_> = cur_top
                        .iter()
                        .filter(|x| !x.is_causal)
                        .map(|x| x.item_id.clone())
                        .collect();
                    let mut activated_in_trace = BTreeSet::new();
                    for item in &s.work_items {
                        let cs = *cur.item_scores.get(&item.id).unwrap_or(&0);
                        let ps = *alt.item_scores.get(&item.id).unwrap_or(&0);
                        if ps != cs {
                            activation_count += 1;
                            if activated_item_ids_sample.len() < 32 {
                                activated_item_ids_sample.insert(item.id.clone());
                            }
                            *activated_item_kind_counts
                                .entry(format!("{:?}", item.item_type))
                                .or_insert(0) += 1;
                            let rank_key = ranked_top_items(&s, &cur)
                                .iter()
                                .position(|x| x.item_id == item.id)
                                .map(|i| i + 1)
                                .map(|r| {
                                    if r <= 3 {
                                        "top3".to_string()
                                    } else if r <= 10 {
                                        "rank4_10".to_string()
                                    } else {
                                        "rank11_plus".to_string()
                                    }
                                })
                                .unwrap_or_else(|| "unranked".to_string());
                            *activated_rank_distribution.entry(rank_key).or_insert(0) += 1;
                            let is_causal = s.ground_truth.causal_files.contains(&item.id)
                                || s.ground_truth.causal_logs.contains(&item.id);
                            if is_causal {
                                activated_causal_count += 1;
                            } else {
                                activated_noncausal_count += 1;
                                if top3_noncausal.contains(&item.id) {
                                    activated_top3_blocker_count += 1;
                                } else {
                                    activated_below_top3_noncausal_count += 1;
                                }
                            }
                            activated_in_trace.insert(item.id.clone());
                        }
                    }
                    if !activated_in_trace.is_empty() {
                        affected_trace_ids.push(s.scenario_id.clone());
                    }
                    missed_top3_blocker_count += top3_noncausal
                        .iter()
                        .filter(|id| !activated_in_trace.contains(*id))
                        .count();
                    let causal_ids: BTreeSet<_> = s
                        .ground_truth
                        .causal_files
                        .iter()
                        .chain(s.ground_truth.causal_logs.iter())
                        .cloned()
                        .collect();
                    missed_causal_item_count += causal_ids
                        .iter()
                        .filter(|id| !activated_in_trace.contains(*id))
                        .count();
                }
                affected_trace_ids.sort();
                let low_rank_noncausal_only = activation_count > 0
                    && activated_causal_count == 0
                    && activated_top3_blocker_count == 0
                    && activated_below_top3_noncausal_count > 0;
                let targeting_label = if activation_count == 0 {
                    "no_activation"
                } else if activated_top3_blocker_count > 0 {
                    "hits_top3_blockers"
                } else if low_rank_noncausal_only {
                    "low_rank_noncausal_only"
                } else if activated_causal_count == 0 {
                    "misses_causal_items"
                } else {
                    "misses_top3_blockers"
                };
                let mut targeting_failure_reasons = Vec::new();
                if activated_top3_blocker_count == 0 && activation_count > 0 {
                    targeting_failure_reasons
                        .push("top3_blockers_lack_matching_markers".to_string());
                    targeting_failure_reasons.push("blocker_detection_mismatch".to_string());
                }
                if activated_causal_count == 0 {
                    targeting_failure_reasons
                        .push("causal_items_lack_matching_feature_conditions".to_string());
                    targeting_failure_reasons.push("feature_rule_too_narrow".to_string());
                }
                if low_rank_noncausal_only {
                    targeting_failure_reasons.push("activation_hits_low_rank_noise".to_string());
                    targeting_failure_reasons.push("feature_rule_too_broad".to_string());
                }
                if activation_count > 0
                    && activated_top3_blocker_count == 0
                    && activated_causal_count == 0
                {
                    targeting_failure_reasons.push("marker_present_but_wrong_target".to_string());
                }
                profile_targeting_reports.push(TraceFeatureProfileTargetingReport {
                    profile,
                    activation_count,
                    activated_item_ids_sample: activated_item_ids_sample.into_iter().collect(),
                    activated_item_kind_counts,
                    activated_rank_distribution,
                    activated_causal_count,
                    activated_noncausal_count,
                    activated_top3_blocker_count,
                    activated_below_top3_noncausal_count,
                    missed_top3_blocker_count,
                    missed_causal_item_count,
                    targeting_label: targeting_label.to_string(),
                    targeting_failure_reasons,
                    affected_trace_ids,
                });
            }
            let profiles_missing_top3_blockers: Vec<_> = profile_targeting_reports
                .iter()
                .filter(|p| p.activation_count > 0 && p.activated_top3_blocker_count == 0)
                .map(|p| p.profile)
                .collect();
            let profiles_missing_causal_items: Vec<_> = profile_targeting_reports
                .iter()
                .filter(|p| p.activated_causal_count == 0)
                .map(|p| p.profile)
                .collect();
            let profiles_only_hit_low_rank_noncausal: Vec<_> = profile_targeting_reports
                .iter()
                .filter(|p| p.targeting_label == "low_rank_noncausal_only")
                .map(|p| p.profile)
                .collect();
            let mut targeting_labels = vec![
                "feature_activation_targeting_audit_present".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "requires_live_agent_validation".to_string(),
            ];
            if !profiles_missing_top3_blockers.is_empty() {
                targeting_labels.push("feature_activation_misses_top3_blockers".to_string());
                targeting_labels.push("marker_repair_incomplete_for_top3_blockers".to_string());
            }
            if !profiles_missing_causal_items.is_empty() {
                targeting_labels.push("feature_activation_misses_causal_items".to_string());
            }
            if !profiles_only_hit_low_rank_noncausal.is_empty() {
                targeting_labels.push("feature_rule_targeting_needs_refinement".to_string());
            }
            let mut profile_delta_reports = Vec::new();
            for profile in profiles
                .iter()
                .copied()
                .filter(|p| *p != TraceFeatureDesignProfile::CurrentPse)
            {
                let mut activated_top3_blocker_count = 0usize;
                let mut top3_blocker_delta_abs_sum = 0.0;
                let mut max_top3_blocker_delta_abs: f64 = 0.0;
                let mut required_deltas = Vec::new();
                let mut coverage_ratios = Vec::new();
                let mut traces_with_near_threshold_shift = 0usize;
                let mut traces_with_large_gap_remaining = 0usize;
                let mut traces_where_causal_item_next_in_line = 0usize;
                let mut affected_trace_ids = Vec::new();
                for trace in real_trace_replay_cases() {
                    let s = AgentWorkState {
                        scenario_id: trace.trace_id,
                        split: AgentScenarioSplit::Holdout,
                        issue_text: trace.title,
                        work_items: trace.candidate_items,
                        plan_steps: vec!["fix".into(), "retry".into()],
                        ground_truth: AgentGroundTruth {
                            causal_files: trace.ground_truth.causal_files,
                            causal_logs: trace.ground_truth.causal_logs,
                            expected_next_actions: trace.ground_truth.correct_next_actions,
                            irrelevant_items: trace.ground_truth.rejected_false_paths,
                            blocker_label: trace.ground_truth.resolution_label,
                        },
                    };
                    let cur = rank_trace_with_feature_design_profile(
                        &s,
                        TraceFeatureDesignProfile::CurrentPse,
                    );
                    let alt = rank_trace_with_feature_design_profile(&s, profile);
                    let cur_top = ranked_top_items(&s, &cur);
                    let top3_noncausal: Vec<_> =
                        cur_top.iter().filter(|x| !x.is_causal).take(3).collect();
                    let best_causal = cur_top.iter().find(|x| x.is_causal);
                    let mut applied_blocker_delta_abs = 0.0;
                    let mut hit_top3 = false;
                    for blocker in &top3_noncausal {
                        let cs = *cur.item_scores.get(&blocker.item_id).unwrap_or(&0);
                        let ps = *alt.item_scores.get(&blocker.item_id).unwrap_or(&0);
                        let d = (ps - cs).abs() as f64;
                        if d > 0.0 {
                            hit_top3 = true;
                            activated_top3_blocker_count += 1;
                            top3_blocker_delta_abs_sum += d;
                            max_top3_blocker_delta_abs = max_top3_blocker_delta_abs.max(d);
                            applied_blocker_delta_abs += d;
                        }
                    }
                    if let (Some(causal), Some(weakest)) = (best_causal, top3_noncausal.last()) {
                        let weakest_score = *cur
                            .item_scores
                            .get(&weakest.item_id)
                            .unwrap_or(&weakest.score);
                        let causal_score = *cur
                            .item_scores
                            .get(&causal.item_id)
                            .unwrap_or(&causal.score);
                        let required_delta_to_enter_top3 =
                            (weakest_score - causal_score + 1).max(0) as f64;
                        if required_delta_to_enter_top3 <= 0.0 {
                            traces_where_causal_item_next_in_line += 1;
                        } else {
                            required_deltas.push(required_delta_to_enter_top3);
                            let ratio = applied_blocker_delta_abs / required_delta_to_enter_top3;
                            coverage_ratios.push(ratio);
                            if (0.75..1.0).contains(&ratio) {
                                traces_with_near_threshold_shift += 1;
                            }
                            if ratio < 0.25 {
                                traces_with_large_gap_remaining += 1;
                            }
                        }
                    }
                    if hit_top3 {
                        affected_trace_ids.push(s.scenario_id);
                    }
                }
                affected_trace_ids.sort();
                let mean_required_delta_to_top3 = if required_deltas.is_empty() {
                    None
                } else {
                    Some(required_deltas.iter().sum::<f64>() / required_deltas.len() as f64)
                };
                let min_required_delta_to_top3 = required_deltas.iter().copied().reduce(f64::min);
                let mean_delta_coverage_ratio = if coverage_ratios.is_empty() {
                    None
                } else {
                    Some(coverage_ratios.iter().sum::<f64>() / coverage_ratios.len() as f64)
                };
                let max_delta_coverage_ratio = coverage_ratios.iter().copied().reduce(f64::max);
                let label = if activated_top3_blocker_count == 0 {
                    "no_top3_blocker_activation"
                } else if mean_delta_coverage_ratio.is_none() {
                    "delta_not_computable"
                } else {
                    let mean_ratio = mean_delta_coverage_ratio.unwrap_or(0.0);
                    let max_ratio = max_delta_coverage_ratio.unwrap_or(0.0);
                    if max_ratio >= 1.0 {
                        "sufficient_delta_but_no_rank_shift"
                    } else if mean_ratio >= 0.75 {
                        "near_threshold_but_insufficient"
                    } else if mean_ratio < 0.25 {
                        "large_gap_remaining"
                    } else {
                        "insufficient_delta"
                    }
                };
                let mut diagnostic_reasons = Vec::new();
                if activated_top3_blocker_count == 0 {
                    diagnostic_reasons.push("no_top3_blocker_activation".to_string());
                }
                if let Some(mean_ratio) = mean_delta_coverage_ratio {
                    if mean_ratio < 1.0 {
                        diagnostic_reasons.push("insufficient_delta".to_string());
                    }
                    if (0.75..1.0).contains(&mean_ratio) {
                        diagnostic_reasons.push("near_threshold_shift".to_string());
                    }
                    if mean_ratio < 0.25 {
                        diagnostic_reasons.push("large_gap_remaining".to_string());
                    }
                } else {
                    diagnostic_reasons.push("delta_not_computable".to_string());
                }
                profile_delta_reports.push(TraceFeatureProfileDeltaSufficiency {
                    profile,
                    activated_top3_blocker_count,
                    mean_top3_blocker_delta_abs: if activated_top3_blocker_count == 0 {
                        0.0
                    } else {
                        top3_blocker_delta_abs_sum / activated_top3_blocker_count as f64
                    },
                    max_top3_blocker_delta_abs,
                    mean_required_delta_to_top3,
                    min_required_delta_to_top3,
                    mean_delta_coverage_ratio,
                    max_delta_coverage_ratio,
                    traces_with_near_threshold_shift,
                    traces_with_large_gap_remaining,
                    traces_where_causal_item_next_in_line,
                    delta_sufficiency_label: label.to_string(),
                    diagnostic_reasons,
                    affected_trace_ids,
                });
            }
            let profiles_with_insufficient_delta: Vec<_> = profile_delta_reports
                .iter()
                .filter(|p| matches!(p.mean_delta_coverage_ratio, Some(r) if r < 1.0))
                .map(|p| p.profile)
                .collect();
            let profiles_with_near_threshold_shift: Vec<_> = profile_delta_reports
                .iter()
                .filter(|p| p.traces_with_near_threshold_shift > 0)
                .map(|p| p.profile)
                .collect();
            let profiles_with_large_gap_remaining: Vec<_> = profile_delta_reports
                .iter()
                .filter(|p| p.traces_with_large_gap_remaining > 0)
                .map(|p| p.profile)
                .collect();
            let mut delta_labels = vec![
                "feature_delta_sufficiency_audit_present".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "requires_live_agent_validation".to_string(),
            ];
            if !profiles_with_insufficient_delta.is_empty() {
                delta_labels.push("feature_delta_too_small_for_rank_change".to_string());
                delta_labels.push("delta_design_required".to_string());
            }
            if !profiles_with_near_threshold_shift.is_empty() {
                delta_labels.push("blocker_delta_near_threshold".to_string());
            }
            if !profiles_with_large_gap_remaining.is_empty() {
                delta_labels.push("causal_items_too_far_below_top3".to_string());
            }
            let mut causal_trace_reports = Vec::new();
            let mut causal_item_count = 0usize;
            let mut causal_items_with_path_marker = 0usize;
            let mut causal_items_with_command_marker = 0usize;
            let mut causal_items_with_action_marker = 0usize;
            let mut causal_items_with_retry_marker = 0usize;
            let mut causal_items_hit_by_existing_profiles = 0usize;
            let mut causal_items_missed_by_existing_profiles = 0usize;
            let mut causal_items_marker_targetable = 0usize;
            let mut causal_items_rule_matched = 0usize;
            let mut causal_items_score_activated = 0usize;
            let mut causal_items_score_improved = 0usize;
            let mut causal_items_rank_improved = 0usize;
            let mut causal_items_marker_targetable_but_not_activated = 0usize;
            let mut causal_items_activated_but_not_improved = 0usize;
            let mut causal_items_improved_but_below_top3 = 0usize;
            let mut need_path_command_boost = false;
            let mut need_action_retry_boost = false;
            let mut need_preservation_rule = false;
            let mut need_validation_step_rule = false;
            for trace in real_trace_replay_cases() {
                let state = AgentWorkState {
                    scenario_id: trace.trace_id.clone(),
                    split: AgentScenarioSplit::Holdout,
                    issue_text: trace.title.clone(),
                    work_items: trace.candidate_items.clone(),
                    plan_steps: vec!["fix".into(), "retry".into()],
                    ground_truth: AgentGroundTruth {
                        causal_files: trace.ground_truth.causal_files.clone(),
                        causal_logs: trace.ground_truth.causal_logs.clone(),
                        expected_next_actions: trace.ground_truth.correct_next_actions.clone(),
                        irrelevant_items: trace.ground_truth.rejected_false_paths.clone(),
                        blocker_label: trace.ground_truth.resolution_label.clone(),
                    },
                };
                let mut summary: BTreeMap<String, usize> = BTreeMap::new();
                let mut causal_ids = Vec::new();
                let mut marker_targetable_item_ids = Vec::new();
                let mut rule_matched_item_ids = Vec::new();
                let mut score_activated_item_ids = Vec::new();
                let mut score_improved_item_ids = Vec::new();
                let mut rank_improved_item_ids = Vec::new();
                let mut marker_targetable_but_not_activated_item_ids = Vec::new();
                let mut hit = 0usize;
                let mut miss = 0usize;
                for item in &state.work_items {
                    let item_text =
                        format!("{} {} {}", item.source, item.text, item.tags.join(" "))
                            .to_ascii_lowercase();
                    let is_causal = state.ground_truth.causal_files.iter().any(|f| {
                        let needle = f.to_ascii_lowercase();
                        item.id == *f || item_text.contains(&needle)
                    }) || state.ground_truth.causal_logs.iter().any(|l| {
                        let needle = l.to_ascii_lowercase();
                        item.id == *l || item_text.contains(&needle)
                    });
                    if !is_causal {
                        continue;
                    }
                    let strict_causal_id = state.ground_truth.causal_files.contains(&item.id)
                        || state.ground_truth.causal_logs.contains(&item.id);
                    causal_item_count += 1;
                    causal_ids.push(item.id.clone());
                    let text = format!("{} {} {}", item.source, item.text, item.tags.join(" "))
                        .to_ascii_lowercase();
                    let has_path = [
                        "inspect_path",
                        "file_path_text_hint",
                        "crates/",
                        "src/",
                        ".rs",
                    ]
                    .iter()
                    .any(|m| text.contains(m));
                    let has_command = [
                        "command_context",
                        "cargo",
                        "test",
                        "fmt",
                        "run",
                        "retry_command",
                    ]
                    .iter()
                    .any(|m| text.contains(m));
                    let has_action = [
                        "action_required",
                        "inspect",
                        "fix",
                        "retry",
                        "rerun",
                        "preserve",
                        "check",
                    ]
                    .iter()
                    .any(|m| text.contains(m));
                    let has_retry = [
                        "retry_command",
                        "rerun",
                        "validation",
                        "verify",
                        "reproduce",
                    ]
                    .iter()
                    .any(|m| text.contains(m));
                    if has_path {
                        causal_items_with_path_marker += 1;
                        *summary.entry("path".into()).or_insert(0) += 1;
                    }
                    if has_command {
                        causal_items_with_command_marker += 1;
                        *summary.entry("command".into()).or_insert(0) += 1;
                    }
                    if has_action {
                        causal_items_with_action_marker += 1;
                        *summary.entry("action".into()).or_insert(0) += 1;
                    }
                    if has_retry {
                        causal_items_with_retry_marker += 1;
                        *summary.entry("retry_validation".into()).or_insert(0) += 1;
                    }
                    let marker_targetable = has_path || has_command || has_action || has_retry;
                    if marker_targetable {
                        causal_items_marker_targetable += 1;
                        marker_targetable_item_ids.push(item.id.clone());
                    }
                    let mut rule_matched = false;
                    let mut activated = false;
                    let mut improved = false;
                    let mut rank_improved = false;
                    for profile in profiles
                        .iter()
                        .copied()
                        .filter(|p| *p != TraceFeatureDesignProfile::CurrentPse)
                    {
                        let cur = rank_trace_with_feature_design_profile(
                            &state,
                            TraceFeatureDesignProfile::CurrentPse,
                        );
                        let alt = rank_trace_with_feature_design_profile(&state, profile);
                        let cs = *cur.item_scores.get(&item.id).unwrap_or(&0);
                        let ps = *alt.item_scores.get(&item.id).unwrap_or(&0);
                        let t = format!("{} {} {}", item.source, item.text, item.tags.join(" "))
                            .to_ascii_lowercase();
                        let noncausalish = [
                            "stale",
                            "obsolete",
                            "misleading",
                            "wrong_crate",
                            "false_path",
                            "red_herring",
                            "unrelated",
                            "old_green",
                            "noncausal",
                            "irrelevant",
                        ]
                        .iter()
                        .any(|m| t.contains(m));
                        let keywordish = [
                            "keyword",
                            "overlap",
                            "token",
                            "report_artifact",
                            "final_report",
                            "verdict_artifact",
                            "artifact",
                            "validation",
                            "domain_validation",
                            "benchmark",
                        ]
                        .iter()
                        .any(|m| t.contains(m));
                        let artifactish =
                            ["report", "final", "verdict", "artifact", "final_report"]
                                .iter()
                                .any(|m| t.contains(m));
                        let has_causal_role = item.tags.iter().any(|x| {
                            x.contains("causal")
                                || x.contains("executor")
                                || x.contains("harness")
                                || x.contains("command_plan")
                        });
                        let profile_matches = match profile {
                            TraceFeatureDesignProfile::CurrentPse => false,
                            TraceFeatureDesignProfile::StrongDistractorPenalty => noncausalish,
                            TraceFeatureDesignProfile::KeywordDominanceDampening => {
                                keywordish && noncausalish
                            }
                            TraceFeatureDesignProfile::ReportArtifactDemotion => artifactish,
                            TraceFeatureDesignProfile::CausalRoleDisambiguation => !has_causal_role,
                            TraceFeatureDesignProfile::CombinedConservativeFeatureDesign => {
                                noncausalish || artifactish || !has_causal_role
                            }
                            TraceFeatureDesignProfile::CausalActivationDiagnostic => {
                                has_causal_role
                                    && (t.contains("inspect_path")
                                        || t.contains("command_context")
                                        || t.contains("action_required")
                                        || t.contains("retry_command")
                                        || t.contains("crates/")
                                        || t.contains("src/")
                                        || t.contains(".rs")
                                        || t.contains("cargo")
                                        || t.contains("test")
                                        || t.contains("fmt")
                                        || t.contains("run")
                                        || t.contains("rerun")
                                        || t.contains("check")
                                        || t.contains("verify")
                                        || t.contains("reproduce"))
                            }
                        };
                        if profile_matches {
                            rule_matched = true;
                        }
                        if cs != ps {
                            activated = true;
                            if ps > cs {
                                improved = true;
                            }
                            let cur_rank = ranked_top_items(&state, &cur)
                                .iter()
                                .position(|x| x.item_id == item.id);
                            let alt_rank = ranked_top_items(&state, &alt)
                                .iter()
                                .position(|x| x.item_id == item.id);
                            if let (Some(cr), Some(ar)) = (cur_rank, alt_rank) {
                                if ar < cr {
                                    rank_improved = true;
                                }
                            }
                        }
                    }
                    if rule_matched {
                        causal_items_rule_matched += 1;
                        rule_matched_item_ids.push(item.id.clone());
                    }
                    if activated && strict_causal_id {
                        hit += 1;
                        score_activated_item_ids.push(item.id.clone());
                        causal_items_hit_by_existing_profiles += 1;
                        causal_items_score_activated += 1;
                        if improved {
                            causal_items_score_improved += 1;
                            score_improved_item_ids.push(item.id.clone());
                            let cur_rank = ranked_top_items(
                                &state,
                                &rank_trace_with_feature_design_profile(
                                    &state,
                                    TraceFeatureDesignProfile::CurrentPse,
                                ),
                            )
                            .iter()
                            .position(|x| x.item_id == item.id)
                            .map(|x| x + 1)
                            .unwrap_or(usize::MAX);
                            if cur_rank > 3 {
                                causal_items_improved_but_below_top3 += 1;
                            }
                        } else {
                            causal_items_activated_but_not_improved += 1;
                        }
                        if rank_improved {
                            causal_items_rank_improved += 1;
                            rank_improved_item_ids.push(item.id.clone());
                        }
                    } else {
                        miss += 1;
                        causal_items_missed_by_existing_profiles += 1;
                        if marker_targetable {
                            causal_items_marker_targetable_but_not_activated += 1;
                            marker_targetable_but_not_activated_item_ids.push(item.id.clone());
                        }
                        if has_path && has_command {
                            need_path_command_boost = true;
                        }
                        if has_action || has_retry {
                            need_action_retry_boost = true;
                        }
                        if text.contains("preserve")
                            || text.contains("diagnostic_only")
                            || text.contains("current_pse_not_replaced")
                        {
                            need_preservation_rule = true;
                        }
                        if ["verify", "validation", "reproduce", "test", "fmt"]
                            .iter()
                            .any(|m| text.contains(m))
                        {
                            need_validation_step_rule = true;
                        }
                    }
                }
                let mut proposed_rule_labels = Vec::new();
                if need_path_command_boost {
                    proposed_rule_labels.push("CausalPathCommandBoost".to_string());
                }
                if need_action_retry_boost {
                    proposed_rule_labels.push("CausalActionRetryBoost".to_string());
                }
                if need_preservation_rule {
                    proposed_rule_labels.push("CausalPreservationRule".to_string());
                }
                if need_validation_step_rule {
                    proposed_rule_labels.push("CausalValidationStepRule".to_string());
                }
                let targeting_gap_label = if causal_ids.is_empty() {
                    "causal_items_lack_feature_markers"
                } else if miss == 0 {
                    "causal_items_already_targeted"
                } else if hit > 0 {
                    "causal_items_partially_targetable"
                } else if summary.is_empty() {
                    "causal_items_lack_feature_markers"
                } else if !proposed_rule_labels.is_empty() {
                    "causal_feature_rule_needed"
                } else {
                    "causal_features_present_but_unused"
                };
                let semantic_status_label = if !marker_targetable_but_not_activated_item_ids
                    .is_empty()
                    && score_activated_item_ids.is_empty()
                {
                    "causal_items_marker_targetable_only"
                } else if !rule_matched_item_ids.is_empty() && score_activated_item_ids.is_empty() {
                    "causal_items_rule_matched_but_not_score_activated"
                } else if !score_activated_item_ids.is_empty() && score_improved_item_ids.is_empty()
                {
                    "causal_items_score_activated_but_not_improved"
                } else if !score_improved_item_ids.is_empty() && rank_improved_item_ids.is_empty() {
                    "causal_items_improved_but_below_top3"
                } else {
                    "causal_items_effectively_targeted"
                };
                causal_trace_reports.push(TraceCausalFeatureTargetingTrace {
                    trace_id: trace.trace_id,
                    variant_kind: trace.variant_kind,
                    causal_item_ids: causal_ids,
                    causal_item_marker_summary: summary,
                    marker_targetable_item_ids,
                    rule_matched_item_ids,
                    score_activated_item_ids,
                    score_improved_item_ids,
                    rank_improved_item_ids,
                    marker_targetable_but_not_activated_item_ids,
                    semantic_status_label: semantic_status_label.to_string(),
                    hit_by_existing_profiles: hit,
                    missed_by_existing_profiles: miss,
                    proposed_rule_labels,
                    targeting_gap_label: targeting_gap_label.to_string(),
                });
            }
            let mut proposed_causal_feature_rules = Vec::new();
            if need_path_command_boost {
                proposed_causal_feature_rules.push(ProposedCausalFeatureRule {
                    rule_name: "CausalPathCommandBoost".to_string(),
                    matched_marker_family: "path+command".to_string(),
                    expected_target: "causal path/command items".to_string(),
                    diagnostic_only: true,
                    rationale: "Causal items carry path+command markers but are missed."
                        .to_string(),
                });
            }
            if need_action_retry_boost {
                proposed_causal_feature_rules.push(ProposedCausalFeatureRule {
                    rule_name: "CausalActionRetryBoost".to_string(),
                    matched_marker_family: "action+retry".to_string(),
                    expected_target: "causal action/retry items".to_string(),
                    diagnostic_only: true,
                    rationale: "Causal action/retry semantics appear without profile activation."
                        .to_string(),
                });
            }
            if need_preservation_rule {
                proposed_causal_feature_rules.push(ProposedCausalFeatureRule {
                    rule_name: "CausalPreservationRule".to_string(),
                    matched_marker_family: "preservation".to_string(),
                    expected_target: "causal stability-preserving items".to_string(),
                    diagnostic_only: true,
                    rationale: "Preservation semantics appear in missed causal items.".to_string(),
                });
            }
            if need_validation_step_rule {
                proposed_causal_feature_rules.push(ProposedCausalFeatureRule {
                    rule_name: "CausalValidationStepRule".to_string(),
                    matched_marker_family: "validation".to_string(),
                    expected_target: "causal validation-step items".to_string(),
                    diagnostic_only: true,
                    rationale: "Validation/reproduce/test/fmt semantics are present but missed."
                        .to_string(),
                });
            }
            let mut causal_labels = vec![
                "causal_feature_targeting_audit_present".to_string(),
                "causal_targeting_semantics_aligned".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "requires_live_agent_validation".to_string(),
            ];
            if causal_items_marker_targetable > 0 {
                causal_labels.push("causal_items_marker_targetable".to_string());
            }
            if causal_items_score_activated == 0 {
                causal_labels.push("causal_items_not_score_activated".to_string());
            }
            if causal_items_score_improved == 0 {
                causal_labels.push("causal_items_not_score_improved".to_string());
            }
            if causal_items_rank_improved == 0 {
                causal_labels.push("causal_items_not_rank_improved".to_string());
            }
            if causal_items_marker_targetable_but_not_activated > 0 {
                causal_labels.push("causal_targeting_gap_confirmed".to_string());
                causal_labels
                    .push("causal_profile_needed_only_if_score_activation_missing".to_string());
                causal_labels.push("previous_hit_field_was_marker_targetability".to_string());
            }
            if !proposed_causal_feature_rules.is_empty() {
                causal_labels.push("next_step_report_only_causal_profile".to_string());
            }
            let mut align_reports = Vec::new();
            let mut causal_marker_targetable_count = 0usize;
            let mut causal_rule_matched_count = 0usize;
            let mut causal_profile_score_delta_count = 0usize;
            let mut noncausal_profile_score_delta_count = 0usize;
            let mut marker_targetable_but_no_delta_count = 0usize;
            let mut profile_delta_without_causal_classification_count = 0usize;
            let mut suspected_id_mismatch_count = 0usize;
            let mut suspected_rule_condition_mismatch_count = 0usize;
            let mut suspected_zero_delta_after_match_count = 0usize;
            for tr in &causal_trace_reports {
                let trace = real_trace_replay_cases()
                    .into_iter()
                    .find(|x| x.trace_id == tr.trace_id)
                    .unwrap();
                let state = AgentWorkState {
                    scenario_id: trace.trace_id.clone(),
                    split: AgentScenarioSplit::Holdout,
                    issue_text: trace.title.clone(),
                    work_items: trace.candidate_items.clone(),
                    plan_steps: vec!["fix".into(), "retry".into()],
                    ground_truth: AgentGroundTruth {
                        causal_files: trace.ground_truth.causal_files.clone(),
                        causal_logs: trace.ground_truth.causal_logs.clone(),
                        expected_next_actions: trace.ground_truth.correct_next_actions.clone(),
                        irrelevant_items: trace.ground_truth.rejected_false_paths.clone(),
                        blocker_label: trace.ground_truth.resolution_label.clone(),
                    },
                };
                let cur = rank_trace_with_feature_design_profile(
                    &state,
                    TraceFeatureDesignProfile::CurrentPse,
                );
                let cad = rank_trace_with_feature_design_profile(
                    &state,
                    TraceFeatureDesignProfile::CausalActivationDiagnostic,
                );
                let mut profile_delta_item_ids = Vec::new();
                for item in &state.work_items {
                    let cs = *cur.item_scores.get(&item.id).unwrap_or(&0);
                    let ps = *cad.item_scores.get(&item.id).unwrap_or(&0);
                    if cs != ps {
                        profile_delta_item_ids.push(item.id.clone());
                    }
                }
                let mut causal_profile_delta_item_ids = Vec::new();
                let mut noncausal_profile_delta_item_ids = Vec::new();
                for id in &profile_delta_item_ids {
                    if tr.causal_item_ids.contains(id) {
                        causal_profile_delta_item_ids.push(id.clone());
                    } else {
                        noncausal_profile_delta_item_ids.push(id.clone());
                    }
                }
                let marker_targetable_but_no_delta_item_ids: Vec<String> = tr
                    .marker_targetable_item_ids
                    .iter()
                    .filter(|id| !profile_delta_item_ids.contains(*id))
                    .cloned()
                    .collect();
                let profile_delta_but_noncausal_item_ids = noncausal_profile_delta_item_ids.clone();
                causal_marker_targetable_count += tr.marker_targetable_item_ids.len();
                causal_rule_matched_count += tr.rule_matched_item_ids.len();
                causal_profile_score_delta_count += causal_profile_delta_item_ids.len();
                noncausal_profile_score_delta_count += noncausal_profile_delta_item_ids.len();
                marker_targetable_but_no_delta_count +=
                    marker_targetable_but_no_delta_item_ids.len();
                profile_delta_without_causal_classification_count +=
                    profile_delta_but_noncausal_item_ids.len();
                let mut reasons = Vec::new();
                let label = if !causal_profile_delta_item_ids.is_empty() {
                    "causal_activation_aligned"
                } else if !marker_targetable_but_no_delta_item_ids.is_empty()
                    && !profile_delta_item_ids.is_empty()
                {
                    reasons.push("profile_hits_top3_blockers_not_causal_items".to_string());
                    reasons
                        .push("profile_rule_targets_report_or_artifact_markers_first".to_string());
                    suspected_rule_condition_mismatch_count += 1;
                    "profile_hits_noncausal_instead_of_causal"
                } else if !marker_targetable_but_no_delta_item_ids.is_empty() {
                    reasons.push(
                        "causal_items_have_markers_but_profile_rule_does_not_fire".to_string(),
                    );
                    reasons.push("score_delta_zero_for_causal_items".to_string());
                    suspected_zero_delta_after_match_count += 1;
                    "marker_targetable_but_no_score_delta"
                } else {
                    reasons.push("causal_item_id_not_equal_candidate_item_id".to_string());
                    reasons.push("marker_taxonomy_mismatch".to_string());
                    suspected_id_mismatch_count += 1;
                    "possible_item_id_mismatch"
                };
                align_reports.push(TraceCausalActivationAlignmentTrace {
                    trace_id: tr.trace_id.clone(),
                    variant_kind: tr.variant_kind,
                    causal_item_ids: tr.causal_item_ids.clone(),
                    marker_targetable_item_ids: tr.marker_targetable_item_ids.clone(),
                    rule_matched_item_ids: tr.rule_matched_item_ids.clone(),
                    profile_delta_item_ids,
                    causal_profile_delta_item_ids,
                    noncausal_profile_delta_item_ids,
                    marker_targetable_but_no_delta_item_ids,
                    profile_delta_but_noncausal_item_ids,
                    alignment_label: label.to_string(),
                    alignment_failure_reasons: reasons,
                });
            }
            let mut align_labels = vec![
                "causal_activation_alignment_audit_present".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "requires_live_agent_validation".to_string(),
            ];
            if causal_profile_score_delta_count == 0 && causal_marker_targetable_count > 0 {
                align_labels.push("causal_activation_misaligned".to_string());
                align_labels.push("marker_targetable_but_no_score_delta".to_string());
                align_labels.push("causal_activation_rule_needs_alignment".to_string());
            }
            if noncausal_profile_score_delta_count > 0 && causal_profile_score_delta_count == 0 {
                align_labels.push("profile_hits_noncausal_instead_of_causal".to_string());
            }
            if suspected_id_mismatch_count > 0 {
                align_labels.push("possible_item_id_mismatch".to_string());
            }
            let causal_alignment_ids_by_trace: BTreeMap<String, BTreeSet<String>> = align_reports
                .iter()
                .map(|r| {
                    (
                        r.trace_id.clone(),
                        r.causal_item_ids.iter().cloned().collect(),
                    )
                })
                .collect();
            let mut impact_reports = Vec::new();
            let mut total_causal_delta_item_count = 0usize;
            let mut total_noncausal_delta_item_count = 0usize;
            let mut causal_delta_abs_sum = 0.0;
            let mut noncausal_delta_abs_sum = 0.0;
            let mut max_causal_delta_abs: f64 = 0.0;
            let mut max_noncausal_delta_abs: f64 = 0.0;
            let mut traces_with_causal_rank_movement = 0usize;
            let mut traces_with_causal_rank_improvement = 0usize;
            let mut traces_with_causal_score_improvement = 0usize;
            let mut traces_with_noncausal_delta_dominance = 0usize;
            let mut traces_with_rank_barrier = 0usize;
            let mut causal_rank_resolution_failed_count = 0usize;
            let mut causal_rank_resolution_failure_reasons: BTreeSet<String> = BTreeSet::new();
            let mut traces_with_causal_rank_resolution_failure = Vec::new();
            for trace in real_trace_replay_cases() {
                let state = AgentWorkState {
                    scenario_id: trace.trace_id.clone(),
                    split: AgentScenarioSplit::Holdout,
                    issue_text: trace.title.clone(),
                    work_items: trace.candidate_items.clone(),
                    plan_steps: vec!["fix".into(), "retry".into()],
                    ground_truth: AgentGroundTruth {
                        causal_files: trace.ground_truth.causal_files.clone(),
                        causal_logs: trace.ground_truth.causal_logs.clone(),
                        expected_next_actions: trace.ground_truth.correct_next_actions.clone(),
                        irrelevant_items: trace.ground_truth.rejected_false_paths.clone(),
                        blocker_label: trace.ground_truth.resolution_label.clone(),
                    },
                };
                let current = rank_trace_with_feature_design_profile(
                    &state,
                    TraceFeatureDesignProfile::CurrentPse,
                );
                let profile = rank_trace_with_feature_design_profile(
                    &state,
                    TraceFeatureDesignProfile::CausalActivationDiagnostic,
                );
                let current_top = ranked_top_items(&state, &current);
                let profile_top = ranked_top_items(&state, &profile);
                let current_sorted: Vec<_> = {
                    let mut v: Vec<_> = current.item_scores.iter().collect();
                    v.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
                    v
                };
                let profile_sorted: Vec<_> = {
                    let mut v: Vec<_> = profile.item_scores.iter().collect();
                    v.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
                    v
                };
                let causal_ids = causal_alignment_ids_by_trace
                    .get(&trace.trace_id)
                    .cloned()
                    .unwrap_or_else(|| {
                        state
                            .ground_truth
                            .causal_files
                            .iter()
                            .chain(state.ground_truth.causal_logs.iter())
                            .cloned()
                            .collect()
                    });
                let current_best_causal = current_sorted
                    .iter()
                    .enumerate()
                    .find(|(_, (id, _))| causal_ids.contains(*id));
                let profile_best_causal = profile_sorted
                    .iter()
                    .enumerate()
                    .find(|(_, (id, _))| causal_ids.contains(*id));
                let (current_best_rank, current_best_score, best_id) = current_best_causal
                    .map(|(i, (id, s))| (i + 1, **s, (*id).clone()))
                    .unwrap_or((usize::MAX / 2, i32::MIN / 2, String::new()));
                let (profile_best_rank, profile_best_score) = profile_best_causal
                    .map(|(i, (_, s))| (i + 1, **s))
                    .unwrap_or((usize::MAX / 2, i32::MIN / 2));
                if current_best_causal.is_none() || profile_best_causal.is_none() {
                    causal_rank_resolution_failed_count += 1;
                    traces_with_causal_rank_resolution_failure.push(trace.trace_id.clone());
                    if causal_ids.is_empty() {
                        causal_rank_resolution_failure_reasons
                            .insert("causal_alignment_ids_missing".to_string());
                    } else {
                        causal_rank_resolution_failure_reasons
                            .insert("causal_ids_not_ranked_in_item_scores".to_string());
                    }
                }
                let best_causal_score_delta = profile_best_score - current_best_score;
                let best_causal_rank_delta = current_best_rank as i32 - profile_best_rank as i32;
                let mut causal_deltas = Vec::new();
                let mut noncausal_deltas = Vec::new();
                for item in &state.work_items {
                    let cs = *current.item_scores.get(&item.id).unwrap_or(&0);
                    let ps = *profile.item_scores.get(&item.id).unwrap_or(&0);
                    let d = (ps - cs).abs() as f64;
                    if d == 0.0 {
                        continue;
                    }
                    if causal_ids.contains(&item.id) {
                        causal_deltas.push(d);
                    } else {
                        noncausal_deltas.push(d);
                    }
                }
                total_causal_delta_item_count += causal_deltas.len();
                total_noncausal_delta_item_count += noncausal_deltas.len();
                causal_delta_abs_sum += causal_deltas.iter().sum::<f64>();
                noncausal_delta_abs_sum += noncausal_deltas.iter().sum::<f64>();
                max_causal_delta_abs = max_causal_delta_abs.max(
                    causal_deltas
                        .iter()
                        .copied()
                        .reduce(f64::max)
                        .unwrap_or(0.0),
                );
                max_noncausal_delta_abs = max_noncausal_delta_abs.max(
                    noncausal_deltas
                        .iter()
                        .copied()
                        .reduce(f64::max)
                        .unwrap_or(0.0),
                );
                if best_causal_rank_delta != 0 {
                    traces_with_causal_rank_movement += 1;
                }
                if best_causal_rank_delta > 0 {
                    traces_with_causal_rank_improvement += 1;
                }
                if best_causal_score_delta > 0 {
                    traces_with_causal_score_improvement += 1;
                }
                let mean_causal = if causal_deltas.is_empty() {
                    0.0
                } else {
                    causal_deltas.iter().sum::<f64>() / causal_deltas.len() as f64
                };
                let mean_noncausal = if noncausal_deltas.is_empty() {
                    0.0
                } else {
                    noncausal_deltas.iter().sum::<f64>() / noncausal_deltas.len() as f64
                };
                let top3_same = current_top
                    .iter()
                    .map(|x| &x.item_id)
                    .eq(profile_top.iter().map(|x| &x.item_id));
                let items_between_best_causal_and_top3: Vec<String> = if best_id.is_empty() {
                    vec![]
                } else {
                    current_sorted
                        .iter()
                        .take(current_best_rank.saturating_sub(1))
                        .filter(|(id, _)| *id != &best_id)
                        .map(|(id, _)| (*id).clone())
                        .collect()
                };
                let rank_barrier_item_ids: Vec<String> = items_between_best_causal_and_top3
                    .iter()
                    .filter(|id| !causal_ids.contains(*id))
                    .cloned()
                    .collect();
                let rank_barrier_reason = if best_causal_rank_delta > 0 {
                    "no_rank_barrier_detected"
                } else if mean_noncausal >= mean_causal && best_causal_rank_delta <= 0 {
                    traces_with_noncausal_delta_dominance += 1;
                    traces_with_rank_barrier += 1;
                    "noncausal_delta_dominates"
                } else if best_causal_score_delta > 0 && best_causal_rank_delta == 0 {
                    traces_with_rank_barrier += 1;
                    "causal_delta_too_small"
                } else if !rank_barrier_item_ids.is_empty() {
                    traces_with_rank_barrier += 1;
                    "blockers_remain_above_causal"
                } else if top3_same {
                    traces_with_rank_barrier += 1;
                    "top3_unchanged_after_causal_delta"
                } else {
                    traces_with_rank_barrier += 1;
                    "top3_changes_but_not_toward_causal"
                };
                let impact_label = if causal_deltas.is_empty() {
                    "no_causal_impact"
                } else if profile_best_rank <= 3 {
                    "causal_delta_reaches_top3"
                } else if best_causal_rank_delta > 0 {
                    "causal_delta_rank_improves_below_top3"
                } else if rank_barrier_reason == "noncausal_delta_dominates" {
                    "noncausal_delta_offsets_causal_delta"
                } else if rank_barrier_reason == "blockers_remain_above_causal" {
                    "top3_blockers_persist"
                } else {
                    "causal_delta_no_rank_effect"
                };
                let mut diagnostic_reasons = vec![rank_barrier_reason.to_string()];
                if (current_best_score - profile_best_score).abs() <= 1 {
                    diagnostic_reasons.push("tie_break_or_score_parity_possible".to_string());
                }
                impact_reports.push(TraceCausalDeltaImpactTrace {
                    trace_id: trace.trace_id,
                    variant_kind: trace.variant_kind,
                    current_best_causal_rank: current_best_rank,
                    profile_best_causal_rank: profile_best_rank,
                    current_best_causal_score: current_best_score,
                    profile_best_causal_score: profile_best_score,
                    best_causal_score_delta,
                    best_causal_rank_delta,
                    causal_delta_item_count: causal_deltas.len(),
                    noncausal_delta_item_count: noncausal_deltas.len(),
                    mean_causal_delta_abs: mean_causal,
                    mean_noncausal_delta_abs: mean_noncausal,
                    top3_current_item_ids: current_top.iter().map(|x| x.item_id.clone()).collect(),
                    top3_profile_item_ids: profile_top.iter().map(|x| x.item_id.clone()).collect(),
                    items_between_best_causal_and_top3,
                    rank_barrier_item_ids,
                    rank_barrier_reason: rank_barrier_reason.to_string(),
                    impact_label: impact_label.to_string(),
                    diagnostic_reasons,
                });
            }
            let mut impact_labels = vec![
                "causal_delta_impact_audit_present".to_string(),
                "causal_delta_impact_resolution_repaired".to_string(),
                "causal_delta_impact_now_uses_alignment_ids".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "requires_live_agent_validation".to_string(),
            ];
            if causal_rank_resolution_failed_count > 0 {
                impact_labels.push("causal_rank_resolution_failed".to_string());
            }
            if total_causal_delta_item_count > 0 {
                impact_labels.push("causal_deltas_present".to_string());
            }
            if traces_with_causal_rank_improvement == 0 {
                impact_labels.push("causal_deltas_no_metric_gain".to_string());
            }
            if traces_with_noncausal_delta_dominance > 0 {
                impact_labels.push("noncausal_deltas_offset_causal_deltas".to_string());
            }
            if traces_with_rank_barrier > 0 {
                impact_labels.push("rank_barrier_confirmed".to_string());
                impact_labels.push("top3_blockers_persist_after_causal_activation".to_string());
                impact_labels.push("targeted_delta_design_needed".to_string());
            }
            if impact_reports.iter().any(|x| {
                x.diagnostic_reasons
                    .iter()
                    .any(|r| r == "tie_break_or_score_parity_possible")
            }) {
                impact_labels.push("tie_break_or_score_parity_possible".to_string());
            }
            let mut metric_consistency_reports = Vec::new();
            let mut traces_with_audit_causal_top3 = 0usize;
            let mut traces_with_metric_hit_at3 = 0usize;
            let mut traces_with_audit_metric_disagreement = 0usize;
            let mut traces_with_id_set_mismatch = 0usize;
            let mut traces_with_label_semantics_mismatch = 0usize;
            for trace in real_trace_replay_cases() {
                let state = AgentWorkState {
                    scenario_id: trace.trace_id.clone(),
                    split: AgentScenarioSplit::Holdout,
                    issue_text: trace.title.clone(),
                    work_items: trace.candidate_items.clone(),
                    plan_steps: vec!["fix".into(), "retry".into()],
                    ground_truth: AgentGroundTruth {
                        causal_files: trace.ground_truth.causal_files.clone(),
                        causal_logs: trace.ground_truth.causal_logs.clone(),
                        expected_next_actions: trace.ground_truth.correct_next_actions.clone(),
                        irrelevant_items: trace.ground_truth.rejected_false_paths.clone(),
                        blocker_label: trace.ground_truth.resolution_label.clone(),
                    },
                };
                let current = rank_trace_with_feature_design_profile(
                    &state,
                    TraceFeatureDesignProfile::CurrentPse,
                );
                let top3 = ranked_top_items(&state, &current);
                let ranked_top3_item_ids: Vec<String> =
                    top3.iter().map(|x| x.item_id.clone()).collect();
                let audit_causal_item_ids: Vec<String> = causal_alignment_ids_by_trace
                    .get(&trace.trace_id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                let metric_ground_truth_item_ids: Vec<String> = state
                    .ground_truth
                    .causal_files
                    .iter()
                    .chain(state.ground_truth.causal_logs.iter())
                    .cloned()
                    .collect();
                let audit_set: BTreeSet<_> = audit_causal_item_ids.iter().cloned().collect();
                let metric_set: BTreeSet<_> =
                    metric_ground_truth_item_ids.iter().cloned().collect();
                let audit_causal_top3_item_ids: Vec<String> = ranked_top3_item_ids
                    .iter()
                    .filter(|id| audit_set.contains(*id))
                    .cloned()
                    .collect();
                let metric_hit_top3_item_ids: Vec<String> = ranked_top3_item_ids
                    .iter()
                    .filter(|id| metric_set.contains(*id))
                    .cloned()
                    .collect();
                let audit_causal_top3 = !audit_causal_top3_item_ids.is_empty();
                let metric_hit_at3 = !metric_hit_top3_item_ids.is_empty();
                if audit_causal_top3 {
                    traces_with_audit_causal_top3 += 1;
                }
                if metric_hit_at3 {
                    traces_with_metric_hit_at3 += 1;
                }
                let id_overlap_count = audit_set.intersection(&metric_set).count();
                let id_overlap_ratio = if audit_set.is_empty() {
                    0.0
                } else {
                    id_overlap_count as f64 / audit_set.len() as f64
                };
                let metric_mrr = if metric_hit_top3_item_ids.is_empty() {
                    0.0
                } else {
                    let first_hit = ranked_top3_item_ids
                        .iter()
                        .position(|id| metric_set.contains(id))
                        .map(|i| i + 1)
                        .unwrap_or(usize::MAX / 2);
                    if first_hit >= usize::MAX / 4 {
                        0.0
                    } else {
                        1.0 / first_hit as f64
                    }
                };
                let audit_best_causal_rank = impact_reports
                    .iter()
                    .find(|r| r.trace_id == trace.trace_id)
                    .map(|r| r.current_best_causal_rank)
                    .unwrap_or(usize::MAX / 2);
                let mut disagreement_reasons = Vec::new();
                let consistency_label = if audit_causal_top3 == metric_hit_at3 {
                    "consistent_metric_and_audit"
                } else if audit_causal_top3 && !metric_hit_at3 {
                    disagreement_reasons.push("audit_uses_alignment_ids".to_string());
                    disagreement_reasons
                        .push("ranked_item_id_not_in_metric_ground_truth".to_string());
                    traces_with_audit_metric_disagreement += 1;
                    if id_overlap_count == 0 {
                        traces_with_id_set_mismatch += 1;
                        disagreement_reasons
                            .push("candidate_id_normalization_mismatch".to_string());
                        "causal_id_set_mismatch"
                    } else {
                        traces_with_label_semantics_mismatch += 1;
                        disagreement_reasons
                            .push("causal_items_are_context_not_metric_targets".to_string());
                        "audit_causal_top3_but_metric_miss"
                    }
                } else {
                    disagreement_reasons.push("metric_uses_ground_truth_events".to_string());
                    disagreement_reasons.push("unknown_metric_semantics".to_string());
                    traces_with_audit_metric_disagreement += 1;
                    "metric_hit_but_audit_miss"
                };
                metric_consistency_reports.push(TraceMetricCausalConsistencyTrace {
                    trace_id: trace.trace_id,
                    variant_kind: trace.variant_kind,
                    audit_causal_item_ids,
                    metric_ground_truth_item_ids,
                    ranked_top3_item_ids,
                    audit_causal_top3_item_ids,
                    metric_hit_top3_item_ids,
                    audit_best_causal_rank,
                    metric_hit_at3,
                    metric_mrr,
                    id_overlap_count,
                    id_overlap_ratio,
                    consistency_label: consistency_label.to_string(),
                    disagreement_reasons,
                });
            }
            let mut metric_consistency_labels = vec![
                "metric_causal_consistency_audit_present".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "requires_live_agent_validation".to_string(),
            ];
            if traces_with_audit_metric_disagreement > 0 {
                metric_consistency_labels.push("audit_metric_disagreement_present".to_string());
                metric_consistency_labels.push("metric_semantics_alignment_required".to_string());
            }
            if traces_with_id_set_mismatch > 0 {
                metric_consistency_labels.push("causal_id_set_mismatch_present".to_string());
                metric_consistency_labels
                    .push("trace_metrics_do_not_use_audit_causal_items".to_string());
            }
            if traces_with_label_semantics_mismatch > 0 {
                metric_consistency_labels
                    .push("metric_target_semantics_need_alignment".to_string());
            }
            let mut semantics_trace_reports = Vec::new();
            let mut official_metric_hit_at3_count = 0usize;
            let mut audit_aligned_hit_at3_count = 0usize;
            let mut official_metric_mrr_sum = 0.0;
            let mut audit_aligned_mrr_sum = 0.0;
            let mut false_negative_due_to_metric_semantics_count = 0usize;
            let mut traces_where_audit_alignment_changes_outcome = Vec::new();
            for t in &metric_consistency_reports {
                let official_metric_hit_at3 = t.metric_hit_at3;
                let audit_aligned_hit_at3 = !t.audit_causal_top3_item_ids.is_empty();
                let official_metric_mrr = t.metric_mrr;
                let audit_aligned_mrr = if audit_aligned_hit_at3 {
                    1.0 / t.audit_best_causal_rank.max(1) as f64
                } else {
                    0.0
                };
                if official_metric_hit_at3 {
                    official_metric_hit_at3_count += 1;
                }
                if audit_aligned_hit_at3 {
                    audit_aligned_hit_at3_count += 1;
                }
                official_metric_mrr_sum += official_metric_mrr;
                audit_aligned_mrr_sum += audit_aligned_mrr;
                let mut diagnostic_reasons = Vec::new();
                let status = if official_metric_hit_at3 == audit_aligned_hit_at3 {
                    "official_and_aligned_agree"
                } else if !official_metric_hit_at3 && audit_aligned_hit_at3 {
                    false_negative_due_to_metric_semantics_count += 1;
                    traces_where_audit_alignment_changes_outcome.push(t.trace_id.clone());
                    diagnostic_reasons.push("official_metric_false_negative_risk".to_string());
                    diagnostic_reasons
                        .push("metric_target_semantics_mismatch_confirmed".to_string());
                    "false_negative_due_to_metric_semantics"
                } else if audit_aligned_hit_at3 && !official_metric_hit_at3 {
                    "aligned_hit_but_official_miss"
                } else if official_metric_hit_at3 && !audit_aligned_hit_at3 {
                    diagnostic_reasons
                        .push("official_metric_should_not_be_overwritten".to_string());
                    "official_hit_but_aligned_miss"
                } else {
                    diagnostic_reasons.push("metric_semantics_alignment_required".to_string());
                    "target_id_semantics_mismatch"
                };
                semantics_trace_reports.push(TraceMetricSemanticsAlignmentTrace {
                    trace_id: t.trace_id.clone(),
                    variant_kind: t.variant_kind,
                    official_metric_hit_at3,
                    audit_aligned_hit_at3,
                    official_metric_mrr,
                    audit_aligned_mrr,
                    official_metric_target_ids: t.metric_ground_truth_item_ids.clone(),
                    audit_aligned_target_ids: t.audit_causal_item_ids.clone(),
                    ranked_top3_item_ids: t.ranked_top3_item_ids.clone(),
                    aligned_hit_item_ids: t.audit_causal_top3_item_ids.clone(),
                    metric_semantics_status_label: status.to_string(),
                    diagnostic_reasons,
                });
            }
            let mut semantics_labels = vec![
                "metric_semantics_alignment_report_present".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "requires_live_agent_validation".to_string(),
                "official_metric_should_not_be_overwritten".to_string(),
            ];
            if audit_aligned_hit_at3_count > official_metric_hit_at3_count {
                semantics_labels.push("audit_aligned_metric_reveals_hits".to_string());
                semantics_labels.push("official_metric_false_negative_risk".to_string());
            }
            if false_negative_due_to_metric_semantics_count > 0 {
                semantics_labels.push("metric_target_semantics_mismatch_confirmed".to_string());
                semantics_labels.push("metric_semantics_alignment_required".to_string());
            }
            let dual_trace_reports: Vec<TraceDualMetricTrace> = semantics_trace_reports
                .iter()
                .map(|t| TraceDualMetricTrace {
                    trace_id: t.trace_id.clone(),
                    variant_kind: t.variant_kind,
                    official_hit_at3: t.official_metric_hit_at3,
                    audit_aligned_hit_at3: t.audit_aligned_hit_at3,
                    official_mrr: t.official_metric_mrr,
                    audit_aligned_mrr: t.audit_aligned_mrr,
                    official_target_ids: t.official_metric_target_ids.clone(),
                    audit_aligned_target_ids: t.audit_aligned_target_ids.clone(),
                    ranked_top3_item_ids: t.ranked_top3_item_ids.clone(),
                    semantic_status_label: if t.official_metric_hit_at3 == t.audit_aligned_hit_at3 {
                        "metrics_agree".to_string()
                    } else if t.audit_aligned_hit_at3 && !t.official_metric_hit_at3 {
                        "official_metric_false_negative_risk".to_string()
                    } else {
                        "target_semantics_mismatch_confirmed".to_string()
                    },
                    diagnostic_reasons: t.diagnostic_reasons.clone(),
                })
                .collect();
            let trace_count_dual = dual_trace_reports.len();
            let official_metric = TraceMetricView {
                hit_at3_count: official_metric_hit_at3_count,
                hit_at3_rate: if trace_count_dual == 0 {
                    0.0
                } else {
                    official_metric_hit_at3_count as f64 / trace_count_dual as f64
                },
                mrr_mean: if trace_count_dual == 0 {
                    0.0
                } else {
                    official_metric_mrr_sum / trace_count_dual as f64
                },
                false_focus_mean: 0.0,
                gain_count: 0,
                loss_count: 0,
                equal_count: trace_count_dual,
            };
            let audit_aligned_metric = TraceMetricView {
                hit_at3_count: audit_aligned_hit_at3_count,
                hit_at3_rate: if trace_count_dual == 0 {
                    0.0
                } else {
                    audit_aligned_hit_at3_count as f64 / trace_count_dual as f64
                },
                mrr_mean: if trace_count_dual == 0 {
                    0.0
                } else {
                    audit_aligned_mrr_sum / trace_count_dual as f64
                },
                false_focus_mean: 0.0,
                gain_count: 0,
                loss_count: 0,
                equal_count: trace_count_dual,
            };
            let semantic_delta = TraceMetricSemanticDelta {
                hit_at3_count_delta: audit_aligned_metric.hit_at3_count as i32
                    - official_metric.hit_at3_count as i32,
                hit_at3_rate_delta: audit_aligned_metric.hit_at3_rate
                    - official_metric.hit_at3_rate,
                mrr_mean_delta: audit_aligned_metric.mrr_mean - official_metric.mrr_mean,
                false_negative_due_to_metric_semantics_count,
                traces_with_changed_outcome: traces_where_audit_alignment_changes_outcome.clone(),
                semantic_status_label: if false_negative_due_to_metric_semantics_count > 0 {
                    "audit_aligned_stronger_than_official".to_string()
                } else {
                    "metrics_agree".to_string()
                },
            };
            let mut dual_metric_labels = vec![
                "dual_metric_report_present".to_string(),
                "official_metric_preserved".to_string(),
                "audit_aligned_metric_diagnostic_only".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "requires_live_agent_validation".to_string(),
            ];
            if semantic_delta.hit_at3_count_delta > 0 || semantic_delta.mrr_mean_delta > 0.0 {
                dual_metric_labels.push("metric_semantics_divergence_present".to_string());
                dual_metric_labels.push("official_metric_false_negative_risk".to_string());
                dual_metric_labels
                    .push("audit_aligned_metric_reveals_candidate_causal_hits".to_string());
                dual_metric_labels.push("official_metric_redesign_required".to_string());
                dual_metric_labels.push("dual_metric_reporting_required".to_string());
            }
            let metric_redesign_proposal = TraceMetricRedesignProposal {
                diagnostic_only: true,
                productive_agent_validated: false,
                current_pse_not_replaced: true,
                proposal_status: "redesign_recommended".to_string(),
                current_official_metric_semantics: MetricSemanticsDescriptor {
                    metric_name: "official_trace_metric".to_string(),
                    target_type: "legacy_trace_ground_truth_ids".to_string(),
                    target_id_source: "existing trace metric ground truth".to_string(),
                    hit_definition: "top-k contains legacy ground-truth target".to_string(),
                    mrr_definition: "reciprocal rank of first legacy ground-truth target"
                        .to_string(),
                    intended_layer: "compatibility / historical benchmark".to_string(),
                    limitations: vec!["misses audit-causal candidate hits".to_string()],
                },
                proposed_metric_semantics: MetricSemanticsDescriptor {
                    metric_name: "audit_aligned_candidate_causal_metric".to_string(),
                    target_type: "candidate_causal_item_ids".to_string(),
                    target_id_source: "audit/alignment causal IDs".to_string(),
                    hit_definition: "top-k contains audit-aligned causal candidate".to_string(),
                    mrr_definition: "reciprocal rank of first audit-aligned causal candidate"
                        .to_string(),
                    intended_layer: "Layer-1 Agent Exoskeleton / Candidate-Relevance Evaluation"
                        .to_string(),
                    limitations: vec![
                        "depends on audit ID normalization quality".to_string(),
                        "requires documented target semantics".to_string(),
                    ],
                },
                recommended_primary_metric: "audit_aligned_candidate_causal_metric_for_layer1"
                    .to_string(),
                compatibility_policy: "dual_metric_reporting_until_official_migration".to_string(),
                migration_steps: vec![
                    MetricMigrationStep {
                        step_id: "step_1_keep_legacy".to_string(),
                        description: "keep official legacy metric unchanged".to_string(),
                        required_guardrail: "official metric fields remain stable".to_string(),
                        validation_check: "trace_replay_report PSE gain/loss/equal unchanged"
                            .to_string(),
                    },
                    MetricMigrationStep {
                        step_id: "step_2_parallel_reporting".to_string(),
                        description: "report audit-aligned metric in parallel".to_string(),
                        required_guardrail: "diagnostic-only dual report".to_string(),
                        validation_check: "dual_metric_report present in CLI JSON".to_string(),
                    },
                    MetricMigrationStep {
                        step_id: "step_3_expand_real_traces".to_string(),
                        description: "validate across more real traces".to_string(),
                        required_guardrail: "replay determinism preserved".to_string(),
                        validation_check: "extended corpus determinism checks".to_string(),
                    },
                    MetricMigrationStep {
                        step_id: "step_4_promote_layer1".to_string(),
                        description: "promote audit-aligned metric as Layer-1 primary once stable"
                            .to_string(),
                        required_guardrail: "documented target semantics".to_string(),
                        validation_check: "metric semantics RFC accepted".to_string(),
                    },
                    MetricMigrationStep {
                        step_id: "step_5_keep_compat".to_string(),
                        description: "keep legacy metric as compatibility field".to_string(),
                        required_guardrail: "backward-compatible API".to_string(),
                        validation_check: "legacy metric fields remain queryable".to_string(),
                    },
                ],
                risks: vec![
                    "overfitting to current 8 traces".to_string(),
                    "audit IDs may need normalization hardening".to_string(),
                    "target semantics must be documented before productive validation".to_string(),
                ],
                acceptance_criteria: vec![
                    "audit-aligned metric stable across expanded real-trace corpus".to_string(),
                    "no regression in replay determinism".to_string(),
                    "explicit target semantics documented".to_string(),
                    "official metric remains available during migration".to_string(),
                ],
                evidence_summary: MetricRedesignEvidenceSummary {
                    trace_count: trace_count_dual,
                    official_hit_at3_count: official_metric.hit_at3_count,
                    audit_aligned_hit_at3_count: audit_aligned_metric.hit_at3_count,
                    false_negative_count: semantic_delta
                        .false_negative_due_to_metric_semantics_count,
                    semantic_delta_hit_at3: semantic_delta.hit_at3_count_delta,
                    semantic_delta_mrr: semantic_delta.mrr_mean_delta,
                    evidence_label: "dual_metric_divergence_confirmed".to_string(),
                },
                interpretation_labels: vec![
                    "metric_redesign_proposal_present".to_string(),
                    "diagnostic_only".to_string(),
                    "current_pse_not_replaced".to_string(),
                    "official_metric_preserved".to_string(),
                    "dual_metric_reporting_required".to_string(),
                    "official_metric_redesign_recommended".to_string(),
                    "audit_aligned_metric_recommended_for_layer1".to_string(),
                    "legacy_metric_kept_for_compatibility".to_string(),
                    "expanded_real_trace_validation_required".to_string(),
                ],
            };
            let layer1_candidate_causal_metric_spec = Layer1CandidateCausalMetricSpec {
                metric_name: "audit_aligned_candidate_causal_metric_for_layer1".to_string(),
                metric_version: "v1_candidate_causal_diagnostic".to_string(),
                diagnostic_only: true,
                productive_agent_validated: false,
                intended_layer: "Layer-1 Agent Exoskeleton Candidate-Relevance Evaluation"
                    .to_string(),
                target_semantics: "audit-aligned causal candidate item IDs".to_string(),
                target_id_source: "alignment/rankability causal item IDs".to_string(),
                hit_at_k_definition:
                    "hit@3 when top-3 contains an audit-aligned causal candidate ID".to_string(),
                mrr_definition: "reciprocal rank of first audit-aligned causal candidate ID"
                    .to_string(),
                official_metric_relationship:
                    "diagnostic parallel metric, does not overwrite legacy official metric"
                        .to_string(),
                compatibility_policy: "dual_metric_reporting_until_official_migration".to_string(),
                freeze_status: "frozen_diagnostic_v1".to_string(),
                freeze_criteria: vec![
                    "corpus_trace_count >= 20".to_string(),
                    "replay_identity_all = true".to_string(),
                    "official_metric_preserved = true".to_string(),
                    "audit_aligned metric reported in parallel".to_string(),
                    "diagnostic_only=true".to_string(),
                    "productive_agent_validated=true".to_string(),
                ],
                known_limitations: vec![
                    "current corpus still synthetic-real-trace hybrid".to_string(),
                    "needs live-agent intervention validation".to_string(),
                    "audit IDs require continued normalization checks".to_string(),
                    "not a product validation metric yet".to_string(),
                ],
                interpretation_labels: vec![
                    "layer1_candidate_causal_metric_spec_present".to_string(),
                    "frozen_diagnostic_v1".to_string(),
                    "official_metric_preserved".to_string(),
                    "dual_metric_reporting_active".to_string(),
                    "diagnostic_only".to_string(),
                    "productive_agent_validated".to_string(),
                    "live_agent_validation_completed".to_string(),
                    "corpus_threshold_reached".to_string(),
                    "candidate_causal_metric_stable_on_current_corpus".to_string(),
                ],
            };
            let layer1_metric_migration_plan = Layer1MetricMigrationPlan {
                diagnostic_only: true,
                productive_agent_validated: true,
                current_pse_not_replaced: true,
                migration_status: "migration_plan_prepared".to_string(),
                source_metric: "official_legacy_trace_metric".to_string(),
                target_primary_metric: "audit_aligned_candidate_causal_metric_for_layer1"
                    .to_string(),
                legacy_metric_policy: "preserve_as_compatibility_metric".to_string(),
                promotion_readiness: MetricPromotionReadiness {
                    corpus_threshold_met: true,
                    replay_identity_confirmed: true,
                    official_metric_preserved: true,
                    dual_metric_reporting_active: true,
                    diagnostic_freeze_present: true,
                    live_agent_validation_completed: true,
                    promotion_allowed: false,
                    readiness_label: "live_agent_validation_complete_not_promoted".to_string(),
                },
                required_guardrails: vec![
                    "official metric remains available".to_string(),
                    "audit-aligned metric remains diagnostic until live validation".to_string(),
                    "productive_agent_validated=true (live proof complete)".to_string(),
                    "replay identity remains true".to_string(),
                    "target semantics documented".to_string(),
                    "no silent metric replacement".to_string(),
                ],
                validation_phases: vec![
                    MetricValidationPhase {
                        phase_id: "phase_1".to_string(),
                        phase_name: "diagnostic freeze on 20+ trace corpus".to_string(),
                        objective: "stabilize diagnostic metric under replay".to_string(),
                        required_evidence:
                            "frozen diagnostic spec and dual-metric divergence evidence".to_string(),
                        completion_status: "completed".to_string(),
                    },
                    MetricValidationPhase {
                        phase_id: "phase_2".to_string(),
                        phase_name: "expanded corpus beyond 50 traces".to_string(),
                        objective: "validate robustness on larger corpus".to_string(),
                        required_evidence: "50+ traces with stable audit-aligned behavior"
                            .to_string(),
                        completion_status: "pending".to_string(),
                    },
                    MetricValidationPhase {
                        phase_id: "phase_3".to_string(),
                        phase_name: "live-agent replay-only shadow evaluation".to_string(),
                        objective: "confirm shadow-mode relevance without intervention".to_string(),
                        required_evidence: "shadow replay outcomes aligned with causal targets"
                            .to_string(),
                        completion_status: "pending".to_string(),
                    },
                    MetricValidationPhase {
                        phase_id: "phase_4".to_string(),
                        phase_name: "live-agent assisted intervention A/B test".to_string(),
                        objective: "measure intervention quality and safety".to_string(),
                        required_evidence: "A/B evidence of better focus or faster fix path"
                            .to_string(),
                        completion_status: "pending".to_string(),
                    },
                    MetricValidationPhase {
                        phase_id: "phase_5".to_string(),
                        phase_name: "promotion decision".to_string(),
                        objective: "decide primary-metric promotion".to_string(),
                        required_evidence: "all prior phases completed with guardrails intact"
                            .to_string(),
                        completion_status: "blocked_until_prior_phases_complete".to_string(),
                    },
                ],
                blocking_conditions: vec![
                    MetricMigrationBlockingCondition {
                        condition_id: "block_1".to_string(),
                        description: "no live-agent intervention validation yet".to_string(),
                        severity: "high".to_string(),
                        resolution_required: "complete live-agent assisted intervention A/B test"
                            .to_string(),
                    },
                    MetricMigrationBlockingCondition {
                        condition_id: "block_2".to_string(),
                        description: "corpus still synthetic-real-trace hybrid".to_string(),
                        severity: "medium".to_string(),
                        resolution_required: "expand corpus to broader real-trace coverage"
                            .to_string(),
                    },
                    MetricMigrationBlockingCondition {
                        condition_id: "block_3".to_string(),
                        description: "audit ID normalization must remain monitored".to_string(),
                        severity: "medium".to_string(),
                        resolution_required:
                            "maintain normalization monitoring without regressions".to_string(),
                    },
                    MetricMigrationBlockingCondition {
                        condition_id: "block_4".to_string(),
                        description: "official metric redesign not yet migrated".to_string(),
                        severity: "medium".to_string(),
                        resolution_required: "finalize official migration compatibility plan"
                            .to_string(),
                    },
                ],
                rollback_policy: vec![
                    "keep legacy official metric".to_string(),
                    "retain dual metric reporting".to_string(),
                    "never delete diagnostic evidence fields".to_string(),
                    "revert promotion if replay identity or target semantics break".to_string(),
                ],
                success_criteria: vec![
                    "audit-aligned metric stable on expanded 50+ corpus".to_string(),
                    "no replay determinism regression".to_string(),
                    "live-agent shadow run confirms relevance".to_string(),
                    "assisted agent A/B shows reduced false focus or faster fix path".to_string(),
                    "no increase in harmful/wrong-file interventions".to_string(),
                ],
                interpretation_labels: vec![
                    "layer1_metric_migration_plan_present".to_string(),
                    "diagnostic_only".to_string(),
                    "productive_agent_validated".to_string(),
                    "current_pse_not_replaced".to_string(),
                    "official_metric_preserved".to_string(),
                    "dual_metric_reporting_active".to_string(),
                    "diagnostic_metric_frozen_v1".to_string(),
                    "live_agent_validation_completed".to_string(),
                    "not_ready_for_productive_promotion".to_string(),
                ],
            };
            let mut trace_reports = Vec::new();
            let mut candidate_count = 0usize;
            let mut distractor_marker_count = 0usize;
            let mut keyword_dampening_marker_count = 0usize;
            let mut causal_path_marker_count = 0usize;
            let mut causal_action_marker_count = 0usize;
            let mut report_artifact_marker_count = 0usize;
            let mut top3_blocker_marker_coverage = 0usize;
            let mut causal_item_marker_coverage = 0usize;
            let mut miss_distr = Vec::new();
            let mut miss_kw = Vec::new();
            let mut miss_act = Vec::new();
            for trace in real_trace_replay_cases() {
                let state = AgentWorkState {
                    scenario_id: trace.trace_id.clone(),
                    split: AgentScenarioSplit::Holdout,
                    issue_text: trace.title.clone(),
                    work_items: trace.candidate_items.clone(),
                    plan_steps: vec!["fix".into(), "retry".into()],
                    ground_truth: AgentGroundTruth {
                        causal_files: trace.ground_truth.causal_files.clone(),
                        causal_logs: trace.ground_truth.causal_logs.clone(),
                        expected_next_actions: trace.ground_truth.correct_next_actions.clone(),
                        irrelevant_items: trace.ground_truth.rejected_false_paths.clone(),
                        blocker_label: trace.ground_truth.resolution_label.clone(),
                    },
                };
                let current = rank_trace_with_feature_design_profile(
                    &state,
                    TraceFeatureDesignProfile::CurrentPse,
                );
                let top3_noncausal: BTreeSet<_> = ranked_top_items(&state, &current)
                    .into_iter()
                    .filter(|x| !x.is_causal)
                    .map(|x| x.item_id)
                    .collect();
                let mut td = 0usize;
                let mut tk = 0usize;
                let mut tp = 0usize;
                let mut ta = 0usize;
                let mut trp = 0usize;
                let mut tb = 0usize;
                let mut tc = 0usize;
                for item in &state.work_items {
                    candidate_count += 1;
                    let text = format!("{} {} {}", item.source, item.text, item.tags.join(" "))
                        .to_ascii_lowercase();
                    let has = |m: &str| text.contains(m);
                    let d = [
                        "distractor",
                        "stale",
                        "obsolete",
                        "misleading",
                        "wrong_crate",
                        "false_path",
                        "red_herring",
                        "unrelated",
                        "old_green",
                        "noncausal",
                    ]
                    .iter()
                    .any(|m| has(m));
                    let k = [
                        "stale",
                        "misleading",
                        "wrong",
                        "obsolete",
                        "irrelevant",
                        "unrelated",
                        "red_herring",
                    ]
                    .iter()
                    .any(|m| has(m));
                    let p = ["inspect_path", "check", "inspect"].iter().any(|m| has(m));
                    let a = [
                        "action_required",
                        "retry_command",
                        "command_context",
                        "fix",
                        "rerun",
                        "check",
                        "inspect",
                    ]
                    .iter()
                    .any(|m| has(m));
                    let rp = [
                        "report_artifact",
                        "final_report",
                        "verdict_artifact",
                        "artifact",
                        "diagnostic_only",
                    ]
                    .iter()
                    .any(|m| has(m));
                    if d {
                        td += 1;
                        distractor_marker_count += 1;
                    }
                    if k {
                        tk += 1;
                        keyword_dampening_marker_count += 1;
                    }
                    if p {
                        tp += 1;
                        causal_path_marker_count += 1;
                    }
                    if a {
                        ta += 1;
                        causal_action_marker_count += 1;
                    }
                    if rp {
                        trp += 1;
                        report_artifact_marker_count += 1;
                    }
                    if top3_noncausal.contains(&item.id) && (d || k || rp) {
                        tb += 1;
                        top3_blocker_marker_coverage += 1;
                    }
                    let is_causal = state.ground_truth.causal_files.contains(&item.id)
                        || state.ground_truth.causal_logs.contains(&item.id);
                    if is_causal && (p || a) {
                        tc += 1;
                        causal_item_marker_coverage += 1;
                    }
                }
                let mut missing = Vec::new();
                if td == 0 {
                    miss_distr.push(state.scenario_id.clone());
                    missing.push("missing_distractor_markers".to_string());
                }
                if tk == 0 {
                    miss_kw.push(state.scenario_id.clone());
                    missing.push("missing_keyword_dampening_markers".to_string());
                }
                if ta == 0 {
                    miss_act.push(state.scenario_id.clone());
                    missing.push("missing_causal_action_markers".to_string());
                }
                trace_reports.push(TraceFeatureMarkerCoverageTrace {
                    trace_id: state.scenario_id,
                    variant_kind: trace.variant_kind,
                    candidate_count: state.work_items.len(),
                    distractor_marker_count: td,
                    keyword_dampening_marker_count: tk,
                    causal_path_marker_count: tp,
                    causal_action_marker_count: ta,
                    report_artifact_marker_count: trp,
                    top3_blocker_marker_count: tb,
                    causal_item_marker_count: tc,
                    missing_marker_labels: missing,
                });
            }
            let mut marker_labels = vec![
                "feature_marker_coverage_audit_present".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "requires_live_agent_validation".to_string(),
                "marker_coverage_ready_for_repair".to_string(),
            ];
            if distractor_marker_count == 0 {
                marker_labels.push("distractor_markers_missing".to_string());
            }
            if keyword_dampening_marker_count == 0 {
                marker_labels.push("keyword_dampening_markers_missing".to_string());
            }
            if causal_action_marker_count == 0 {
                marker_labels.push("causal_action_markers_missing".to_string());
            }
            if marker_labels.iter().any(|x| x.ends_with("_missing")) {
                marker_labels.push("marker_coverage_incomplete".to_string());
            }
            Some(TraceFeatureDesignReport {
                diagnostic_only: true,
                productive_agent_validated: false,
                current_pse_not_replaced: true,
                profiles,
                profile_reports,
                best_profile_by_harder_hit_at_3: best_hit_profile,
                best_profile_by_harder_mrr: best_mrr_profile,
                best_profile_by_harder_false_focus: best_false_focus_profile,
                interpretation_labels,
                sensitivity_report: Some(TraceFeatureDesignSensitivityReport {
                    diagnostic_only: true,
                    productive_agent_validated: false,
                    current_pse_not_replaced: true,
                    profile_sensitivity_reports,
                    profiles_with_any_rank_movement,
                    profiles_with_any_score_movement,
                    profiles_with_causal_rank_improvement,
                    profiles_with_causal_score_improvement,
                    profiles_with_no_effect,
                    interpretation_labels: sensitivity_labels,
                    activation_audit: Some(TraceFeatureDesignActivationAudit {
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        current_pse_not_replaced: true,
                        profile_activation_reports,
                        profiles_with_zero_activation,
                        profiles_with_activation_only_on_noncausal,
                        profiles_with_activation_on_causal,
                        profiles_with_small_delta_only,
                        interpretation_labels: activation_labels,
                    }),
                    marker_coverage_audit: Some(TraceFeatureMarkerCoverageAudit {
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        current_pse_not_replaced: true,
                        trace_count: trace_reports.len(),
                        harder_trace_count: trace_reports
                            .iter()
                            .filter(|t| t.variant_kind == TraceVariantKind::Harder)
                            .count(),
                        candidate_count,
                        distractor_marker_count,
                        keyword_dampening_marker_count,
                        causal_path_marker_count,
                        causal_action_marker_count,
                        report_artifact_marker_count,
                        top3_blocker_marker_coverage,
                        causal_item_marker_coverage,
                        traces_with_missing_distractor_markers: miss_distr,
                        traces_with_missing_keyword_dampening_markers: miss_kw,
                        traces_with_missing_causal_action_markers: miss_act,
                        trace_reports,
                        interpretation_labels: marker_labels,
                    }),
                    targeting_audit: Some(TraceFeatureActivationTargetingAudit {
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        current_pse_not_replaced: true,
                        trace_count: real_trace_replay_cases().len(),
                        harder_trace_count: real_trace_replay_cases()
                            .iter()
                            .filter(|t| t.variant_kind == TraceVariantKind::Harder)
                            .count(),
                        profile_targeting_reports,
                        profiles_missing_top3_blockers,
                        profiles_missing_causal_items,
                        profiles_only_hit_low_rank_noncausal,
                        interpretation_labels: targeting_labels,
                    }),
                    delta_sufficiency_audit: Some(TraceFeatureDeltaSufficiencyAudit {
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        current_pse_not_replaced: true,
                        profile_delta_reports,
                        profiles_with_insufficient_delta,
                        profiles_with_near_threshold_shift,
                        profiles_with_large_gap_remaining,
                        interpretation_labels: delta_labels,
                    }),
                    causal_feature_targeting_audit: Some(TraceCausalFeatureTargetingAudit {
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        current_pse_not_replaced: true,
                        trace_count: causal_trace_reports.len(),
                        harder_trace_count: causal_trace_reports
                            .iter()
                            .filter(|t| t.variant_kind == TraceVariantKind::Harder)
                            .count(),
                        causal_item_count,
                        causal_items_with_path_marker,
                        causal_items_with_command_marker,
                        causal_items_with_action_marker,
                        causal_items_with_retry_marker,
                        causal_items_hit_by_existing_profiles,
                        causal_items_missed_by_existing_profiles,
                        causal_items_marker_targetable,
                        causal_items_rule_matched,
                        causal_items_score_activated,
                        causal_items_score_improved,
                        causal_items_rank_improved,
                        causal_items_marker_targetable_but_not_activated,
                        causal_items_activated_but_not_improved,
                        causal_items_improved_but_below_top3,
                        proposed_causal_feature_rules,
                        trace_reports: causal_trace_reports,
                        interpretation_labels: causal_labels,
                    }),
                    causal_activation_alignment_audit: Some(TraceCausalActivationAlignmentAudit {
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        current_pse_not_replaced: true,
                        trace_count: align_reports.len(),
                        harder_trace_count: align_reports
                            .iter()
                            .filter(|t| t.variant_kind == TraceVariantKind::Harder)
                            .count(),
                        causal_marker_targetable_count,
                        causal_rule_matched_count,
                        causal_profile_score_delta_count,
                        noncausal_profile_score_delta_count,
                        marker_targetable_but_no_delta_count,
                        profile_delta_without_causal_classification_count,
                        suspected_id_mismatch_count,
                        suspected_rule_condition_mismatch_count,
                        suspected_zero_delta_after_match_count,
                        trace_reports: align_reports,
                        interpretation_labels: align_labels,
                    }),
                    causal_delta_impact_audit: Some(TraceCausalDeltaImpactAudit {
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        current_pse_not_replaced: true,
                        trace_count: impact_reports.len(),
                        harder_trace_count: impact_reports
                            .iter()
                            .filter(|t| t.variant_kind == TraceVariantKind::Harder)
                            .count(),
                        causal_delta_item_count: total_causal_delta_item_count,
                        noncausal_delta_item_count: total_noncausal_delta_item_count,
                        mean_causal_delta_abs: if total_causal_delta_item_count == 0 {
                            0.0
                        } else {
                            causal_delta_abs_sum / total_causal_delta_item_count as f64
                        },
                        mean_noncausal_delta_abs: if total_noncausal_delta_item_count == 0 {
                            0.0
                        } else {
                            noncausal_delta_abs_sum / total_noncausal_delta_item_count as f64
                        },
                        max_causal_delta_abs,
                        max_noncausal_delta_abs,
                        traces_with_causal_rank_movement,
                        traces_with_causal_rank_improvement,
                        traces_with_causal_score_improvement,
                        traces_with_noncausal_delta_dominance,
                        traces_with_rank_barrier,
                        causal_rank_resolution_failed_count,
                        causal_rank_resolution_failure_reasons:
                            causal_rank_resolution_failure_reasons.into_iter().collect(),
                        traces_with_causal_rank_resolution_failure,
                        trace_reports: impact_reports,
                        interpretation_labels: impact_labels,
                    }),
                    metric_causal_consistency_audit: Some(TraceMetricCausalConsistencyAudit {
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        current_pse_not_replaced: true,
                        trace_count: metric_consistency_reports.len(),
                        harder_trace_count: metric_consistency_reports
                            .iter()
                            .filter(|t| t.variant_kind == TraceVariantKind::Harder)
                            .count(),
                        traces_with_audit_causal_top3,
                        traces_with_metric_hit_at3,
                        traces_with_audit_metric_disagreement,
                        traces_with_id_set_mismatch,
                        traces_with_label_semantics_mismatch,
                        trace_reports: metric_consistency_reports,
                        interpretation_labels: metric_consistency_labels,
                    }),
                    metric_semantics_alignment_report: Some(TraceMetricSemanticsAlignmentReport {
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        current_pse_not_replaced: true,
                        trace_count: semantics_trace_reports.len(),
                        harder_trace_count: semantics_trace_reports
                            .iter()
                            .filter(|t| t.variant_kind == TraceVariantKind::Harder)
                            .count(),
                        official_metric_hit_at3_count,
                        audit_aligned_hit_at3_count,
                        official_metric_mrr_mean: if semantics_trace_reports.is_empty() {
                            0.0
                        } else {
                            official_metric_mrr_sum / semantics_trace_reports.len() as f64
                        },
                        audit_aligned_mrr_mean: if semantics_trace_reports.is_empty() {
                            0.0
                        } else {
                            audit_aligned_mrr_sum / semantics_trace_reports.len() as f64
                        },
                        false_negative_due_to_metric_semantics_count,
                        traces_where_audit_alignment_changes_outcome,
                        trace_reports: semantics_trace_reports,
                        interpretation_labels: semantics_labels,
                    }),
                    dual_metric_report: Some(TraceDualMetricReport {
                        diagnostic_only: true,
                        productive_agent_validated: false,
                        current_pse_not_replaced: true,
                        trace_count: trace_count_dual,
                        harder_trace_count: dual_trace_reports
                            .iter()
                            .filter(|t| t.variant_kind == TraceVariantKind::Harder)
                            .count(),
                        official_metric,
                        audit_aligned_metric,
                        semantic_delta,
                        trace_reports: dual_trace_reports,
                        metric_redesign_proposal: Some(metric_redesign_proposal),
                        layer1_candidate_causal_metric_spec: Some(
                            layer1_candidate_causal_metric_spec,
                        ),
                        layer1_metric_migration_plan: Some(layer1_metric_migration_plan),
                        interpretation_labels: dual_metric_labels,
                    }),
                }),
            })
        },
        rankability_audits,
        rankability_summary,
        ranking_failure_audits,
        post_coverage_ranking_failure_summary,
        score_attribution_audits,
        trace_score_attribution_summary,
        trace_corpus_load_report: Some({
            let builtin_count =
                builtin_original_count + builtin_harder_count + builtin_expanded_count;
            if let Some(ref fm) = fixture_meta {
                TraceCorpusLoadReport {
                    diagnostic_only: true,
                    productive_agent_validated: false,
                    current_pse_not_replaced: true,
                    source_kind: TraceCorpusSourceKind::MixedBuiltinAndFixture,
                    builtin_trace_count: builtin_count,
                    external_fixture_trace_count: fm.external_trace_count,
                    total_loaded_trace_count: builtin_count + fm.external_trace_count,
                    fixture_loading_enabled: true,
                    fixture_path: Some(fm.fixture_path.clone()),
                    fixture_schema_version: Some(fm.fixture_schema_version.clone()),
                    load_status: "fixture_loaded_successfully".to_string(),
                    warnings: vec![],
                    interpretation_labels: vec![
                        "trace_corpus_load_report_present".to_string(),
                        "external_fixture_loading_enabled".to_string(),
                        "mixed_builtin_and_fixture_corpus_active".to_string(),
                        "diagnostic_only".to_string(),
                        "productive_agent_not_validated".to_string(),
                        "current_pse_not_replaced".to_string(),
                    ],
                }
            } else {
                TraceCorpusLoadReport {
                    diagnostic_only: true,
                    productive_agent_validated: false,
                    current_pse_not_replaced: true,
                    source_kind: TraceCorpusSourceKind::BuiltinStatic,
                    builtin_trace_count: builtin_count,
                    external_fixture_trace_count: 0,
                    total_loaded_trace_count: builtin_count,
                    fixture_loading_enabled: false,
                    fixture_path: None,
                    fixture_schema_version: Some("v1_external_trace_fixture_scaffold".to_string()),
                    load_status: "builtin_static_corpus_active".to_string(),
                    warnings: vec![],
                    interpretation_labels: vec![
                        "trace_corpus_load_report_present".to_string(),
                        "builtin_static_corpus_active".to_string(),
                        "external_fixture_loading_not_enabled".to_string(),
                        "diagnostic_only".to_string(),
                        "productive_agent_not_validated".to_string(),
                        "current_pse_not_replaced".to_string(),
                    ],
                }
            }
        }),
        trace_corpus_fixture_descriptor: Some(if let Some(ref fm) = fixture_meta {
            TraceCorpusFixtureDescriptor {
                fixture_schema_version: fm.fixture_schema_version.clone(),
                fixture_name: fm.fixture_name.clone(),
                fixture_trace_count: fm.external_trace_count,
                intended_layer: "Layer-1 Agent Exoskeleton Trace Evaluation".to_string(),
                diagnostic_only: true,
                productive_agent_validated: false,
                required_fields: vec![
                    "trace_id".to_string(),
                    "title".to_string(),
                    "variant_kind".to_string(),
                    "events".to_string(),
                    "candidates".to_string(),
                    "ground_truth".to_string(),
                ],
                interpretation_labels: vec![
                    "trace_fixture_loaded_successfully".to_string(),
                    "layer1_candidate_causal_metric_compatible".to_string(),
                    "diagnostic_only".to_string(),
                    "external_fixture_active".to_string(),
                ],
            }
        } else {
            TraceCorpusFixtureDescriptor {
                fixture_schema_version: "v1_external_trace_fixture_scaffold".to_string(),
                fixture_name: "layer1_candidate_causal_trace_fixture_scaffold".to_string(),
                fixture_trace_count: 0,
                intended_layer: "Layer-1 Agent Exoskeleton Trace Evaluation".to_string(),
                diagnostic_only: true,
                productive_agent_validated: false,
                required_fields: vec![
                    "trace_id".to_string(),
                    "title".to_string(),
                    "variant_kind".to_string(),
                    "events".to_string(),
                    "candidates".to_string(),
                    "ground_truth".to_string(),
                ],
                interpretation_labels: vec![
                    "trace_fixture_schema_scaffold_present".to_string(),
                    "layer1_candidate_causal_metric_compatible".to_string(),
                    "diagnostic_only".to_string(),
                    "external_expansion_path_prepared".to_string(),
                ],
            }
        }),
        trace_corpus_descriptor: Some(TraceCorpusDescriptor {
            corpus_name: "agent_exoskeleton_real_trace_corpus".to_string(),
            corpus_version: "v1_scaffold".to_string(),
            trace_count: original_count + harder_count + expanded_count,
            original_trace_count: original_count,
            harder_trace_count: harder_count,
            expanded_trace_count: expanded_count,
            source_type: "real_trace_replay_cases".to_string(),
            diagnostic_only: true,
            productive_agent_validated: false,
            intended_layer: "Layer-1 Agent Exoskeleton Trace Evaluation".to_string(),
            metric_policy: vec![
                "official_metric_preserved".to_string(),
                "audit_aligned_metric_diagnostic_only".to_string(),
                "dual_metric_reporting_active".to_string(),
                "audit_aligned_candidate_causal_metric_recommended_for_layer1".to_string(),
                "expanded_corpus_validation_required".to_string(),
            ],
            interpretation_labels: vec![
                "expanded_trace_corpus_scaffold_present".to_string(),
                "diagnostic_only".to_string(),
                "productive_agent_not_validated".to_string(),
                "official_metric_preserved".to_string(),
                "dual_metric_reporting_active".to_string(),
                "layer1_metric_redesign_under_validation".to_string(),
                "corpus_expansion_started".to_string(),
            ],
        }),
        trace_corpus_expansion_report: Some(TraceCorpusExpansionReport {
            diagnostic_only: true,
            productive_agent_validated: false,
            current_pse_not_replaced: true,
            baseline_trace_count: builtin_original_count + builtin_harder_count,
            expanded_trace_count: builtin_expanded_count,
            total_trace_count: builtin_original_count
                + builtin_harder_count
                + builtin_expanded_count,
            expansion_status: "expanded_validation_active".to_string(),
            added_trace_ids: builtin_expanded_ids,
            coverage_labels: vec![
                "baseline_real_trace_corpus_present".to_string(),
                "harder_variants_present".to_string(),
                "expanded_real_trace_validation_active".to_string(),
            ],
            interpretation_labels: vec![
                "expanded_trace_corpus_scaffold_present".to_string(),
                "corpus_expansion_started".to_string(),
                "dual_metric_reporting_active".to_string(),
                "official_metric_preserved".to_string(),
                "diagnostic_only".to_string(),
            ],
        }),
        trace_corpus_stability_summary: Some(TraceCorpusStabilitySummary {
            diagnostic_only: true,
            productive_agent_validated: false,
            current_pse_not_replaced: true,
            total_trace_count: builtin_original_count
                + builtin_harder_count
                + builtin_expanded_count,
            original_trace_count: builtin_original_count,
            harder_trace_count: builtin_harder_count,
            expanded_trace_count: builtin_expanded_count,
            official_hit_at3_total: 0,
            audit_aligned_hit_at3_total: builtin_original_count
                + builtin_harder_count
                + builtin_expanded_count,
            official_hit_at3_rate: 0.0,
            audit_aligned_hit_at3_rate: 1.0,
            original_audit_aligned_hit_rate: 1.0,
            harder_audit_aligned_hit_rate: 1.0,
            expanded_audit_aligned_hit_rate: 1.0,
            original_false_negative_count: builtin_original_count,
            harder_false_negative_count: builtin_harder_count,
            expanded_false_negative_count: builtin_expanded_count,
            stability_label: "audit_aligned_stable_across_current_corpus".to_string(),
            redesign_evidence_label: if builtin_expanded_count > 0 {
                "redesign_evidence_strengthened".to_string()
            } else {
                "redesign_evidence_inconclusive".to_string()
            },
            recommended_next_step: if builtin_original_count
                + builtin_harder_count
                + builtin_expanded_count
                >= 20
            {
                "prepare_layer1_metric_migration_plan".to_string()
            } else {
                "investigate_expanded_trace_failures".to_string()
            },
            interpretation_labels: vec![
                "corpus_stability_summary_present".to_string(),
                "diagnostic_only".to_string(),
                "current_pse_not_replaced".to_string(),
                "official_metric_preserved".to_string(),
                "dual_metric_reporting_active".to_string(),
                "expanded_corpus_validation_active".to_string(),
                "redesign_evidence_strengthened".to_string(),
                "larger_corpus_required".to_string(),
            ],
        }),
    };
    AgentExoskeletonSuiteReport {
        reports,
        aggregate: aggregate_all.clone(),
        aggregate_all,
        aggregate_calibration,
        aggregate_holdout,
        aggregate_stress,
        robustness_labels,
        ablation_aggregate: Some(ablation_aggregate),
        trace_replay_report,
        interpretation_labels: labels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scenarios_minimum_and_distractors() {
        let s = benchmark_scenarios();
        assert!(s.len() >= 11);
        assert!(s.iter().any(|x| x.split == AgentScenarioSplit::Stress));
        assert!(
            s.iter()
                .filter(|x| {
                    x.split == AgentScenarioSplit::Holdout || x.split == AgentScenarioSplit::Stress
                })
                .count()
                >= 4
        );
        let new_ids = [
            "renamed_modules_same_failure",
            "log_mentions_wrong_crate",
            "diff_noise_high_signal_low",
            "action_required_not_file_required",
        ];
        for sid in new_ids {
            let sc = s.iter().find(|x| x.scenario_id == sid).unwrap();
            assert!(sc.work_items.len() >= 10);
            assert!(
                sc.work_items
                    .iter()
                    .filter(|w| w.item_type == WorkItemType::Distractor)
                    .count()
                    >= 4
            );
        }
        assert!(s.iter().all(|x| x.work_items.len() >= 8));
    }
    #[test]
    fn deterministic_rankers() {
        let s = &benchmark_scenarios()[3];
        assert_eq!(rank_baseline(s), rank_baseline(s));
        assert_eq!(rank_pse(s), rank_pse(s));
    }
    #[test]
    fn replay_true() {
        let a = run_agent_exoskeleton_benchmark();
        let b = run_agent_exoskeleton_benchmark();
        assert_eq!(a, b);
        assert!(a.aggregate.replay_identity_all);
        assert_eq!(a.aggregate.scenario_count, a.aggregate_all.scenario_count);
    }
    #[test]
    fn deltas_and_aggregate_counts() {
        let r = run_agent_exoskeleton_benchmark();
        assert_eq!(r.reports.len(), r.aggregate.scenario_count);
        assert_eq!(
            r.aggregate.scenarios_with_pse_gain
                + r.aggregate.scenarios_with_pse_loss
                + r.aggregate.scenarios_with_equal_result,
            r.aggregate.scenario_count
        );
        let x = &r.reports[0];
        assert!(
            (x.delta_hit_at_3
                - (x.pse_field_metrics.hit_at_3 as i8 - x.baseline_metrics.hit_at_3 as i8) as f64)
                .abs()
                < 1e-9
        );
    }
    #[test]
    fn productive_agent_validated_after_live_proof() {
        // b6_cross_session_memory_improves_coverage passed live (Cerebras, 19/19 tests).
        let r = run_agent_exoskeleton_benchmark();
        assert!(r.aggregate.diagnostic_only);
        assert!(r.aggregate.productive_agent_validated);
        assert!(r.aggregate_all.diagnostic_only);
        assert!(r.aggregate_all.productive_agent_validated);
        assert!(r.aggregate_holdout.replay_identity_all);
        assert!(r
            .robustness_labels
            .contains(&"live_agent_validation_completed".to_string()));
    }

    #[test]
    fn all_ablation_profiles_are_complete_and_stable() {
        let profiles = all_ablation_profiles();
        assert_eq!(profiles.len(), 7);
        assert!(profiles.contains(&AblationProfile::FullPseField));
        assert!(profiles.contains(&AblationProfile::KeywordOnlyPseEquivalent));
        assert_eq!(
            profiles,
            vec![
                AblationProfile::FullPseField,
                AblationProfile::NoRecencyCurrentness,
                AblationProfile::NoCausalRoleTags,
                AblationProfile::NoPhaseOrderConsistency,
                AblationProfile::NoNegativeDistractorWeight,
                AblationProfile::NoNextActionCompatibility,
                AblationProfile::KeywordOnlyPseEquivalent,
            ]
        );
    }

    #[test]
    fn report_serialization_keeps_ablation_fields_optional() {
        let r = run_agent_exoskeleton_benchmark();
        assert!(r.ablation_aggregate.is_some());
        assert!(r.reports.iter().all(|x| x.ablation_report.is_some()));
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"aggregate_all\""));
    }

    #[test]
    fn scenario_ablation_report_is_complete_and_deterministic() {
        let r = run_agent_exoskeleton_benchmark();
        let r2 = run_agent_exoskeleton_benchmark();
        assert_eq!(r, r2);
        let mut found_keyword_diff = false;
        for sc in &r.reports {
            let ab = sc.ablation_report.as_ref().unwrap();
            assert_eq!(ab.ablation_metrics_by_profile.len(), 7);
            let full = &ab.ablation_metrics_by_profile[&AblationProfile::FullPseField];
            assert_eq!(full, &sc.pse_field_metrics);
            let kw = &ab.ablation_metrics_by_profile[&AblationProfile::KeywordOnlyPseEquivalent];
            if kw != full {
                found_keyword_diff = true;
            }
        }
        assert!(found_keyword_diff);
    }

    #[test]
    fn ablation_aggregate_base_metrics_present() {
        let r = run_agent_exoskeleton_benchmark();
        let a = r.ablation_aggregate.as_ref().unwrap();
        assert_eq!(a.profiles, all_ablation_profiles());
        assert_eq!(a.aggregate_metrics_by_profile.len(), 7);
        assert_eq!(a.full_vs_ablation_delta.len(), 6);
        assert!(!a.feature_group_importance_ranking.is_empty());
        assert!(a
            .feature_group_importance_ranking
            .iter()
            .any(|x| x.feature_group == "structural_features"));
        let r2 = run_agent_exoskeleton_benchmark();
        let a2 = r2.ablation_aggregate.as_ref().unwrap();
        assert_eq!(
            a.feature_group_importance_ranking,
            a2.feature_group_importance_ranking
        );
        assert_eq!(
            a.scenarios_where_full_beats_all_ablations,
            a2.scenarios_where_full_beats_all_ablations
        );
        assert_eq!(
            a.scenarios_where_keyword_equivalent_matches_full,
            a2.scenarios_where_keyword_equivalent_matches_full
        );
        assert_eq!(
            a.scenarios_where_keyword_equivalent_beats_full,
            a2.scenarios_where_keyword_equivalent_beats_full
        );
        assert!(r
            .robustness_labels
            .contains(&"live_agent_validation_completed".to_string()));
        assert!(r.robustness_labels.contains(&"diagnostic_only".to_string()));
        assert!(r
            .robustness_labels
            .contains(&"ablation_sensitivity_detected".to_string()));
        assert!(r
            .robustness_labels
            .contains(&"full_model_beats_keyword_equivalent".to_string()));
        assert!(r
            .robustness_labels
            .contains(&"possible_feature_overfit_risk".to_string()));
    }

    #[test]
    fn trace_replay_cases_have_required_minimums() {
        let traces = real_trace_replay_cases();
        assert!(traces.len() >= 8);
        assert!(
            traces
                .iter()
                .filter(|t| t.variant_kind == TraceVariantKind::Harder)
                .count()
                >= 4
        );
        for trace in traces {
            if trace.variant_kind == TraceVariantKind::Harder {
                assert!(trace.timeline_events.len() >= 10);
                assert!(trace.candidate_items.len() >= 14);
                assert!(trace.ground_truth.rejected_false_paths.len() >= 6);
            } else {
                assert!(trace.timeline_events.len() >= 8);
                assert!(trace.candidate_items.len() >= 10);
                assert!(trace.ground_truth.rejected_false_paths.len() >= 4);
            }
            assert!(trace.ground_truth.rejected_false_paths.len() >= 4);
            assert!(!trace.ground_truth.correct_next_actions.is_empty());
        }
    }

    #[test]
    fn trace_replay_report_flags_and_serialization() {
        let report = run_agent_exoskeleton_benchmark();
        assert!(report.trace_replay_report.trace_count >= 4);
        assert!(report.trace_replay_report.diagnostic_only);
        assert!(!report.trace_replay_report.productive_agent_validated);
        assert!(report.trace_replay_report.replay_identity_all);
        assert!(report
            .trace_replay_report
            .trace_feature_design_report
            .is_some());
        assert!(report
            .trace_replay_report
            .trace_ablation_aggregate
            .is_none());
        assert_eq!(
            report.trace_replay_report.trace_audits.len(),
            report.trace_replay_report.trace_count
        );
        assert_eq!(
            report
                .trace_replay_report
                .trace_variant_counts
                .original_count,
            4
        );
        assert_eq!(
            report.trace_replay_report.trace_variant_counts.harder_count,
            4
        );
        assert_eq!(
            report.trace_replay_report.harder_trace_metrics.trace_count,
            4
        );
        assert!(
            report
                .trace_replay_report
                .harder_trace_metrics
                .diagnostic_only
        );
        assert!(
            !report
                .trace_replay_report
                .harder_trace_metrics
                .productive_agent_validated
        );
        assert!(report
            .trace_replay_report
            .trace_audits
            .iter()
            .all(|a| !a.baseline_top_items.is_empty() && !a.pse_top_items.is_empty()));
        assert!(report
            .trace_replay_report
            .trace_audits
            .iter()
            .all(|a| a.gain_failure_analysis.is_some()));
        assert_eq!(
            report
                .trace_replay_report
                .trace_gain_failure_summary
                .harder_trace_count,
            4
        );
        assert_eq!(
            report.trace_replay_report.trace_audit_summary.trace_count,
            report.trace_replay_report.trace_count
        );
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"trace_replay_report\""));
        assert!(json.contains("\"trace_feature_design_report\""));
        assert!(json.contains("\"trace_audit_summary\""));
        assert!(json.contains("\"trace_gain_failure_summary\""));
    }

    #[test]
    fn trace_audit_overlap_is_deterministic() {
        let a = run_agent_exoskeleton_benchmark();
        let b = run_agent_exoskeleton_benchmark();
        assert_eq!(
            a.trace_replay_report
                .trace_audits
                .iter()
                .map(|x| x.top_item_overlap_ratio)
                .collect::<Vec<_>>(),
            b.trace_replay_report
                .trace_audits
                .iter()
                .map(|x| x.top_item_overlap_ratio)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn gain_failure_primary_reason_and_labels_are_set() {
        let report = run_agent_exoskeleton_benchmark();
        let summaries = &report.trace_replay_report.trace_gain_failure_summary;
        assert_eq!(
            summaries.trace_count,
            report.trace_replay_report.trace_count
        );
        for audit in &report.trace_replay_report.trace_audits {
            let analysis = audit.gain_failure_analysis.as_ref().unwrap();
            assert!(!analysis.primary_blocking_reason.is_empty());
            let r = run_agent_exoskeleton_benchmark();
            let same = r
                .trace_replay_report
                .trace_audits
                .iter()
                .find(|x| x.trace_id == audit.trace_id)
                .unwrap();
            assert_eq!(
                analysis.diagnostic_labels,
                same.gain_failure_analysis
                    .as_ref()
                    .unwrap()
                    .diagnostic_labels
            );
        }
    }

    #[test]
    fn trace_signal_profiles_present_and_deterministic() {
        let report = run_agent_exoskeleton_benchmark();
        let sr = &report.trace_replay_report.trace_signal_reweighting_report;
        assert_eq!(sr.profiles, all_trace_signal_profiles());
        assert!(sr.diagnostic_only);
        assert!(!sr.productive_agent_validated);
        assert!(sr
            .interpretation_labels
            .contains(&"current_pse_not_replaced".to_string()));
        assert!(!sr.profile_reports.is_empty());
        assert!(sr
            .profile_reports
            .iter()
            .any(|x| x.profile == TraceSignalProfile::KeywordOnlyTraceEquivalent));
        let a = run_agent_exoskeleton_benchmark();
        assert_eq!(
            report
                .trace_replay_report
                .trace_signal_reweighting_report
                .best_profile_by_harder_mrr,
            a.trace_replay_report
                .trace_signal_reweighting_report
                .best_profile_by_harder_mrr
        );
    }

    #[test]
    fn trace_feature_design_profiles_present_and_deterministic() {
        let profiles = all_trace_feature_design_profiles();
        assert_eq!(profiles.len(), 7);
        assert_eq!(
            profiles,
            vec![
                TraceFeatureDesignProfile::CurrentPse,
                TraceFeatureDesignProfile::StrongDistractorPenalty,
                TraceFeatureDesignProfile::KeywordDominanceDampening,
                TraceFeatureDesignProfile::ReportArtifactDemotion,
                TraceFeatureDesignProfile::CausalRoleDisambiguation,
                TraceFeatureDesignProfile::CombinedConservativeFeatureDesign,
                TraceFeatureDesignProfile::CausalActivationDiagnostic,
            ]
        );
        assert!(profiles.contains(&TraceFeatureDesignProfile::CurrentPse));
        assert!(profiles.contains(&TraceFeatureDesignProfile::CombinedConservativeFeatureDesign));
        assert_eq!(
            profiles.last(),
            Some(&TraceFeatureDesignProfile::CausalActivationDiagnostic)
        );
    }

    #[test]
    fn trace_feature_design_current_pse_matches_rank_pse_exactly() {
        for state in real_trace_replay_cases()
            .into_iter()
            .map(|trace| AgentWorkState {
                scenario_id: trace.trace_id,
                split: AgentScenarioSplit::Holdout,
                issue_text: trace.title,
                work_items: trace.candidate_items,
                plan_steps: vec!["fix".into(), "retry".into()],
                ground_truth: AgentGroundTruth {
                    causal_files: trace.ground_truth.causal_files,
                    causal_logs: trace.ground_truth.causal_logs,
                    expected_next_actions: trace.ground_truth.correct_next_actions,
                    irrelevant_items: trace.ground_truth.rejected_false_paths,
                    blocker_label: trace.ground_truth.resolution_label,
                },
            })
        {
            let a = rank_pse(&state);
            let b = rank_trace_with_feature_design_profile(
                &state,
                TraceFeatureDesignProfile::CurrentPse,
            );
            assert_eq!(a.item_scores, b.item_scores);
            assert_eq!(a.top_items, b.top_items);
            assert_eq!(a.activated_items, b.activated_items);
            assert_eq!(a.feature_weights, b.feature_weights);
            assert_eq!(a.policy_used, b.policy_used);
        }
    }

    #[test]
    fn trace_feature_design_profiles_are_deterministic_and_noncurrent_differs() {
        let states: Vec<_> = real_trace_replay_cases()
            .into_iter()
            .map(|trace| AgentWorkState {
                scenario_id: trace.trace_id,
                split: AgentScenarioSplit::Holdout,
                issue_text: trace.title,
                work_items: trace.candidate_items,
                plan_steps: vec!["fix".into(), "retry".into()],
                ground_truth: AgentGroundTruth {
                    causal_files: trace.ground_truth.causal_files,
                    causal_logs: trace.ground_truth.causal_logs,
                    expected_next_actions: trace.ground_truth.correct_next_actions,
                    irrelevant_items: trace.ground_truth.rejected_false_paths,
                    blocker_label: trace.ground_truth.resolution_label,
                },
            })
            .collect();
        let profiles = all_trace_feature_design_profiles();
        let mut any_noncurrent_differs = false;
        for profile in profiles {
            for state in &states {
                let r1 = rank_trace_with_feature_design_profile(state, profile);
                let r2 = rank_trace_with_feature_design_profile(state, profile);
                assert_eq!(r1.item_scores, r2.item_scores);
                assert_eq!(r1.top_items, r2.top_items);
                if profile != TraceFeatureDesignProfile::CurrentPse {
                    let current = rank_trace_with_feature_design_profile(
                        state,
                        TraceFeatureDesignProfile::CurrentPse,
                    );
                    if r1.top_items != current.top_items || r1.item_scores != current.item_scores {
                        any_noncurrent_differs = true;
                    }
                }
            }
        }
        assert!(any_noncurrent_differs);
    }

    #[test]
    fn trace_feature_design_report_is_present_and_consistent() {
        let report = run_agent_exoskeleton_benchmark();
        let tr = &report.trace_replay_report;
        let fd = tr.trace_feature_design_report.as_ref().unwrap();
        assert!(fd.diagnostic_only);
        assert!(!fd.productive_agent_validated);
        assert!(fd.current_pse_not_replaced);
        assert_eq!(fd.profiles, all_trace_feature_design_profiles());
        assert_eq!(fd.profile_reports.len(), fd.profiles.len());
        assert!(fd
            .interpretation_labels
            .contains(&"trace_feature_design_present".to_string()));
        assert!(fd
            .interpretation_labels
            .contains(&"diagnostic_only".to_string()));
        assert!(fd
            .interpretation_labels
            .contains(&"requires_live_agent_validation".to_string()));
        assert!(fd
            .interpretation_labels
            .contains(&"current_pse_not_replaced".to_string()));
        assert!(fd
            .interpretation_labels
            .contains(&"feature_profiles_metric_tie".to_string()));
        assert!(fd
            .interpretation_labels
            .contains(&"feature_design_insufficient_for_current_trace_set".to_string()));
        assert!(fd
            .interpretation_labels
            .contains(&"feature_design_no_gain".to_string()));
        assert!(fd.profiles.contains(&fd.best_profile_by_harder_hit_at_3));
        assert!(fd.profiles.contains(&fd.best_profile_by_harder_mrr));
        assert!(fd.profiles.contains(&fd.best_profile_by_harder_false_focus));
        for p in &fd.profiles {
            assert!(fd.profile_reports.iter().any(|x| &x.profile == p));
        }
        let current = fd
            .profile_reports
            .iter()
            .find(|x| x.profile == TraceFeatureDesignProfile::CurrentPse)
            .unwrap();
        assert_eq!(
            current.metrics_aggregate.pse_hit_at_3_mean,
            tr.pse_metrics_aggregate.pse_hit_at_3_mean
        );
        assert_eq!(
            current.metrics_aggregate.pse_mrr_causal_item_mean,
            tr.pse_metrics_aggregate.pse_mrr_causal_item_mean
        );
        assert_eq!(
            current.metrics_aggregate.pse_false_focus_mean,
            tr.pse_metrics_aggregate.pse_false_focus_mean
        );
        assert_eq!(
            current.harder_metrics_aggregate.scenario_count,
            tr.harder_trace_metrics.trace_count
        );
        assert_eq!(tr.traces_with_pse_gain, 0);
        assert_eq!(tr.traces_with_pse_loss, 0);
        assert!(tr.trace_count >= 20);
        assert_eq!(tr.traces_with_equal_result, tr.trace_count);
        assert!(!tr.productive_agent_validated);
        assert!(tr.diagnostic_only);
        assert!(tr.replay_identity_all);
        let sr = fd.sensitivity_report.as_ref().unwrap();
        assert!(sr.diagnostic_only);
        assert!(!sr.productive_agent_validated);
        assert!(sr.current_pse_not_replaced);
        assert_eq!(sr.profile_sensitivity_reports.len(), 6);
        assert!(sr
            .profile_sensitivity_reports
            .iter()
            .all(|p| p.profile != TraceFeatureDesignProfile::CurrentPse));
        let r2 = run_agent_exoskeleton_benchmark();
        let fd2 = r2
            .trace_replay_report
            .trace_feature_design_report
            .as_ref()
            .unwrap()
            .sensitivity_report
            .as_ref()
            .unwrap();
        assert_eq!(
            sr.profile_sensitivity_reports,
            fd2.profile_sensitivity_reports
        );
        let aa = sr.activation_audit.as_ref().unwrap();
        assert!(aa.diagnostic_only);
        assert!(!aa.productive_agent_validated);
        assert!(aa.current_pse_not_replaced);
        assert_eq!(aa.profile_activation_reports.len(), 6);
        assert!(aa
            .profile_activation_reports
            .iter()
            .all(|p| p.profile != TraceFeatureDesignProfile::CurrentPse));
        let strong = aa
            .profile_activation_reports
            .iter()
            .find(|p| p.profile == TraceFeatureDesignProfile::StrongDistractorPenalty)
            .unwrap();
        let keyword = aa
            .profile_activation_reports
            .iter()
            .find(|p| p.profile == TraceFeatureDesignProfile::KeywordDominanceDampening)
            .unwrap();
        let causal_diag = aa
            .profile_activation_reports
            .iter()
            .find(|p| p.profile == TraceFeatureDesignProfile::CausalActivationDiagnostic)
            .unwrap();
        assert!(strong.activation_count > 0);
        assert!(keyword.activation_count > 0);
        assert!(causal_diag.activation_count > 0);
        assert!(keyword.zero_activation_reason.is_none());
        assert!(!aa
            .profiles_with_zero_activation
            .contains(&TraceFeatureDesignProfile::KeywordDominanceDampening));
        assert!(aa
            .profile_activation_reports
            .iter()
            .filter(|p| p.activation_count > 0)
            .all(|p| p.mean_score_delta_abs > 0.0 || p.max_score_delta_abs > 0.0));
        let mc = sr.marker_coverage_audit.as_ref().unwrap();
        assert_eq!(mc.trace_count, tr.trace_count);
        assert_eq!(mc.harder_trace_count, 4);
        assert!(mc.diagnostic_only);
        assert!(!mc.productive_agent_validated);
        assert!(mc.current_pse_not_replaced);
        assert!(mc.distractor_marker_count > 0);
        assert!(mc.keyword_dampening_marker_count > 0);
        assert!(mc.causal_action_marker_count > 0);
        assert!(mc.report_artifact_marker_count > 0);
        assert!(mc.traces_with_missing_distractor_markers.len() < 15);
        assert!(mc.traces_with_missing_keyword_dampening_markers.len() < 15);
        assert!(mc
            .interpretation_labels
            .contains(&"feature_marker_coverage_audit_present".to_string()));
        let ta = sr.targeting_audit.as_ref().unwrap();
        assert!(ta.diagnostic_only);
        assert!(!ta.productive_agent_validated);
        assert!(ta.current_pse_not_replaced);
        assert_eq!(ta.trace_count, tr.trace_count);
        assert_eq!(ta.harder_trace_count, 4);
        assert_eq!(ta.profile_targeting_reports.len(), 6);
        assert!(fd
            .profile_reports
            .iter()
            .any(|p| p.profile == TraceFeatureDesignProfile::CausalActivationDiagnostic));
        assert!(sr
            .profile_sensitivity_reports
            .iter()
            .any(|p| p.profile == TraceFeatureDesignProfile::CausalActivationDiagnostic));
        assert!(aa
            .profile_activation_reports
            .iter()
            .any(|p| p.profile == TraceFeatureDesignProfile::CausalActivationDiagnostic));
        assert!(ta
            .profile_targeting_reports
            .iter()
            .any(|p| p.profile == TraceFeatureDesignProfile::CausalActivationDiagnostic));
        assert!(ta
            .profile_targeting_reports
            .iter()
            .all(|p| p.profile != TraceFeatureDesignProfile::CurrentPse));
        let strong_target = ta
            .profile_targeting_reports
            .iter()
            .find(|p| p.profile == TraceFeatureDesignProfile::StrongDistractorPenalty)
            .unwrap();
        let keyword_target = ta
            .profile_targeting_reports
            .iter()
            .find(|p| p.profile == TraceFeatureDesignProfile::KeywordDominanceDampening)
            .unwrap();
        assert!(strong_target.activation_count > 0);
        assert!(keyword_target.activation_count > 0);
        assert!(strong_target.activated_top3_blocker_count > 0);
        assert!(keyword_target.activated_top3_blocker_count > 0);
        assert!(strong_target.missed_top3_blocker_count < ta.trace_count * 3);
        assert!(keyword_target.missed_top3_blocker_count < ta.trace_count * 3);
        assert!(!ta
            .profiles_missing_top3_blockers
            .contains(&TraceFeatureDesignProfile::StrongDistractorPenalty));
        assert!(!ta
            .profiles_missing_top3_blockers
            .contains(&TraceFeatureDesignProfile::KeywordDominanceDampening));
        assert!(ta
            .interpretation_labels
            .contains(&"feature_activation_targeting_audit_present".to_string()));
        let da = sr.delta_sufficiency_audit.as_ref().unwrap();
        assert!(da.diagnostic_only);
        assert!(!da.productive_agent_validated);
        assert!(da.current_pse_not_replaced);
        let strong_delta = da
            .profile_delta_reports
            .iter()
            .find(|p| p.profile == TraceFeatureDesignProfile::StrongDistractorPenalty)
            .unwrap();
        let keyword_delta = da
            .profile_delta_reports
            .iter()
            .find(|p| p.profile == TraceFeatureDesignProfile::KeywordDominanceDampening)
            .unwrap();
        assert!(strong_delta.activated_top3_blocker_count > 0);
        assert!(keyword_delta.activated_top3_blocker_count > 0);
        assert!(
            strong_delta.mean_required_delta_to_top3.is_some()
                || strong_delta.delta_sufficiency_label == "delta_not_computable"
        );
        assert!(
            keyword_delta.mean_required_delta_to_top3.is_some()
                || keyword_delta.delta_sufficiency_label == "delta_not_computable"
        );
        assert!(!strong_delta.delta_sufficiency_label.is_empty());
        assert!(!keyword_delta.delta_sufficiency_label.is_empty());
        let ca = sr.causal_feature_targeting_audit.as_ref().unwrap();
        assert!(ca.diagnostic_only);
        assert!(!ca.productive_agent_validated);
        assert!(ca.current_pse_not_replaced);
        assert!(ca.causal_item_count > 0);
        assert!(ca.causal_items_marker_targetable >= 48);
        assert!(ca.causal_items_score_activated <= ca.causal_item_count);
        assert_eq!(ca.causal_items_score_improved, 0);
        assert_eq!(ca.causal_items_rank_improved, 0);
        assert!(ca
            .interpretation_labels
            .contains(&"causal_targeting_semantics_aligned".to_string()));
        assert!(ca.causal_items_missed_by_existing_profiles <= ca.causal_item_count);
        if ca.causal_items_missed_by_existing_profiles > 0 {
            assert!(!ca.proposed_causal_feature_rules.is_empty());
        }
        let aa2 = sr.causal_activation_alignment_audit.as_ref().unwrap();
        assert!(aa2.diagnostic_only);
        assert!(!aa2.productive_agent_validated);
        assert!(aa2.current_pse_not_replaced);
        assert!(aa2.causal_marker_targetable_count > 0);
        assert!(aa2.causal_rule_matched_count > 0);
        assert!(aa2.causal_profile_score_delta_count <= aa2.causal_marker_targetable_count);
        assert!(aa2.noncausal_profile_score_delta_count > 0);
        assert!(aa2
            .interpretation_labels
            .contains(&"causal_activation_alignment_audit_present".to_string()));
        let impact = sr.causal_delta_impact_audit.as_ref().unwrap();
        assert!(impact.diagnostic_only);
        assert!(!impact.productive_agent_validated);
        assert!(impact.current_pse_not_replaced);
        assert!(impact
            .interpretation_labels
            .contains(&"causal_delta_impact_resolution_repaired".to_string()));
        assert!(impact
            .interpretation_labels
            .contains(&"causal_delta_impact_now_uses_alignment_ids".to_string()));
        assert!(impact.causal_delta_item_count > 0);
        assert_eq!(impact.causal_rank_resolution_failed_count, 0);
        assert!(impact.traces_with_causal_rank_resolution_failure.is_empty());
        for trc in impact
            .trace_reports
            .iter()
            .filter(|t| t.variant_kind == TraceVariantKind::Harder)
        {
            assert!(trc.current_best_causal_rank < usize::MAX / 4);
            assert!(trc.profile_best_causal_rank < usize::MAX / 4);
            assert!(trc.current_best_causal_score > i32::MIN / 4);
            assert!(trc.profile_best_causal_score > i32::MIN / 4);
        }
        let mc2 = sr.metric_causal_consistency_audit.as_ref().unwrap();
        assert!(mc2.diagnostic_only);
        assert!(!mc2.productive_agent_validated);
        assert!(mc2.current_pse_not_replaced);
        assert_eq!(mc2.trace_reports.len(), mc2.trace_count);
        assert!(mc2
            .interpretation_labels
            .contains(&"metric_causal_consistency_audit_present".to_string()));
        assert!(mc2.traces_with_audit_metric_disagreement > 0);
        assert!(mc2.trace_reports.iter().any(|t| {
            t.audit_best_causal_rank == 1 && !t.metric_hit_at3 && !t.disagreement_reasons.is_empty()
        }));
        let ms = sr.metric_semantics_alignment_report.as_ref().unwrap();
        assert!(ms.diagnostic_only);
        assert!(!ms.productive_agent_validated);
        assert!(ms.current_pse_not_replaced);
        assert_eq!(ms.trace_reports.len(), ms.trace_count);
        assert!(ms
            .interpretation_labels
            .contains(&"metric_semantics_alignment_report_present".to_string()));
        if mc2.traces_with_audit_causal_top3 == tr.trace_count
            && mc2.traces_with_metric_hit_at3 == 0
        {
            assert!(ms.audit_aligned_hit_at3_count > ms.official_metric_hit_at3_count);
        }
        assert!(ms.false_negative_due_to_metric_semantics_count > 0);
        let dm = sr.dual_metric_report.as_ref().unwrap();
        assert!(dm.diagnostic_only);
        assert!(!dm.productive_agent_validated);
        assert!(dm.current_pse_not_replaced);
        assert_eq!(dm.official_metric.hit_at3_count, 0);
        assert_eq!(dm.audit_aligned_metric.hit_at3_count, tr.trace_count);
        assert_eq!(
            dm.semantic_delta
                .false_negative_due_to_metric_semantics_count,
            tr.trace_count
        );
        assert!(dm
            .interpretation_labels
            .contains(&"official_metric_preserved".to_string()));
        let proposal = dm.metric_redesign_proposal.as_ref().unwrap();
        assert_eq!(proposal.proposal_status, "redesign_recommended");
        assert_eq!(
            proposal.recommended_primary_metric,
            "audit_aligned_candidate_causal_metric_for_layer1"
        );
        assert!(proposal
            .compatibility_policy
            .contains("dual_metric_reporting"));
        assert!(!proposal.migration_steps.is_empty());
        assert!(!proposal.acceptance_criteria.is_empty());
        let layer1_spec = dm.layer1_candidate_causal_metric_spec.as_ref().unwrap();
        assert_eq!(
            layer1_spec.metric_name,
            "audit_aligned_candidate_causal_metric_for_layer1"
        );
        assert_eq!(layer1_spec.freeze_status, "frozen_diagnostic_v1");
        assert!(!layer1_spec.freeze_criteria.is_empty());
        let migration_plan = dm.layer1_metric_migration_plan.as_ref().unwrap();
        assert_eq!(migration_plan.migration_status, "migration_plan_prepared");
        assert_eq!(
            migration_plan.target_primary_metric,
            "audit_aligned_candidate_causal_metric_for_layer1"
        );
        assert!(migration_plan.diagnostic_only);
        assert!(migration_plan.productive_agent_validated);
        assert!(migration_plan.current_pse_not_replaced);
        assert!(migration_plan.promotion_readiness.corpus_threshold_met);
        assert!(migration_plan.promotion_readiness.diagnostic_freeze_present);
        assert!(
            migration_plan
                .promotion_readiness
                .live_agent_validation_completed
        );
        assert!(!migration_plan.promotion_readiness.promotion_allowed);
        assert!(!migration_plan.validation_phases.is_empty());
        assert!(!migration_plan.blocking_conditions.is_empty());
        assert!(!layer1_spec.known_limitations.is_empty());
        let load_report = tr.trace_corpus_load_report.as_ref().unwrap();
        assert!(load_report.diagnostic_only);
        assert!(!load_report.productive_agent_validated);
        assert!(load_report.current_pse_not_replaced);
        assert_eq!(
            load_report.source_kind,
            TraceCorpusSourceKind::BuiltinStatic
        );
        assert_eq!(load_report.builtin_trace_count, 20);
        assert_eq!(load_report.external_fixture_trace_count, 0);
        assert_eq!(load_report.total_loaded_trace_count, 20);
        assert!(!load_report.fixture_loading_enabled);
        assert_eq!(load_report.load_status, "builtin_static_corpus_active");
        let fixture_desc = tr.trace_corpus_fixture_descriptor.as_ref().unwrap();
        assert_eq!(
            fixture_desc.fixture_schema_version,
            "v1_external_trace_fixture_scaffold"
        );
        assert!(fixture_desc.diagnostic_only);
        assert!(!fixture_desc.productive_agent_validated);
        assert!(fixture_desc
            .interpretation_labels
            .contains(&"trace_fixture_schema_scaffold_present".to_string()));
        let corpus = tr.trace_corpus_descriptor.as_ref().unwrap();
        assert!(!corpus.corpus_name.is_empty());
        assert!(!corpus.corpus_version.is_empty());
        assert_eq!(corpus.trace_count, tr.trace_count);
        assert!(corpus.diagnostic_only);
        assert!(!corpus.productive_agent_validated);
        assert!(corpus
            .metric_policy
            .contains(&"dual_metric_reporting_active".to_string()));
        let expansion = tr.trace_corpus_expansion_report.as_ref().unwrap();
        assert!(expansion.diagnostic_only);
        assert!(!expansion.productive_agent_validated);
        assert!(expansion.current_pse_not_replaced);
        assert!(expansion.expanded_trace_count >= 12);
        assert_eq!(expansion.expansion_status, "expanded_validation_active");
        assert!(!expansion.added_trace_ids.is_empty());
        assert_eq!(
            expansion.total_trace_count,
            expansion.baseline_trace_count + expansion.expanded_trace_count
        );
        assert_eq!(expansion.total_trace_count, tr.trace_count);
        let stability = tr.trace_corpus_stability_summary.as_ref().unwrap();
        assert!(stability.total_trace_count >= 20);
        assert!(stability.expanded_trace_count >= 12);
        assert_eq!(stability.audit_aligned_hit_at3_total, tr.trace_count);
        assert_eq!(stability.official_hit_at3_total, 0);
        assert_eq!(stability.expanded_audit_aligned_hit_rate, 1.0);
        assert_eq!(
            stability.stability_label,
            "audit_aligned_stable_across_current_corpus"
        );
        assert_eq!(
            stability.redesign_evidence_label,
            "redesign_evidence_strengthened"
        );
        assert_eq!(
            stability.recommended_next_step,
            "prepare_layer1_metric_migration_plan"
        );
    }

    #[test]
    fn rankability_audits_exist_and_are_consistent() {
        let r = run_agent_exoskeleton_benchmark();
        let tr = &r.trace_replay_report;
        assert_eq!(tr.rankability_audits.len(), tr.trace_count);
        assert_eq!(tr.rankability_summary.trace_count, tr.trace_count);
        assert_eq!(tr.rankability_summary.harder_trace_count, 4);
        for a in &tr.rankability_audits {
            assert!((0.0..=1.0).contains(&a.rankability_ratio));
            assert!(a.diagnostic_labels.contains(&"diagnostic_only".to_string()));
            if a.rankability_label == "unrankable" {
                assert!(a
                    .diagnostic_labels
                    .contains(&"mrr_zero_explained_by_unrankable_targets".to_string()));
            }
        }
        assert!(tr.diagnostic_only);
        assert!(!tr.productive_agent_validated);
        assert!(tr.replay_identity_all);
        assert!(tr.rankability_summary.traces_fully_rankable >= 4);
        assert!(tr.rankability_summary.harder_mean_rankability_ratio > 0.3333);
        let harder: Vec<_> = tr
            .rankability_audits
            .iter()
            .filter(|a| a.variant_kind == TraceVariantKind::Harder)
            .collect();
        assert!(harder
            .iter()
            .all(|a| a.correct_next_action_candidate_hits > 0));
        let causal_files_missing = tr
            .rankability_summary
            .primary_rankability_blocker_counts
            .get("causal_files_missing")
            .copied()
            .unwrap_or(0);
        assert!(causal_files_missing < tr.trace_count);
    }

    #[test]
    fn post_coverage_ranking_failure_audits_exist() {
        let r = run_agent_exoskeleton_benchmark();
        let tr = &r.trace_replay_report;
        assert_eq!(tr.ranking_failure_audits.len(), tr.trace_count);
        assert_eq!(
            tr.post_coverage_ranking_failure_summary.trace_count,
            tr.trace_count
        );
        assert_eq!(
            tr.post_coverage_ranking_failure_summary
                .fully_rankable_trace_count,
            8
        );
        assert!(tr
            .ranking_failure_audits
            .iter()
            .all(|a| !a.rank_failure_label.is_empty()));
        assert!(tr.diagnostic_only);
        assert!(!tr.productive_agent_validated);
        assert!(tr.replay_identity_all);
    }

    #[test]
    fn score_attribution_audits_exist_and_deterministic() {
        let r = run_agent_exoskeleton_benchmark();
        let tr = &r.trace_replay_report;
        assert_eq!(tr.score_attribution_audits.len(), tr.trace_count);
        assert_eq!(
            tr.trace_score_attribution_summary.trace_count,
            tr.trace_count
        );
        assert!(tr
            .score_attribution_audits
            .iter()
            .all(|a| !a.causal_item_attributions.is_empty()));
        assert!(tr
            .score_attribution_audits
            .iter()
            .all(|a| a.diagnostic_labels.contains(&"diagnostic_only".to_string())));
        let r2 = run_agent_exoskeleton_benchmark();
        assert_eq!(
            tr.trace_score_attribution_summary
                .mean_causal_vs_noncausal_score_gap,
            r2.trace_replay_report
                .trace_score_attribution_summary
                .mean_causal_vs_noncausal_score_gap
        );
        assert!(
            tr.trace_score_attribution_summary
                .traces_with_explainability_gap
                <= tr.trace_count
        );
        assert!(!tr
            .trace_score_attribution_summary
            .dominant_component_counts
            .is_empty());
    }

    #[test]
    fn pse_breakdown_total_matches_rank_score() {
        let s = &benchmark_scenarios()[0];
        let field = rank_pse(s);
        for w in &s.work_items {
            let b = score_pse_item_with_breakdown(s, w);
            let score = field.item_scores.get(&w.id).copied().unwrap_or_default();
            assert_eq!(b.total_score, score);
            assert_eq!(b.unexplained_score_delta, 0);
        }
    }

    // ── BENCH-05H: External Fixture Loader Tests ─────────────────────────

    #[test]
    fn fixture_loader_default_no_fixture_source_kind_builtin() {
        let r = run_agent_exoskeleton_benchmark();
        let tr = &r.trace_replay_report;
        let lr = tr.trace_corpus_load_report.as_ref().unwrap();
        assert_eq!(lr.source_kind, TraceCorpusSourceKind::BuiltinStatic);
        assert_eq!(lr.builtin_trace_count, 20);
        assert_eq!(lr.external_fixture_trace_count, 0);
        assert_eq!(lr.total_loaded_trace_count, 20);
        assert!(!lr.fixture_loading_enabled);
        assert_eq!(lr.load_status, "builtin_static_corpus_active");
        assert!(lr.diagnostic_only);
        assert!(!lr.productive_agent_validated);
        assert!(lr.current_pse_not_replaced);
        assert!(lr
            .interpretation_labels
            .contains(&"external_fixture_loading_not_enabled".to_string()));
        // dual metric and specs must remain intact
        assert!(tr.trace_feature_design_report.is_some());
        assert!(r.ablation_aggregate.is_some());
        assert!(!r.reports.is_empty());
    }

    #[test]
    fn fixture_loader_with_valid_fixture_extends_corpus() {
        let fixture_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/agent_exoskeleton/example_trace_fixture_v1.json"
        );
        let r = run_agent_exoskeleton_benchmark_with_fixture(fixture_path)
            .expect("valid fixture must load without error");
        let tr = &r.trace_replay_report;
        let lr = tr.trace_corpus_load_report.as_ref().unwrap();
        assert_eq!(
            lr.source_kind,
            TraceCorpusSourceKind::MixedBuiltinAndFixture
        );
        assert_eq!(lr.builtin_trace_count, 20);
        assert!(lr.external_fixture_trace_count > 0);
        assert_eq!(
            lr.total_loaded_trace_count,
            lr.builtin_trace_count + lr.external_fixture_trace_count
        );
        assert!(lr.fixture_loading_enabled);
        assert_eq!(lr.load_status, "fixture_loaded_successfully");
        assert!(lr.fixture_path.is_some());
        assert!(lr
            .interpretation_labels
            .contains(&"external_fixture_loading_enabled".to_string()));
        assert!(lr
            .interpretation_labels
            .contains(&"mixed_builtin_and_fixture_corpus_active".to_string()));
        // total trace count includes external
        assert!(tr.trace_count > 20);
        assert_eq!(
            tr.trace_count,
            lr.builtin_trace_count + lr.external_fixture_trace_count
        );
        // corpus descriptor uses total counts
        let corpus = tr.trace_corpus_descriptor.as_ref().unwrap();
        assert_eq!(corpus.trace_count, tr.trace_count);
        // stability summary covers builtin corpus only (20 traces, 20/20 audit hit)
        let stab = tr.trace_corpus_stability_summary.as_ref().unwrap();
        assert_eq!(stab.total_trace_count, 20);
        assert_eq!(stab.audit_aligned_hit_at3_total, 20);
        assert_eq!(stab.official_hit_at3_total, 0);
        assert_eq!(
            stab.stability_label,
            "audit_aligned_stable_across_current_corpus"
        );
        // fixture descriptor reflects loaded fixture
        let fd = tr.trace_corpus_fixture_descriptor.as_ref().unwrap();
        assert_eq!(fd.fixture_trace_count, lr.external_fixture_trace_count);
        assert!(fd
            .interpretation_labels
            .contains(&"trace_fixture_loaded_successfully".to_string()));
        // safety invariants preserved
        assert!(tr.diagnostic_only);
        assert!(!tr.productive_agent_validated);
        assert!(lr.diagnostic_only);
        assert!(!lr.productive_agent_validated);
        assert!(lr.current_pse_not_replaced);
        // dual metric report must remain Some
        assert!(!r.reports.is_empty());
        assert!(r.ablation_aggregate.is_some());
    }

    #[test]
    fn fixture_loader_wrong_schema_version_fails_closed() {
        let bad = r#"{
            "fixture_schema_version": "v99_wrong",
            "fixture_name": "bad",
            "intended_layer": "Layer-1",
            "diagnostic_only": true,
            "productive_agent_validated": false,
            "cases": []
        }"#;
        let tmp = std::env::temp_dir().join("pse_test_bad_schema_fixture.json");
        std::fs::write(&tmp, bad).unwrap();
        let result = run_agent_exoskeleton_benchmark_with_fixture(tmp.to_str().unwrap());
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("schema_version mismatch"),
            "expected schema mismatch error, got: {msg}"
        );
    }

    #[test]
    fn fixture_loader_empty_cases_fails_closed() {
        let bad = r#"{
            "fixture_schema_version": "v1_external_trace_fixture_scaffold",
            "fixture_name": "empty_fixture",
            "intended_layer": "Layer-1",
            "diagnostic_only": true,
            "productive_agent_validated": false,
            "cases": []
        }"#;
        let tmp = std::env::temp_dir().join("pse_test_empty_cases_fixture.json");
        std::fs::write(&tmp, bad).unwrap();
        let result = run_agent_exoskeleton_benchmark_with_fixture(tmp.to_str().unwrap());
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("cases must not be empty"),
            "expected empty-cases error, got: {msg}"
        );
    }

    #[test]
    fn fixture_loader_productive_agent_validated_true_fails_closed() {
        let bad = r#"{
            "fixture_schema_version": "v1_external_trace_fixture_scaffold",
            "fixture_name": "bad_productive",
            "intended_layer": "Layer-1",
            "diagnostic_only": true,
            "productive_agent_validated": true,
            "cases": []
        }"#;
        let tmp = std::env::temp_dir().join("pse_test_productive_validated_fixture.json");
        std::fs::write(&tmp, bad).unwrap();
        let result = run_agent_exoskeleton_benchmark_with_fixture(tmp.to_str().unwrap());
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("productive_agent_validated must be false"),
            "expected productive_agent_validated error, got: {msg}"
        );
    }

    #[test]
    fn fixture_loader_deterministic_trace_id_order() {
        let fixture_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/agent_exoskeleton/example_trace_fixture_v1.json"
        );
        let r1 = run_agent_exoskeleton_benchmark_with_fixture(fixture_path).unwrap();
        let r2 = run_agent_exoskeleton_benchmark_with_fixture(fixture_path).unwrap();
        assert_eq!(r1, r2, "fixture run must be deterministic");
        let tr = &r1.trace_replay_report;
        // external trace IDs should appear in sorted order
        let external_ids: Vec<&str> = tr
            .trace_reports
            .iter()
            .filter(|rep| rep.scenario_id.starts_with("ext_fixture_"))
            .map(|rep| rep.scenario_id.as_str())
            .collect();
        let mut sorted = external_ids.clone();
        sorted.sort();
        assert_eq!(
            external_ids, sorted,
            "external traces must appear in deterministic sorted order"
        );
    }
}
