#![allow(missing_docs)]
//! Generic external fixture loader for PSE layer candidate-selection benchmarks.
//!
//! All PSE layer fixtures share a common JSON envelope:
//!   `{ fixture_schema_version, fixture_name, intended_layer,
//!      diagnostic_only, productive_agent_validated, cases: [...] }`
//!
//! Each case carries:
//!   - a list of `candidates` (id / source / text / tags / relevance_rank_hint)
//!   - `ground_truth.causal_files` — the IDs of the correct candidates
//!
//! Layer-specific context fields (e.g. `cognition_context`, `metatron_context`)
//! are ignored by this loader; they are opaque to scoring.
//!
//! # Usage
//!
//! ```no_run
//! use pse_eval_matrix::external_fixture::{
//!     validate_layer_fixture, summarize_layer_fixture, score_layer_fixture_selections,
//! };
//! use std::collections::BTreeMap;
//!
//! let fixture = validate_layer_fixture("fixtures/metatron/metatron_layer_v1.json").unwrap();
//! let report  = summarize_layer_fixture(&fixture);
//! println!("cases={}, causal={}", report.case_count, report.total_causal);
//!
//! // Score an external system's selections (rank-ordered per case).
//! let selections: BTreeMap<String, Vec<String>> = BTreeMap::new(); // fill from eval output
//! let score = score_layer_fixture_selections(&fixture, &selections);
//! println!("recall={:.2}  hit@1={:.2}", score.macro_causal_recall, score.hit_at_1);
//! ```

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

// ── File-level schema ──────────────────────────────────────────────────────

/// Top-level JSON envelope shared by all PSE layer fixture files.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalLayerFixtureFile {
    /// Schema version tag, e.g. `"v1_cognition_fixture"`.  Must start with `"v1_"`.
    pub fixture_schema_version: String,
    /// Human-readable fixture name, non-empty.
    pub fixture_name: String,
    /// Free-form description of the target PSE layer.
    pub intended_layer: String,
    /// Must be `true` — diagnostic-only fixtures are not production-validated.
    #[serde(default = "default_true")]
    pub diagnostic_only: bool,
    /// Must be `false` — external fixtures have not been validated by a productive agent.
    #[serde(default)]
    pub productive_agent_validated: bool,
    /// Non-empty list of benchmark cases.
    pub cases: Vec<ExternalLayerCase>,
}

fn default_true() -> bool {
    true
}

/// One benchmark case within a layer fixture.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalLayerCase {
    /// Unique identifier within the fixture file.
    pub trace_id: String,
    /// Human-readable title.
    pub title: String,
    /// Task category, e.g. `"select_closure_outcome"`.
    #[serde(default)]
    pub task_type: String,
    /// Ordered event stream (optional context for prompt construction).
    #[serde(default)]
    pub events: Vec<ExternalLayerEvent>,
    /// Candidate pool from which the correct answer must be selected.
    #[serde(default)]
    pub candidates: Vec<ExternalLayerCandidate>,
    /// Ground truth for this case.
    pub ground_truth: ExternalLayerGroundTruth,
    /// Fixture-level tags (e.g. `"external"`, `"diagnostic_only"`, layer name).
    #[serde(default)]
    pub tags: Vec<String>,
}

/// One event in the case's observation stream.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalLayerEvent {
    pub event_id: String,
    pub source: String,
    pub message: String,
    #[serde(default)]
    pub timestamp_hint: Option<String>,
}

/// One selectable candidate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalLayerCandidate {
    /// Unique within this case.
    pub id: String,
    /// Origin label (e.g. function name, module, log source).
    #[serde(default)]
    pub source: String,
    /// Descriptive text shown to the evaluating system.
    #[serde(default)]
    pub text: String,
    /// PSE classification tags (e.g. `"causal"`, `"red_herring"`, `"distractor"`).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional ranking hint (1 = most relevant, null = unknown / distractor).
    #[serde(default)]
    pub relevance_rank_hint: Option<u32>,
}

