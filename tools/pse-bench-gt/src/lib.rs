//! Ground-truth precision/recall benchmark suite for the Post-Symbolic Engine.
//!
//! This crate provides the *measurement apparatus* for Research Strand A.
//! It is independent of any domain adapter or detector: a detector produces
//! a stream of [`Detection`]s, a scenario carries a stream of
//! [`GroundTruthEvent`]s, and [`score_detections`] turns the pair into a
//! [`Metrics`] record with precision, recall, F1 and AUPRC.
//!
//! The scorer is the pure-math core of the suite. Domain scenarios,
//! PSE/baseline runners, and CLI bench binaries are added in later
//! increments and consume this module unchanged.

use crate::runner::{GateTickDiagnostic, RunnerDiagnostics};
use pse_types::Config;
use serde::{Deserialize, Serialize};

pub mod baselines;
pub mod runner;
pub mod scenarios;

// ─── BenchGtJsonOutput ───────────────────────────────────────────────────────

/// Machine-readable JSON output for one bench_gt scenario run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchGtJsonOutput {
    pub scenario: String,
    pub n_observations: u64,
    pub tolerance_ticks: u64,
    pub ground_truth_events: Vec<GroundTruthEvent>,
    pub metrics_per_source: std::collections::BTreeMap<String, Metrics>,
    pub pse_metrics: Option<Metrics>,
    pub stl_zscore_metrics: Option<Metrics>,
    pub isoforest_metrics: Option<Metrics>,
    pub aggregate_metrics: Metrics,
    pub pse_debug: PseDebug,
    /// SHA-256 hex of the serialized Config.
    pub config_hash: String,
    /// SHA-256 hex of the serialized ground-truth event list.
    pub data_hash: String,
}

/// Build a `BenchGtJsonOutput` from a `ScenarioResult`.
///
/// `config_bytes` is the canonical JSON serialization of the `Config` used for
/// the run; `ground_truth_bytes` is the canonical JSON of the GT event list.
pub fn build_json_output(
    result: &scenarios::ScenarioResult,
    config_bytes: &[u8],
) -> BenchGtJsonOutput {
    let per_source = metrics_by_source(
        &result.ground_truth,
        &result.detections,
        result.tolerance_ticks,
    );

    let pse_metrics = {
        // Merge crystal + memory_hit detections into a single PSE combined view.
        // Even when the run emits zero PSE detections, we still serialize
        // explicit zero-valued metrics so downstream domain summaries can
        // distinguish "PSE ran with no detections" from "field missing".
        let pse_dets: Vec<Detection> = result
            .detections
            .iter()
            .filter(|d| d.source.starts_with("pse_"))
            .cloned()
            .collect();
        Some(score_detections(
            &result.ground_truth,
            &pse_dets,
            result.tolerance_ticks,
        ))
    };
    let mut pse_debug = build_pse_debug(&result.detections, &result.runner_diagnostics);
    pse_debug.gate_calibration =
        build_gate_calibration(&result.ground_truth, &result.runner_diagnostics.gate_ticks);
    let stl_zscore_metrics = per_source.get("stl_zscore").cloned();
    let isoforest_metrics = per_source.get("isoforest").cloned();

    // Hash the config bytes.
    let config_hash = sha256_hex(config_bytes);

    // Hash the ground-truth event list (canonical JSON).
    let gt_bytes = serde_json::to_vec(&result.ground_truth).unwrap_or_default();
    let data_hash = sha256_hex(&gt_bytes);

    BenchGtJsonOutput {
        scenario: result.scenario.clone(),
        n_observations: result.n_observations,
        tolerance_ticks: result.tolerance_ticks,
        ground_truth_events: result.ground_truth.clone(),
        metrics_per_source: per_source,
        pse_metrics,
        stl_zscore_metrics,
        isoforest_metrics,
        aggregate_metrics: result.metrics.clone(),
        pse_debug,
        config_hash,
        data_hash,
    }
}

