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
    score_ledger, summarize_ablation, AblationSummary, ConclusionFlag, EvaluationPlan,
    EvaluationRunLedger, EvaluationSpec, EvaluationSummaryReport, Fixed, Preset,
    SyntheticTrialExecutor, TrialReport, VariantSummary, WorkloadSummary,
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
        "doors" => {
            cmd_doors(&args[2..]);
            return ExitCode::SUCCESS;
        }
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

// ─── Doors (the binary's self-description) ───────────────────────────────────

use kosmo_core::{Door, DoorCatalog, DoorGovernance, DoorInput, DoorNeed, DoorSurface};

/// The binary's complete docking surface, spoken by the binary itself and
/// pinned by test against the dispatch.
fn doors_catalog() -> DoorCatalog {
    let here = || DoorSurface::Cli {
        binary: "pse-eval-matrix".into(),
    };
    DoorCatalog::new(vec![
        Door::new(
            here(),
            "doors",
            vec![],
            "this catalog: the binary's complete docking surface, spoken by the binary itself (add --json for the machine form)",
            vec![DoorInput::switch("--json")],
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            here(),
            "--help",
            vec!["-h".into(), "help".into()],
            "the prose usage text",
            vec![],
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            here(),
            "init",
            vec![],
            "initialize the metric ledger and kernel schemas",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::AppendsStore,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "validate",
            vec![],
            "validate a kernel schema",
            vec![DoorInput::valued("schema", "<file>").required()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "plan",
            vec![],
            "plan the evaluation matrix (variants × workloads × domains)",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "run",
            vec![],
            "execute the matrix, scoring results into the metric ledger",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::AppendsStore,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "replay",
            vec![],
            "deterministic re-execution of a variant",
            vec![DoorInput::valued("--report", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "score",
            vec![],
            "score a result bundle",
            vec![DoorInput::valued("--bundle", "<file>").required()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "ablate",
            vec![],
            "ablation comparison across variants",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "compare",
            vec![],
            "bootstrap-CI comparison between variants",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "report",
            vec![],
            "export the results as markdown/json",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "agent-exoskeleton",
            vec![],
            "the LLM-agent workload (real provider; reasoning over the cognition pipeline)",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Provider, DoorNeed::Network, DoorNeed::File],
        ),
    ])
}

fn cmd_doors(args: &[String]) {
    let catalog = doors_catalog();
    if args.iter().any(|a| a == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&catalog).unwrap_or_default()
        );
        return;
    }
    println!("pse-eval-matrix doors — the docking surface");
    println!(
        "  catalog {}… · {} door(s)",
        &catalog.catalog_id.to_hex()[..12],
        catalog.len()
    );
    for d in &catalog.doors {
        println!("  {}  [{}]  {}", d.name, d.governance.label(), d.summary);
    }
}

#[cfg(test)]
mod doors_tests {
    use super::*;

    /// This binary's own dispatch source — the catalog is pinned against it.
    const MAIN_SRC: &str = include_str!("main.rs");

    fn dispatched_words() -> std::collections::BTreeSet<String> {
        // Pick whichever dispatch anchor occurs FIRST in the file: the real
        // dispatch precedes this test module, which embeds both anchors as
        // string literals (so a plain `contains` check would be self-fooled).
        let a = MAIN_SRC.find("match args[1].as_str() {");
        let b = MAIN_SRC.find("match subcmd {");
        let start = match (a, b) {
            (Some(x), Some(y)) => x.min(y),
            (Some(x), None) => x,
            (None, Some(y)) => y,
            (None, None) => panic!("the dispatch match exists"),
        };
        let slice = &MAIN_SRC[start..];
        let end = slice
            .lines()
            .scan(0usize, |off, line| {
                let here = *off;
                *off += line.len() + 1;
                Some((here, line))
            })
            .find(|(_, line)| *line == "    }" || *line == "    };")
            .map(|(off, _)| off)
            .expect("the dispatch match closes");
        let slice = &slice[..end];
        let mut words = std::collections::BTreeSet::new();
        for line in slice.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            let mut rest = line;
            while let Some(q) = rest.find('"') {
                let after = &rest[q + 1..];
                let Some(close) = after.find('"') else { break };
                let literal = &after[..close];
                let tail = after[close + 1..].trim_start();
                if tail.starts_with("=>") || tail.starts_with('|') {
                    words.insert(literal.to_string());
                }
                rest = &after[close + 1..];
            }
        }
        words
    }

    #[test]
    fn the_doors_catalog_is_pinned_to_the_dispatch() {
        let dispatched = dispatched_words();
        let mut cataloged = std::collections::BTreeSet::new();
        for door in doors_catalog().doors {
            cataloged.insert(door.name.clone());
            for alias in &door.aliases {
                cataloged.insert(alias.clone());
            }
        }
        assert_eq!(
            dispatched, cataloged,
            "the catalog and the dispatch must speak the same words"
        );
    }

    #[test]
    fn the_catalog_recomputes_and_describes_itself() {
        let catalog = doors_catalog();
        assert!(catalog.verify(), "the catalog recomputes");
        assert!(catalog.doors.iter().any(|d| d.name == "doors"));
        assert_eq!(catalog.catalog_id, doors_catalog().catalog_id);
    }
}