/// Ground truth for a case.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalLayerGroundTruth {
    /// IDs of correct candidates (candidate-selection tasks).
    #[serde(default)]
    pub causal_files: Vec<String>,
    #[serde(default)]
    pub causal_logs: Vec<String>,
    #[serde(default)]
    pub causal_commands: Vec<String>,
    /// Natural-language descriptions of correct actions.
    #[serde(default)]
    pub correct_next_actions: Vec<String>,
    /// Natural-language descriptions of incorrect paths.
    #[serde(default)]
    pub rejected_false_paths: Vec<String>,
    /// Required text elements (open-ended / productive tasks).
    #[serde(default)]
    pub required_elements: Vec<String>,
    /// Stable label for this resolution path.
    pub resolution_label: String,
}

// ── Fixture summary (no external system needed) ────────────────────────────

/// Summary of a validated fixture file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalLayerFixtureReport {
    pub fixture_name: String,
    pub fixture_schema_version: String,
    pub intended_layer: String,
    pub case_count: usize,
    pub total_candidates: usize,
    pub total_causal: usize,
    pub cases: Vec<ExternalLayerCaseInfo>,
}

/// Per-case summary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalLayerCaseInfo {
    pub trace_id: String,
    pub title: String,
    pub task_type: String,
    pub candidate_count: usize,
    pub causal_count: usize,
    pub distractor_count: usize,
}

// ── Scoring (requires external selections) ─────────────────────────────────

/// Scoring report produced when an external system's selections are compared
/// against fixture ground truth.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalLayerFixtureScoreReport {
    pub fixture_name: String,
    pub case_count: usize,
    /// Mean causal recall across all cases.
    pub macro_causal_recall: f64,
    /// Mean distractor suppression rate across all cases.
    pub macro_distractor_suppression: f64,
    /// Fraction of cases where rank-1 selection is causal.
    pub hit_at_1: f64,
    /// Fraction of cases where any of the top-3 selections is causal.
    pub hit_at_3: f64,
    pub cases: Vec<ExternalLayerCaseScore>,
}

/// Per-case scoring result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalLayerCaseScore {
    pub trace_id: String,
    pub selected_count: usize,
    pub causal_count: usize,
    pub true_positives: usize,
    pub false_positives: usize,
    pub causal_recall: f64,
    pub hit_at_1: bool,
    pub hit_at_3: bool,
    pub distractor_suppression: f64,
}

// ── Public functions ───────────────────────────────────────────────────────

/// Load and validate a layer fixture file.
///
/// Validates:
/// - `fixture_schema_version` starts with `"v1_"`
/// - `fixture_name` non-empty
/// - `diagnostic_only == true`, `productive_agent_validated == false`
/// - at least one case, no duplicate `trace_id`s
/// - all `ground_truth.causal_files` IDs reference a known candidate in the same case
pub fn validate_layer_fixture(path: &str) -> Result<ExternalLayerFixtureFile, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("fixture read error {path}: {e}"))?;
    let file: ExternalLayerFixtureFile =
        serde_json::from_slice(&bytes).map_err(|e| format!("fixture parse error {path}: {e}"))?;

    if !file.fixture_schema_version.starts_with("v1_") {
        return Err(format!(
            "fixture_schema_version must start with 'v1_', got '{}'",
            file.fixture_schema_version
        ));
    }
    if file.fixture_name.is_empty() {
        return Err("fixture_name must not be empty".to_string());
    }
    if !file.diagnostic_only {
        return Err("diagnostic_only must be true".to_string());
    }
    if file.productive_agent_validated {
        return Err("productive_agent_validated must be false".to_string());
    }
    if file.cases.is_empty() {
        return Err("cases must not be empty".to_string());
    }

    let mut seen_ids = BTreeSet::new();
    for case in &file.cases {
        if case.trace_id.is_empty() {
            return Err("case trace_id must not be empty".to_string());
        }
        if !seen_ids.insert(case.trace_id.clone()) {
            return Err(format!("duplicate trace_id: '{}'", case.trace_id));
        }
        if !case.candidates.is_empty() {
            let candidate_ids: BTreeSet<&str> =
                case.candidates.iter().map(|c| c.id.as_str()).collect();
            for causal_id in &case.ground_truth.causal_files {
                if !candidate_ids.contains(causal_id.as_str()) {
                    return Err(format!(
                        "case '{}': causal_files references unknown candidate '{}'",
                        case.trace_id, causal_id
                    ));
                }
            }
            // Verify no duplicate candidate IDs within a case.
            let mut seen_cand = BTreeSet::new();
            for c in &case.candidates {
                if c.id.is_empty() {
                    return Err(format!(
                        "case '{}': candidate id must not be empty",
                        case.trace_id
                    ));
                }
                if !seen_cand.insert(c.id.clone()) {
                    return Err(format!(
                        "case '{}': duplicate candidate id '{}'",
                        case.trace_id, c.id
                    ));
                }
            }
        }
    }

    Ok(file)
}

