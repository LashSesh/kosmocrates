//! Command-line driver for the ground-truth benchmark suite.
//!
//! Currently runs a single scenario (seismo) and prints a markdown summary.
//! Additional scenarios and baseline comparators are wired in later.

use pse_bench_gt::scenarios::{run_seismo_scenario, ScenarioResult, SEISMO_DEFAULT_TOLERANCE};
use pse_types::Config;

fn main() {
    let config = Config::default();

    println!("# PSE Ground-Truth Benchmark");
    println!();

    let seismo = run_seismo_scenario(&config, SEISMO_DEFAULT_TOLERANCE);
    print_scenario_report(&seismo);
}

fn print_scenario_report(result: &ScenarioResult) {
    println!("## Scenario: `{}`", result.scenario);
    println!();
    println!(
        "{} observations, tolerance = {} ticks, {} ground-truth event(s).",
        result.n_observations,
        result.tolerance_ticks,
        result.ground_truth.len()
    );
    println!();

    println!("### Ground-truth events");
    println!();
    println!("| Label | Start | End | Severity |");
    println!("|-------|-------|-----|----------|");
    for ev in &result.ground_truth {
        println!(
            "| `{}` | {} | {} | {:.2} |",
            ev.label, ev.start_index, ev.end_index, ev.severity
        );
    }
    println!();

    println!("### PSE detections");
    println!();
    let crystals = result.detections.iter().filter(|d| d.source == "pse_crystal").count();
    let hits = result.detections.iter().filter(|d| d.source == "pse_memory_hit").count();
    println!("- `pse_crystal` detections: **{}**", crystals);
    println!("- `pse_memory_hit` detections: **{}**", hits);
    println!();

    println!("### Metrics");
    println!();
    println!("| Metric | Value |");
    println!("|--------|-------|");
    println!("| TP | {} |", result.metrics.tp);
    println!("| FP | {} |", result.metrics.fp);
    println!("| FN | {} |", result.metrics.fn_);
    println!("| Precision | {:.4} |", result.metrics.precision);
    println!("| Recall | {:.4} |", result.metrics.recall);
    println!("| F1 | {:.4} |", result.metrics.f1);
    match result.metrics.auprc {
        Some(a) => println!("| AUPRC | {:.4} |", a),
        None => println!("| AUPRC | _undefined (constant scores)_ |"),
    }
    println!();
}
