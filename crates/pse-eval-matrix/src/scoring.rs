//! Scoring (§6).
//!
//! `score_ledger` is the **only** path from observed metric data to
//! aggregated values. It does NOT recompute metrics — it aggregates
//! `MetricObservation`s using the declared `AggregationPolicy`.
//!
//! Capability profiles, Safety-Adjusted Utility, Cognition Uplift and
//! Layer Marginal Utility are derived from those aggregates plus the
//! metric direction.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ledger::{EvaluationRunLedger, RunStatus};
use crate::metrics::{AggregationPolicy, MetricDirection, MetricSpec};
use crate::primitives::{fixed_sub, EvalError, Fixed, Hash256};
use crate::reports::{TrialReport, VariantSummary, WorkloadSummary};

/// `CapabilityProfile` (§6.2, §14.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityProfile {
    /// Variant id.
    pub variant_id: String,
    /// `U_task`.
    pub task_utility: Fixed,
    /// `U_replay`.
    pub replay_utility: Fixed,
    /// `U_safety`.
    pub safety_utility: Fixed,
    /// `U_cognition`.
    pub cognition_utility: Fixed,
    /// `U_efficiency`.
    pub efficiency_utility: Fixed,
    /// `U_calibration`.
    pub calibration_utility: Fixed,
    /// `U_robustness`.
    pub robustness_utility: Fixed,
    /// `SAU = U_task · U_replay · U_safety · (1 − P_catastrophic)`.
    pub safety_adjusted_utility: Fixed,
}

/// Aggregate every variant's metric observations into per-variant
/// summaries + per-workload summaries, restricted to **valid**
/// observations (§7.2 rule 5: replay-mismatched runs are excluded).
#[allow(clippy::type_complexity)]
pub fn score_ledger(
    ledger: &EvaluationRunLedger,
    reports: &[TrialReport],
    metric_specs: &[MetricSpec],
) -> Result<(Vec<VariantSummary>, Vec<WorkloadSummary>, Vec<Hash256>), EvalError> {
    // Build a fast lookup from entry → trial report.
    let report_by_id: BTreeMap<&Hash256, &TrialReport> =
        reports.iter().map(|r| (&r.trial_id, r)).collect();

    // Per-variant aggregation buckets.
    let mut variant_buckets: BTreeMap<String, BTreeMap<String, Vec<Fixed>>> = BTreeMap::new();
    let mut variant_safety_events: BTreeMap<String, u64> = BTreeMap::new();
    let mut completed_per_variant: BTreeMap<String, u64> = BTreeMap::new();
    let mut invalid_per_variant: BTreeMap<String, u64> = BTreeMap::new();
    let mut workload_buckets: BTreeMap<String, BTreeMap<String, Vec<Fixed>>> = BTreeMap::new();
    let mut workload_trial_counts: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();

    let mut invalid_runs: Vec<Hash256> = Vec::new();

    for entry in &ledger.entries {
        // Find the trial report by report_hash. We allow either keying
        // (some pipelines use `trial_id`, some use `report_hash`).
        let report = reports
            .iter()
            .find(|r| r.run_descriptor_hash == entry.run_descriptor_hash)
            .or_else(|| report_by_id.get(&entry.report_hash).copied())
            .ok_or_else(|| EvalError::Schema {
                reason: format!(
                    "ledger entry {} has no matching trial report",
                    entry.entry_id.hex()
                ),
            })?;

        if entry.status != RunStatus::Completed {
            *invalid_per_variant
                .entry(entry.variant_id.clone())
                .or_insert(0) += 1;
            invalid_runs.push(report.trial_id.clone());
            continue;
        }
        *completed_per_variant
            .entry(entry.variant_id.clone())
            .or_insert(0) += 1;
        *variant_safety_events
            .entry(entry.variant_id.clone())
            .or_insert(0) += report.outputs.safety_event_count;

        let variant_bucket = variant_buckets.entry(entry.variant_id.clone()).or_default();
        let workload_bucket = workload_buckets
            .entry(entry.workload_id.clone())
            .or_default();
        *workload_trial_counts
            .entry(entry.workload_id.clone())
            .or_default()
            .entry(entry.variant_id.clone())
            .or_insert(0) += 1;

        for obs in &report.metrics {
            if !obs.valid {
                continue;
            }
            variant_bucket
                .entry(obs.metric_id.clone())
                .or_default()
                .push(obs.value.clone());
            workload_bucket
                .entry(format!("{}::{}", entry.variant_id, obs.metric_id))
                .or_default()
                .push(obs.value.clone());
        }
    }

    // Map metric_id → spec for aggregation lookups.
    let mut spec_by_id: BTreeMap<&str, &MetricSpec> = BTreeMap::new();
    for ms in metric_specs {
        spec_by_id.insert(ms.metric_id.as_str(), ms);
    }

    // Build per-variant summaries.
    let mut variant_summaries = Vec::new();
    for (variant_id, bucket) in variant_buckets {
        let mut metric_values = BTreeMap::new();
        for (metric_id, values) in bucket {
            // Metric ids not in the declared spec list are dropped
            // from aggregation; they remain in the trial reports.
            let Some(spec) = spec_by_id.get(metric_id.as_str()).copied() else {
                continue;
            };
            metric_values.insert(metric_id, aggregate(&values, spec.aggregation));
        }
        let summary = VariantSummary {
            variant_id: variant_id.clone(),
            completed_trials: *completed_per_variant.get(&variant_id).unwrap_or(&0),
            invalid_trials: *invalid_per_variant.get(&variant_id).unwrap_or(&0),
            metric_values,
            safety_event_count: *variant_safety_events.get(&variant_id).unwrap_or(&0),
        };
        variant_summaries.push(summary);
    }

    // Build per-workload summaries (primary metric mean per variant).
    let mut workload_summaries = Vec::new();
    for (workload_id, bucket) in workload_buckets {
        let mut primary_means: BTreeMap<String, Fixed> = BTreeMap::new();
        for (key, values) in bucket {
            let mut parts = key.splitn(2, "::");
            let variant_id = parts.next().unwrap_or("").to_string();
            let metric_id = parts.next().unwrap_or("");
            if let Some(spec) = spec_by_id.get(metric_id) {
                if !spec.primary {
                    continue;
                }
                let agg = aggregate(&values, spec.aggregation);
                // Record one primary metric per variant per workload — if
                // multiple primaries are declared, the last one wins
                // alphabetically (deterministic, stable). The summary
                // report is the place to expand to multi-metric tables.
                primary_means.insert(variant_id, agg);
            }
        }
        workload_summaries.push(WorkloadSummary {
            workload_id: workload_id.clone(),
            trial_counts: workload_trial_counts
                .remove(&workload_id)
                .unwrap_or_default(),
            primary_metric_means: primary_means,
        });
    }
    Ok((variant_summaries, workload_summaries, invalid_runs))
}