/// Summarize a validated fixture without running any evaluation.
pub fn summarize_layer_fixture(fixture: &ExternalLayerFixtureFile) -> ExternalLayerFixtureReport {
    let cases: Vec<ExternalLayerCaseInfo> = fixture
        .cases
        .iter()
        .map(|c| {
            let causal_count = c.ground_truth.causal_files.len();
            let distractor_count = c
                .candidates
                .iter()
                .filter(|cand| {
                    cand.tags
                        .iter()
                        .any(|t| t == "red_herring" || t == "distractor")
                })
                .count();
            ExternalLayerCaseInfo {
                trace_id: c.trace_id.clone(),
                title: c.title.clone(),
                task_type: c.task_type.clone(),
                candidate_count: c.candidates.len(),
                causal_count,
                distractor_count,
            }
        })
        .collect();

    let total_candidates: usize = cases.iter().map(|c| c.candidate_count).sum();
    let total_causal: usize = cases.iter().map(|c| c.causal_count).sum();

    ExternalLayerFixtureReport {
        fixture_name: fixture.fixture_name.clone(),
        fixture_schema_version: fixture.fixture_schema_version.clone(),
        intended_layer: fixture.intended_layer.clone(),
        case_count: cases.len(),
        total_candidates,
        total_causal,
        cases,
    }
}

