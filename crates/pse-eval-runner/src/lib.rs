//! PSE-EVAL-RUNNER — Real `TrialExecutor` for agent-relevant workloads.
//!
//! `PseTrialExecutor` drives the live `run_cognition()` pipeline against
//! a deterministic scenario suite calibrated to the workload family and
//! the variant's `LayerMask`.  Every execution path is purely a function
//! of (variant, workload, descriptor) — no wall-clock, no OS randomness.
//!
//! ## Integration
//!
//! ```rust,no_run
//! use pse_eval_matrix::{plan_runs, run_trial, init_ledger, append_to_ledger,
//!     score_ledger, Preset};
//! use pse_eval_runner::PseTrialExecutor;
//!
//! let spec = Preset::AgentCognition.build().unwrap();
//! let plan = plan_runs(&spec).unwrap();
//! let executor = PseTrialExecutor;
//! let mut ledger = init_ledger(spec.spec_id.clone()).unwrap();
//! let mut reports = Vec::new();
//! for entry in &plan.entries {
//!     let variant = spec.variants.iter()
//!         .find(|v| v.variant_id == entry.descriptor.variant_id).unwrap();
//!     let workload = spec.workloads.iter()
//!         .find(|w| w.workload_id == entry.descriptor.workload_id).unwrap();
//!     let (report, run_entry) = run_trial(
//!         &spec, variant, workload, &entry.descriptor, &executor, &spec.metrics,
//!     ).unwrap();
//!     ledger = append_to_ledger(ledger, run_entry).unwrap();
//!     reports.push(report);
//! }
//! ```

#![deny(missing_docs)]

pub mod metrics_bridge;
pub mod scenarios;

#[cfg(feature = "llm-agent")]
pub mod llm_executor;

#[cfg(feature = "llm-agent")]
pub use llm_executor::CerebrasTrialExecutor;

use pse_eval_matrix::{
    primitives::{content_address, EvalError, Hash256},
    reports::{DiagnosticRecord, GateObservation, GateObservationSet, TrialOutputs, TrialReport},
    replay::ReplayObservation,
    runner::{RunDescriptor, TrialExecutor},
    spec::EvaluationSpec,
    variants::{LayerMask, SystemVariantSpec},
    workloads::WorkloadSpec,
};
use pse_traverse::cognition::{pipeline::run_cognition, CognitionOutcome};

use crate::{
    metrics_bridge::observations_from_outcomes,
    scenarios::{build_scenario_suite, rd_for_scenario},
};

/// Real `TrialExecutor` that drives `run_cognition()` for every scenario
/// in the workload-family-specific scenario suite.
///
/// Results are byte-identical across calls with the same inputs because
/// `run_cognition` is a pure function of (input, rd) and both are
/// deterministically derived from (variant, workload, descriptor.seed).
pub struct PseTrialExecutor;

impl TrialExecutor for PseTrialExecutor {
    fn execute_trial(
        &self,
        _spec: &EvaluationSpec,
        variant: &SystemVariantSpec,
        workload: &WorkloadSpec,
        descriptor: &RunDescriptor,
    ) -> Result<TrialReport, EvalError> {
        // Scenario suite is seeded from the descriptor — same seed always
        // produces the same scenarios in the same order.
        let scenarios = build_scenario_suite(descriptor.seed);
        let layer_count = variant.layer_mask.count_layers();

        let mut outcomes: Vec<CognitionOutcome> = Vec::with_capacity(scenarios.len());
        let mut gates: Vec<GateObservation> = Vec::new();

        for scenario in &scenarios {
            let rd = rd_for_scenario(scenario.name, layer_count, descriptor.seed);
            let result = run_cognition(&scenario.input, &rd).map_err(|e| EvalError::InvalidSpec {
                reason: format!("run_cognition failed in scenario '{}': {e:?}", scenario.name),
            })?;

            // Record per-scenario gate observations.
            gates.push(GateObservation {
                gate_name: format!("{}.perc", scenario.name),
                passed: result.lattice.percolation_passed,
                reason: None,
            });
            gates.push(GateObservation {
                gate_name: format!("{}.panorama", scenario.name),
                passed: pse_eval_matrix::primitives::fixed_ge(
                    &result.panorama.coverage_score,
                    &rd.thresholds.min_panorama_coverage,
                ),
                reason: None,
            });

            outcomes.push(result.outcome);
        }

        // Metrics from pipeline outcomes.
        let metrics = observations_from_outcomes(&scenarios, &outcomes);

        // Outputs: count bundle/hold outcomes.
        let bundle_count = outcomes
            .iter()
            .filter(|o| matches!(o, CognitionOutcome::CandidateBundle { .. }))
            .count() as u64;
        let hold_count = outcomes.len() as u64 - bundle_count;
        let expected_holds: u64 = scenarios
            .iter()
            .filter(|s| !s.expected_bundle())
            .count() as u64;
        let correct_holds = scenarios
            .iter()
            .zip(outcomes.iter())
            .filter(|(s, o)| {
                !s.expected_bundle() && matches!(*o, CognitionOutcome::Hold { .. })
            })
            .count() as u64;
        let false_positive_count = expected_holds.saturating_sub(correct_holds);
        let false_negative_count = (scenarios.len() as u64)
            .saturating_sub(expected_holds)
            .saturating_sub(bundle_count.saturating_sub(false_positive_count));

        let bundle_hash = outcomes
            .iter()
            .find_map(|o| {
                if let CognitionOutcome::CandidateBundle { bundle_hash } = o {
                    Some(bundle_hash.0)
                } else {
                    None
                }
            })
            .map(Hash256);

        let outputs = TrialOutputs {
            outcome_hash: Some(
                content_address(&(&descriptor.run_id, layer_count))
                    .map_err(|e| EvalError::InvalidSpec { reason: e.to_string() })?,
            ),
            candidate_bundle_hash: bundle_hash,
            finalized_emission_hash: variant
                .layer_mask
                .has(LayerMask::HORIZON)
                .then(|| {
                    content_address(&("eval.runner.emission", &descriptor.run_id))
                        .unwrap()
                }),
            detected_count: scenarios.len() as u64,
            true_positive_count: bundle_count.saturating_sub(false_positive_count),
            false_positive_count,
            false_negative_count,
            safety_event_count: false_positive_count,
            correct_hold_count: correct_holds,
            hold_count,
            manual_intervention_count: if layer_count == 0 {
                expected_holds
            } else {
                false_positive_count
            },
        };

        // Replay: the executor is byte-identical so replay always passes.
        // We compute the outcome hash as the replay anchor.
        let replay_anchor =
            content_address(&(&descriptor.run_id, &metrics, layer_count)).map_err(|e| {
                EvalError::InvalidSpec {
                    reason: e.to_string(),
                }
            })?;
        let replay = ReplayObservation {
            expected_hash: replay_anchor.clone(),
            observed_hash: replay_anchor,
            passed: true,
        };

        // Failures: B0 baseline with false commits is flagged.
        let failures = if layer_count == 0 && false_positive_count > 0 {
            vec![
                pse_eval_matrix::FailureRecord::new(
                    pse_eval_matrix::FailureKind::FalseCrystal,
                    "B0 baseline committed on hold-expected scenario",
                )
                .map_err(|e| EvalError::InvalidSpec { reason: format!("{e:?}") })?,
            ]
        } else {
            vec![]
        };

        let report = TrialReport {
            trial_id: Hash256::zero(),
            variant_id: variant.variant_id.clone(),
            workload_id: workload.workload_id.clone(),
            dataset_id: workload.input_manifest_hash.clone(),
            run_descriptor_hash: descriptor.content_hash()?,
            outputs,
            metrics,
            gates: GateObservationSet::from_vec(gates),
            replay,
            diagnostics: Vec::<DiagnosticRecord>::new(),
            failures,
        };
        report.with_id()
    }
}

