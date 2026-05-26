#![allow(
    clippy::derivable_impls,
    clippy::field_reassign_with_default,
    clippy::type_complexity
)]
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
    augment_calibration_report_with_counterfactuals(
        &mut pse_debug.calibration_report,
        &result.ground_truth,
        &result.runner_diagnostics.gate_ticks,
    );
    pse_debug.calibration_report.field_diagnostic_report = build_field_diagnostic_report(
        pse_metrics.as_ref(),
        pse_debug
            .calibration_report
            .axis_policy_report
            .candidate_activation_metrics
            .as_ref(),
        pse_debug
            .calibration_report
            .eventization_report
            .applied_diagnostic_eventized_metrics
            .as_ref(),
    );
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

fn axis_value(t: &GateTickDiagnostic, axis: &str) -> f64 {
    match axis {
        "d" => t.gate_d,
        "q" => t.gate_q,
        "r" => t.gate_r,
        "g" => t.gate_g,
        "j" => t.gate_j,
        "p" => t.gate_p,
        "n" => t.gate_n,
        "k" => t.gate_k,
        _ => 0.0,
    }
}

fn augment_calibration_report_with_counterfactuals(
    report: &mut CalibrationReport,
    ground_truth: &[GroundTruthEvent],
    ticks: &[GateTickDiagnostic],
) {
    let axes = ["d", "q", "r", "g", "j", "p", "n", "k"];
    let in_gt: Vec<_> = ticks.iter().filter(|t| t.in_ground_truth_window).collect();
    let out_gt: Vec<_> = ticks.iter().filter(|t| !t.in_ground_truth_window).collect();
    let axis_rows: Vec<AxisDiscriminativityRow> = axes
        .iter()
        .map(|axis| {
            let gt_mean = if in_gt.is_empty() {
                0.0
            } else {
                in_gt.iter().map(|t| axis_value(t, axis)).sum::<f64>() / in_gt.len() as f64
            };
            let out_mean = if out_gt.is_empty() {
                0.0
            } else {
                out_gt.iter().map(|t| axis_value(t, axis)).sum::<f64>() / out_gt.len() as f64
            };
            let delta = gt_mean - out_mean;
            let discriminative = delta > 0.0;
            let suggested_role = if !discriminative {
                "fail_closed"
            } else if delta > 0.2 {
                "mandatory"
            } else if delta > 0.05 {
                "optional"
            } else {
                "disabled"
            };
            AxisDiscriminativityRow {
                axis: (*axis).to_string(),
                gt_mean,
                out_mean,
                delta,
                direction: if delta > 0.0 {
                    "gt>out".into()
                } else if delta < 0.0 {
                    "gt<out".into()
                } else {
                    "equal".into()
                },
                discriminative,
                suggested_role: suggested_role.into(),
            }
        })
        .collect();
    report.axis_discriminativity = axis_rows.clone();

    let policy_specs: Vec<(&str, Vec<String>, String)> = {
        let discrim: Vec<String> = axis_rows
            .iter()
            .filter(|r| r.discriminative)
            .map(|r| r.axis.clone())
            .collect();
        let mut sorted = axis_rows.clone();
        sorted.sort_by(|a, b| b.delta.total_cmp(&a.delta).then(a.axis.cmp(&b.axis)));
        let top_k: Vec<String> = sorted.iter().take(3).map(|r| r.axis.clone()).collect();
        let dominant = sorted
            .first()
            .filter(|top| {
                top.delta > 0.15
                    && sorted
                        .get(1)
                        .map(|n| top.delta > n.delta * 1.5)
                        .unwrap_or(true)
            })
            .map(|r| r.axis.clone())
            .unwrap_or_else(|| "d".to_string());
        vec![
            (
                "current_fail_closed_all_axes",
                axes.iter().map(|s| (*s).to_string()).collect(),
                "all axes mandatory, fail-closed".into(),
            ),
            (
                "discriminative_axes_only",
                discrim.clone(),
                "only axes with gt_mean > out_mean".into(),
            ),
            (
                "top_k_axes_by_delta",
                top_k,
                "top-3 discriminative deltas mandatory".into(),
            ),
            (
                "weighted_axis_score",
                discrim.clone(),
                "weighted score over discriminative axes (>=0.5)".into(),
            ),
            (
                "d_axis_only",
                vec![dominant],
                "single dominant axis only".into(),
            ),
        ]
    };

    let mut comparison = Vec::new();
    for (name, active_axes, reason) in policy_specs {
        let disabled_axes: Vec<String> = axes
            .iter()
            .filter(|a| !active_axes.iter().any(|x| x == *a))
            .map(|a| (*a).to_string())
            .collect();
        let dets: Vec<Detection> = if name == "weighted_axis_score" {
            let weights: Vec<(String, f64)> = axis_rows
                .iter()
                .filter(|r| r.discriminative)
                .map(|r| (r.axis.clone(), r.delta.max(0.0)))
                .collect();
            let total = weights.iter().map(|(_, w)| *w).sum::<f64>();
            ticks
                .iter()
                .filter_map(|t| {
                    if total <= f64::EPSILON {
                        return None;
                    }
                    let s = weights
                        .iter()
                        .map(|(a, w)| axis_value(t, a) * *w)
                        .sum::<f64>()
                        / total;
                    (s >= 0.5).then(|| Detection::new(t.tick, s, "pse_counterfactual_policy"))
                })
                .collect()
        } else {
            ticks
                .iter()
                .filter(|t| {
                    !active_axes.is_empty() && active_axes.iter().all(|a| axis_value(t, a) >= 0.5)
                })
                .map(|t| Detection::new(t.tick, t.gate_r, "pse_counterfactual_policy"))
                .collect()
        };
        let m = score_detections(ground_truth, &dets, 0);
        comparison.push(PolicyComparisonRow {
            policy: name.into(),
            hypothetical_detection_count: dets.len() as u64,
            hypothetical_tp: m.tp,
            hypothetical_fp: m.fp,
            hypothetical_fn: m.fn_,
            hypothetical_precision: m.precision,
            hypothetical_recall: m.recall,
            hypothetical_f1: m.f1,
            active_axes,
            disabled_axes,
            reason,
        });
    }
    comparison.sort_by(|a, b| {
        b.hypothetical_f1
            .total_cmp(&a.hypothetical_f1)
            .then(a.policy.cmp(&b.policy))
    });
    report.best_policy_by_f1 = comparison.first().map(|r| r.policy.clone());
    report.best_policy_active_axes = comparison
        .first()
        .map(|r| r.active_axes.clone())
        .unwrap_or_default();
    report.policy_comparison = comparison;
    if report.best_policy_by_f1.as_deref() != Some("current_fail_closed_all_axes") {
        report
            .warnings
            .push("counterfactual best-policy is calibration-only; validate on validation/test before apply".into());
    }
    report.eventization_report = build_eventization_profile(report, &axis_rows)
        .map(|(profile, expected)| EventizationReport {
            eventization_profile_built: true,
            eventization_profile_hash: Some(profile.profile_hash),
            selected_strategy: Some(profile.strategy.strategy_name),
            selected_strategy_reason: Some(
                "best_f1_then_precision_then_recall_then_fp_then_cooldown".into(),
            ),
            selected_policy: Some(profile.source_policy_name),
            built_expected_eventized_metrics: Some(expected.clone()),
            applied_diagnostic_eventized_metrics: None,
            productive_pse_metrics: None,
            eventization_apply_input_source: None,
            eventization_apply_input_count: None,
            expected_eventized_metrics: Some(expected),
            calibrated_eventization_applied: false,
            warnings: profile.warnings,
        })
        .unwrap_or_default();
}

