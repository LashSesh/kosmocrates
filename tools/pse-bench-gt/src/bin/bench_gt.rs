//! Command-line driver for the ground-truth benchmark suite.
//!
//! Usage:
//!   bench_gt                                   # text report, all scenarios
//!   bench_gt --scenario seismo                 # text report, one scenario
//!   bench_gt --scenario seismo --format json --out /tmp/seismo.json

use pse_bench_gt::scenarios::{
    run_binance_scenario, run_seismo_scenario, run_vitals_scenario, ScenarioResult,
    BINANCE_DEFAULT_TOLERANCE, SEISMO_DEFAULT_TOLERANCE, VITALS_DEFAULT_TOLERANCE,
};
use pse_bench_gt::{build_json_output, metrics_by_source, BenchGtJsonOutput, Metrics};
use pse_types::Config;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(e) = run(&args[1..]) {
        eprintln!("bench_gt error: {e}");
        std::process::exit(1);
    }
}

fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let scenario_filter = flag(args, "--scenario");
    let format = flag(args, "--format").unwrap_or_else(|| "text".into());
    let out_path = flag(args, "--out");

    let config = Config::default();
    let config_bytes = serde_json::to_vec(&config)?;

    let scenarios: Vec<ScenarioResult> = match scenario_filter.as_deref() {
        Some("seismo") => vec![run_seismo_scenario(&config, SEISMO_DEFAULT_TOLERANCE)],
        Some("vitals") => vec![run_vitals_scenario(&config, VITALS_DEFAULT_TOLERANCE)],
        Some("binance") => vec![run_binance_scenario(&config, BINANCE_DEFAULT_TOLERANCE)],
        None => vec![
            run_seismo_scenario(&config, SEISMO_DEFAULT_TOLERANCE),
            run_vitals_scenario(&config, VITALS_DEFAULT_TOLERANCE),
            run_binance_scenario(&config, BINANCE_DEFAULT_TOLERANCE),
        ],
        Some(other) => {
            return Err(format!("unknown scenario: {other}; use seismo, vitals, or binance").into())
        }
    };

    match format.as_str() {
        "json" => {
            if scenarios.len() != 1 {
                return Err(
                    "--format json requires exactly one --scenario (seismo | vitals | binance)"
                        .into(),
                );
            }
            let output: BenchGtJsonOutput = build_json_output(&scenarios[0], &config_bytes);
            let json = serde_json::to_string_pretty(&output)?;
            match &out_path {
                Some(path) => {
                    if let Some(parent) = std::path::Path::new(path).parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(path, &json)?;
                    eprintln!("bench_gt: wrote JSON → {path}");
                }
                None => println!("{json}"),
            }
        }
        "text" | "md" | "markdown" => {
            if let Some(path) = &out_path {
                let mut text = String::new();
                text.push_str("# PSE Ground-Truth Benchmark\n\n");
                for s in &scenarios {
                    text.push_str(&format_scenario_text(s));
                }
                if let Some(parent) = std::path::Path::new(path).parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(path, &text)?;
                eprintln!("bench_gt: wrote text → {path}");
            } else {
                println!("# PSE Ground-Truth Benchmark\n");
                for s in &scenarios {
                    print!("{}", format_scenario_text(s));
                }
            }
        }
        other => return Err(format!("unknown format: {other}; use text or json").into()),
    }

    Ok(())
}

fn flag(args: &[String], name: &str) -> Option<String> {
    let pos = args.iter().position(|a| a == name)?;
    args.get(pos + 1).cloned()
}

fn format_scenario_text(result: &ScenarioResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("## Scenario: `{}`\n\n", result.scenario));
    out.push_str(&format!(
        "{} observations, tolerance = {} ticks, {} ground-truth event(s).\n\n",
        result.n_observations,
        result.tolerance_ticks,
        result.ground_truth.len()
    ));

    out.push_str("### Ground-truth events\n\n");
    out.push_str("| Label | Start | End | Severity |\n");
    out.push_str("|-------|-------|-----|----------|\n");
    for ev in &result.ground_truth {
        out.push_str(&format!(
            "| `{}` | {} | {} | {:.2} |\n",
            ev.label, ev.start_index, ev.end_index, ev.severity
        ));
    }
    out.push('\n');

    out.push_str("### Detections per source\n\n");
    let by_src = bucket_count_by_source(&result.detections);
    out.push_str("| Source | # detections |\n");
    out.push_str("|--------|---------------|\n");
    for (src, n) in &by_src {
        out.push_str(&format!("| `{src}` | {n} |\n"));
    }
    out.push('\n');

    out.push_str("### Metrics per source\n\n");
    let per_source = metrics_by_source(
        &result.ground_truth,
        &result.detections,
        result.tolerance_ticks,
    );
    out.push_str("| Source | TP | FP | FN | Precision | Recall | F1 | AUPRC |\n");
    out.push_str("|--------|----|----|----|-----------|--------|-----|-------|\n");
    for (src, m) in &per_source {
        out.push_str(&format!(
            "| `{src}` | {} | {} | {} | {:.4} | {:.4} | {:.4} | {} |\n",
            m.tp,
            m.fp,
            m.fn_,
            m.precision,
            m.recall,
            m.f1,
            fmt_auprc(m)
        ));
    }
    out.push('\n');

    out.push_str("### Aggregate (all sources combined)\n\n");
    let m = &result.metrics;
    out.push_str("| Metric | Value |\n");
    out.push_str("|--------|-------|\n");
    out.push_str(&format!("| TP | {} |\n", m.tp));
    out.push_str(&format!("| FP | {} |\n", m.fp));
    out.push_str(&format!("| FN | {} |\n", m.fn_));
    out.push_str(&format!("| Precision | {:.4} |\n", m.precision));
    out.push_str(&format!("| Recall | {:.4} |\n", m.recall));
    out.push_str(&format!("| F1 | {:.4} |\n", m.f1));
    out.push_str(&format!("| AUPRC | {} |\n", fmt_auprc(m)));
    out.push('\n');

    out
}

fn bucket_count_by_source(
    detections: &[pse_bench_gt::Detection],
) -> std::collections::BTreeMap<String, usize> {
    let mut out = std::collections::BTreeMap::new();
    for d in detections {
        *out.entry(d.source.clone()).or_insert(0) += 1;
    }
    out
}

fn fmt_auprc(m: &Metrics) -> String {
    match m.auprc {
        Some(a) => format!("{a:.4}"),
        None => "_undefined_".to_string(),
    }
}
