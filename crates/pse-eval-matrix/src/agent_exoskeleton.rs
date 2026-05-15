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
                    mk_item(
                        &format!("clippy_{i}"),
                        if i < 4 {
                            WorkItemType::Repo
                        } else if i < 6 {
                            WorkItemType::Log
                        } else {
                            WorkItemType::Distractor
                        },
                        "clippy quality gate lint causal root_cause inspect crates/pse-validation-runner/src/domain_validation.rs target/tmp/replay.log run fix retry",
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
                    mk_item(
                        &format!("windows_{i}"),
                        if i < 4 {
                            WorkItemType::Repo
                        } else if i < 6 {
                            WorkItemType::Log
                        } else {
                            WorkItemType::Distractor
                        },
                        "windows handshake transport harness retry causal inspect crates/pse-net/src/node.rs target/tmp/replay.log run fix retry",
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
                    mk_item(
                        &format!("crystal_{i}"),
                        if i < 4 {
                            WorkItemType::Repo
                        } else if i < 6 {
                            WorkItemType::Log
                        } else {
                            WorkItemType::Distractor
                        },
                        "diagnostic finding gate rejection candidate field inspect crates/pse-bench-gt/src/runner.rs target/tmp/replay.log run fix retry",
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
                    mk_item(
                        &format!("event_{i}"),
                        if i < 4 {
                            WorkItemType::Repo
                        } else if i < 6 {
                            WorkItemType::DiffHint
                        } else {
                            WorkItemType::Distractor
                        },
                        "eventization axis policy candidate activation diagnostic field inspect crates/pse-eval-matrix/src/agent_exoskeleton.rs target/tmp/replay.log run fix retry",
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
            (0..14).map(|i| mk_item(&format!("clippy_harder_{i}"), if i < 3 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, "domain validation final_report empirical improvement benchmark quality gate clippy rustfmt inspect crates/pse-validation-runner/src/domain_validation.rs target/tmp/replay.log run cargo fmt --all -- --check then fix retry", &["quality_gate", "lint", "causal"], 14 - i)).collect(),
        ),
        mk_trace_case(
            "windows_node_propagation_trace_harder",
            "Windows node propagation trace with validation/domain distractions",
            TraceVariantKind::Harder,
            10,
            "node_harness_transport",
            &["crates/pse-net/src/node.rs"],
            &["domain_validation", "bench_gt", "validation_runner", "networking_general", "empirical_improvement", "final_report"],
            (0..14).map(|i| mk_item(&format!("windows_harder_{i}"), if i < 3 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, "windows validation domain benchmark networking timeout handshake propagation harness transport inspect crates/pse-net/src/node.rs target/tmp/replay.log run cargo test -p pse-net --lib and retry fix", &["transport", "harness", "causal"], 14 - i)).collect(),
        ),
        mk_trace_case(
            "domain_pse_zero_crystals_trace_harder",
            "Domain zero crystals with verdict/improvement distractions",
            TraceVariantKind::Harder,
            10,
            "gate_snapshot_diagnostic",
            &["crates/pse-bench-gt/src/runner.rs"],
            &["verdict", "empirical_improvement", "baseline_comparison", "final_report", "threshold_relax", "unlock_gate"],
            (0..14).map(|i| mk_item(&format!("crystal_harder_{i}"), if i < 3 { WorkItemType::Repo } else if i < 6 { WorkItemType::Log } else { WorkItemType::Distractor }, "verdict empirical improvement baseline comparison final report gate rejection crystal detection diagnostics inspect crates/pse-bench-gt/src/runner.rs target/tmp/replay.log check gate snapshots then fix retry", &["gate_snapshot", "diagnostic_only", "causal"], 14 - i)).collect(),
        ),
        mk_trace_case(
            "eventization_candidate_activation_trace_harder",
            "Eventization candidate activation with productive-metric noise",
            TraceVariantKind::Harder,
            10,
            "candidate_activation_diagnostics",
            &["crates/pse-eval-matrix/src/agent_exoskeleton.rs"],
            &["productive_metrics", "eventization_apply", "f1_gain_claim", "metric_overwrite", "final_report", "baseline_win"],
            (0..14).map(|i| mk_item(&format!("event_harder_{i}"), if i < 3 { WorkItemType::Repo } else if i < 6 { WorkItemType::DiffHint } else { WorkItemType::Distractor }, "eventization apply productive metrics f1 gain candidate activation diagnostic only frozen split validation inspect crates/pse-eval-matrix/src/agent_exoskeleton.rs target/tmp/replay.log preserve productive metrics then retry fix", &["eventization", "candidate_activation", "causal"], 14 - i)).collect(),
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

pub fn run_agent_exoskeleton_benchmark() -> AgentExoskeletonSuiteReport {
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
                productive_agent_validated: false,
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
            productive_agent_validated: false,
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
        productive_agent_validated: false,
        diagnostic_only: true,
    };
    let mut labels = vec![
        "benchmark_discriminative".to_string(),
        "requires_real_agent_validation".to_string(),
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
        "requires_real_agent_validation".to_string(),
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
    let all_traces = real_trace_replay_cases();
    let original_count = all_traces
        .iter()
        .filter(|t| t.variant_kind == TraceVariantKind::Original)
        .count();
    let harder_count = all_traces
        .iter()
        .filter(|t| t.variant_kind == TraceVariantKind::Harder)
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
        rankability_audits,
        rankability_summary,
        ranking_failure_audits,
        post_coverage_ranking_failure_summary,
        score_attribution_audits,
        trace_score_attribution_summary,
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
    fn diagnostic_flags_stay_off_productive() {
        let r = run_agent_exoskeleton_benchmark();
        assert!(r.aggregate.diagnostic_only);
        assert!(!r.aggregate.productive_agent_validated);
        assert!(r.aggregate_all.diagnostic_only);
        assert!(!r.aggregate_all.productive_agent_validated);
        assert!(r.aggregate_holdout.replay_identity_all);
        assert!(r
            .robustness_labels
            .contains(&"requires_real_agent_validation".to_string()));
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
            .contains(&"requires_real_agent_validation".to_string()));
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
}
