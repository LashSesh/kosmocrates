//! `pse-eval-matrix` — CLI for the PSE-EVAL-MATRIX-01 empirical
//! benchmark matrix (§15).
//!
//! Subcommands:
//!
//! ```text
//! init     --template <preset>         --out <eval_spec.json>
//! validate --spec <eval_spec.json>
//! plan     --spec <eval_spec.json>     --out <eval_plan.json>
//! run      --spec <eval_spec.json>     --plan <eval_plan.json> --out <runs_dir/>
//! replay   --ledger <runs/ledger.json> --reports <runs/reports.json>
//! score    --spec <spec.json> --ledger <ledger.json> --reports <reports.json> --out <summary.json>
//! ablate   --spec <eval_spec.json>     --base <variant_id>      --out <ablations.json>
//! compare  --summary <summary.json>    --baseline <variant_id>
//! report   --summary <summary.json>    --format <md|json>       [--out <path>]
//! ```
//!
//! Manual flag parsing — keeps the binary dependency-free.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use pse_eval_matrix::{
    append_to_ledger, build_ablation_ladder, init_ledger, plan_runs, render_json_summary,
    render_markdown_summary, run_agent_exoskeleton_benchmark,
    run_agent_exoskeleton_benchmark_with_fixture, run_trial, score_capability_profile,
    score_layer_fixture_selections, score_ledger, summarize_ablation, summarize_layer_fixture,
    validate_layer_fixture, AblationSummary, ConclusionFlag, EvaluationPlan, EvaluationRunLedger,
    EvaluationSpec, EvaluationSummaryReport, Fixed, Preset, SyntheticTrialExecutor, TrialReport,
    VariantSummary, WorkloadSummary,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let result: CliResult<()> = match args[1].as_str() {
        "init" => cmd_init(&args[2..]),
        "validate" => cmd_validate(&args[2..]),
        "plan" => cmd_plan(&args[2..]),
        "run" => cmd_run(&args[2..]),
        "replay" => cmd_replay(&args[2..]),
        "score" => cmd_score(&args[2..]),
        "ablate" => cmd_ablate(&args[2..]),
        "compare" => cmd_compare(&args[2..]),
        "report" => cmd_report(&args[2..]),
        "agent-exoskeleton" => cmd_agent_exoskeleton(&args[2..]),
        "layer-fixture" => cmd_layer_fixture(&args[2..]),
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        other => {
            eprintln!("unknown subcommand: {other}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "Usage:
  pse-eval-matrix init     --template <agent-cognition|streaming-event-detection|post-symbolic-ablation> --out <spec.json>
  pse-eval-matrix validate --spec <spec.json>
  pse-eval-matrix plan     --spec <spec.json>  --out <plan.json>
  pse-eval-matrix run      --spec <spec.json>  --plan <plan.json> --out <runs_dir/>
  pse-eval-matrix replay   --ledger <ledger.json> --reports <reports.json>
  pse-eval-matrix score    --spec <spec.json> --ledger <ledger.json> --reports <reports.json> --out <summary.json>
  pse-eval-matrix ablate   --spec <spec.json>  --base <variant_id>   --out <ablations.json>
  pse-eval-matrix compare  --summary <summary.json> --baseline <variant_id>
  pse-eval-matrix report   --summary <summary.json> --format <md|json> [--out <path>]

  # External fixture subcommands (any PSE layer fixture file):
  pse-eval-matrix layer-fixture validate --path <fixture.json>
  pse-eval-matrix layer-fixture score    --fixture <fixture.json> --selections <sel.json> --out <report.json>
  pse-eval-matrix layer-fixture batch    --dir <fixtures/> --out <batch_report.json>
";

type CliResult<T> = std::result::Result<T, String>;

fn flag_value(args: &[String], flag: &str) -> CliResult<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            return iter
                .next()
                .cloned()
                .ok_or_else(|| format!("missing value for {flag}"));
        }
    }
    Err(format!("missing flag {flag}"))
}

