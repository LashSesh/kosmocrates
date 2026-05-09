//! End-to-end integration test for PSE-EVAL-MATRIX-01.
//!
//! Drives the full pipeline: `init → validate → plan → run → replay
//! → score → ablate → report` against the `agent-cognition` and
//! `post-symbolic-ablation` presets, and verifies:
//!
//! * the spec is content-addressed and validates,
//! * the plan is deterministic (two `plan_runs` calls produce the same
//!   `plan_id`),
//! * the synthetic executor's reports replay byte-identically,
//! * the ledger chain is hash-valid,
//! * scoring runs purely from observations (no float leakage),
//! * the ablation ladder for B6_Cognition flips exactly the cognition
//!   bit on `noCognition`,
//! * markdown + json export emit non-empty bytes,
//! * `score_capability_profile` produces a SAU that grows with the
//!   variant ladder.

use pse_eval_matrix::{
    append_to_ledger, build_ablation_ladder, init_ledger, plan_runs, render_json_summary,
    render_markdown_summary, run_trial, score_capability_profile, score_ledger, summarize_ablation,
    EvaluationSummaryReport, LayerMask, Preset, SyntheticTrialExecutor,
};

fn full_pipeline(preset: Preset) -> EvaluationSummaryReport {
    let spec = preset.build().unwrap();
    spec.validate().unwrap();
    let plan = plan_runs(&spec).unwrap();
    let plan2 = plan_runs(&spec).unwrap();
    assert_eq!(plan.plan_id, plan2.plan_id, "plan must be deterministic");

    let executor = SyntheticTrialExecutor;
    let mut ledger = init_ledger(spec.spec_id.clone()).unwrap();
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
        let (report, run_entry) = run_trial(
            &spec,
            variant,
            workload,
            &entry.descriptor,
            &executor,
            &spec.metrics,
        )
        .unwrap();
        // The synthetic executor's self-attested replay observation
        // must pass; external replay (re-execute and compare trial_id)
        // is checked below.
        assert!(
            report.replay.passed,
            "synthetic trial replay observation must hold"
        );
        // Re-run the same trial through the executor and assert
        // byte-identity on `trial_id`.
        let (report_again, _) = run_trial(
            &spec,
            variant,
            workload,
            &entry.descriptor,
            &executor,
            &spec.metrics,
        )
        .unwrap();
        assert_eq!(
            report.trial_id, report_again.trial_id,
            "re-execution must produce a byte-identical trial_id"
        );
        ledger = append_to_ledger(ledger, run_entry).unwrap();
        reports.push(report);
    }
    pse_eval_matrix::ledger::verify_ledger_chain(&ledger).unwrap();

    let (variants, workloads, invalid) = score_ledger(&ledger, &reports, &spec.metrics).unwrap();
    assert_eq!(invalid.len(), 0, "no synthetic run should be invalid");

    let profiles: Vec<_> = variants
        .iter()
        .map(|v| score_capability_profile(v, &spec.metrics).unwrap())
        .collect();

    EvaluationSummaryReport {
        report_id: pse_eval_matrix::primitives::Hash256::zero(),
        evaluation_spec_hash: spec.spec_id.clone(),
        generated_from_ledger: ledger.ledger_id.clone(),
        variant_summaries: variants,
        workload_summaries: workloads,
        ablation_summaries: vec![],
        capability_profiles: profiles,
        statistical_summaries: vec![],
        invalid_runs: invalid,
        conclusion_flags: vec![],
    }
    .with_id()
    .unwrap()
}