fn build_gate_calibration(
    ground_truth: &[GroundTruthEvent],
    ticks: &[GateTickDiagnostic],
) -> GateCalibrationDiagnostics {
    let thresholds = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    let mut sweep = Vec::new();
    for &thr in &thresholds {
        let dets: Vec<Detection> = ticks
            .iter()
            .filter(|t| t.gate_d >= thr && t.gate_q >= thr && t.gate_g >= thr && t.gate_k >= thr)
            .map(|t| Detection::new(t.tick, t.gate_r, "pse_counterfactual_gate"))
            .collect();
        let m = score_detections(ground_truth, &dets, 0);
        sweep.push(ThresholdSweepRow {
            threshold: thr,
            hypothetical_detection_count: dets.len() as u64,
            hypothetical_tp: m.tp,
            hypothetical_fp: m.fp,
            hypothetical_fn: m.fn_,
            hypothetical_precision: m.precision,
            hypothetical_recall: m.recall,
            hypothetical_f1: m.f1,
        });
    }
    let best_threshold_by_f1 = sweep
        .iter()
        .max_by(|a, b| a.hypothetical_f1.total_cmp(&b.hypothetical_f1))
        .map(|r| r.threshold);
    let first_threshold_with_any_detection = sweep
        .iter()
        .find(|r| r.hypothetical_detection_count > 0)
        .map(|r| r.threshold);
    let first_threshold_with_true_positive = sweep
        .iter()
        .find(|r| r.hypothetical_tp > 0)
        .map(|r| r.threshold);
    GateCalibrationDiagnostics {
        thresholds_tested: thresholds,
        sweep,
        best_threshold_by_f1,
        first_threshold_with_any_detection,
        first_threshold_with_true_positive,
    }
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(data);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PseDebug {
    pub crystal_count: u64,
    pub detection_count: u64,
    pub memory_hit_count: u64,
    pub first_detection_tick: Option<u64>,
    pub detection_ticks: Vec<u64>,
    pub max_score: Option<f64>,
    pub threshold: Option<f64>,
    pub filtered_count: u64,
    pub source_counts: std::collections::BTreeMap<String, u64>,
    pub observation_count: u64,
    pub window_count: u64,
    pub adapter_event_count: u64,
    pub candidate_count: u64,
    pub gate_pass_count: u64,
    pub gate_hold_count: u64,
    pub gate_reject_count: u64,
    pub last_gate_reason: Option<String>,
    pub max_resonance_score: Option<f64>,
    pub max_kappa: Option<f64>,
    pub min_threshold: Option<f64>,
    pub warmup_remaining: Option<u64>,
    pub calibration_mode: Option<String>,
    pub engine_outcome_counts: std::collections::BTreeMap<String, u64>,
    pub gate_calibration: GateCalibrationDiagnostics,
    pub calibration_report: CalibrationReport,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GateCalibrationDiagnostics {
    pub thresholds_tested: Vec<f64>,
    pub sweep: Vec<ThresholdSweepRow>,
    pub best_threshold_by_f1: Option<f64>,
    pub first_threshold_with_any_detection: Option<f64>,
    pub first_threshold_with_true_positive: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ThresholdSweepRow {
    pub threshold: f64,
    pub hypothetical_detection_count: u64,
    pub hypothetical_tp: u64,
    pub hypothetical_fp: u64,
    pub hypothetical_fn: u64,
    pub hypothetical_precision: f64,
    pub hypothetical_recall: f64,
    pub hypothetical_f1: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateAxisThresholds {
    pub d: f64,
    pub q: f64,
    pub r: f64,
    pub g: f64,
    pub j: f64,
    pub p: f64,
    pub n: f64,
    pub k: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CalibrationSplit {
    Calibration,
    Validation,
    Test,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CalibrationSource {
    DomainBench,
    Synthetic,
    External,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CalibrationPolicy {
    UncalibratedStatic,
    CounterfactualSweepOnly,
    CalibratedValidationFrozen,
    TestFrozenApplied,
}

impl Default for CalibrationPolicy {
    fn default() -> Self {
        Self::UncalibratedStatic
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateCalibrationProfile {
    pub profile_version: String,
    pub source_scenario: String,
    pub split: CalibrationSplit,
    pub source: CalibrationSource,
    pub policy: CalibrationPolicy,
    pub thresholds: GateAxisThresholds,
    pub diagnostics_hash: String,
    pub profile_hash: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationReport {
    pub run_mode: String,
    pub counterfactual_sweep: bool,
    pub calibrated_profile_applied: bool,
    pub profile_hash: Option<String>,
    pub active_policy: CalibrationPolicy,
    pub derived_thresholds: Option<GateAxisThresholds>,
    pub applied_thresholds: Option<GateAxisThresholds>,
    pub fail_closed_axes: Vec<String>,
    pub per_axis_reason: std::collections::BTreeMap<String, String>,
    pub profile_gate_pass_count_after_apply: Option<u64>,
    pub profile_gate_reject_count_after_apply: Option<u64>,
    pub profile_counterfactual_consistency: Option<bool>,
    pub warnings: Vec<String>,
    pub source_split: Option<CalibrationSplit>,
    pub applied_split: Option<CalibrationSplit>,
    pub test_frozen: bool,
}

fn fail_closed_metadata_from_profile(
    profile: &GateCalibrationProfile,
) -> (Vec<String>, std::collections::BTreeMap<String, String>) {
    let mut axes = Vec::new();
    let mut reasons = std::collections::BTreeMap::new();
    for w in &profile.warnings {
        if let Some(rest) = w.strip_prefix("axis ") {
            let axis = rest
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string();
            if !axis.is_empty() {
                axes.push(axis.clone());
                reasons.insert(axis, "gt_mean <= out_mean".into());
            }
        }
    }
    axes.sort();
    axes.dedup();
    (axes, reasons)
}

impl Default for CalibrationReport {
    fn default() -> Self {
        Self {
            run_mode: "uncalibrated_static".into(),
            counterfactual_sweep: true,
            calibrated_profile_applied: false,
            profile_hash: None,
            active_policy: CalibrationPolicy::UncalibratedStatic,
            derived_thresholds: None,
            applied_thresholds: None,
            fail_closed_axes: Vec::new(),
            per_axis_reason: std::collections::BTreeMap::new(),
            profile_gate_pass_count_after_apply: None,
            profile_gate_reject_count_after_apply: None,
            profile_counterfactual_consistency: None,
            warnings: Vec::new(),
            source_split: None,
            applied_split: None,
            test_frozen: false,
        }
    }
}

fn build_pse_debug(detections: &[Detection], diag: &RunnerDiagnostics) -> PseDebug {
    let pse_dets: Vec<&Detection> = detections
        .iter()
        .filter(|d| d.source.starts_with("pse_"))
        .collect();
    let mut source_counts = std::collections::BTreeMap::new();
    for d in &pse_dets {
        *source_counts.entry(d.source.clone()).or_insert(0) += 1;
    }
    let detection_ticks: Vec<u64> = pse_dets.iter().map(|d| d.tick).collect();
    let max_score = pse_dets
        .iter()
        .map(|d| d.score)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let profile = build_gate_calibration_profile(diag, "unknown_scenario");
    let report = if let Some(profile) = &profile {
        let (fail_closed_axes, per_axis_reason) = fail_closed_metadata_from_profile(profile);
        CalibrationReport {
            run_mode: "calibration_profile_built".into(),
            counterfactual_sweep: true,
            calibrated_profile_applied: false,
            profile_hash: Some(profile.profile_hash.clone()),
            active_policy: profile.policy.clone(),
            derived_thresholds: Some(profile.thresholds.clone()),
            applied_thresholds: None,
            fail_closed_axes,
            per_axis_reason,
            profile_gate_pass_count_after_apply: None,
            profile_gate_reject_count_after_apply: None,
            profile_counterfactual_consistency: None,
            warnings: profile.warnings.clone(),
            source_split: Some(profile.split.clone()),
            applied_split: None,
            test_frozen: false,
        }
    } else {
        CalibrationReport::default()
    };

    PseDebug {
        crystal_count: *source_counts.get("pse_crystal").unwrap_or(&0),
        detection_count: pse_dets.len() as u64,
        memory_hit_count: *source_counts.get("pse_memory_hit").unwrap_or(&0),
        first_detection_tick: detection_ticks.first().copied(),
        detection_ticks,
        max_score,
        threshold: None,
        filtered_count: 0,
        source_counts,
        observation_count: diag.observation_count,
        window_count: diag.window_count,
        adapter_event_count: diag.adapter_event_count,
        candidate_count: diag.candidate_count,
        gate_pass_count: diag.gate_pass_count,
        gate_hold_count: diag.gate_hold_count,
        gate_reject_count: diag.gate_reject_count,
        last_gate_reason: diag.last_gate_reason.clone(),
        max_resonance_score: diag.max_resonance_score,
        max_kappa: diag.max_kappa,
        min_threshold: diag.min_threshold,
        warmup_remaining: diag.warmup_remaining,
        calibration_mode: Some(diag.calibration_mode.clone()),
        engine_outcome_counts: diag.engine_outcome_counts.clone(),
        gate_calibration: GateCalibrationDiagnostics::default(),
        calibration_report: report,
    }
}

pub fn build_gate_calibration_profile(
    diag: &RunnerDiagnostics,
    source_scenario: &str,
) -> Option<GateCalibrationProfile> {
    if diag.gate_ticks.is_empty() {
        return None;
    }
    let mut warnings = Vec::new();
    let in_gt: Vec<_> = diag
        .gate_ticks
        .iter()
        .filter(|t| t.in_ground_truth_window)
        .collect();
    let out_gt: Vec<_> = diag
        .gate_ticks
        .iter()
        .filter(|t| !t.in_ground_truth_window)
        .collect();
    if in_gt.is_empty() || out_gt.is_empty() {
        warnings.push("insufficient gt separation for robust calibration".into());
    }
    let mut fail_closed_axes = Vec::new();
    let mut per_axis_reason: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    let mut derive = |axis: &str, f: fn(&crate::runner::GateTickDiagnostic) -> f64| -> f64 {
        let gt_mean = if in_gt.is_empty() {
            0.0
        } else {
            in_gt.iter().map(|t| f(t)).sum::<f64>() / in_gt.len() as f64
        };
        let out_mean = if out_gt.is_empty() {
            0.0
        } else {
            out_gt.iter().map(|t| f(t)).sum::<f64>() / out_gt.len() as f64
        };
        if gt_mean <= out_mean {
            fail_closed_axes.push(axis.to_string());
            per_axis_reason.insert(axis.to_string(), "gt_mean <= out_mean".into());
            warnings.push(format!("axis {axis} fail-closed: gt_mean <= out_mean"));
            1.0
        } else {
            ((gt_mean + out_mean) / 2.0).clamp(0.0, 1.0)
        }
    };
    let thresholds = GateAxisThresholds {
        d: derive("d", |t| t.gate_d),
        q: derive("q", |t| t.gate_q),
        r: derive("r", |t| t.gate_r),
        g: derive("g", |t| t.gate_g),
        j: derive("j", |t| t.gate_j),
        p: derive("p", |t| t.gate_p),
        n: derive("n", |t| t.gate_n),
        k: derive("k", |t| t.gate_k),
    };
    if [
        thresholds.d,
        thresholds.q,
        thresholds.r,
        thresholds.g,
        thresholds.j,
        thresholds.p,
        thresholds.n,
        thresholds.k,
    ]
    .iter()
    .all(|&x| x >= 0.999)
    {
        warnings.push("fail-closed profile: no discriminative gt-sensitive signal found".into());
    }
    if !fail_closed_axes.is_empty() {
        warnings.push("derived profile is stricter than global counterfactual sweep".into());
    }
    let diagnostics_hash = sha256_hex(&serde_json::to_vec(&diag.gate_ticks).ok()?);
    let mut partial = GateCalibrationProfile {
        profile_version: "1".into(),
        source_scenario: source_scenario.into(),
        split: CalibrationSplit::Calibration,
        source: CalibrationSource::DomainBench,
        policy: CalibrationPolicy::CalibratedValidationFrozen,
        thresholds,
        diagnostics_hash,
        profile_hash: String::new(),
        warnings,
    };
    let profile_hash = sha256_hex(&serde_json::to_vec(&partial).ok()?);
    partial.profile_hash = profile_hash;
    Some(partial)
}

pub fn apply_frozen_calibration_profile(
    config: &Config,
    profile: &GateCalibrationProfile,
    applied_split: CalibrationSplit,
    expected_profile_hash: &str,
) -> Result<(Config, CalibrationReport), String> {
    if profile.profile_hash.is_empty() || expected_profile_hash.is_empty() {
        return Err("missing profile hash".into());
    }
    if profile.profile_hash != expected_profile_hash {
        return Err("profile hash mismatch".into());
    }
    let all = [
        profile.thresholds.d,
        profile.thresholds.q,
        profile.thresholds.r,
        profile.thresholds.g,
        profile.thresholds.j,
        profile.thresholds.p,
        profile.thresholds.n,
        profile.thresholds.k,
    ];
    if all.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0) {
        return Err("invalid threshold value in profile".into());
    }
    let mut warnings = profile.warnings.clone();
    let (fail_closed_axes, per_axis_reason) = fail_closed_metadata_from_profile(profile);
    let test_frozen = matches!(applied_split, CalibrationSplit::Test);
    if matches!(profile.split, CalibrationSplit::Test)
        && matches!(applied_split, CalibrationSplit::Test)
    {
        warnings
            .push("test-profile applied to test split is not valid for performance proof".into());
    }
    let mut cfg = config.clone();
    cfg.thresholds.d = profile.thresholds.d;
    cfg.thresholds.q = profile.thresholds.q;
    cfg.thresholds.r = profile.thresholds.r;
    cfg.thresholds.g = profile.thresholds.g;
    cfg.thresholds.j = profile.thresholds.j;
    cfg.thresholds.p = profile.thresholds.p;
    cfg.thresholds.n = profile.thresholds.n;
    cfg.thresholds.k = profile.thresholds.k;

    let run_mode = if test_frozen {
        "test_frozen_profile_applied"
    } else {
        "calibrated_profile_applied"
    };
    let report = CalibrationReport {
        run_mode: run_mode.into(),
        counterfactual_sweep: true,
        calibrated_profile_applied: true,
        profile_hash: Some(profile.profile_hash.clone()),
        active_policy: profile.policy.clone(),
        derived_thresholds: Some(profile.thresholds.clone()),
        applied_thresholds: Some(profile.thresholds.clone()),
        fail_closed_axes,
        per_axis_reason,
        profile_gate_pass_count_after_apply: None,
        profile_gate_reject_count_after_apply: None,
        profile_counterfactual_consistency: None,
        warnings,
        source_split: Some(profile.split.clone()),
        applied_split: Some(applied_split),
        test_frozen,
    };
    Ok((cfg, report))
}

// ─── Ground-Truth Event ──────────────────────────────────────────────────────

/// A canonical ground-truth event, normalized across adapters.
///
/// `start_index` is inclusive, `end_index` is exclusive. Indices are in the
/// natural tick space of the scenario (e.g. observation ordinals for seismo,
/// interleaved sample indices for vitals, candle ordinals for binance).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroundTruthEvent {
    /// Inclusive start tick.
    pub start_index: u64,
    /// Exclusive end tick.
    pub end_index: u64,
    /// Semantic label, e.g. `"mainshock"` or `"volatility_regime_shift"`.
    pub label: String,
    /// Domain-specific severity, used as a tie-breaker in AUPRC when two
    /// ground-truth events compete for the same detection.
    pub severity: f64,
}

impl GroundTruthEvent {
    /// True iff `tick` falls in this event's half-open interval.
    pub fn contains(&self, tick: u64) -> bool {
        tick >= self.start_index && tick < self.end_index
    }

    /// Half-open interval length in ticks.
    pub fn len(&self) -> u64 {
        self.end_index.saturating_sub(self.start_index)
    }

    /// True iff the interval is empty.
    pub fn is_empty(&self) -> bool {
        self.end_index <= self.start_index
    }
}

// ─── Detection ───────────────────────────────────────────────────────────────

/// A single detector output: *"something happened at tick T with score S"*.
///
/// `source` is a free-form string describing what produced the detection
/// ("pse_crystal", "pse_memory_hit", "isoforest", "stl_zscore"). It lets
/// aggregate reporting split metrics by detector channel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    /// Tick at which the detection fired.
    pub tick: u64,
    /// Detection score in [0, 1]; higher = more confident.
    pub score: f64,
    /// Free-form source identifier.
    pub source: String,
}

impl Detection {
    pub fn new(tick: u64, score: f64, source: impl Into<String>) -> Self {
        Self {
            tick,
            score,
            source: source.into(),
        }
    }
}

// ─── Metrics ─────────────────────────────────────────────────────────────────

/// Precision/recall/F1/AUPRC for a detection-vs-ground-truth comparison.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Metrics {
    /// True positives: ground-truth events matched by at least one detection.
    pub tp: u64,
    /// False positives: detections that match no ground-truth event.
    pub fp: u64,
    /// False negatives: ground-truth events not matched by any detection.
    pub fn_: u64,
    /// TP / (TP + FP); 0 when there are no detections.
    pub precision: f64,
    /// TP / (TP + FN); 0 when there are no ground-truth events.
    pub recall: f64,
    /// 2 * P * R / (P + R); 0 when P = R = 0.
    pub f1: f64,
    /// Area under the precision/recall curve, trapezoidal. `None` when
    /// detections carry no score variance (constant-score input → undefined).
    pub auprc: Option<f64>,
}

// ─── Scoring ─────────────────────────────────────────────────────────────────

/// Match detections to ground-truth events and return aggregate metrics.
///
/// Matching semantics:
///  - A detection at tick `T` matches a ground-truth event `E` iff `T` lies
///    in `[E.start_index − tolerance_ticks, E.end_index + tolerance_ticks)`.
///  - Each GT event may consume **at most one** detection (TP = 1 per event).
///    Extra detections in the same window become FPs so detector "spam"
///    does not inflate precision.
///  - Within a GT window, the highest-score detection is picked as the match
///    (greedy by score); the rest are FPs.
///  - Unmatched GT events are FNs; detections not covered by any GT window
///    are FPs.
///
/// The AUPRC is computed by sweeping a threshold over detection scores from
/// high to low and evaluating (precision, recall) at each unique cut; areas
/// are summed trapezoidally. If all detection scores are identical, AUPRC
/// returns `None` (undefined — a detector with no discrimination gives no
/// meaningful curve).
pub fn score_detections(
    ground_truth: &[GroundTruthEvent],
    detections: &[Detection],
    tolerance_ticks: u64,
) -> Metrics {
    // ---- Greedy matching at the native tolerance, no score filter. ----
    let (tp, fp, fn_) =
        match_at_threshold(ground_truth, detections, tolerance_ticks, f64::NEG_INFINITY);
    let precision = if tp + fp == 0 {
        0.0
    } else {
        tp as f64 / (tp + fp) as f64
    };
    let recall = if tp + fn_ == 0 {
        0.0
    } else {
        tp as f64 / (tp + fn_) as f64
    };
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    let auprc = compute_auprc(ground_truth, detections, tolerance_ticks);
    Metrics {
        tp,
        fp,
        fn_,
        precision,
        recall,
        f1,
        auprc,
    }
}

/// Greedy TP/FP/FN count at a given minimum score.
fn match_at_threshold(
    ground_truth: &[GroundTruthEvent],
    detections: &[Detection],
    tolerance_ticks: u64,
    min_score: f64,
) -> (u64, u64, u64) {
    // Filter detections by score threshold.
    let active: Vec<&Detection> = detections.iter().filter(|d| d.score >= min_score).collect();

    // For each GT event, find the highest-score detection that falls in its
    // extended window. That detection is "consumed" by this match.
    let mut consumed = vec![false; active.len()];
    let mut tp = 0u64;
    let mut fn_ = 0u64;

    for ev in ground_truth {
        let lo = ev.start_index.saturating_sub(tolerance_ticks);
        let hi = ev.end_index.saturating_add(tolerance_ticks);
        // Pick the highest-score non-consumed detection in [lo, hi).
        let mut best: Option<(usize, f64)> = None;
        for (i, d) in active.iter().enumerate() {
            if consumed[i] {
                continue;
            }
            if d.tick < lo || d.tick >= hi {
                continue;
            }
            match best {
                None => best = Some((i, d.score)),
                Some((_, s)) if d.score > s => best = Some((i, d.score)),
                _ => {}
            }
        }
        match best {
            Some((i, _)) => {
                consumed[i] = true;
                tp += 1;
            }
            None => {
                fn_ += 1;
            }
        }
    }

    // Every unconsumed detection is a false positive.
    let fp = consumed.iter().filter(|c| !**c).count() as u64;
    (tp, fp, fn_)
}

/// Group detections by their `source` and compute [`Metrics`] per group.
///
/// Useful for split reporting when multiple detectors (PSE, baseline_a,
/// baseline_b) write into the same detection vector. Sources are returned
/// in deterministic alphabetical order.
pub fn metrics_by_source(
    ground_truth: &[GroundTruthEvent],
    detections: &[Detection],
    tolerance_ticks: u64,
) -> std::collections::BTreeMap<String, Metrics> {
    let mut buckets: std::collections::BTreeMap<String, Vec<Detection>> =
        std::collections::BTreeMap::new();
    for d in detections {
        buckets.entry(d.source.clone()).or_default().push(d.clone());
    }
    buckets
        .into_iter()
        .map(|(src, dets)| (src, score_detections(ground_truth, &dets, tolerance_ticks)))
        .collect()
}

/// Trapezoidal AUPRC by sweeping thresholds over unique detection scores.
fn compute_auprc(
    ground_truth: &[GroundTruthEvent],
    detections: &[Detection],
    tolerance_ticks: u64,
) -> Option<f64> {
    if ground_truth.is_empty() || detections.is_empty() {
        return Some(0.0);
    }

    // Unique descending scores. If there is no variance, AUPRC is undefined.
    let mut scores: Vec<f64> = detections.iter().map(|d| d.score).collect();
    scores.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    scores.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    if scores.len() < 2 {
        return None;
    }

    // PR curve points: start at (recall=0, precision=1) by convention.
    let mut points: Vec<(f64, f64)> = Vec::with_capacity(scores.len() + 1);
    points.push((0.0, 1.0));
    for &s in &scores {
        let (tp, fp, fn_) = match_at_threshold(ground_truth, detections, tolerance_ticks, s);
        let precision = if tp + fp == 0 {
            1.0
        } else {
            tp as f64 / (tp + fp) as f64
        };
        let recall = if tp + fn_ == 0 {
            0.0
        } else {
            tp as f64 / (tp + fn_) as f64
        };
        points.push((recall, precision));
    }

    // Sort by recall ascending (trapezoidal requires monotone x).
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Trapezoidal integration.
    let mut area = 0.0;
    for pair in points.windows(2) {
        let (r0, p0) = pair[0];
        let (r1, p1) = pair[1];
        area += (r1 - r0) * (p0 + p1) * 0.5;
    }
    Some(area.clamp(0.0, 1.0))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn gt(start: u64, end: u64, label: &str) -> GroundTruthEvent {
        GroundTruthEvent {
            start_index: start,
            end_index: end,
            label: label.into(),
            severity: 1.0,
        }
    }

    fn det(tick: u64, score: f64) -> Detection {
        Detection::new(tick, score, "test")
    }

    #[test]
    fn empty_inputs_give_zero_metrics() {
        let m = score_detections(&[], &[], 0);
        assert_eq!(m.tp, 0);
        assert_eq!(m.fp, 0);
        assert_eq!(m.fn_, 0);
        assert_eq!(m.precision, 0.0);
        assert_eq!(m.recall, 0.0);
        assert_eq!(m.f1, 0.0);
    }

    #[test]
    fn gt_without_detections_is_all_false_negatives() {
        let gts = vec![gt(10, 20, "a"), gt(30, 40, "b")];
        let m = score_detections(&gts, &[], 0);
        assert_eq!(m.tp, 0);
        assert_eq!(m.fp, 0);
        assert_eq!(m.fn_, 2);
        assert_eq!(m.recall, 0.0);
        assert_eq!(m.precision, 0.0);
    }

    #[test]
    fn detections_without_gt_are_all_false_positives() {
        let dets = vec![det(5, 0.5), det(15, 0.7)];
        let m = score_detections(&[], &dets, 0);
        assert_eq!(m.tp, 0);
        assert_eq!(m.fp, 2);
        assert_eq!(m.fn_, 0);
    }

    #[test]
    fn exact_hit_inside_window_is_tp() {
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(15, 0.9)];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 1);
        assert_eq!(m.fp, 0);
        assert_eq!(m.fn_, 0);
        assert_eq!(m.precision, 1.0);
        assert_eq!(m.recall, 1.0);
        assert_eq!(m.f1, 1.0);
    }

    #[test]
    fn detection_on_exact_boundary_start_is_tp() {
        // start_index is inclusive → tick == start is a match at tol=0.
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(10, 0.9)];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 1);
    }

    #[test]
    fn detection_on_exact_boundary_end_is_miss_at_zero_tolerance() {
        // end_index is exclusive → tick == end is a miss at tol=0.
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(20, 0.9)];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 0);
        assert_eq!(m.fp, 1);
        assert_eq!(m.fn_, 1);
    }

    #[test]
    fn tolerance_window_extends_match() {
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(5, 0.9), det(25, 0.5)];
        // tol = 5 makes [5, 25) match; tick 25 is still outside since exclusive.
        let m = score_detections(&gts, &dets, 5);
        assert_eq!(m.tp, 1);
        // tick 25 is OUTSIDE the extended [5,25) window → FP
        assert_eq!(m.fp, 1);
    }

    #[test]
    fn extra_detections_in_same_window_are_false_positives() {
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(12, 0.9), det(15, 0.7), det(17, 0.5)];
        let m = score_detections(&gts, &dets, 0);
        // One TP (highest-score pick), two FPs.
        assert_eq!(m.tp, 1);
        assert_eq!(m.fp, 2);
        assert_eq!(m.fn_, 0);
    }

    #[test]
    fn two_events_two_detections_match_disjointly() {
        let gts = vec![gt(10, 20, "a"), gt(30, 40, "b")];
        let dets = vec![det(15, 0.6), det(35, 0.8)];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 2);
        assert_eq!(m.fp, 0);
        assert_eq!(m.fn_, 0);
    }

    #[test]
    fn highest_score_wins_within_window() {
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(12, 0.3), det(15, 0.9), det(17, 0.6)];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 1);
        assert_eq!(m.fp, 2);
        // AUPRC should be > 0 since the high-score detection is actually a hit.
        assert!(m.auprc.unwrap_or(0.0) > 0.0);
    }

    #[test]
    fn auprc_is_one_for_perfect_separator() {
        // Two GT events, two matching high-score detections, two distractor
        // low-score detections far from any GT.
        let gts = vec![gt(10, 20, "a"), gt(30, 40, "b")];
        let dets = vec![det(15, 0.95), det(35, 0.95), det(100, 0.10), det(200, 0.05)];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 2);
        assert_eq!(m.fn_, 0);
        let auprc = m.auprc.expect("auprc should be defined");
        assert!(auprc > 0.99, "expected near-perfect AUPRC, got {}", auprc);
    }

    #[test]
    fn auprc_undefined_for_constant_scores() {
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(15, 0.5), det(25, 0.5)];
        let m = score_detections(&gts, &dets, 0);
        assert!(m.auprc.is_none());
    }

    #[test]
    fn ground_truth_event_contains_is_half_open() {
        let ev = gt(10, 20, "a");
        assert!(!ev.contains(9));
        assert!(ev.contains(10));
        assert!(ev.contains(19));
        assert!(!ev.contains(20));
    }

    #[test]
    fn json_output_has_pse_metrics_even_when_no_pse_detections() {
        let result = scenarios::ScenarioResult {
            scenario: "no_pse".into(),
            n_observations: 3,
            tolerance_ticks: 0,
            ground_truth: vec![gt(1, 2, "event")],
            detections: vec![Detection::new(1, 0.9, "stl_zscore")],
            metrics: Metrics::default(),
            runner_diagnostics: RunnerDiagnostics::default(),
        };
        let out = build_json_output(&result, br#"{}"#);
        let pse = out.pse_metrics.expect("pse metrics should be present");
        assert_eq!(pse.tp, 0);
        assert_eq!(pse.fp, 0);
        assert_eq!(pse.fn_, 1);
        assert_eq!(pse.f1, 0.0);
    }

    #[test]
    fn profile_builder_is_deterministic_for_same_input() {
        let mut diag = RunnerDiagnostics::default();
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 1,
            in_ground_truth_window: true,
            gate_d: 0.9,
            gate_q: 0.8,
            gate_g: 0.7,
            gate_k: 0.6,
            gate_r: 0.5,
            gate_j: 0.7,
            gate_p: 0.4,
            gate_n: 0.3,
            kairos: false,
        });
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 2,
            in_ground_truth_window: false,
            gate_d: 0.1,
            gate_q: 0.2,
            gate_g: 0.3,
            gate_k: 0.4,
            gate_r: 0.5,
            gate_j: 0.3,
            gate_p: 0.2,
            gate_n: 0.1,
            kairos: false,
        });
        let p1 = build_gate_calibration_profile(&diag, "s").unwrap();
        let p2 = build_gate_calibration_profile(&diag, "s").unwrap();
        assert_eq!(p1.profile_hash, p2.profile_hash);
    }

    #[test]
    fn profile_hash_changes_when_input_changes() {
        let mut diag = RunnerDiagnostics::default();
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 1,
            in_ground_truth_window: true,
            gate_d: 0.9,
            gate_q: 0.8,
            gate_g: 0.7,
            gate_k: 0.6,
            gate_r: 0.5,
            gate_j: 0.7,
            gate_p: 0.4,
            gate_n: 0.3,
            kairos: false,
        });
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 2,
            in_ground_truth_window: false,
            gate_d: 0.2,
            gate_q: 0.2,
            gate_g: 0.3,
            gate_k: 0.4,
            gate_r: 0.5,
            gate_j: 0.3,
            gate_p: 0.2,
            gate_n: 0.1,
            kairos: false,
        });
        let p1 = build_gate_calibration_profile(&diag, "s").unwrap();
        diag.gate_ticks[1].gate_d = 0.25;
        let p2 = build_gate_calibration_profile(&diag, "s").unwrap();
        assert_ne!(p1.profile_hash, p2.profile_hash);
    }

    #[test]
    fn apply_requires_explicit_matching_profile_hash() {
        let cfg = Config::default();
        let mut diag = RunnerDiagnostics::default();
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 1,
            in_ground_truth_window: true,
            gate_d: 0.9,
            gate_q: 0.9,
            gate_g: 0.9,
            gate_k: 0.9,
            gate_r: 0.9,
            gate_j: 0.9,
            gate_p: 0.9,
            gate_n: 0.9,
            kairos: false,
        });
        let p = build_gate_calibration_profile(&diag, "s").unwrap();
        assert!(
            apply_frozen_calibration_profile(&cfg, &p, CalibrationSplit::Validation, "").is_err()
        );
        assert!(
            apply_frozen_calibration_profile(&cfg, &p, CalibrationSplit::Validation, "wrong")
                .is_err()
        );
    }

    #[test]
    fn apply_changes_thresholds_only_when_explicitly_called() {
        let cfg = Config::default();
        let mut diag = RunnerDiagnostics::default();
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 1,
            in_ground_truth_window: true,
            gate_d: 0.9,
            gate_q: 0.8,
            gate_g: 0.7,
            gate_k: 0.6,
            gate_r: 0.6,
            gate_j: 0.7,
            gate_p: 0.8,
            gate_n: 0.9,
            kairos: false,
        });
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 2,
            in_ground_truth_window: false,
            gate_d: 0.1,
            gate_q: 0.1,
            gate_g: 0.1,
            gate_k: 0.1,
            gate_r: 0.1,
            gate_j: 0.1,
            gate_p: 0.1,
            gate_n: 0.1,
            kairos: false,
        });
        let p = build_gate_calibration_profile(&diag, "s").unwrap();
        assert_eq!(cfg.thresholds.d, 0.5);
        let (applied, report) = apply_frozen_calibration_profile(
            &cfg,
            &p,
            CalibrationSplit::Validation,
            &p.profile_hash,
        )
        .unwrap();
        assert_eq!(applied.thresholds.d, p.thresholds.d);
        assert!(report.calibrated_profile_applied);
        assert_eq!(report.run_mode, "calibrated_profile_applied");
    }

    #[test]
    fn default_run_remains_uncalibrated_static() {
        let out = build_json_output(
            &scenarios::ScenarioResult {
                scenario: "no_pse".into(),
                n_observations: 1,
                tolerance_ticks: 0,
                ground_truth: vec![],
                detections: vec![],
                metrics: Metrics::default(),
                runner_diagnostics: RunnerDiagnostics::default(),
            },
            br#"{}"#,
        );
        assert_eq!(
            out.pse_debug.calibration_report.run_mode,
            "uncalibrated_static"
        );
        assert!(!out.pse_debug.calibration_report.calibrated_profile_applied);
    }

    #[test]
    fn axis_fail_closed_sets_threshold_and_warning() {
        let mut diag = RunnerDiagnostics::default();
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 1,
            in_ground_truth_window: true,
            gate_d: 0.9,
            gate_q: 0.2,
            gate_g: 0.9,
            gate_k: 0.9,
            gate_r: 0.9,
            gate_j: 0.9,
            gate_p: 0.9,
            gate_n: 0.9,
            kairos: false,
        });
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 2,
            in_ground_truth_window: false,
            gate_d: 0.1,
            gate_q: 0.8,
            gate_g: 0.1,
            gate_k: 0.1,
            gate_r: 0.1,
            gate_j: 0.1,
            gate_p: 0.1,
            gate_n: 0.1,
            kairos: false,
        });
        let p = build_gate_calibration_profile(&diag, "s").unwrap();
        assert_eq!(p.thresholds.q, 1.0);
        assert!(p.warnings.iter().any(|w| w.contains("axis q fail-closed")));
    }

    #[test]
    fn fail_closed_axes_imply_non_empty_warnings() {
        let mut diag = RunnerDiagnostics::default();
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 1,
            in_ground_truth_window: true,
            gate_d: 0.2,
            gate_q: 0.2,
            gate_g: 0.2,
            gate_k: 0.2,
            gate_r: 0.2,
            gate_j: 0.2,
            gate_p: 0.2,
            gate_n: 0.2,
            kairos: false,
        });
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 2,
            in_ground_truth_window: false,
            gate_d: 0.3,
            gate_q: 0.3,
            gate_g: 0.3,
            gate_k: 0.3,
            gate_r: 0.3,
            gate_j: 0.3,
            gate_p: 0.3,
            gate_n: 0.3,
            kairos: false,
        });
        let p = build_gate_calibration_profile(&diag, "s").unwrap();
        let (cfg, report) = apply_frozen_calibration_profile(
            &Config::default(),
            &p,
            CalibrationSplit::Validation,
            &p.profile_hash,
        )
        .unwrap();
        assert_eq!(cfg.thresholds.q, 1.0);
        assert!(!report.fail_closed_axes.is_empty());
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn active_policy_matches_multi_axis_profile_usage() {
        let mut diag = RunnerDiagnostics::default();
        diag.gate_ticks.push(crate::runner::GateTickDiagnostic {
            tick: 1,
            in_ground_truth_window: true,
            gate_d: 0.9,
            gate_q: 0.9,
            gate_g: 0.9,
            gate_k: 0.9,
            gate_r: 0.9,
            gate_j: 0.9,
            gate_p: 0.9,
            gate_n: 0.9,
            kairos: false,
        });
        let p = build_gate_calibration_profile(&diag, "s").unwrap();
        assert!(matches!(
            p.policy,
            CalibrationPolicy::CalibratedValidationFrozen
        ));
    }
}
