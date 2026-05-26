use pse_bench_gt::scenarios::{run_binance_scenario, run_seismo_scenario, run_vitals_scenario};
use pse_bench_gt::{scenarios::ScenarioResult, score_detections};
use pse_types::Config;
use serde::Serialize;

#[derive(Serialize, Clone, Default)]
pub struct ScenarioBenchmark {
    pub name: String,
    pub pse_f1: f64,
    pub pse_precision: f64,
    pub pse_recall: f64,
    pub stl_f1: f64,
    pub stl_precision: f64,
    pub stl_recall: f64,
    pub isoforest_f1: f64,
    pub isoforest_precision: f64,
    pub isoforest_recall: f64,
    pub n_pse_detections: usize,
    pub n_ground_truth: usize,
}

#[derive(Serialize, Clone, Default)]
pub struct B5Results {
    pub seismo: ScenarioBenchmark,
    pub vitals: ScenarioBenchmark,
    pub binance: ScenarioBenchmark,
}

fn benchmark_scenario(result: &ScenarioResult, name: &str) -> ScenarioBenchmark {
    let tol = result.tolerance_ticks;

    let pse_dets: Vec<_> = result
        .detections
        .iter()
        .filter(|d| d.source.starts_with("pse_"))
        .cloned()
        .collect();
    let stl_dets: Vec<_> = result
        .detections
        .iter()
        .filter(|d| d.source == "stl_zscore")
        .cloned()
        .collect();
    let iso_dets: Vec<_> = result
        .detections
        .iter()
        .filter(|d| d.source == "isoforest")
        .cloned()
        .collect();

    let pse_m = score_detections(&result.ground_truth, &pse_dets, tol);
    let stl_m = score_detections(&result.ground_truth, &stl_dets, tol);
    let iso_m = score_detections(&result.ground_truth, &iso_dets, tol);

    ScenarioBenchmark {
        name: name.to_string(),
        pse_f1: pse_m.f1,
        pse_precision: pse_m.precision,
        pse_recall: pse_m.recall,
        stl_f1: stl_m.f1,
        stl_precision: stl_m.precision,
        stl_recall: stl_m.recall,
        isoforest_f1: iso_m.f1,
        isoforest_precision: iso_m.precision,
        isoforest_recall: iso_m.recall,
        n_pse_detections: pse_dets.len(),
        n_ground_truth: result.ground_truth.len(),
    }
}

pub fn run() -> B5Results {
    let config = Config::preset_anomaly_detection();

    let seismo = run_seismo_scenario(&config, 5);
    let vitals = run_vitals_scenario(&config, 20);
    let binance = run_binance_scenario(&config, 3);

    B5Results {
        seismo: benchmark_scenario(&seismo, &seismo.scenario),
        vitals: benchmark_scenario(&vitals, &vitals.scenario),
        binance: benchmark_scenario(&binance, &binance.scenario),
    }
}