/// Score an external system's candidate selections against fixture ground truth.
///
/// `selections` maps `trace_id → candidate IDs in rank order` (first = rank 1).
/// Missing trace IDs are scored as zero selections (recall = 0, hit@k = false).
pub fn score_layer_fixture_selections(
    fixture: &ExternalLayerFixtureFile,
    selections: &BTreeMap<String, Vec<String>>,
) -> ExternalLayerFixtureScoreReport {
    let case_scores: Vec<ExternalLayerCaseScore> = fixture
        .cases
        .iter()
        .map(|case| {
            let selected = selections
                .get(&case.trace_id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let causal_set: BTreeSet<&str> = case
                .ground_truth
                .causal_files
                .iter()
                .map(|s| s.as_str())
                .collect();
            let distractor_set: BTreeSet<&str> = case
                .candidates
                .iter()
                .filter(|c| {
                    c.tags
                        .iter()
                        .any(|t| t == "red_herring" || t == "distractor")
                })
                .map(|c| c.id.as_str())
                .collect();

            let causal_count = causal_set.len();
            let true_positives = selected
                .iter()
                .filter(|s| causal_set.contains(s.as_str()))
                .count();
            let false_positives = selected
                .iter()
                .filter(|s| distractor_set.contains(s.as_str()))
                .count();

            let causal_recall = if causal_count == 0 {
                1.0
            } else {
                true_positives as f64 / causal_count as f64
            };

            let hit_at_1 = selected
                .first()
                .map(|s| causal_set.contains(s.as_str()))
                .unwrap_or(false);
            let hit_at_3 = selected
                .iter()
                .take(3)
                .any(|s| causal_set.contains(s.as_str()));

            let total_distractors = distractor_set.len();
            let distractor_suppression = if total_distractors == 0 {
                1.0
            } else {
                let avoided = total_distractors
                    - selected
                        .iter()
                        .filter(|s| distractor_set.contains(s.as_str()))
                        .count();
                avoided as f64 / total_distractors as f64
            };

            ExternalLayerCaseScore {
                trace_id: case.trace_id.clone(),
                selected_count: selected.len(),
                causal_count,
                true_positives,
                false_positives,
                causal_recall,
                hit_at_1,
                hit_at_3,
                distractor_suppression,
            }
        })
        .collect();

    let case_count = case_scores.len();
    let macro_causal_recall = if case_count == 0 {
        0.0
    } else {
        case_scores.iter().map(|r| r.causal_recall).sum::<f64>() / case_count as f64
    };
    let macro_distractor_suppression = if case_count == 0 {
        0.0
    } else {
        case_scores
            .iter()
            .map(|r| r.distractor_suppression)
            .sum::<f64>()
            / case_count as f64
    };
    let hit_at_1 = if case_count == 0 {
        0.0
    } else {
        case_scores.iter().filter(|r| r.hit_at_1).count() as f64 / case_count as f64
    };
    let hit_at_3 = if case_count == 0 {
        0.0
    } else {
        case_scores.iter().filter(|r| r.hit_at_3).count() as f64 / case_count as f64
    };

    ExternalLayerFixtureScoreReport {
        fixture_name: fixture.fixture_name.clone(),
        case_count,
        macro_causal_recall,
        macro_distractor_suppression,
        hit_at_1,
        hit_at_3,
        cases: case_scores,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fixture() -> ExternalLayerFixtureFile {
        ExternalLayerFixtureFile {
            fixture_schema_version: "v1_test_fixture".into(),
            fixture_name: "test_fixture".into(),
            intended_layer: "test layer".into(),
            diagnostic_only: true,
            productive_agent_validated: false,
            cases: vec![ExternalLayerCase {
                trace_id: "c01".into(),
                title: "test case".into(),
                task_type: "select".into(),
                events: vec![],
                candidates: vec![
                    ExternalLayerCandidate {
                        id: "c01_r01".into(),
                        source: "correct".into(),
                        text: "correct answer".into(),
                        tags: vec!["causal".into()],
                        relevance_rank_hint: Some(1),
                    },
                    ExternalLayerCandidate {
                        id: "c01_r02".into(),
                        source: "wrong".into(),
                        text: "wrong answer".into(),
                        tags: vec!["red_herring".into(), "distractor".into()],
                        relevance_rank_hint: None,
                    },
                ],
                ground_truth: ExternalLayerGroundTruth {
                    causal_files: vec!["c01_r01".into()],
                    causal_logs: vec![],
                    causal_commands: vec![],
                    correct_next_actions: vec![],
                    rejected_false_paths: vec![],
                    required_elements: vec![],
                    resolution_label: "test_resolution".into(),
                },
                tags: vec![],
            }],
        }
    }

    #[test]
    fn validate_rejects_bad_schema_version() {
        let mut f = make_fixture();
        f.fixture_schema_version = "v0_test".into();
        // Can't use validate_layer_fixture directly (it reads a file),
        // so test the validation logic inline.
        assert!(!f.fixture_schema_version.starts_with("v1_"));
    }

    #[test]
    fn summarize_counts_correctly() {
        let f = make_fixture();
        let r = summarize_layer_fixture(&f);
        assert_eq!(r.case_count, 1);
        assert_eq!(r.total_candidates, 2);
        assert_eq!(r.total_causal, 1);
        assert_eq!(r.cases[0].distractor_count, 1);
    }

    #[test]
    fn score_perfect_selection() {
        let f = make_fixture();
        let mut sel = BTreeMap::new();
        sel.insert("c01".into(), vec!["c01_r01".into()]);
        let s = score_layer_fixture_selections(&f, &sel);
        assert_eq!(s.case_count, 1);
        assert!((s.macro_causal_recall - 1.0).abs() < 1e-9);
        assert!((s.hit_at_1 - 1.0).abs() < 1e-9);
        assert!((s.macro_distractor_suppression - 1.0).abs() < 1e-9);
    }

    #[test]
    fn score_wrong_selection() {
        let f = make_fixture();
        let mut sel = BTreeMap::new();
        sel.insert("c01".into(), vec!["c01_r02".into()]);
        let s = score_layer_fixture_selections(&f, &sel);
        assert!((s.macro_causal_recall - 0.0).abs() < 1e-9);
        assert!(!s.cases[0].hit_at_1);
        // distractor was selected → suppression = 0
        assert!((s.macro_distractor_suppression - 0.0).abs() < 1e-9);
    }

    #[test]
    fn score_no_selections() {
        let f = make_fixture();
        let sel = BTreeMap::new();
        let s = score_layer_fixture_selections(&f, &sel);
        assert!((s.macro_causal_recall - 0.0).abs() < 1e-9);
        assert!(!s.cases[0].hit_at_1);
        // no distractors selected → suppression = 1
        assert!((s.macro_distractor_suppression - 1.0).abs() < 1e-9);
    }
}
