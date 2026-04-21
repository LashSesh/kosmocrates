//! Ground-truth benchmark scenarios.
//!
//! Each scenario packages (observation stream, ground-truth labels,
//! domain adapter, tolerance window) into a single `run_*_scenario()` call
//! that returns a [`ScenarioResult`] with PSE detections and aggregate
//! metrics. Additional scenarios (vitals, binance) are added in later
//! increments; seismo is the first one because its ground-truth indices
//! are the sharpest (one index = one event).

use pse_adapter_seismo::{embedded_seismo_data, embedded_seismo_ground_truth, SeismoAdapter};
use pse_core::GlobalState;
use pse_types::Config;
use serde::{Deserialize, Serialize};

use crate::baselines::{isoforest, stl_zscore};
use crate::{score_detections, Detection, GroundTruthEvent, Metrics};

/// Default tolerance windows per scenario. Reasoning:
///  - seismo: 1 event per tick; tolerance 5 accounts for the fact that PSE
///    may need a few ticks of graph-building before it latches on to the
///    mainshock anomaly.
pub const SEISMO_DEFAULT_TOLERANCE: u64 = 5;

/// Aggregate result of running one scenario end-to-end.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioResult {
    /// Human-readable scenario name, e.g. `"seismo_mainshock"`.
    pub scenario: String,
    /// Number of observations fed into the engine.
    pub n_observations: u64,
    /// Tolerance window used for matching (in ticks).
    pub tolerance_ticks: u64,
    /// Ground-truth events after normalization to `bench-gt` canonical form.
    pub ground_truth: Vec<GroundTruthEvent>,
    /// All detections emitted by PSE during the run.
    pub detections: Vec<Detection>,
    /// Aggregate metrics (precision, recall, F1, AUPRC).
    pub metrics: Metrics,
}

/// Run the seismo ground-truth scenario end-to-end against a fresh PSE
/// engine instance plus the registered classical baselines (currently
/// detrended rolling-z-score on event magnitudes).
///
/// Uses [`embedded_seismo_data`] as the input stream (200 events, with an
/// M6.0 mainshock at index 184 and 15 aftershocks at 185..200) and
/// [`embedded_seismo_ground_truth`] as the labels, both normalized into
/// `pse-bench-gt`'s canonical [`GroundTruthEvent`] form. The combined
/// [`ScenarioResult::detections`] vector carries multiple `source` tags
/// (`pse_crystal`, `pse_memory_hit`, `stl_zscore`); use
/// [`crate::metrics_by_source`] to split them.
pub fn run_seismo_scenario(config: &Config, tolerance_ticks: u64) -> ScenarioResult {
    let events = embedded_seismo_data();
    let adapter = SeismoAdapter::new("pacific_rim");
    let mut state = GlobalState::new(config);

    // Pre-serialize so the hot loop in the runner only calls macro_step.
    let payloads: Vec<Vec<u8>> = events
        .iter()
        .map(|e| serde_json::to_vec(e).expect("seismo event must serialize"))
        .collect();

    let mut detections = crate::runner::run_pse(&mut state, &payloads, config, &adapter);

    // Classical baselines on event magnitudes — same tick frame as PSE.
    let features = extract_seismo_features(&events);
    let stl_cfg = stl_zscore::StlZscoreConfig::default();
    detections.extend(stl_zscore::detect(&features, &stl_cfg));

    // Isolation Forest expects a row-per-tick matrix; promote the 1-D
    // magnitude series into 1-feature vectors.
    let if_samples: Vec<Vec<f64>> = features.iter().map(|m| vec![*m]).collect();
    let if_cfg = isoforest::IsoForestConfig::default();
    detections.extend(isoforest::detect(&if_samples, &if_cfg));

    let ground_truth: Vec<GroundTruthEvent> = embedded_seismo_ground_truth()
        .into_iter()
        .map(|e| GroundTruthEvent {
            start_index: e.start_index as u64,
            end_index: e.end_index as u64,
            label: e.label.to_string(),
            severity: e.severity,
        })
        .collect();

    let metrics = score_detections(&ground_truth, &detections, tolerance_ticks);

    ScenarioResult {
        scenario: "seismo_mainshock".into(),
        n_observations: payloads.len() as u64,
        tolerance_ticks,
        ground_truth,
        detections,
        metrics,
    }
}

/// Extract the per-tick scalar feature for a seismo stream that is fed to
/// classical baselines: the event magnitude. Index `i` in the returned vec
/// corresponds to PSE tick `i + 1`.
pub fn extract_seismo_features(events: &[pse_adapter_seismo::SeismoEvent]) -> Vec<f64> {
    events.iter().map(|e| e.magnitude).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seismo_scenario_runs_to_completion() {
        let config = Config::default();
        let result = run_seismo_scenario(&config, SEISMO_DEFAULT_TOLERANCE);
        assert_eq!(result.n_observations, 200);
        assert_eq!(result.ground_truth.len(), 2);
        // The runner must have been invoked and produced a well-formed
        // metrics record. We do NOT assert specific precision/recall here —
        // that is the actual research question studied by the CLI bench;
        // this test only proves end-to-end wiring.
        assert!(result.metrics.tp + result.metrics.fn_ > 0);
    }

    #[test]
    fn seismo_scenario_ground_truth_is_normalized() {
        let config = Config::default();
        let result = run_seismo_scenario(&config, 0);
        let mainshock = result.ground_truth.iter()
            .find(|e| e.label == "mainshock")
            .expect("mainshock must be present after normalization");
        assert_eq!(mainshock.start_index, 184);
        assert_eq!(mainshock.end_index, 185);
        assert_eq!(mainshock.severity, 6.0);
    }

    /// Documented empirical property of the seismo embedded dataset.
    ///
    /// The "mainshock" at M6.0 is **not statistically anomalous in
    /// magnitude alone** because the background events span [1.5, 6.0)
    /// uniformly. This test pins that property in code so downstream
    /// users of the seismo benchmark know that magnitude-only baselines
    /// have a structural ceiling on this scenario; meaningful detection
    /// requires features that capture spatio-temporal clustering of
    /// the aftershock sequence.
    #[test]
    fn seismo_mainshock_is_not_magnitude_anomaly_against_background() {
        let events = pse_adapter_seismo::embedded_seismo_data();
        let bg: Vec<f64> = events[0..184].iter().map(|e| e.magnitude).collect();
        let mean = bg.iter().sum::<f64>() / bg.len() as f64;
        let var = bg.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
            / (bg.len() as f64 - 1.0);
        let std = var.sqrt();
        let z_main = (events[184].magnitude - mean) / std;
        // Loose bound — the mainshock z-score is well below 3σ.
        assert!(
            z_main.abs() < 2.5,
            "expected mainshock NOT to be a 3σ outlier in magnitude, \
             got z = {:.3} (background mean={:.3}, std={:.3})",
            z_main,
            mean,
            std
        );
        // Several background events also reach magnitude >= 5.5, so the
        // mainshock is not even at the extreme tail of the empirical
        // distribution.
        let extremes = bg.iter().filter(|&&m| m >= 5.5).count();
        assert!(
            extremes > 0,
            "expected at least one background event with mag>=5.5; \
             got {}, which means our 'no magnitude anomaly' premise \
             would not hold",
            extremes
        );
    }
}