/// Build a `CapabilityProfile` for one variant from its summary +
/// metric specs. This routine ONLY reads the summary; it does not
/// re-derive metrics.
pub fn score_capability_profile(
    summary: &VariantSummary,
    metric_specs: &[MetricSpec],
) -> Result<CapabilityProfile, EvalError> {
    let by_id: BTreeMap<&str, &MetricSpec> = metric_specs
        .iter()
        .map(|m| (m.metric_id.as_str(), m))
        .collect();
    let zero = Fixed::Rational { num: 0, den: 1 };
    let one = Fixed::Rational { num: 1, den: 1 };

    let lookup = |metric: &str| -> Fixed {
        match (summary.metric_values.get(metric), by_id.get(metric)) {
            (Some(value), Some(spec)) => match spec.direction {
                // For `LowerIsBetter` we map value → `1 − value` (clamped
                // to [0, 1]) so the utility scale is uniform.
                MetricDirection::LowerIsBetter => clamp01(fixed_sub(&one, value)),
                _ => value.clone(),
            },
            _ => zero.clone(),
        }
    };

    let task_utility = lookup("task_success");
    let replay_utility = lookup("replay_identity");
    let safety_utility = lookup("false_commit_rate");
    let efficiency_utility = clamp01(Fixed::Rational { num: 1, den: 1 }); // throughput/latency aren't normalised yet — Stub.
    let calibration_utility = lookup("hold_correctness");
    let robustness_utility = clamp01(Fixed::Rational { num: 1, den: 1 });
    let cognition_utility = summary
        .metric_values
        .get("panorama_coverage")
        .cloned()
        .unwrap_or(zero.clone());

    // P_catastrophic = safety_event_count / max(1, completed_trials).
    let p_cat = if summary.completed_trials == 0 {
        zero.clone()
    } else {
        Fixed::Rational {
            num: summary.safety_event_count as i128,
            den: summary.completed_trials as i128,
        }
    };
    let one_minus_pcat = clamp01(fixed_sub(&one, &p_cat));
    let safety_adjusted_utility = mul(
        &mul(&mul(&task_utility, &replay_utility), &safety_utility),
        &one_minus_pcat,
    );

    Ok(CapabilityProfile {
        variant_id: summary.variant_id.clone(),
        task_utility,
        replay_utility,
        safety_utility,
        cognition_utility,
        efficiency_utility,
        calibration_utility,
        robustness_utility,
        safety_adjusted_utility,
    })
}