pub fn build_eventization_profile(
    report: &CalibrationReport,
    axis_rows: &[AxisDiscriminativityRow],
) -> Option<(EventizationProfile, EventizationExpectedMetrics)> {
    let baseline = report.policy_comparison.first()?;
    let source_policy_name = baseline.policy.clone();
    let total_neg_delta = axis_rows.iter().filter(|r| r.delta <= 0.0).count() as u64;
    let candidates = [
        ("cooldown_5", 5_u64),
        ("cooldown_10", 10_u64),
        ("cooldown_20", 20_u64),
    ];
    let mut scored: Vec<(String, u64, EventizationExpectedMetrics)> = candidates
        .into_iter()
        .map(|(name, cd)| {
            let fp_shrink = (cd as f64 / 2.0).max(1.0);
            let fp = (baseline.hypothetical_fp as f64 / fp_shrink).round() as u64;
            let recall_drop = if cd >= 20 {
                0.60
            } else if cd >= 10 {
                0.10
            } else {
                0.05
            };
            let tp = ((baseline.hypothetical_tp as f64) * (1.0 - recall_drop)).round() as u64;
            let fn_ = baseline.hypothetical_fn + baseline.hypothetical_tp.saturating_sub(tp);
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
            let f1 = if precision + recall <= f64::EPSILON {
                0.0
            } else {
                2.0 * precision * recall / (precision + recall)
            };
            (
                name.to_string(),
                cd,
                EventizationExpectedMetrics {
                    hypothetical_detection_count: tp + fp,
                    hypothetical_tp: tp,
                    hypothetical_fp: fp,
                    hypothetical_fn: fn_,
                    hypothetical_precision: precision,
                    hypothetical_recall: recall,
                    hypothetical_f1: f1,
                },
            )
        })
        .collect();
    scored.sort_by(|a, b| {
        b.2.hypothetical_f1
            .total_cmp(&a.2.hypothetical_f1)
            .then(
                b.2.hypothetical_precision
                    .total_cmp(&a.2.hypothetical_precision),
            )
            .then(b.2.hypothetical_recall.total_cmp(&a.2.hypothetical_recall))
            .then(a.2.hypothetical_fp.cmp(&b.2.hypothetical_fp))
            .then(a.1.cmp(&b.1))
    });
    let (name, cooldown, expected) = scored.first()?.clone();
    let mut warnings = Vec::new();
    if baseline.hypothetical_fp > 50 {
        warnings.push(
            "raw policy has high FP load; eventization is calibration-only mitigation".into(),
        );
    }
    if expected.hypothetical_recall < baseline.hypothetical_recall {
        warnings
            .push("eventization may reduce recall; validation/test confirmation required".into());
    }
    if total_neg_delta > 3 {
        warnings
            .push("multiple non-discriminative axes indicate unstable selectivity regime".into());
    }
    let diagnostics_hash =
        sha256_hex(&serde_json::to_vec(&(axis_rows, &report.policy_comparison)).ok()?);
    let mut profile = EventizationProfile {
        strategy: EventizationStrategy {
            strategy_name: name,
            cooldown_ticks: Some(cooldown),
            selected_by: EventizationSelectionMode::BestF1,
        },
        source_policy_name,
        source_split: CalibrationSplit::Calibration,
        policy: EventizationPolicy::CounterfactualOnly,
        source: EventizationSource::PolicyComparison,
        diagnostics_hash,
        profile_hash: String::new(),
        expected_fp_budget: expected.hypothetical_fp,
        warnings,
    };
    profile.profile_hash = sha256_hex(&serde_json::to_vec(&profile).ok()?);
    Some((profile, expected))
}

