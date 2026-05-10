//! Run planner + `TrialExecutor` trait + a deterministic synthetic
//! executor (§10.5, §16).
//!
//! The executor is intentionally pluggable: the eval-matrix crate must
//! be able to run *against* PSE / pse-traverse / cognition without
//! hard-wiring those layers. The synthetic executor is what tests and
//! the CLI smoke fixtures use; production callers wrap their real
//! pipeline in their own `TrialExecutor` impl.

use serde::{Deserialize, Serialize};

use crate::failure_taxonomy::{FailureKind, FailureRecord};
use crate::ledger::{
    append_to_ledger, init_ledger, EvaluationRunEntry, EvaluationRunLedger, RunStatus,
};
use crate::metrics::{MetricObservation, MetricSpec};
use crate::primitives::{content_address, EvalError, Fixed, Hash256};
use crate::replay::ReplayObservation;
use crate::reports::{
    DiagnosticRecord, GateObservation, GateObservationSet, TrialOutputs, TrialReport,
};
use crate::spec::EvaluationSpec;
use crate::variants::{LayerMask, SystemVariantSpec};
use crate::workloads::{WorkloadFamily, WorkloadSpec};

/// Deterministic run descriptor — every execution path content-hashes
/// the descriptor so replay is byte-identical.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RunDescriptor {
    /// Stable run id.
    pub run_id: String,
    /// Variant id.
    pub variant_id: String,
    /// Workload id.
    pub workload_id: String,
    /// Dataset id.
    pub dataset_id: Hash256,
    /// Run-local seed (NEVER wall-clock, NEVER OS-randomness).
    pub seed: u64,
    /// Spec id this descriptor anchors to.
    pub spec_id: Hash256,
}

impl RunDescriptor {
    /// Stable content hash of the descriptor.
    pub fn content_hash(&self) -> Result<Hash256, EvalError> {
        content_address(self)
    }
}

/// One row of the deterministic run plan.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EvaluationPlanEntry {
    /// Descriptor for this row.
    pub descriptor: RunDescriptor,
    /// Descriptor hash (for cross-reference).
    pub descriptor_hash: Hash256,
}

/// `EvaluationPlan` (§14, §15.1 plan).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationPlan {
    /// Content-addressed plan id.
    pub plan_id: Hash256,
    /// Anchoring spec hash.
    pub evaluation_spec_hash: Hash256,
    /// Sorted, deterministic run rows.
    pub entries: Vec<EvaluationPlanEntry>,
}

