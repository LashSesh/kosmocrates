use pse_bench_gt::scenarios::{run_seismo_scenario, run_vitals_scenario, run_binance_scenario};
use pse_types::Config;
use serde::Serialize;

#[derive(Serialize, Clone, Default)]
pub struct ScenarioBenchmark {
    pub name: String,
    pub pse_f1: f64,
    pub pse_precision: f64,
    pub pse_recall: f64,
    pub n_detections: usize,
    pub n_ground_truth: usize,
}

#[derive(Serialize, Clone, Default)]
pub struct B5Results {
    pub seismo: ScenarioBenchmark,
    pub vitals: ScenarioBenchmark,
    pub binance: ScenarioBenchmark,
}

pub fn run() -> B5Results {
    let config = Config::preset_streaming();

    let seismo = run_seismo_scenario(&config, 5);
    let vitals  = run_vitals_scenario(&config, 20);
    let binance = run_binance_scenario(&config, 3);

    B5Results {
        seismo: ScenarioBenchmark {
            name: seismo.scenario,
            pse_f1: seismo.metrics.f1,
            pse_precision: seismo.metrics.precision,
            pse_recall: seismo.metrics.recall,
            n_detections: seismo.detections.len(),
            n_ground_truth: seismo.ground_truth.len(),
        },
        vitals: ScenarioBenchmark {
            name: vitals.scenario,
            pse_f1: vitals.metrics.f1,
            pse_precision: vitals.metrics.precision,
            pse_recall: vitals.metrics.recall,
            n_detections: vitals.detections.len(),
            n_ground_truth: vitals.ground_truth.len(),
        },
        binance: ScenarioBenchmark {
            name: binance.scenario,
            pse_f1: binance.metrics.f1,
            pse_precision: binance.metrics.precision,
            pse_recall: binance.metrics.recall,
            n_detections: binance.detections.len(),
            n_ground_truth: binance.ground_truth.len(),
        },
    }
}
