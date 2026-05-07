//! Window-size ablation study + JSON output.
//!
//! Sweeps the sliding-window size across each scenario and writes both
//! a markdown table to stdout and a JSON file to disk for downstream
//! plotting / analysis.
//!
//! Run with:
//!   `cargo run --release -p pse-bench-gt --bin window_ablation`
//!
//! The output JSON has the schema:
//! ```json
//! {
//!   "scenarios": [
//!     {
//!       "name": "seismo_mainshock",
//!       "tolerance_ticks": 5,
//!       "rows": [
//!         { "window_size": 1,  "by_source": { "isoforest": {"tp":1,...}, ... } },
//!         { "window_size": 4,  "by_source": { ... } },
//!         ...
//!       ]
//!     },
//!     ...
//!   ]
//! }
//! ```

use pse_bench_gt::scenarios::{
    run_binance_scenario_with, run_seismo_scenario_with, run_vitals_scenario_with,
    BINANCE_DEFAULT_TOLERANCE, SEISMO_DEFAULT_TOLERANCE, VITALS_DEFAULT_TOLERANCE,
};
use pse_bench_gt::{metrics_by_source, Metrics};
use pse_types::Config;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

const WINDOW_SIZES: &[usize] = &[1, 4, 8, 16, 32];

#[derive(Serialize)]
struct AblationRow {
    window_size: usize,
    by_source: BTreeMap<String, Metrics>,
}

#[derive(Serialize)]
struct ScenarioAblation {
    name: String,
    tolerance_ticks: u64,
    rows: Vec<AblationRow>,
}

#[derive(Serialize)]
struct AblationReport {
    scenarios: Vec<ScenarioAblation>,
}

fn main() {
    let cfg = Config::default();

    println!("# PSE Window-Size Ablation");
    println!();
    println!(
        "Sweeps the sliding-window size across each scenario. Per-source \
         metrics are reported so the contribution of PSE versus the classical \
         baselines can be read independently. JSON written to \
         `bench_window_ablation.json`."
    );
    println!();

    let mut report = AblationReport {
        scenarios: Vec::new(),
    };

    for (name, runner) in scenarios() {
        println!("## Scenario: `{}`", name);
        println!();
        let tolerance = match name {
            "seismo_mainshock" => SEISMO_DEFAULT_TOLERANCE,
            "vitals_afib_patient_b" => VITALS_DEFAULT_TOLERANCE,
            "binance_regime_shift" => BINANCE_DEFAULT_TOLERANCE,
            _ => 5,
        };
        let mut all_sources: std::collections::BTreeSet<String> = Default::default();
        let mut rows = Vec::new();

        // First pass: collect rows and observe the source set.
        for &w in WINDOW_SIZES {
            let result = runner(&cfg, tolerance, w);
            let by_source = metrics_by_source(&result.ground_truth, &result.detections, tolerance);
            for src in by_source.keys() {
                all_sources.insert(src.clone());
            }
            rows.push((w, by_source));
        }

        // Print one row per window size, with a column per known source.
        let sources: Vec<String> = all_sources.into_iter().collect();
        let header_extra: String = sources
            .iter()
            .map(|s| format!(" {} F1 |", s))
            .collect::<Vec<_>>()
            .join("");
        println!("| Window |{}", header_extra);
        let separator: String = sources
            .iter()
            .map(|_| " --- |".to_string())
            .collect::<Vec<_>>()
            .join("");
        println!("|--------|{}", separator);
        let mut json_rows = Vec::new();
        for (w, by_src) in &rows {
            let cells: String = sources
                .iter()
                .map(|s| {
                    by_src
                        .get(s)
                        .map(|m| format!(" {:.4} |", m.f1))
                        .unwrap_or_else(|| " — |".to_string())
                })
                .collect::<Vec<_>>()
                .join("");
            println!("| {} |{}", w, cells);
            json_rows.push(AblationRow {
                window_size: *w,
                by_source: by_src.clone(),
            });
        }
        println!();

        report.scenarios.push(ScenarioAblation {
            name: name.to_string(),
            tolerance_ticks: tolerance,
            rows: json_rows,
        });
    }

    let path = PathBuf::from("bench_window_ablation.json");
    let json = serde_json::to_string_pretty(&report).expect("serializable report");
    std::fs::write(&path, json).expect("write JSON output");
    println!("JSON written to `{}`.", path.display());
}

type Runner = fn(&Config, u64, usize) -> pse_bench_gt::scenarios::ScenarioResult;

fn scenarios() -> Vec<(&'static str, Runner)> {
    vec![
        ("seismo_mainshock", run_seismo_scenario_with as Runner),
        ("vitals_afib_patient_b", run_vitals_scenario_with as Runner),
        ("binance_regime_shift", run_binance_scenario_with as Runner),
    ]
}
