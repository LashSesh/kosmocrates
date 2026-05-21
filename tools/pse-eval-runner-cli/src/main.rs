//! PSE Eval Runner CLI
//!
//! Führt den Reasoning-Agent (Plan → Execute → Evaluate) durch die
//! PSE-Kognitionspipeline.  Ohne CEREBRAS_API_KEY läuft ein
//! deterministischer Stub.
//!
//! Usage:
//! ```text
//!   pse_eval_runner run   [--preset NAME] [--model MODEL]
//!   pse_eval_runner score --bundle bundle.json
//!   pse_eval_runner show  --bundle bundle.json
//! ```
//!
//! Presets: agent-cognition (default), post-symbolic-ablation, lpcm-fragment-collapse

use std::env;
use std::process::ExitCode;

use pse_eval_matrix::{
    append_to_ledger, init_ledger, plan_runs, render_markdown_summary, run_trial,
    score_capability_profile, score_ledger, EvaluationSummaryReport, Fixed, Preset, TrialReport,
};
use pse_eval_runner::CerebrasTrialExecutor;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("run");

    let result = match subcmd {
        "run" => cmd_run(&args[2..]),
        "score" => cmd_score(&args[2..]),
        "show" => cmd_show(&args[2..]),
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        other => Err(format!("unbekannter Subcommand: {other}\n\n{USAGE}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Fehler: {e}");
            ExitCode::FAILURE
        }
    }
}

// ── run ───────────────────────────────────────────────────────────────────

fn cmd_run(args: &[String]) -> Result<(), String> {
    let preset_tag = flag_value(args, "--preset").unwrap_or_else(|_| "agent-cognition".into());
    let model = flag_value(args, "--model").unwrap_or_else(|_| "llama-3.3-70b".into());
    let out = flag_value(args, "--out").unwrap_or_else(|_| "eval_bundle.json".into());

    let preset = Preset::from_tag(&preset_tag)
        .ok_or_else(|| format!("unbekanntes Preset '{preset_tag}'"))?;
    let spec = preset.build().map_err(|e| format!("Preset build: {e:?}"))?;

    let api_key = env::var("CEREBRAS_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        eprintln!("CEREBRAS_API_KEY nicht gesetzt — läuft im Stub-Modus (deterministisch).");
    } else {
        eprintln!("Cerebras API aktiv: model={model}");
    }

    let executor = CerebrasTrialExecutor { model };
    let plan = plan_runs(&spec).map_err(|e| format!("plan_runs: {e:?}"))?;

    eprintln!(
        "Preset: {}  |  Varianten: {}  |  Workloads: {}  |  Trials: {}",
        preset_tag,
        spec.variants.len(),
        spec.workloads.len(),
        plan.entries.len()
    );

    let mut ledger = init_ledger(spec.spec_id.clone()).map_err(|e| e.to_string())?;
    let mut reports: Vec<TrialReport> = Vec::new();

    for (i, entry) in plan.entries.iter().enumerate() {
        let variant = spec
            .variants
            .iter()
            .find(|v| v.variant_id == entry.descriptor.variant_id)
            .ok_or_else(|| format!("Variante {} nicht gefunden", entry.descriptor.variant_id))?;
        let workload = spec
            .workloads
            .iter()
            .find(|w| w.workload_id == entry.descriptor.workload_id)
            .ok_or_else(|| format!("Workload {} nicht gefunden", entry.descriptor.workload_id))?;

        eprint!(
            "  [{:>2}/{}] {}  x  {} … ",
            i + 1,
            plan.entries.len(),
            variant.variant_id,
            workload.workload_id
        );

        let (report, run_entry) = run_trial(
            &spec,
            variant,
            workload,
            &entry.descriptor,
            &executor,
            &spec.metrics,
        )
        .map_err(|e| format!("run_trial: {e:?}"))?;

        let fcr = report
            .metrics
            .iter()
            .find(|m| m.metric_id == "false_commit_rate")
            .map(|m| format_fixed(&m.value))
            .unwrap_or_else(|| "?".into());
        let sc = report
            .metrics
            .iter()
            .find(|m| m.metric_id == "structural_coverage")
            .map(|m| format_fixed(&m.value))
            .unwrap_or_else(|| "?".into());

        eprintln!("structural_coverage={sc}  false_commit_rate={fcr}");

        ledger = append_to_ledger(ledger, run_entry).map_err(|e| e.to_string())?;
        reports.push(report);
    }

    let bundle = RunBundle {
        preset: preset_tag,
        spec,
        plan_id: plan.plan_id,
        ledger,
        reports,
    };
    let json = serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())?;
    std::fs::write(&out, json).map_err(|e| format!("write {out}: {e}"))?;
    eprintln!("\nBundle gespeichert: {out}");
    Ok(())
}

