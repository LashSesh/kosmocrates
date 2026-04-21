//! Command-line driver for the ground-truth benchmark suite.
//!
//! Currently runs a single scenario (seismo) and prints a markdown summary
//! split by detection source so PSE and the classical baselines can be
//! compared side-by-side. Additional scenarios join in later increments.

use pse_bench_gt::scenarios::{run_seismo_scenario, ScenarioResult, SEISMO_DEFAULT_TOLERANCE};
use pse_bench_gt::{metrics_by_source, Metrics};
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

    println!("### Detections per source");
    println!();
    let by_src = bucket_count_by_source(&result.detections);
    println!("| Source | # detections |");
    println!("|--------|--------------|");
    for (src, n) in &by_src {
        println!("| `{}` | {} |", src, n);
    }
    println!();

    println!("### Metrics per source");
    println!();
    let per_source = metrics_by_source(
        &result.ground_truth,
        &result.detections,
        result.tolerance_ticks,
    );
    println!("| Source | TP | FP | FN | Precision | Recall | F1 | AUPRC |");
    println!("|--------|----|----|----|-----------|--------|-----|-------|");
    for (src, m) in &per_source {
        println!(
            "| `{}` | {} | {} | {} | {:.4} | {:.4} | {:.4} | {} |",
            src, m.tp, m.fp, m.fn_, m.precision, m.recall, m.f1, fmt_auprc(m)
        );
    }
    println!();

    println!("### Aggregate (all sources combined)");
    println!();
    let m = &result.metrics;
    println!("| Metric | Value |");
    println!("|--------|-------|");
    println!("| TP | {} |", m.tp);
    println!("| FP | {} |", m.fp);
    println!("| FN | {} |", m.fn_);
    println!("| Precision | {:.4} |", m.precision);
    println!("| Recall | {:.4} |", m.recall);
    println!("| F1 | {:.4} |", m.f1);
    println!("| AUPRC | {} |", fmt_auprc(m));
    println!();
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
        Some(a) => format!("{:.4}", a),
        None => "_undefined_".to_string(),
    }
}