impl EvaluationPlan {
    /// Recompute `plan_id`.
    pub fn with_id(mut self) -> Result<Self, EvalError> {
        self.entries.sort();
        let mut probe = self.clone();
        probe.plan_id = Hash256::zero();
        self.plan_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Plan every (variant × workload) trial deterministically per §15.1.
pub fn plan_runs(spec: &EvaluationSpec) -> Result<EvaluationPlan, EvalError> {
    spec.validate()?;
    let mut entries = Vec::with_capacity(spec.variants.len() * spec.workloads.len());
    for variant in &spec.variants {
        for workload in &spec.workloads {
            // Seed = stable hash of (variant.config_hash, workload, spec_id) reduced to u64.
            let probe = (
                &variant.variant_id,
                &workload.workload_id,
                &workload.input_manifest_hash,
                &spec.spec_id,
            );
            let h = content_address(&probe)?;
            let seed = u64::from_le_bytes(h.0[..8].try_into().unwrap_or([0u8; 8]));
            let descriptor = RunDescriptor {
                run_id: format!("{}.{}.{}", variant.variant_id, workload.workload_id, seed),
                variant_id: variant.variant_id.clone(),
                workload_id: workload.workload_id.clone(),
                dataset_id: workload.input_manifest_hash.clone(),
                seed,
                spec_id: spec.spec_id.clone(),
            };
            let descriptor_hash = descriptor.content_hash()?;
            entries.push(EvaluationPlanEntry {
                descriptor,
                descriptor_hash,
            });
        }
    }
    let plan = EvaluationPlan {
        plan_id: Hash256::zero(),
        evaluation_spec_hash: spec.spec_id.clone(),
        entries,
    };
    plan.with_id()
}

/// Trait every trial-producing pipeline implements. The synthetic
/// executor is the reference implementation used by tests; the CLI
/// allows external pipelines to plug in.
pub trait TrialExecutor {
    /// Execute one trial deterministically. The implementation MUST
    /// be a pure function of (spec, variant, workload, descriptor) —
    /// no wall-clock, no OS randomness, no hidden state.
    fn execute_trial(
        &self,
        spec: &EvaluationSpec,
        variant: &SystemVariantSpec,
        workload: &WorkloadSpec,
        descriptor: &RunDescriptor,
    ) -> Result<TrialReport, EvalError>;
}

/// Deterministic synthetic executor used by tests and golden fixtures.
///
/// It produces metric observations whose values depend on the variant's
/// `LayerMask` only (so the variant ladder is observably monotone), with
/// a tiny seeded perturbation derived from the descriptor's content hash
/// — float-free, no platform RNG.
pub struct SyntheticTrialExecutor;

impl TrialExecutor for SyntheticTrialExecutor {
    fn execute_trial(
        &self,
        _spec: &EvaluationSpec,
        variant: &SystemVariantSpec,
        workload: &WorkloadSpec,
        descriptor: &RunDescriptor,
    ) -> Result<TrialReport, EvalError> {
        // Float-free perturbation: seed % 7 → 0..6, scaled to /1000.
        let perturbation = Fixed::Rational {
            num: (descriptor.seed % 7) as i128,
            den: 1000,
        };
        // Base task success increases with active layers; clamp at 0.95.
        let layer_count = variant.layer_mask.count_layers() as i128;
        let base_task = Fixed::Rational {
            num: 50 + 5 * layer_count,
            den: 100,
        }
        .min(Fixed::Rational { num: 95, den: 100 });
        let task_success = clamp01(crate::primitives::fixed_sub(&base_task, &perturbation));

        // F1 grows similarly but slightly slower.
        let base_f1 = Fixed::Rational {
            num: 40 + 4 * layer_count,
            den: 100,
        }
        .min(Fixed::Rational { num: 92, den: 100 });
        let f1 = clamp01(crate::primitives::fixed_sub(&base_f1, &perturbation));

        // False-commit rate decreases as layers increase. B0 baseline:
        // ~20 %. B7 full stack: ~2 %.
        let base_false = Fixed::Rational {
            num: 20i128.saturating_sub(2 * layer_count).max(2),
            den: 100,
        };
        let false_commit_rate = base_false;

        // Replay always passes for the synthetic executor (the whole
        // point of the synthetic executor is to be byte-identical).
        let replay_identity = Fixed::Rational { num: 1, den: 1 };

        // Hold correctness same shape as task_success, slightly lower.
        let hold_correctness = clamp01(crate::primitives::fixed_sub(
            &Fixed::Rational {
                num: 45 + 5 * layer_count,
                den: 100,
            },
            &perturbation,
        ));

        // Detection counts derived from f1 (informational, used by the
        // workload-specific outputs).
        let detected_count: u64 = 10;
        let true_positive_count: u64 = (8 + (layer_count.min(2) as u64)).min(detected_count);
        let false_positive_count = detected_count - true_positive_count;
        let false_negative_count: u64 = 2 - layer_count.min(2) as u64;

        // Safety events drop monotonically.
        let safety_event_count: u64 = if variant.layer_mask.count_layers() == 0 {
            3
        } else if variant.layer_mask.has(LayerMask::HORIZON) {
            0
        } else {
            1
        };

        let outputs = TrialOutputs {
            outcome_hash: Some(content_address(&("synthetic.outcome", &descriptor.run_id))?),
            candidate_bundle_hash: variant
                .layer_mask
                .has(LayerMask::COGNITION)
                .then(|| content_address(&("synthetic.bundle", &descriptor.run_id)).unwrap()),
            finalized_emission_hash: variant
                .layer_mask
                .has(LayerMask::HORIZON)
                .then(|| content_address(&("synthetic.emission", &descriptor.run_id)).unwrap()),
            detected_count,
            true_positive_count,
            false_positive_count,
            false_negative_count,
            safety_event_count,
            correct_hold_count: 4,
            hold_count: 5,
            manual_intervention_count: if variant.layer_mask.count_layers() == 0 {
                3
            } else {
                1
            },
        };

        // Build metric observations.
        let mut metrics = Vec::new();
        metrics.push(MetricObservation::ok("task_success", task_success));
        metrics.push(MetricObservation::ok("f1", f1));
        metrics.push(MetricObservation::ok(
            "precision",
            clamp01(Fixed::Rational {
                num: true_positive_count as i128,
                den: (detected_count.max(1)) as i128,
            }),
        ));
        metrics.push(MetricObservation::ok(
            "recall",
            clamp01(Fixed::Rational {
                num: true_positive_count as i128,
                den: (true_positive_count + false_negative_count).max(1) as i128,
            }),
        ));
        metrics.push(MetricObservation::ok(
            "false_commit_rate",
            false_commit_rate,
        ));
        metrics.push(MetricObservation::ok("hold_correctness", hold_correctness));
        metrics.push(MetricObservation::ok("replay_identity", replay_identity));
        metrics.push(MetricObservation::ok(
            "regression_rate",
            Fixed::Rational {
                num: 1,
                den: (layer_count.max(1) + 1) * 10,
            },
        ));
        metrics.push(MetricObservation::ok(
            "manual_intervention",
            Fixed::Rational {
                num: outputs.manual_intervention_count as i128,
                den: 1,
            },
        ));
        metrics.push(MetricObservation::ok(
            "throughput_obs_per_sec",
            Fixed::Rational {
                num: 100_000 - 5_000 * layer_count,
                den: 1,
            },
        ));
        metrics.push(MetricObservation::ok(
            "pipeline_latency_us",
            Fixed::Rational {
                num: 50 + 10 * layer_count,
                den: 1,
            },
        ));

        // Workload-family-specific extra metrics.
        match workload.family {
            WorkloadFamily::CognitionPanorama if variant.layer_mask.has(LayerMask::COGNITION) => {
                metrics.push(MetricObservation::ok(
                    "panorama_coverage",
                    Fixed::Rational {
                        num: 70 + 3 * layer_count,
                        den: 100,
                    },
                ));
            }
            WorkloadFamily::HorizonFinalization if variant.layer_mask.has(LayerMask::HORIZON) => {
                metrics.push(MetricObservation::ok(
                    "horizon_alignment",
                    Fixed::Rational {
                        num: 80 + 2 * layer_count,
                        den: 100,
                    },
                ));
            }
            WorkloadFamily::MorphoCellSubstrate => {
                // PHASEMATRIX-HIVEMIND-03 cell-substrate observations.
                // Variants without the CELL_SUBSTRATE bit register
                // floor values (i.e. they do not run the substrate, so
                // every primary substrate metric stays at the
                // fail-closed floor and the boundary-violation rate is
                // pinned high). Variants with the bit set produce a
                // monotone uplift in layer count.
                let active = variant.layer_mask.has(LayerMask::CELL_SUBSTRATE);
                let cluster_formation = if active {
                    Fixed::Rational {
                        num: 70 + 2 * layer_count,
                        den: 100,
                    }
                } else {
                    Fixed::Rational { num: 0, den: 100 }
                };
                let morpho_compliance = if active {
                    Fixed::Rational {
                        num: 80 + layer_count,
                        den: 100,
                    }
                } else {
                    Fixed::Rational { num: 0, den: 100 }
                };
                let convergence_stability = if active {
                    Fixed::Rational {
                        num: 75 + layer_count,
                        den: 100,
                    }
                } else {
                    Fixed::Rational { num: 0, den: 100 }
                };
                let intent_rate = if active {
                    Fixed::Rational {
                        num: 60 + 2 * layer_count,
                        den: 100,
                    }
                } else {
                    Fixed::Rational { num: 0, den: 100 }
                };
                let dissolution_pres = if active {
                    Fixed::Rational { num: 1, den: 1 }
                } else {
                    Fixed::Rational { num: 0, den: 1 }
                };
                let funnel_acyclic = if active {
                    Fixed::Rational { num: 1, den: 1 }
                } else {
                    Fixed::Rational { num: 0, den: 1 }
                };
                let boundary_violation = if active {
                    Fixed::Rational {
                        num: 1,
                        den: 1000 + 100 * layer_count,
                    }
                } else {
                    Fixed::Rational { num: 80, den: 100 }
                };
                let compaction_eff = if active {
                    Fixed::Rational {
                        num: 32 + 4 * layer_count,
                        den: 1,
                    }
                } else {
                    Fixed::Rational { num: 0, den: 1 }
                };
                let handoff_utility = if active {
                    Fixed::Rational {
                        num: 70 + 2 * layer_count,
                        den: 100,
                    }
                } else {
                    Fixed::Rational { num: 0, den: 100 }
                };
                let self_coherence = if active {
                    Fixed::Rational {
                        num: 80 + layer_count,
                        den: 100,
                    }
                } else {
                    Fixed::Rational { num: 0, den: 100 }
                };

                metrics.push(MetricObservation::ok(
                    "cluster_formation_rate",
                    cluster_formation,
                ));
                metrics.push(MetricObservation::ok(
                    "morphology_gate_compliance",
                    morpho_compliance,
                ));
                metrics.push(MetricObservation::ok(
                    "convergence_stability",
                    convergence_stability,
                ));
                metrics.push(MetricObservation::ok("intent_generation_rate", intent_rate));
                metrics.push(MetricObservation::ok(
                    "dissolution_trace_preservation",
                    dissolution_pres,
                ));
                metrics.push(MetricObservation::ok(
                    "funnel_acyclicity_rate",
                    funnel_acyclic,
                ));
                metrics.push(MetricObservation::ok(
                    "matrix_boundary_violation_rate",
                    boundary_violation,
                ));
                metrics.push(MetricObservation::ok(
                    "working_state_compaction_efficiency",
                    compaction_eff,
                ));
                metrics.push(MetricObservation::ok(
                    "handoff_candidate_utility",
                    handoff_utility,
                ));
                metrics.push(MetricObservation::ok(
                    "substrate_self_coherence",
                    self_coherence,
                ));
            }
            _ => {}
        }

        // Gates — one per active layer + a synthesised composite gate.
        let mut gates = vec![GateObservation {
            gate_name: "composite".into(),
            passed: variant.layer_mask.count_layers() >= 1,
            reason: None,
        }];
        if variant.layer_mask.has(LayerMask::HORIZON) {
            gates.push(GateObservation {
                gate_name: "horizon_crossing".into(),
                passed: true,
                reason: None,
            });
        }
        if variant.layer_mask.has(LayerMask::COGNITION) {
            gates.push(GateObservation {
                gate_name: "cognition_handoff".into(),
                passed: true,
                reason: None,
            });
        }
        if variant.layer_mask.has(LayerMask::CELL_SUBSTRATE) {
            gates.push(GateObservation {
                gate_name: "cell_substrate_dissolution".into(),
                passed: true,
                reason: None,
            });
        }

        let replay = ReplayObservation {
            expected_hash: descriptor.content_hash()?,
            observed_hash: descriptor.content_hash()?,
            passed: true,
        };

        // No failures recorded for healthy synthetic runs except B0
        // baseline which records a token false-crystal pseudo-event so
        // safety_event_count > 0 is observable.
        let mut failures = Vec::new();
        if variant.layer_mask.count_layers() == 0 {
            failures.push(FailureRecord::new(
                FailureKind::FalseCrystal,
                "synthetic baseline pseudo-failure",
            )?);
        }

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

/// Run a single trial and return the (variant, workload, dataset)-keyed
/// `TrialReport` plus the `EvaluationRunEntry` ready for the ledger.
pub fn run_trial(
    spec: &EvaluationSpec,
    variant: &SystemVariantSpec,
    workload: &WorkloadSpec,
    descriptor: &RunDescriptor,
    executor: &dyn TrialExecutor,
    metric_specs: &[MetricSpec],
) -> Result<(TrialReport, EvaluationRunEntry), EvalError> {
    let report = executor.execute_trial(spec, variant, workload, descriptor)?;

    // Apply MetricSpec invalidation rules: filter observations whose
    // declared rules are violated by the trial. This is purely a
    // bookkeeping pass — observations stay in the report, but their
    // `valid` flag is updated.
    let mut report = report;
    for obs in &mut report.metrics {
        if let Some(ms) = metric_specs.iter().find(|m| m.metric_id == obs.metric_id) {
            for rule in &ms.invalidation_rules {
                use crate::metrics::InvalidationRule::*;
                match rule {
                    RequireReplayPass if !report.replay.passed => {
                        obs.valid = false;
                        obs.invalid_reason = Some("replay failed".into());
                    }
                    RequireGroundTruth if workload.ground_truth_profile.is_none() => {
                        obs.valid = false;
                        obs.invalid_reason = Some("workload has no ground truth".into());
                    }
                    _ => {}
                }
            }
        }
    }
    let report = report.with_id()?;

    let report_hash = content_address(&report)?;
    let replay_hash = content_address(&report.replay)?;
    let status = if report.replay.passed {
        RunStatus::Completed
    } else {
        RunStatus::InvalidReplayMismatch
    };

    let entry = EvaluationRunEntry {
        entry_id: Hash256::zero(),
        variant_id: variant.variant_id.clone(),
        workload_id: workload.workload_id.clone(),
        dataset_id: workload.input_manifest_hash.clone(),
        run_descriptor_hash: descriptor.content_hash()?,
        report_hash,
        replay_hash,
        status,
    }
    .with_id()?;
    Ok((report, entry))
}

/// Convenience: drive every plan entry through an executor and append
/// the resulting entries to a fresh ledger. Returns the final ledger and
/// the per-trial reports.
pub fn run_plan(
    spec: &EvaluationSpec,
    plan: &EvaluationPlan,
    executor: &dyn TrialExecutor,
) -> Result<(EvaluationRunLedger, Vec<TrialReport>), EvalError> {
    let mut ledger = init_ledger(spec.spec_id.clone())?;
    let mut reports = Vec::with_capacity(plan.entries.len());
    for entry in &plan.entries {
        let variant = spec
            .variants
            .iter()
            .find(|v| v.variant_id == entry.descriptor.variant_id)
            .ok_or_else(|| EvalError::InvalidSpec {
                reason: format!(
                    "plan references unknown variant {}",
                    entry.descriptor.variant_id
                ),
            })?;
        let workload = spec
            .workloads
            .iter()
            .find(|w| w.workload_id == entry.descriptor.workload_id)
            .ok_or_else(|| EvalError::InvalidSpec {
                reason: format!(
                    "plan references unknown workload {}",
                    entry.descriptor.workload_id
                ),
            })?;
        let (report, run_entry) = run_trial(
            spec,
            variant,
            workload,
            &entry.descriptor,
            executor,
            &spec.metrics,
        )?;
        ledger = append_to_ledger(ledger, run_entry)?;
        reports.push(report);
    }
    Ok((ledger, reports))
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
    use crate::datasets::DatasetManifest;
    use crate::metrics::MetricSpec;
    use crate::spec::{
        DeterminismPolicy, EvaluationPurpose, ExperimentDesign, OutputPolicy, StatisticalPlan,
    };
    use crate::variants::SystemVariantSpec;
    use crate::workloads::WorkloadSpec;
    use std::collections::BTreeMap;

    fn minimal_spec() -> EvaluationSpec {
        let dataset = DatasetManifest::synthetic("d.test");
        let gt = crate::ground_truth::GroundTruthProfile::synthetic_exact();
        let workload = WorkloadSpec::stream_event(
            "w.s",
            dataset.dataset_id.clone(),
            Some(gt.profile_id.clone()),
        );
        EvaluationSpec {
            spec_id: Hash256::zero(),
            schema_version: "eval-matrix-v0.1".into(),
            purpose: EvaluationPurpose::AblationStudy,
            variants: vec![
                SystemVariantSpec::baseline(),
                SystemVariantSpec::cognition(),
            ],
            workloads: vec![workload],
            datasets: vec![dataset],
            metrics: vec![
                MetricSpec::task_success(),
                MetricSpec::false_commit_rate(),
                MetricSpec::replay_identity(),
            ],
            experiment_design: ExperimentDesign::default_design(),
            statistical_plan: StatisticalPlan::default_plan(),
            determinism_policy: DeterminismPolicy::default(),
            output_policy: OutputPolicy::default_policy(),
            tags: BTreeMap::new(),
        }
        .with_id()
        .unwrap()
    }

    #[test]
    fn plan_is_deterministic() {
        let spec = minimal_spec();
        let p1 = plan_runs(&spec).unwrap();
        let p2 = plan_runs(&spec).unwrap();
        assert_eq!(p1.plan_id, p2.plan_id);
        assert_eq!(p1.entries.len(), 2); // 2 variants × 1 workload
    }

    #[test]
    fn synthetic_executor_is_byte_identical() {
        let spec = minimal_spec();
        let plan = plan_runs(&spec).unwrap();
        let exec = SyntheticTrialExecutor;
        let (l1, r1) = run_plan(&spec, &plan, &exec).unwrap();
        let (l2, r2) = run_plan(&spec, &plan, &exec).unwrap();
        assert_eq!(l1.ledger_id, l2.ledger_id);
        assert_eq!(r1.len(), r2.len());
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.trial_id, b.trial_id);
        }
    }

    fn morpho_spec() -> EvaluationSpec {
        let dataset = DatasetManifest::synthetic("d.morpho");
        let gt = crate::ground_truth::GroundTruthProfile::synthetic_exact();
        let workload = WorkloadSpec::morpho_cell_substrate(
            "w.morpho",
            dataset.dataset_id.clone(),
            Some(gt.profile_id.clone()),
        );
        EvaluationSpec {
            spec_id: Hash256::zero(),
            schema_version: "eval-matrix-v0.1".into(),
            purpose: EvaluationPurpose::AblationStudy,
            variants: vec![
                SystemVariantSpec::baseline(),
                SystemVariantSpec::full_stack(),
                SystemVariantSpec::phase_matrix_substrate(),
            ],
            workloads: vec![workload],
            datasets: vec![dataset],
            metrics: {
                let mut m = vec![MetricSpec::task_success(), MetricSpec::replay_identity()];
                m.extend(crate::cell_substrate_metrics::cell_substrate_metric_specs());
                m
            },
            experiment_design: ExperimentDesign::default_design(),
            statistical_plan: StatisticalPlan::default_plan(),
            determinism_policy: DeterminismPolicy::default(),
            output_policy: OutputPolicy::default_policy(),
            tags: BTreeMap::new(),
        }
        .with_id()
        .unwrap()
    }

    #[test]
    fn b8_phase_matrix_uplifts_cluster_formation_over_b7() {
        let spec = morpho_spec();
        let plan = plan_runs(&spec).unwrap();
        let (_, reports) = run_plan(&spec, &plan, &SyntheticTrialExecutor).unwrap();
        let pick = |variant: &str| -> Fixed {
            reports
                .iter()
                .find(|r| r.variant_id == variant)
                .unwrap()
                .metrics
                .iter()
                .find(|m| m.metric_id == "cluster_formation_rate")
                .unwrap()
                .value
                .clone()
        };
        let b0 = pick("B0_Baseline");
        let b7 = pick("B7_FullStack");
        let b8 = pick("B8_PhaseMatrix");
        // B0 / B7 do not own the substrate, so cluster_formation_rate is
        // pinned to the floor; B8 must strictly outperform both.
        assert_eq!(b0.cmp(&b7), std::cmp::Ordering::Equal);
        assert_eq!(b8.cmp(&b7), std::cmp::Ordering::Greater);
    }

    #[test]
    fn b8_phase_matrix_lowers_boundary_violation_over_b7() {
        let spec = morpho_spec();
        let plan = plan_runs(&spec).unwrap();
        let (_, reports) = run_plan(&spec, &plan, &SyntheticTrialExecutor).unwrap();
        let pick = |variant: &str| -> Fixed {
            reports
                .iter()
                .find(|r| r.variant_id == variant)
                .unwrap()
                .metrics
                .iter()
                .find(|m| m.metric_id == "matrix_boundary_violation_rate")
                .unwrap()
                .value
                .clone()
        };
        let b7 = pick("B7_FullStack");
        let b8 = pick("B8_PhaseMatrix");
        // Lower-is-better: substrate-aware variant must be lower.
        assert_eq!(b8.cmp(&b7), std::cmp::Ordering::Less);
    }

    #[test]
    fn cognition_variant_outperforms_baseline_on_task_success() {
        let spec = minimal_spec();
        let plan = plan_runs(&spec).unwrap();
        let (_, reports) = run_plan(&spec, &plan, &SyntheticTrialExecutor).unwrap();
        let base = reports
            .iter()
            .find(|r| r.variant_id == "B0_Baseline")
            .unwrap();
        let cog = reports
            .iter()
            .find(|r| r.variant_id == "B6_Cognition")
            .unwrap();
        let base_task = base
            .metrics
            .iter()
            .find(|m| m.metric_id == "task_success")
            .unwrap();
        let cog_task = cog
            .metrics
            .iter()
            .find(|m| m.metric_id == "task_success")
            .unwrap();
        assert!(cog_task.value.cmp(&base_task.value) == std::cmp::Ordering::Greater);
    }
}