/// Layer Marginal Utility per §6.5: `LMU_i = Score(B_i) − Score(B_{i−1})`.
pub fn layer_marginal_utility(base_score: &Fixed, upper_score: &Fixed) -> Fixed {
    fixed_sub(upper_score, base_score)
}

/// Cognition Uplift per §6.4: `CU = Score(B6) − Score(B5)`.
pub fn cognition_uplift(score_b5: &Fixed, score_b6: &Fixed) -> Fixed {
    fixed_sub(score_b6, score_b5)
}

/// `SAU = U_task · U_replay · U_safety · (1 − P_catastrophic)` per §6.3.
pub fn safety_adjusted_utility(
    u_task: &Fixed,
    u_replay: &Fixed,
    u_safety: &Fixed,
    p_catastrophic: &Fixed,
) -> Fixed {
    let one = Fixed::Rational { num: 1, den: 1 };
    let factor = clamp01(fixed_sub(&one, p_catastrophic));
    mul(&mul(&mul(u_task, u_replay), u_safety), &factor)
}

fn aggregate(values: &[Fixed], policy: AggregationPolicy) -> Fixed {
    if values.is_empty() {
        return Fixed::Rational { num: 0, den: 1 };
    }
    match policy {
        AggregationPolicy::Mean => {
            let mut s = values[0].clone();
            for v in &values[1..] {
                s = crate::primitives::fixed_add(&s, v);
            }
            // Divide by count using rational arithmetic.
            divide_by(&s, values.len() as i128)
        }
        AggregationPolicy::Sum => {
            let mut s = values[0].clone();
            for v in &values[1..] {
                s = crate::primitives::fixed_add(&s, v);
            }
            s
        }
        AggregationPolicy::Median => {
            let mut sorted = values.to_vec();
            sorted.sort();
            sorted[sorted.len() / 2].clone()
        }
        AggregationPolicy::Max => values
            .iter()
            .max()
            .cloned()
            .unwrap_or(Fixed::Rational { num: 0, den: 1 }),
        AggregationPolicy::Min => values
            .iter()
            .min()
            .cloned()
            .unwrap_or(Fixed::Rational { num: 0, den: 1 }),
        AggregationPolicy::PassThrough => values[0].clone(),
    }
}

fn divide_by(x: &Fixed, n: i128) -> Fixed {
    if n == 0 {
        return Fixed::Rational { num: 0, den: 1 };
    }
    let (xn, xd) = match x {
        Fixed::FixedI64 { raw, scale } => (*raw as i128, 10i128.pow(*scale)),
        Fixed::Rational { num, den } => (*num, *den),
    };
    crate::primitives::normalize_rational(xn, xd.saturating_mul(n))
}

fn mul(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = pair(a);
    let (bn, bd) = pair(b);
    crate::primitives::normalize_rational(an.saturating_mul(bn), ad.saturating_mul(bd))
}

fn pair(x: &Fixed) -> (i128, i128) {
    match x {
        Fixed::FixedI64 { raw, scale } => (*raw as i128, 10i128.pow(*scale)),
        Fixed::Rational { num, den } => (*num, *den),
    }
}

fn clamp01(x: Fixed) -> Fixed {
    let zero = Fixed::Rational { num: 0, den: 1 };
    let one = Fixed::Rational { num: 1, den: 1 };
    if x.cmp(&zero) == std::cmp::Ordering::Less {
        zero
    } else if x.cmp(&one) == std::cmp::Ordering::Greater {
        one
    } else {
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_mean_is_floatfree() {
        let v = vec![
            Fixed::quantize(0.5, 9).unwrap(),
            Fixed::quantize(0.7, 9).unwrap(),
            Fixed::quantize(0.9, 9).unwrap(),
        ];
        let m = aggregate(&v, AggregationPolicy::Mean);
        assert_eq!(m, Fixed::quantize(0.7, 9).unwrap());
    }

    #[test]
    fn lmu_is_signed_difference() {
        let lo = Fixed::quantize(0.5, 9).unwrap();
        let hi = Fixed::quantize(0.7, 9).unwrap();
        assert_eq!(
            layer_marginal_utility(&lo, &hi),
            Fixed::quantize(0.2, 9).unwrap()
        );
    }
}