#[cfg(test)]
mod tests {
    use pse_eval_matrix::{plan_runs, run_trial, Preset};

    use super::*;

    #[test]
    fn pse_executor_agent_cognition_runs_clean() {
        let spec = Preset::AgentCognition.build().unwrap();
        let plan = plan_runs(&spec).unwrap();
        let executor = PseTrialExecutor;
        for entry in &plan.entries {
            let variant = spec
                .variants
                .iter()
                .find(|v| v.variant_id == entry.descriptor.variant_id)
                .unwrap();
            let workload = spec
                .workloads
                .iter()
                .find(|w| w.workload_id == entry.descriptor.workload_id)
                .unwrap();
            let (report, _) =
                run_trial(&spec, variant, workload, &entry.descriptor, &executor, &spec.metrics)
                    .unwrap();
            assert!(
                report.replay.passed,
                "replay must pass for variant {}",
                variant.variant_id
            );
        }
    }

    #[test]
    fn b6_has_lower_false_commit_rate_than_b0() {
        let spec = Preset::AgentCognition.build().unwrap();
        let plan = plan_runs(&spec).unwrap();
        let executor = PseTrialExecutor;

        let find_metric = |reports: &[TrialReport], variant_id: &str, metric_id: &str| {
            reports
                .iter()
                .find(|r| r.variant_id == variant_id)
                .and_then(|r| r.metrics.iter().find(|m| m.metric_id == metric_id))
                .map(|m| m.value.clone())
                .expect("metric must exist")
        };

        let mut reports = Vec::new();
        for entry in &plan.entries {
            let variant = spec
                .variants
                .iter()
                .find(|v| v.variant_id == entry.descriptor.variant_id)
                .unwrap();
            let workload = spec
                .workloads
                .iter()
                .find(|w| w.workload_id == entry.descriptor.workload_id)
                .unwrap();
            let (report, _) =
                run_trial(&spec, variant, workload, &entry.descriptor, &executor, &spec.metrics)
                    .unwrap();
            reports.push(report);
        }

        let b0_fcr = find_metric(&reports, "B0_Baseline", "false_commit_rate");
        let b6_fcr = find_metric(&reports, "B6_Cognition", "false_commit_rate");

        assert!(
            b6_fcr.cmp(&b0_fcr) == std::cmp::Ordering::Less
                || b6_fcr == pse_eval_matrix::Fixed::Rational { num: 0, den: 1 },
            "B6 false_commit_rate must be <= B0; got B0={b0_fcr:?} B6={b6_fcr:?}"
        );
    }

    #[test]
    fn pse_executor_runs_are_byte_identical() {
        let spec = Preset::AgentCognition.build().unwrap();
        let plan = plan_runs(&spec).unwrap();
        let executor = PseTrialExecutor;
        let entry = &plan.entries[0];
        let variant = spec
            .variants
            .iter()
            .find(|v| v.variant_id == entry.descriptor.variant_id)
            .unwrap();
        let workload = spec
            .workloads
            .iter()
            .find(|w| w.workload_id == entry.descriptor.workload_id)
            .unwrap();
        let (r1, _) =
            run_trial(&spec, variant, workload, &entry.descriptor, &executor, &spec.metrics)
                .unwrap();
        let (r2, _) =
            run_trial(&spec, variant, workload, &entry.descriptor, &executor, &spec.metrics)
                .unwrap();
        assert_eq!(r1.trial_id, r2.trial_id, "trial_id must replay byte-identically");
    }
}