pub fn apply_frozen_eventization_profile(
    profile: &EventizationProfile,
    expected_profile_hash: &str,
    detections: &[Detection],
    ground_truth: &[GroundTruthEvent],
) -> Result<(Vec<Detection>, EventizationReport), String> {
    if profile.profile_hash.is_empty() || expected_profile_hash.is_empty() {
        return Err("missing eventization profile hash".into());
    }
    if profile.profile_hash != expected_profile_hash {
        return Err("eventization profile hash mismatch".into());
    }
    if profile.strategy.strategy_name.is_empty() || profile.strategy.cooldown_ticks.is_none() {
        return Err("invalid eventization strategy".into());
    }
    if profile.strategy.strategy_name != "cooldown_5"
        && profile.strategy.strategy_name != "cooldown_10"
        && profile.strategy.strategy_name != "cooldown_20"
    {
        return Err("unsupported eventization strategy".into());
    }
    let cooldown = profile.strategy.cooldown_ticks.unwrap_or(0);
    if cooldown == 0 {
        return Err("cooldown_ticks must be > 0".into());
    }
    let mut sorted = detections.to_vec();
    sorted.sort_by_key(|d| d.tick);
    let mut last_tick: Option<u64> = None;
    let mut eventized = Vec::new();
    for d in sorted {
        let keep = last_tick
            .map(|t| d.tick.saturating_sub(t) >= cooldown)
            .unwrap_or(true);
        if keep {
            last_tick = Some(d.tick);
            eventized.push(d);
        }
    }
    let m = score_detections(ground_truth, &eventized, 0);
    let report = EventizationReport {
        eventization_profile_built: true,
        eventization_profile_hash: Some(profile.profile_hash.clone()),
        selected_strategy: Some(profile.strategy.strategy_name.clone()),
        selected_strategy_reason: Some("profile_frozen_hash_bound_apply".into()),
        selected_policy: Some(profile.source_policy_name.clone()),
        built_expected_eventized_metrics: None,
        applied_diagnostic_eventized_metrics: Some(EventizationExpectedMetrics {
            hypothetical_detection_count: eventized.len() as u64,
            hypothetical_tp: m.tp,
            hypothetical_fp: m.fp,
            hypothetical_fn: m.fn_,
            hypothetical_precision: m.precision,
            hypothetical_recall: m.recall,
            hypothetical_f1: m.f1,
        }),
        productive_pse_metrics: None,
        eventization_apply_input_source: Some("productive_pse_detections".into()),
        eventization_apply_input_count: Some(detections.len() as u64),
        expected_eventized_metrics: Some(EventizationExpectedMetrics {
            hypothetical_detection_count: eventized.len() as u64,
            hypothetical_tp: m.tp,
            hypothetical_fp: m.fp,
            hypothetical_fn: m.fn_,
            hypothetical_precision: m.precision,
            hypothetical_recall: m.recall,
            hypothetical_f1: m.f1,
        }),
        calibrated_eventization_applied: true,
        warnings: vec![
            "applied_diagnostic_eventization: does not mutate productive pse_metrics".into(),
        ],
    };
    Ok((eventized, report))
}

pub fn build_axis_policy_profile(report: &CalibrationReport) -> Option<AxisPolicyProfile> {
    let best = report
        .policy_comparison
        .iter()
        .max_by(|a, b| a.hypothetical_f1.total_cmp(&b.hypothetical_f1))?;
    let diagnostics_hash = sha256_hex(
        &serde_json::to_vec(&(
            report.axis_discriminativity.clone(),
            &report.policy_comparison,
        ))
        .ok()?,
    );
    let mut profile = AxisPolicyProfile {
        strategy_name: best.policy.clone(),
        active_axes: best.active_axes.clone(),
        disabled_axes: best.disabled_axes.clone(),
        threshold: Some(0.5),
        score_rule: Some(if best.policy == "weighted_axis_score" {
            "weighted_mean_active_axes>=threshold".into()
        } else {
            "all_active_axes_gate>=threshold".into()
        }),
        weights: if best.policy == "weighted_axis_score" {
            Some(
                report
                    .axis_discriminativity
                    .iter()
                    .filter(|r| best.active_axes.contains(&r.axis))
                    .map(|r| (r.axis.clone(), r.delta.max(0.0)))
                    .collect(),
            )
        } else {
            None
        },
        expected_metrics: Some(CandidateActivationMetrics {
            detection_count: best.hypothetical_detection_count,
            tp: best.hypothetical_tp,
            fp: best.hypothetical_fp,
            fn_: best.hypothetical_fn,
            precision: best.hypothetical_precision,
            recall: best.hypothetical_recall,
            f1: best.hypothetical_f1,
        }),
        source_split: CalibrationSplit::Calibration,
        diagnostics_hash,
        profile_hash: String::new(),
        warnings: Vec::new(),
    };
    profile.active_axes.sort();
    profile.disabled_axes.sort();
    profile.profile_hash = sha256_hex(&serde_json::to_vec(&profile).ok()?);
    Some(profile)
}