// ── score ─────────────────────────────────────────────────────────────────

fn cmd_score(args: &[String]) -> Result<(), String> {
    let bundle_path = flag_value(args, "--bundle")?;
    let bundle: RunBundle = read_json(&bundle_path)?;

    let (variants, workloads, invalid) =
        score_ledger(&bundle.ledger, &bundle.reports, &bundle.spec.metrics)
            .map_err(|e| format!("score_ledger: {e:?}"))?;

    if !invalid.is_empty() {
        eprintln!("Ungültige Runs: {}", invalid.len());
    }

    let profiles: Vec<_> = variants
        .iter()
        .map(|v| score_capability_profile(v, &bundle.spec.metrics).unwrap())
        .collect();

    let summary = EvaluationSummaryReport {
        report_id: pse_eval_matrix::primitives::Hash256::zero(),
        evaluation_spec_hash: bundle.spec.spec_id.clone(),
        generated_from_ledger: bundle.ledger.ledger_id.clone(),
        variant_summaries: variants,
        workload_summaries: workloads,
        ablation_summaries: vec![],
        capability_profiles: profiles,
        statistical_summaries: vec![],
        invalid_runs: invalid,
        conclusion_flags: vec![],
    }
    .with_id()
    .map_err(|e| format!("with_id: {e:?}"))?;

    let md = render_markdown_summary(&summary);
    println!("{md}");
    Ok(())
}

// ── show ──────────────────────────────────────────────────────────────────

fn cmd_show(args: &[String]) -> Result<(), String> {
    let bundle_path = flag_value(args, "--bundle")?;
    let bundle: RunBundle = read_json(&bundle_path)?;

    println!("Preset:  {}", bundle.preset);
    println!("Trials:  {}", bundle.reports.len());
    println!();
    println!(
        "{:<20} {:<12} {:<12} {:<12}",
        "Variante", "task_success", "false_commit", "struct_cov"
    );
    println!("{}", "-".repeat(58));

    for report in &bundle.reports {
        let ts = get_metric(report, "task_success");
        let fc = get_metric(report, "false_commit_rate");
        let sc = get_metric(report, "structural_coverage");
        println!(
            "{:<20} {:<12} {:<12} {:<12}",
            report.variant_id,
            format_fixed(&ts),
            format_fixed(&fc),
            format_fixed(&sc),
        );
    }
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
struct RunBundle {
    preset: String,
    spec: pse_eval_matrix::EvaluationSpec,
    plan_id: pse_eval_matrix::primitives::Hash256,
    ledger: pse_eval_matrix::EvaluationRunLedger,
    reports: Vec<TrialReport>,
}

fn flag_value(args: &[String], flag: &str) -> Result<String, String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
        .ok_or_else(|| format!("Flag '{flag}' fehlt"))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Datei nicht lesbar '{path}': {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("JSON parse '{path}': {e}"))
}

fn get_metric(report: &TrialReport, id: &str) -> Fixed {
    report
        .metrics
        .iter()
        .find(|m| m.metric_id == id)
        .map(|m| m.value.clone())
        .unwrap_or(Fixed::Rational { num: 0, den: 1 })
}

fn format_fixed(f: &Fixed) -> String {
    match f {
        Fixed::Rational { num, den } if *den != 0 => {
            format!("{:.2}", *num as f64 / *den as f64)
        }
        _ => format!("{f:?}"),
    }
}

const USAGE: &str = "\
pse_eval_runner — PSE Reasoning-Agent Evaluation

Subcommands:
  run    [--preset <name>] [--model <model>] [--out <bundle.json>]
  score  --bundle <bundle.json>
  show   --bundle <bundle.json>

Presets:
  agent-cognition          (Standard — B0 bis B6)
  post-symbolic-ablation   (vollständige B0-B7 Leiter)
  lpcm-fragment-collapse   (B0/B7/B10 mit LPCM-Metriken)

Umgebungsvariablen:
  CEREBRAS_API_KEY   Cerebras API Key (ohne Key: deterministischer Stub)

Beispiele:
  # Stub-Modus (kein API Key nötig):
  pse_eval_runner run --preset agent-cognition

  # Mit Cerebras API:
  CEREBRAS_API_KEY=... pse_eval_runner run --preset agent-cognition --out ergebnis.json
  pse_eval_runner score --bundle ergebnis.json
  pse_eval_runner show  --bundle ergebnis.json
";