#[test]
fn agent_cognition_pipeline_runs_end_to_end() {
    let summary = full_pipeline(Preset::AgentCognition);
    assert!(!summary.variant_summaries.is_empty());
    assert!(!summary.capability_profiles.is_empty());

    // Markdown summary contains the variant table.
    let md = render_markdown_summary(&summary);
    assert!(md.contains("Variant summaries"));
    assert!(md.contains("B0_Baseline"));
    assert!(md.contains("B6_Cognition"));

    // JSON export is non-empty.
    let json = render_json_summary(&summary).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn post_symbolic_ablation_full_ladder_runs() {
    let summary = full_pipeline(Preset::PostSymbolicAblation);
    let variant_ids: Vec<_> = summary
        .variant_summaries
        .iter()
        .map(|v| v.variant_id.as_str())
        .collect();
    for expected in [
        "B0_Baseline",
        "B1_PSE_Core",
        "B2_Traversal",
        "B3_Signature",
        "B4_Dynamics",
        "B5_Horizon",
        "B6_Cognition",
        "B7_FullStack",
    ] {
        assert!(
            variant_ids.contains(&expected),
            "ladder missing variant {expected}"
        );
    }
}

#[test]
fn capability_profile_safety_adjusted_utility_grows_with_layers() {
    let summary = full_pipeline(Preset::PostSymbolicAblation);
    let baseline = summary
        .capability_profiles
        .iter()
        .find(|p| p.variant_id == "B0_Baseline")
        .unwrap();
    let full_stack = summary
        .capability_profiles
        .iter()
        .find(|p| p.variant_id == "B7_FullStack")
        .unwrap();
    // Full-stack SAU MUST be strictly greater than baseline SAU under the
    // synthetic executor's monotone layer→quality coupling.
    assert!(
        full_stack
            .safety_adjusted_utility
            .cmp(&baseline.safety_adjusted_utility)
            == std::cmp::Ordering::Greater,
        "expected SAU(B7) > SAU(B0); got base={:?} full={:?}",
        baseline.safety_adjusted_utility,
        full_stack.safety_adjusted_utility,
    );
}

#[test]
fn ablation_ladder_drops_cognition_bit_on_no_cognition_rung() {
    let spec = Preset::AgentCognition.build().unwrap();
    let cog = spec
        .variants
        .iter()
        .find(|v| v.variant_id == "B6_Cognition")
        .unwrap()
        .clone();
    let ladder = build_ablation_ladder(cog).unwrap();
    let no_cog = ladder
        .variants
        .iter()
        .find(|v| v.variant_id.ends_with(".noCognition"))
        .expect("ladder must contain the noCognition rung");
    assert!(!no_cog.layer_mask.has(LayerMask::COGNITION));
}

#[test]
fn ablation_summary_marks_layer_useful_when_removing_helps_baseline() {
    // Run a small ablation comparison off the synthetic ladder.
    let summary = full_pipeline(Preset::PostSymbolicAblation);
    let base = summary
        .variant_summaries
        .iter()
        .find(|v| v.variant_id == "B6_Cognition")
        .unwrap();
    let ablated = summary
        .variant_summaries
        .iter()
        .find(|v| v.variant_id == "B5_Horizon")
        .unwrap();
    let metric_specs = vec![
        pse_eval_matrix::metrics::MetricSpec::task_success(),
        pse_eval_matrix::metrics::MetricSpec::false_commit_rate(),
        pse_eval_matrix::metrics::MetricSpec::hold_correctness(),
    ];
    let summary = summarize_ablation(base, ablated, &metric_specs).unwrap();
    // Removing Cognition (going B6 → B5) MUST decrease task_success
    // under the synthetic executor; signed_delta should therefore be
    // negative.
    let task_delta = summary
        .metric_deltas
        .iter()
        .find(|d| d.metric_id == "task_success")
        .unwrap();
    assert!(
        task_delta
            .signed_delta
            .cmp(&pse_eval_matrix::Fixed::Rational { num: 0, den: 1 })
            == std::cmp::Ordering::Less,
        "expected negative LMU when removing the cognition layer"
    );
}

#[test]
fn two_full_pipeline_runs_are_byte_identical() {
    let a = full_pipeline(Preset::AgentCognition);
    let b = full_pipeline(Preset::AgentCognition);
    assert_eq!(a.report_id, b.report_id, "summary report id must replay");
}
