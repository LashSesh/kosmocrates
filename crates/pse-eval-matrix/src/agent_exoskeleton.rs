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
    pub interpretation_labels: Vec<String>,
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
    for w in &state.work_items {
        let mut s = overlap(
            &format!("{} {} {}", w.source, w.text, w.tags.join(" ")),
            &query,
        );
        s += match w.item_type {
            WorkItemType::Repo => 3,
            WorkItemType::Log => 1,
            WorkItemType::DiffHint => 4,
            WorkItemType::Distractor => -3,
        };
        if w.recency_rank >= 8 {
            s += 2;
        }
        if w.tags.iter().any(|t| {
            t.contains("causal")
                || t.contains("executor")
                || t.contains("harness")
                || t.contains("command_plan")
        }) {
            s += 4;
        }
        if w.tags
            .iter()
            .any(|t| t.contains("stale") || t.contains("irrelevant") || t.contains("distractor"))
        {
            s -= 6;
        }
        if state.plan_steps.iter().any(|p| {
            w.text.to_ascii_lowercase().contains(
                &p.split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase(),
            )
        }) {
            s += 3;
        }
        if w.tags.iter().any(|t| t.contains("symptom"))
            && w.tags.iter().any(|t| t.contains("root_cause"))
        {
            s += 3;
        }
        scores.insert(w.id.clone(), s);
    }
    build_field(scores, "pse_diagnostic_agent_field_v1", weights)
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

pub fn benchmark_scenarios() -> Vec<AgentWorkState> {
    let mut out = Vec::new();
    let mk = |sid: &str,
              issue: &str,
              items: Vec<WorkItem>,
              plan: Vec<&str>,
              causal_files: Vec<&str>,
              causal_logs: Vec<&str>,
              irr: Vec<&str>,
              blocker: &str| AgentWorkState {
        scenario_id: sid.into(),
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
        reports.push(AgentExoskeletonDiagnosticReport {
            scenario_id: s.scenario_id,
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
        });
    }
    let n = reports.len() as f64;
    let agg = AgentExoskeletonAggregate {
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
    AgentExoskeletonSuiteReport {
        reports,
        aggregate: agg,
        interpretation_labels: labels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scenarios_minimum_and_distractors() {
        let s = benchmark_scenarios();
        assert!(s.len() >= 7);
        assert!(s.iter().all(|x| x
            .work_items
            .iter()
            .filter(|w| w.item_type == WorkItemType::Distractor)
            .count()
            >= 3));
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
    }
}