pub fn apply_frozen_axis_policy_profile(
    profile: &AxisPolicyProfile,
    expected_profile_hash: &str,
    gate_ticks: &[crate::runner::GateTickDiagnostic],
    ground_truth: &[GroundTruthEvent],
) -> Result<(Vec<Detection>, AxisPolicyReport), String> {
    if profile.profile_hash.is_empty() || expected_profile_hash.is_empty() {
        return Err("missing axis policy profile hash".into());
    }
    if profile.profile_hash != expected_profile_hash {
        return Err("axis policy profile hash mismatch".into());
    }
    if profile.active_axes.is_empty() {
        return Err("axis policy active_axes must not be empty".into());
    }
    let valid = ["d", "q", "r", "g", "j", "p", "n", "k"];
    if profile
        .active_axes
        .iter()
        .any(|a| !valid.contains(&a.as_str()))
    {
        return Err("axis policy contains unknown active axis".into());
    }
    let thr = profile.threshold.unwrap_or(0.5);
    let strategy = profile.strategy_name.clone();
    let score_rule = profile.score_rule.clone();
    let expected_from_policy_comparison = profile.expected_metrics.clone();
    let mut dets = Vec::new();
    for t in gate_ticks {
        let pass = if profile.strategy_name == "weighted_axis_score" {
            let weights = profile.weights.as_ref().ok_or(
                "weighted_axis_score profile requires weights for active_axes reconstruction",
            )?;
            let mut num = 0.0;
            let mut den = 0.0;
            for a in &profile.active_axes {
                let w = *weights.get(a).unwrap_or(&0.0);
                num += axis_value(t, a) * w;
                den += w;
            }
            den > f64::EPSILON && (num / den) >= thr
        } else {
            profile.active_axes.iter().all(|a| match a.as_str() {
                "d" => t.gate_d >= thr,
                "q" => t.gate_q >= thr,
                "r" => t.gate_r >= thr,
                "g" => t.gate_g >= thr,
                "j" => t.gate_j >= thr,
                "p" => t.gate_p >= thr,
                "n" => t.gate_n >= thr,
                "k" => t.gate_k >= thr,
                _ => false,
            })
        };
        if pass {
            dets.push(Detection::new(t.tick, t.gate_r, "candidate_activation"));
        }
    }
    let m = score_detections(ground_truth, &dets, 0);
    let applied_metrics = CandidateActivationMetrics {
        detection_count: dets.len() as u64,
        tp: m.tp,
        fp: m.fp,
        fn_: m.fn_,
        precision: m.precision,
        recall: m.recall,
        f1: m.f1,
    };
    let rep = AxisPolicyReport {
        axis_policy_profile_built: true,
        axis_policy_profile_hash: Some(profile.profile_hash.clone()),
        axis_policy_applied: true,
        axis_policy_apply_input_source: Some("gate_ticks".into()),
        candidate_activation_detection_count: dets.len() as u64,
        candidate_activation_ticks: dets.iter().map(|d| d.tick).collect(),
        candidate_activation_metrics: Some(applied_metrics.clone()),
        candidate_activation_eventized_metrics: None,
        selected_strategy: Some(strategy),
        active_axes: profile.active_axes.clone(),
        score_rule,
        threshold: Some(thr),
        expected_candidate_activation_metrics_from_policy_comparison:
            expected_from_policy_comparison.clone(),
        applied_candidate_activation_metrics: Some(applied_metrics.clone()),
        policy_apply_consistency: Some(
            if expected_from_policy_comparison
                .as_ref()
                .map(|e| e == &applied_metrics)
                .unwrap_or(true)
            {
                "match"
            } else {
                "mismatch"
            }
            .into(),
        ),
        policy_apply_consistency_reason: Some(
            if let Some(expected) = &expected_from_policy_comparison {
                if expected == &applied_metrics {
                    "applied candidate metrics match profile expected metrics".into()
                } else {
                    "applied candidate metrics differ from profile expected metrics".into()
                }
            } else {
                "profile does not contain expected metrics from policy comparison".into()
            },
        ),
        warnings: Vec::new(),
    };
    Ok((dets, rep))
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
    pub axis_discriminativity: Vec<AxisDiscriminativityRow>,
    pub policy_comparison: Vec<PolicyComparisonRow>,
    pub best_policy_by_f1: Option<String>,
    pub best_policy_active_axes: Vec<String>,
    pub axis_policy_report: AxisPolicyReport,
    pub eventization_report: EventizationReport,
    pub field_diagnostic_report: FieldDiagnosticReport,
    pub warnings: Vec<String>,
    pub source_split: Option<CalibrationSplit>,
    pub applied_split: Option<CalibrationSplit>,
    pub test_frozen: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum EventizationPolicy {
    #[default]
    Disabled,
    CounterfactualOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum EventizationSource {
    #[default]
    SelectivityAnalysis,
    PolicyComparison,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum EventizationSelectionMode {
    #[default]
    BestF1,
    PrecisionSubjectToRecall,
    FpBudget,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EventizationStrategy {
    pub strategy_name: String,
    pub cooldown_ticks: Option<u64>,
    pub selected_by: EventizationSelectionMode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventizationProfile {
    pub strategy: EventizationStrategy,
    pub source_policy_name: String,
    pub source_split: CalibrationSplit,
    pub policy: EventizationPolicy,
    pub source: EventizationSource,
    pub diagnostics_hash: String,
    pub profile_hash: String,
    pub expected_fp_budget: u64,
    pub warnings: Vec<String>,
}

impl Default for EventizationProfile {
    fn default() -> Self {
        Self {
            strategy: EventizationStrategy::default(),
            source_policy_name: String::new(),
            source_split: CalibrationSplit::Calibration,
            policy: EventizationPolicy::Disabled,
            source: EventizationSource::SelectivityAnalysis,
            diagnostics_hash: String::new(),
            profile_hash: String::new(),
            expected_fp_budget: 0,
            warnings: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct EventizationExpectedMetrics {
    pub hypothetical_detection_count: u64,
    pub hypothetical_tp: u64,
    pub hypothetical_fp: u64,
    pub hypothetical_fn: u64,
    pub hypothetical_precision: f64,
    pub hypothetical_recall: f64,
    pub hypothetical_f1: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EventizationReport {
    pub eventization_profile_built: bool,
    pub eventization_profile_hash: Option<String>,
    pub selected_strategy: Option<String>,
    pub selected_strategy_reason: Option<String>,
    pub selected_policy: Option<String>,
    pub built_expected_eventized_metrics: Option<EventizationExpectedMetrics>,
    pub applied_diagnostic_eventized_metrics: Option<EventizationExpectedMetrics>,
    pub productive_pse_metrics: Option<Metrics>,
    pub eventization_apply_input_source: Option<String>,
    pub eventization_apply_input_count: Option<u64>,
    pub expected_eventized_metrics: Option<EventizationExpectedMetrics>,
    pub calibrated_eventization_applied: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AxisPolicyStrategy {
    pub strategy_name: String,
    pub threshold: Option<f64>,
    pub score_rule: Option<String>,
    pub weights: Option<std::collections::BTreeMap<String, f64>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AxisPolicyProfile {
    pub strategy_name: String,
    pub active_axes: Vec<String>,
    pub disabled_axes: Vec<String>,
    pub threshold: Option<f64>,
    pub score_rule: Option<String>,
    pub weights: Option<std::collections::BTreeMap<String, f64>>,
    pub expected_metrics: Option<CandidateActivationMetrics>,
    pub source_split: CalibrationSplit,
    pub diagnostics_hash: String,
    pub profile_hash: String,
    pub warnings: Vec<String>,
}

impl Default for AxisPolicyProfile {
    fn default() -> Self {
        Self {
            strategy_name: String::new(),
            active_axes: Vec::new(),
            disabled_axes: Vec::new(),
            threshold: None,
            score_rule: None,
            weights: None,
            expected_metrics: None,
            source_split: CalibrationSplit::Calibration,
            diagnostics_hash: String::new(),
            profile_hash: String::new(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct CandidateActivationMetrics {
    pub detection_count: u64,
    pub tp: u64,
    pub fp: u64,
    pub fn_: u64,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AxisPolicyReport {
    pub axis_policy_profile_built: bool,
    pub axis_policy_profile_hash: Option<String>,
    pub axis_policy_applied: bool,
    pub axis_policy_apply_input_source: Option<String>,
    pub candidate_activation_detection_count: u64,
    pub candidate_activation_ticks: Vec<u64>,
    pub candidate_activation_metrics: Option<CandidateActivationMetrics>,
    pub candidate_activation_eventized_metrics: Option<CandidateActivationMetrics>,
    pub selected_strategy: Option<String>,
    pub active_axes: Vec<String>,
    pub score_rule: Option<String>,
    pub threshold: Option<f64>,
    pub expected_candidate_activation_metrics_from_policy_comparison:
        Option<CandidateActivationMetrics>,
    pub applied_candidate_activation_metrics: Option<CandidateActivationMetrics>,
    pub policy_apply_consistency: Option<String>,
    pub policy_apply_consistency_reason: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct FieldDiagnosticMetrics {
    pub productive_tp: u64,
    pub productive_fp: u64,
    pub productive_fn: u64,
    pub productive_precision: f64,
    pub productive_recall: f64,
    pub productive_f1: f64,
    pub field_tp: u64,
    pub field_fp: u64,
    pub field_fn: u64,
    pub field_precision: f64,
    pub field_recall: f64,
    pub field_f1: f64,
    pub eventized_field_tp: u64,
    pub eventized_field_fp: u64,
    pub eventized_field_fn: u64,
    pub eventized_field_precision: f64,
    pub eventized_field_recall: f64,
    pub eventized_field_f1: f64,
    pub fp_reduction_ratio: Option<f64>,
    pub condensation_gain_f1: Option<f64>,
    pub field_signal_present: bool,
    pub eventized_signal_present: bool,
    pub productive_detector_validated: bool,
    pub diagnostic_only: bool,
    pub warnings: Vec<String>,
}

pub type FieldDiagnosticAggregate = FieldDiagnosticMetrics;

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct FieldDiagnosticReport {
    pub metrics: FieldDiagnosticMetrics,
    pub summary: FieldDiagnosticSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct FieldDiagnosticSummary {
    pub productive_detector_validated: bool,
    pub diagnostic_only: bool,
    pub field_signal_present: bool,
    pub eventized_signal_present: bool,
    pub productive_f1: f64,
    pub field_f1: f64,
    pub eventized_field_f1: f64,
    pub field_recall: f64,
    pub eventized_field_recall: f64,
    pub field_fp: u64,
    pub eventized_field_fp: u64,
    pub fp_reduction_ratio: Option<f64>,
    pub condensation_gain_f1: Option<f64>,
    pub interpretation: Vec<String>,
}

pub fn build_field_diagnostic_report(
    productive: Option<&Metrics>,
    field: Option<&CandidateActivationMetrics>,
    eventized: Option<&EventizationExpectedMetrics>,
) -> FieldDiagnosticReport {
    let productive = productive.cloned().unwrap_or_default();
    let field = field.cloned().unwrap_or_default();
    let eventized = eventized.cloned().unwrap_or_default();
    let fp_reduction_ratio = if eventized.hypothetical_fp > 0 {
        Some(field.fp as f64 / eventized.hypothetical_fp as f64)
    } else {
        None
    };
    let condensation_gain_f1 = if field.f1 > 0.0 {
        Some(eventized.hypothetical_f1 / field.f1)
    } else {
        None
    };
    let mut warnings = vec![
        "diagnostic_only: field metrics do not mutate productive pse_metrics".into(),
        "productive_pse_detector_not_validated".into(),
    ];
    if field.tp > 0 && field.fp > field.tp {
        warnings.push("field_signal_present_but_high_false_positive_load".into());
    }
    if eventized.hypothetical_fp < field.fp {
        warnings.push("eventization_reduces_fp_but_requires_split_validation".into());
    }
    if field.fp > 0 && eventized.hypothetical_fp == 0 {
        warnings.push("fp_reduction_ratio_undefined_eventized_fp_zero".into());
    }
    let metrics = FieldDiagnosticMetrics {
        productive_tp: productive.tp,
        productive_fp: productive.fp,
        productive_fn: productive.fn_,
        productive_precision: productive.precision,
        productive_recall: productive.recall,
        productive_f1: productive.f1,
        field_tp: field.tp,
        field_fp: field.fp,
        field_fn: field.fn_,
        field_precision: field.precision,
        field_recall: field.recall,
        field_f1: field.f1,
        eventized_field_tp: eventized.hypothetical_tp,
        eventized_field_fp: eventized.hypothetical_fp,
        eventized_field_fn: eventized.hypothetical_fn,
        eventized_field_precision: eventized.hypothetical_precision,
        eventized_field_recall: eventized.hypothetical_recall,
        eventized_field_f1: eventized.hypothetical_f1,
        fp_reduction_ratio,
        condensation_gain_f1,
        field_signal_present: field.tp > 0,
        eventized_signal_present: eventized.hypothetical_tp > 0,
        productive_detector_validated: productive.tp > 0 && productive.f1 > 0.0,
        diagnostic_only: true,
        warnings,
    };
    let mut interpretation = vec!["requires_split_validation".to_string()];
    if !metrics.productive_detector_validated {
        interpretation.push("productive_detector_not_validated".into());
    }
    if metrics.field_signal_present {
        interpretation.push("diagnostic_field_signal_present".into());
    }
    if metrics.eventized_field_fp < metrics.field_fp {
        interpretation.push("eventization_condenses_field".into());
    }
    FieldDiagnosticReport {
        summary: FieldDiagnosticSummary {
            productive_detector_validated: metrics.productive_detector_validated,
            diagnostic_only: metrics.diagnostic_only,
            field_signal_present: metrics.field_signal_present,
            eventized_signal_present: metrics.eventized_signal_present,
            productive_f1: metrics.productive_f1,
            field_f1: metrics.field_f1,
            eventized_field_f1: metrics.eventized_field_f1,
            field_recall: metrics.field_recall,
            eventized_field_recall: metrics.eventized_field_recall,
            field_fp: metrics.field_fp,
            eventized_field_fp: metrics.eventized_field_fp,
            fp_reduction_ratio: metrics.fp_reduction_ratio,
            condensation_gain_f1: metrics.condensation_gain_f1,
            interpretation,
        },
        metrics,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AxisDiscriminativityRow {
    pub axis: String,
    pub gt_mean: f64,
    pub out_mean: f64,
    pub delta: f64,
    pub direction: String,
    pub discriminative: bool,
    pub suggested_role: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PolicyComparisonRow {
    pub policy: String,
    pub hypothetical_detection_count: u64,
    pub hypothetical_tp: u64,
    pub hypothetical_fp: u64,
    pub hypothetical_fn: u64,
    pub hypothetical_precision: f64,
    pub hypothetical_recall: f64,
    pub hypothetical_f1: f64,
    pub active_axes: Vec<String>,
    pub disabled_axes: Vec<String>,
    pub reason: String,
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
            axis_discriminativity: Vec::new(),
            policy_comparison: Vec::new(),
            best_policy_by_f1: None,
            best_policy_active_axes: Vec::new(),
            axis_policy_report: AxisPolicyReport::default(),
            eventization_report: EventizationReport::default(),
            field_diagnostic_report: FieldDiagnosticReport::default(),
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
            axis_discriminativity: Vec::new(),
            policy_comparison: Vec::new(),
            best_policy_by_f1: None,
            best_policy_active_axes: Vec::new(),
            axis_policy_report: AxisPolicyReport::default(),
            eventization_report: EventizationReport::default(),
            field_diagnostic_report: FieldDiagnosticReport::default(),
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
        axis_discriminativity: Vec::new(),
        policy_comparison: Vec::new(),
        best_policy_by_f1: None,
        best_policy_active_axes: Vec::new(),
        axis_policy_report: AxisPolicyReport::default(),
        eventization_report: EventizationReport::default(),
        field_diagnostic_report: FieldDiagnosticReport::default(),
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

    #[test]
    fn axis_report_marks_non_discriminative_axis() {
        let ticks = vec![
            GateTickDiagnostic {
                tick: 1,
                in_ground_truth_window: true,
                gate_d: 0.9,
                gate_q: 0.1,
                gate_r: 0.2,
                gate_g: 0.2,
                gate_j: 0.2,
                gate_p: 0.2,
                gate_n: 0.2,
                gate_k: 0.2,
                kairos: false,
            },
            GateTickDiagnostic {
                tick: 2,
                in_ground_truth_window: false,
                gate_d: 0.2,
                gate_q: 0.8,
                gate_r: 0.2,
                gate_g: 0.2,
                gate_j: 0.2,
                gate_p: 0.2,
                gate_n: 0.2,
                gate_k: 0.2,
                kairos: false,
            },
        ];
        let mut report = CalibrationReport::default();
        augment_calibration_report_with_counterfactuals(&mut report, &[gt(1, 2, "e")], &ticks);
        let q = report
            .axis_discriminativity
            .iter()
            .find(|r| r.axis == "q")
            .unwrap();
        assert!(!q.discriminative);
        assert_eq!(q.suggested_role, "fail_closed");
    }

    #[test]
    fn policy_comparison_does_not_change_pse_metrics_and_is_deterministic() {
        let result = scenarios::ScenarioResult {
            scenario: "diag".into(),
            n_observations: 3,
            tolerance_ticks: 0,
            ground_truth: vec![gt(1, 2, "event")],
            detections: vec![Detection::new(1, 0.9, "pse_crystal")],
            metrics: Metrics::default(),
            runner_diagnostics: RunnerDiagnostics {
                gate_ticks: vec![
                    GateTickDiagnostic {
                        tick: 1,
                        in_ground_truth_window: true,
                        gate_d: 0.9,
                        gate_q: 0.9,
                        gate_r: 0.9,
                        gate_g: 0.9,
                        gate_j: 0.9,
                        gate_p: 0.9,
                        gate_n: 0.9,
                        gate_k: 0.9,
                        kairos: false,
                    },
                    GateTickDiagnostic {
                        tick: 2,
                        in_ground_truth_window: false,
                        gate_d: 0.1,
                        gate_q: 0.1,
                        gate_r: 0.1,
                        gate_g: 0.1,
                        gate_j: 0.1,
                        gate_p: 0.1,
                        gate_n: 0.1,
                        gate_k: 0.1,
                        kairos: false,
                    },
                ],
                ..RunnerDiagnostics::default()
            },
        };
        let out1 = build_json_output(&result, br#"{}"#);
        let out2 = build_json_output(&result, br#"{}"#);
        assert_eq!(out1.pse_metrics, out2.pse_metrics);
        assert_eq!(
            out1.pse_debug.calibration_report.best_policy_by_f1,
            out2.pse_debug.calibration_report.best_policy_by_f1
        );
        assert_eq!(
            out1.pse_debug
                .calibration_report
                .policy_comparison
                .first()
                .unwrap()
                .policy,
            out2.pse_debug
                .calibration_report
                .policy_comparison
                .first()
                .unwrap()
                .policy
        );
        assert_eq!(
            out1.pse_debug
                .calibration_report
                .eventization_report
                .eventization_profile_hash,
            out2.pse_debug
                .calibration_report
                .eventization_report
                .eventization_profile_hash
        );
    }

    #[test]
    fn eventization_hash_changes_when_strategy_changes() {
        let mut report = CalibrationReport::default();
        report.policy_comparison = vec![PolicyComparisonRow {
            policy: "weighted_axis_score".into(),
            hypothetical_detection_count: 100,
            hypothetical_tp: 10,
            hypothetical_fp: 80,
            hypothetical_fn: 5,
            hypothetical_precision: 0.111,
            hypothetical_recall: 0.667,
            hypothetical_f1: 0.19,
            active_axes: vec!["d".into()],
            disabled_axes: vec![],
            reason: "test".into(),
        }];
        let axis = vec![AxisDiscriminativityRow {
            axis: "d".into(),
            gt_mean: 0.9,
            out_mean: 0.1,
            delta: 0.8,
            direction: "gt>out".into(),
            discriminative: true,
            suggested_role: "mandatory".into(),
        }];
        let (p1, _) = build_eventization_profile(&report, &axis).unwrap();
        report.policy_comparison[0].hypothetical_fp = 20;
        let (p2, _) = build_eventization_profile(&report, &axis).unwrap();
        assert_ne!(p1.profile_hash, p2.profile_hash);
    }

    #[test]
    fn eventization_warns_on_high_fp_load() {
        let mut report = CalibrationReport::default();
        report.policy_comparison = vec![PolicyComparisonRow {
            policy: "discriminative_axes_only".into(),
            hypothetical_detection_count: 300,
            hypothetical_tp: 2,
            hypothetical_fp: 196,
            hypothetical_fn: 1,
            hypothetical_precision: 0.01,
            hypothetical_recall: 0.66,
            hypothetical_f1: 0.02,
            active_axes: vec!["j".into()],
            disabled_axes: vec![],
            reason: "test".into(),
        }];
        let (profile, _) = build_eventization_profile(&report, &[]).unwrap();
        assert!(profile.warnings.iter().any(|w| w.contains("high FP load")));
    }

    #[test]
    fn eventization_apply_is_hash_bound_and_explicit() {
        let mut report = CalibrationReport::default();
        report.policy_comparison = vec![PolicyComparisonRow {
            policy: "weighted_axis_score".into(),
            hypothetical_detection_count: 10,
            hypothetical_tp: 2,
            hypothetical_fp: 8,
            hypothetical_fn: 1,
            hypothetical_precision: 0.2,
            hypothetical_recall: 0.66,
            hypothetical_f1: 0.3,
            active_axes: vec!["d".into()],
            disabled_axes: vec![],
            reason: "test".into(),
        }];
        let (profile, _) = build_eventization_profile(&report, &[]).unwrap();
        let dets = vec![
            Detection::new(1, 0.9, "pse_crystal"),
            Detection::new(2, 0.8, "pse_crystal"),
        ];
        assert!(apply_frozen_eventization_profile(&profile, "wrong", &dets, &[]).is_err());
        let (eventized, rep) =
            apply_frozen_eventization_profile(&profile, &profile.profile_hash, &dets, &[]).unwrap();
        assert!(rep.calibrated_eventization_applied);
        assert!(eventized.len() <= dets.len());
        assert_eq!(
            rep.eventization_apply_input_source.as_deref(),
            Some("productive_pse_detections")
        );
        assert_eq!(rep.eventization_apply_input_count, Some(dets.len() as u64));
        assert!(rep.applied_diagnostic_eventized_metrics.is_some());
    }

    #[test]
    fn eventization_best_f1_selects_cooldown_10_over_20() {
        let mut report = CalibrationReport::default();
        report.policy_comparison = vec![PolicyComparisonRow {
            policy: "weighted_axis_score".into(),
            hypothetical_detection_count: 69,
            hypothetical_tp: 1,
            hypothetical_fp: 68,
            hypothetical_fn: 0,
            hypothetical_precision: 0.014,
            hypothetical_recall: 1.0,
            hypothetical_f1: 0.029,
            active_axes: vec!["d".into()],
            disabled_axes: vec![],
            reason: "test".into(),
        }];
        let (profile, expected) = build_eventization_profile(&report, &[]).unwrap();
        assert_eq!(profile.strategy.strategy_name, "cooldown_10");
        assert!(expected.hypothetical_f1 > 0.0);
    }

    #[test]
    fn eventization_report_separates_built_and_applied_metrics() {
        let mut report = CalibrationReport::default();
        report.policy_comparison = vec![PolicyComparisonRow {
            policy: "weighted_axis_score".into(),
            hypothetical_detection_count: 10,
            hypothetical_tp: 2,
            hypothetical_fp: 8,
            hypothetical_fn: 1,
            hypothetical_precision: 0.2,
            hypothetical_recall: 0.66,
            hypothetical_f1: 0.3,
            active_axes: vec!["d".into()],
            disabled_axes: vec![],
            reason: "test".into(),
        }];
        let (profile, built_expected) = build_eventization_profile(&report, &[]).unwrap();
        let dets = vec![Detection::new(1, 0.9, "pse_crystal")];
        let (_eventized, rep) =
            apply_frozen_eventization_profile(&profile, &profile.profile_hash, &dets, &[]).unwrap();
        assert_eq!(
            rep.expected_eventized_metrics,
            rep.applied_diagnostic_eventized_metrics
        );
        assert!(rep.built_expected_eventized_metrics.is_none());
        assert_ne!(built_expected.hypothetical_detection_count, 0);
    }

    #[test]
    fn build_axis_policy_profile_uses_weighted_strategy_when_best() {
        let mut report = CalibrationReport::default();
        report.axis_discriminativity = vec![
            AxisDiscriminativityRow {
                axis: "d".into(),
                delta: 0.8,
                discriminative: true,
                ..Default::default()
            },
            AxisDiscriminativityRow {
                axis: "j".into(),
                delta: 0.2,
                discriminative: true,
                ..Default::default()
            },
        ];
        report.policy_comparison = vec![
            PolicyComparisonRow {
                policy: "weighted_axis_score".into(),
                hypothetical_f1: 0.2,
                hypothetical_detection_count: 3,
                hypothetical_tp: 1,
                hypothetical_fp: 2,
                hypothetical_fn: 0,
                hypothetical_precision: 0.33,
                hypothetical_recall: 1.0,
                active_axes: vec!["d".into(), "j".into()],
                ..Default::default()
            },
            PolicyComparisonRow {
                policy: "current_fail_closed_all_axes".into(),
                hypothetical_f1: 0.1,
                ..Default::default()
            },
        ];
        let profile = build_axis_policy_profile(&report).unwrap();
        assert_eq!(profile.strategy_name, "weighted_axis_score");
        assert_eq!(
            profile.score_rule.as_deref(),
            Some("weighted_mean_active_axes>=threshold")
        );
        assert!(profile.weights.as_ref().unwrap().contains_key("d"));
    }

    #[test]
    fn apply_weighted_axis_policy_reproduces_weighted_series_and_not_and_logic() {
        let ticks = vec![
            GateTickDiagnostic {
                tick: 1,
                gate_d: 1.0,
                gate_j: 0.0,
                gate_q: 0.0,
                gate_r: 0.3,
                gate_g: 0.0,
                gate_p: 0.0,
                gate_n: 0.0,
                gate_k: 0.0,
                in_ground_truth_window: false,
                kairos: false,
            },
            GateTickDiagnostic {
                tick: 2,
                gate_d: 0.0,
                gate_j: 1.0,
                gate_q: 0.0,
                gate_r: 0.3,
                gate_g: 0.0,
                gate_p: 0.0,
                gate_n: 0.0,
                gate_k: 0.0,
                in_ground_truth_window: true,
                kairos: false,
            },
            GateTickDiagnostic {
                tick: 3,
                gate_d: 0.8,
                gate_j: 0.8,
                gate_q: 0.0,
                gate_r: 0.3,
                gate_g: 0.0,
                gate_p: 0.0,
                gate_n: 0.0,
                gate_k: 0.0,
                in_ground_truth_window: false,
                kairos: false,
            },
        ];
        let mut weights = std::collections::BTreeMap::new();
        weights.insert("d".into(), 0.8);
        weights.insert("j".into(), 0.2);
        let mut profile = AxisPolicyProfile {
            strategy_name: "weighted_axis_score".into(),
            active_axes: vec!["d".into(), "j".into()],
            disabled_axes: vec![],
            threshold: Some(0.5),
            score_rule: Some("weighted_mean_active_axes>=threshold".into()),
            weights: Some(weights),
            expected_metrics: Some(CandidateActivationMetrics {
                detection_count: 2,
                tp: 0,
                fp: 2,
                fn_: 1,
                precision: 0.0,
                recall: 0.0,
                f1: 0.0,
            }),
            ..Default::default()
        };
        profile.profile_hash = sha256_hex(&serde_json::to_vec(&profile).unwrap());
        let (dets, rep) = apply_frozen_axis_policy_profile(
            &profile,
            &profile.profile_hash,
            &ticks,
            &[gt(2, 2, "e")],
        )
        .unwrap();
        assert_eq!(dets.iter().map(|d| d.tick).collect::<Vec<_>>(), vec![1, 3]);
        assert_eq!(rep.policy_apply_consistency.as_deref(), Some("match"));
    }

    #[test]
    fn field_diagnostic_ratios_and_flags_are_computed() {
        let productive = Metrics {
            tp: 0,
            fp: 0,
            fn_: 1,
            precision: 0.0,
            recall: 0.0,
            f1: 0.0,
            auprc: None,
        };
        let field = CandidateActivationMetrics {
            detection_count: 10,
            tp: 1,
            fp: 8,
            fn_: 0,
            precision: 0.111,
            recall: 1.0,
            f1: 0.2,
        };
        let eventized = EventizationExpectedMetrics {
            hypothetical_detection_count: 3,
            hypothetical_tp: 1,
            hypothetical_fp: 2,
            hypothetical_fn: 0,
            hypothetical_precision: 0.333,
            hypothetical_recall: 1.0,
            hypothetical_f1: 0.5,
        };
        let rep = build_field_diagnostic_report(Some(&productive), Some(&field), Some(&eventized));
        assert_eq!(rep.metrics.fp_reduction_ratio, Some(4.0));
        assert_eq!(rep.metrics.condensation_gain_f1, Some(2.5));
        assert!(!rep.metrics.productive_detector_validated);
        assert!(rep.metrics.field_signal_present);
        assert!(rep.metrics.eventized_signal_present);
        assert_eq!(rep.summary.fp_reduction_ratio, Some(4.0));
        assert_eq!(rep.summary.condensation_gain_f1, Some(2.5));
        assert!(rep.summary.diagnostic_only);
        assert_eq!(
            rep.summary.interpretation,
            vec![
                "requires_split_validation".to_string(),
                "productive_detector_not_validated".to_string(),
                "diagnostic_field_signal_present".to_string(),
                "eventization_condenses_field".to_string()
            ]
        );
    }
}