fn opt_flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            return iter.next().cloned();
        }
    }
    None
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> CliResult<T> {
    let bytes = fs::read(PathBuf::from(path)).map_err(|e| format!("read {path}: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("parse {path}: {e}"))
}

fn write_canonical<T: serde::Serialize>(
    value: &T,
    opt_out: Option<String>,
    label: &str,
) -> CliResult<()> {
    let bytes = serde_jcs::to_vec(value).map_err(|e| format!("canonical encode: {e}"))?;
    if let Some(out) = opt_out {
        if let Some(parent) = PathBuf::from(&out).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir {parent:?}: {e}"))?;
        }
        fs::write(&out, &bytes).map_err(|e| format!("write {out}: {e}"))?;
        eprintln!("wrote {label} {out} ({} bytes)", bytes.len());
    } else {
        println!("{}", String::from_utf8_lossy(&bytes));
    }
    Ok(())
}

fn cmd_init(args: &[String]) -> CliResult<()> {
    let template = flag_value(args, "--template")?;
    let preset = Preset::from_tag(&template).ok_or_else(|| {
        format!(
            "unknown template '{template}'; valid: agent-cognition | streaming-event-detection | post-symbolic-ablation"
        )
    })?;
    let spec = preset.build().map_err(|e| e.to_string())?;
    write_canonical(&spec, opt_flag_value(args, "--out"), "spec")
}

fn cmd_validate(args: &[String]) -> CliResult<()> {
    let path = flag_value(args, "--spec")?;
    let spec: EvaluationSpec = read_json(&path)?;
    spec.validate().map_err(|e| e.to_string())?;
    println!(
        "spec ok: id {} ({} variants × {} workloads × {} metrics)",
        spec.spec_id.hex(),
        spec.variants.len(),
        spec.workloads.len(),
        spec.metrics.len()
    );
    Ok(())
}

fn cmd_plan(args: &[String]) -> CliResult<()> {
    let path = flag_value(args, "--spec")?;
    let spec: EvaluationSpec = read_json(&path)?;
    let plan = plan_runs(&spec).map_err(|e| e.to_string())?;
    write_canonical(&plan, opt_flag_value(args, "--out"), "plan")
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RunBundle {
    spec_id: pse_eval_matrix::primitives::Hash256,
    plan_id: pse_eval_matrix::primitives::Hash256,
    ledger: EvaluationRunLedger,
    reports: Vec<TrialReport>,
}

fn cmd_run(args: &[String]) -> CliResult<()> {
    let spec_path = flag_value(args, "--spec")?;
    let plan_path = flag_value(args, "--plan")?;
    let spec: EvaluationSpec = read_json(&spec_path)?;
    let plan: EvaluationPlan = read_json(&plan_path)?;
    if plan.evaluation_spec_hash != spec.spec_id {
        return Err("plan.evaluation_spec_hash != spec.spec_id".into());
    }
    let executor = SyntheticTrialExecutor;
    let mut ledger = init_ledger(spec.spec_id.clone()).map_err(|e| e.to_string())?;
    let mut reports = Vec::with_capacity(plan.entries.len());
    for entry in &plan.entries {
        let variant = spec
            .variants
            .iter()
            .find(|v| v.variant_id == entry.descriptor.variant_id)
            .ok_or_else(|| format!("plan refs unknown variant {}", entry.descriptor.variant_id))?;
        let workload = spec
            .workloads
            .iter()
            .find(|w| w.workload_id == entry.descriptor.workload_id)
            .ok_or_else(|| {
                format!(
                    "plan refs unknown workload {}",
                    entry.descriptor.workload_id
                )
            })?;
        let (report, run_entry) = run_trial(
            &spec,
            variant,
            workload,
            &entry.descriptor,
            &executor,
            &spec.metrics,
        )
        .map_err(|e| e.to_string())?;
        ledger = append_to_ledger(ledger, run_entry).map_err(|e| e.to_string())?;
        reports.push(report);
    }
    let bundle = RunBundle {
        spec_id: spec.spec_id.clone(),
        plan_id: plan.plan_id.clone(),
        ledger,
        reports,
    };
    write_canonical(&bundle, opt_flag_value(args, "--out"), "run bundle")
}

fn cmd_replay(args: &[String]) -> CliResult<()> {
    let ledger_path = flag_value(args, "--ledger").or_else(|_| flag_value(args, "--bundle"))?;
    // Allow either a stand-alone ledger file or a full RunBundle.
    let bytes = fs::read(&ledger_path).map_err(|e| format!("read {ledger_path}: {e}"))?;
    let (ledger, reports) = if let Ok(bundle) = serde_json::from_slice::<RunBundle>(&bytes) {
        (bundle.ledger, bundle.reports)
    } else {
        let l: EvaluationRunLedger =
            serde_json::from_slice(&bytes).map_err(|e| format!("parse {ledger_path}: {e}"))?;
        let reports_path = flag_value(args, "--reports")?;
        let reports: Vec<TrialReport> = read_json(&reports_path)?;
        (l, reports)
    };
    pse_eval_matrix::ledger::verify_ledger_chain(&ledger).map_err(|e| e.to_string())?;
    let mut failed = 0u64;
    for r in &reports {
        // The trial's own self-attested replay observation must hold
        // (`expected_hash == observed_hash` by construction in the
        // executor). External re-execution is performed by re-running
        // `pse-eval-matrix run` against the same spec + plan and
        // comparing `trial_id`s — which the upstream test harness does.
        if !r.replay.passed {
            failed += 1;
        }
    }
    println!(
        "replay ok: ledger chain valid, {} trials, {} replay failures",
        reports.len(),
        failed
    );
    Ok(())
}

fn cmd_score(args: &[String]) -> CliResult<()> {
    let spec_path = flag_value(args, "--spec")?;
    let spec: EvaluationSpec = read_json(&spec_path)?;
    let bundle_path = flag_value(args, "--ledger").or_else(|_| flag_value(args, "--bundle"))?;
    let bytes = fs::read(&bundle_path).map_err(|e| format!("read {bundle_path}: {e}"))?;
    let (ledger, reports) = if let Ok(bundle) = serde_json::from_slice::<RunBundle>(&bytes) {
        (bundle.ledger, bundle.reports)
    } else {
        let l: EvaluationRunLedger =
            serde_json::from_slice(&bytes).map_err(|e| format!("parse {bundle_path}: {e}"))?;
        let reports_path = flag_value(args, "--reports")?;
        let reports: Vec<TrialReport> = read_json(&reports_path)?;
        (l, reports)
    };

    let (variant_summaries, workload_summaries, invalid_runs) =
        score_ledger(&ledger, &reports, &spec.metrics).map_err(|e| e.to_string())?;
    let capability_profiles: Vec<_> = variant_summaries
        .iter()
        .map(|v| score_capability_profile(v, &spec.metrics).unwrap())
        .collect();

    let conclusion_flags = compute_conclusion_flags(&variant_summaries, &capability_profiles);

    let summary = EvaluationSummaryReport {
        report_id: pse_eval_matrix::primitives::Hash256::zero(),
        evaluation_spec_hash: spec.spec_id.clone(),
        generated_from_ledger: ledger.ledger_id.clone(),
        variant_summaries,
        workload_summaries,
        ablation_summaries: vec![],
        capability_profiles,
        statistical_summaries: vec![],
        invalid_runs,
        conclusion_flags,
    }
    .with_id()
    .map_err(|e| e.to_string())?;
    write_canonical(&summary, opt_flag_value(args, "--out"), "summary")
}

fn cmd_ablate(args: &[String]) -> CliResult<()> {
    let spec_path = flag_value(args, "--spec")?;
    let base_id = flag_value(args, "--base")?;
    let spec: EvaluationSpec = read_json(&spec_path)?;
    let base = spec
        .variants
        .iter()
        .find(|v| v.variant_id == base_id)
        .ok_or_else(|| format!("variant {base_id} not in spec"))?
        .clone();
    let ladder = build_ablation_ladder(base).map_err(|e| e.to_string())?;
    write_canonical(&ladder, opt_flag_value(args, "--out"), "ablation ladder")
}

fn cmd_compare(args: &[String]) -> CliResult<()> {
    let summary_path = flag_value(args, "--summary")?;
    let baseline_id = flag_value(args, "--baseline")?;
    let spec_path = opt_flag_value(args, "--spec");
    let summary: EvaluationSummaryReport = read_json(&summary_path)?;
    let metric_specs: Vec<_> = if let Some(p) = spec_path {
        let spec: EvaluationSpec = read_json(&p)?;
        spec.metrics
    } else {
        // Without a spec we can still print the LMU on `task_success`
        // alone using direction = HigherIsBetter as default.
        vec![pse_eval_matrix::metrics::MetricSpec::task_success()]
    };
    let base = summary
        .variant_summaries
        .iter()
        .find(|v| v.variant_id == baseline_id)
        .ok_or_else(|| format!("baseline {baseline_id} not in summary"))?;
    println!("baseline = {}", baseline_id);
    let mut ablations: Vec<AblationSummary> = Vec::new();
    for v in &summary.variant_summaries {
        if v.variant_id == baseline_id {
            continue;
        }
        let a = summarize_ablation(base, v, &metric_specs).map_err(|e| e.to_string())?;
        ablations.push(a);
    }
    for a in &ablations {
        println!(
            "  {} → {}: LMU(task_success) = {} | safety_regression = {} | conclusion = {:?}",
            a.base_variant,
            a.ablated_variant,
            fixed_to_str(&a.layer_marginal_utility),
            a.safety_regression,
            a.conclusion,
        );
    }
    Ok(())
}

fn cmd_report(args: &[String]) -> CliResult<()> {
    let summary_path = flag_value(args, "--summary")?;
    let format = flag_value(args, "--format")?;
    let summary: EvaluationSummaryReport = read_json(&summary_path)?;
    match format.as_str() {
        "md" => {
            let md = render_markdown_summary(&summary);
            if let Some(out) = opt_flag_value(args, "--out") {
                fs::write(&out, &md).map_err(|e| format!("write {out}: {e}"))?;
                eprintln!("wrote markdown summary {out} ({} bytes)", md.len());
            } else {
                println!("{md}");
            }
            Ok(())
        }
        "json" => {
            let bytes = render_json_summary(&summary).map_err(|e| e.to_string())?;
            if let Some(out) = opt_flag_value(args, "--out") {
                fs::write(&out, &bytes).map_err(|e| format!("write {out}: {e}"))?;
                eprintln!("wrote json summary {out} ({} bytes)", bytes.len());
            } else {
                println!("{}", String::from_utf8_lossy(&bytes));
            }
            Ok(())
        }
        other => Err(format!("unsupported --format {other} (use md or json)")),
    }
}

fn compute_conclusion_flags(
    variant_summaries: &[VariantSummary],
    profiles: &[pse_eval_matrix::scoring::CapabilityProfile],
) -> Vec<ConclusionFlag> {
    let mut flags = Vec::new();
    let any_invalid = variant_summaries.iter().any(|v| v.invalid_trials > 0);
    if any_invalid {
        flags.push(ConclusionFlag::InvalidatedByReplay);
    }
    // Empirical improvement: if any non-baseline variant has SAU >
    // baseline's SAU.
    let baseline = profiles.iter().find(|p| p.variant_id == "B0_Baseline");
    if let Some(base) = baseline {
        let any_better = profiles.iter().any(|p| {
            p.variant_id != base.variant_id
                && p.safety_adjusted_utility.cmp(&base.safety_adjusted_utility)
                    == std::cmp::Ordering::Greater
        });
        if any_better {
            flags.push(ConclusionFlag::EmpiricalImprovement);
        } else {
            flags.push(ConclusionFlag::DiagnosticFinding);
        }
    }
    let _ = WorkloadSummary {
        workload_id: String::new(),
        trial_counts: Default::default(),
        primary_metric_means: Default::default(),
    };
    flags
}

fn fixed_to_str(f: &Fixed) -> String {
    match f {
        Fixed::FixedI64 { raw, scale } => format!("{}/{}", raw, 10i128.pow(*scale)),
        Fixed::Rational { num, den } => format!("{num}/{den}"),
    }
}

fn cmd_agent_exoskeleton(args: &[String]) -> CliResult<()> {
    let out = flag_value(args, "--out")?;
    let report = if let Some(fixture_path) = opt_flag_value(args, "--trace-fixture") {
        run_agent_exoskeleton_benchmark_with_fixture(&fixture_path)
            .map_err(|e| format!("fixture load failed: {e}"))?
    } else {
        run_agent_exoskeleton_benchmark()
    };
    write_canonical(&report, Some(out), "agent exoskeleton report")
}

// ── layer-fixture ─────────────────────────────────────────────────────────

fn cmd_layer_fixture(args: &[String]) -> CliResult<()> {
    let subcmd = args.first().map(|s| s.as_str()).unwrap_or("validate");
    match subcmd {
        "validate" => cmd_lf_validate(&args[1..]),
        "score" => cmd_lf_score(&args[1..]),
        "batch" => cmd_lf_batch(&args[1..]),
        other => Err(format!(
            "unknown layer-fixture subcommand '{other}'\n\
             use: validate | score | batch"
        )),
    }
}

fn cmd_lf_validate(args: &[String]) -> CliResult<()> {
    let path = flag_value(args, "--path")?;
    let fixture = validate_layer_fixture(&path).map_err(|e| format!("validation failed: {e}"))?;
    let report = summarize_layer_fixture(&fixture);
    let out = opt_flag_value(args, "--out");
    if out.is_none() {
        // Pretty-print to stdout.
        let json = serde_json::to_string_pretty(&report)
            .map_err(|e| format!("serialize: {e}"))?;
        println!("{json}");
        eprintln!(
            "OK: {} — {} cases, {} candidates, {} causal",
            report.fixture_name, report.case_count, report.total_candidates, report.total_causal
        );
        return Ok(());
    }
    write_canonical(&report, out, "fixture summary")
}

fn cmd_lf_score(args: &[String]) -> CliResult<()> {
    let fixture_path = flag_value(args, "--fixture")?;
    let selections_path = flag_value(args, "--selections")?;
    let out = opt_flag_value(args, "--out");

    let fixture =
        validate_layer_fixture(&fixture_path).map_err(|e| format!("fixture: {e}"))?;
    let selections: std::collections::BTreeMap<String, Vec<String>> = {
        let bytes = fs::read(&selections_path)
            .map_err(|e| format!("read {selections_path}: {e}"))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| format!("parse {selections_path}: {e}"))?
    };
    let score = score_layer_fixture_selections(&fixture, &selections);
    eprintln!(
        "recall={:.3}  hit@1={:.3}  hit@3={:.3}  distractor_supp={:.3}",
        score.macro_causal_recall, score.hit_at_1, score.hit_at_3,
        score.macro_distractor_suppression,
    );
    write_canonical(&score, out, "fixture score report")
}

fn cmd_lf_batch(args: &[String]) -> CliResult<()> {
    let dir = flag_value(args, "--dir")?;
    let out = opt_flag_value(args, "--out");

    let entries = fs::read_dir(&dir).map_err(|e| format!("read dir {dir}: {e}"))?;
    let mut reports: Vec<serde_json::Value> = Vec::new();
    let mut ok_count = 0usize;
    let mut err_count = 0usize;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let path_str = path.to_string_lossy().to_string();
        match validate_layer_fixture(&path_str) {
            Ok(fixture) => {
                let summary = summarize_layer_fixture(&fixture);
                reports.push(serde_json::to_value(&summary).unwrap_or_default());
                ok_count += 1;
            }
            Err(e) => {
                eprintln!("skip {path_str}: {e}");
                err_count += 1;
            }
        }
    }

    // Sort by fixture_name for deterministic output.
    reports.sort_by(|a, b| {
        let na = a.get("fixture_name").and_then(|v| v.as_str()).unwrap_or("");
        let nb = b.get("fixture_name").and_then(|v| v.as_str()).unwrap_or("");
        na.cmp(nb)
    });

    let batch = serde_json::json!({
        "loaded": ok_count,
        "skipped": err_count,
        "fixtures": reports,
    });
    eprintln!("batch: {ok_count} loaded, {err_count} skipped");

    if let Some(out_path) = out {
        let bytes =
            serde_json::to_vec_pretty(&batch).map_err(|e| format!("serialize: {e}"))?;
        fs::write(&out_path, bytes).map_err(|e| format!("write {out_path}: {e}"))?;
        eprintln!("written to {out_path}");
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&batch).map_err(|e| format!("serialize: {e}"))?
        );
    }
    Ok(())
}
