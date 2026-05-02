//! Detrended rolling-z-score anomaly detector.
//!
//! Algorithm (one pass, deterministic, no RNG, fully **causal** —
//! the detector at tick `t` sees only values up to `t`):
//!  1. **Detrend.** For each index `i >= trend_window - 1`, compute the
//!     trailing moving average over `values[i - trend_window + 1 ..= i]`.
//!     The residual is `values[i] - trend[i]`. Indices before the warm-up
//!     boundary are skipped (no detection emitted there).
//!  2. **Z-score.** For each index `i` past the warm-up (and once at least
//!     `zscore_window` residuals exist), compute the mean and standard
//!     deviation of `residuals[i - zscore_window .. i]`. The z-score is
//!     `z[i] = (residuals[i] - mean) / max(std, eps)`.
//!  3. **Threshold.** Emit a [`Detection`] at every `i` where `|z[i]| >=
//!     threshold`. Detection score is `min(|z[i]| / threshold, 1.0)` so a
//!     just-at-threshold detection scores 1.0 by construction; that gives
//!     the AUPRC sweep a meaningful range.
//!
//! The trailing (causal) MA is chosen over a centered one to (a) avoid
//! spike bleed-back into earlier ticks and (b) avoid any lookahead, so
//! the baseline is honest about what an online detector could see.
//!
//! This is "STL-residual-z-score" in the spirit of classical statistical
//! anomaly detection — the trend is removed by a moving average rather
//! than by Loess decomposition (the full STL pipeline is heavy and adds
//! little value at the small sample sizes used in our benchmark scenarios).
//! The baseline is fully deterministic and parameter-free beyond
//! `(trend_window, zscore_window, threshold)`.
//!
//! Tick correspondence: feature value at index `i` corresponds to PSE
//! tick `i + 1`, because PSE increments `commit_index` at the top of
//! every macro-step (so the first observation lives at tick 1).

use crate::Detection;

/// Default detector source string emitted on every detection.
pub const SOURCE: &str = "stl_zscore";

/// Configuration for the detrended rolling z-score detector.
#[derive(Clone, Debug)]
pub struct StlZscoreConfig {
    /// Centered moving-average window used for trend extraction.
    pub trend_window: usize,
    /// Backward-looking window for residual mean/std.
    pub zscore_window: usize,
    /// Detection threshold on |z|. A value of 3.0 corresponds to the
    /// classical "3-sigma rule".
    pub threshold: f64,
}

impl Default for StlZscoreConfig {
    fn default() -> Self {
        Self {
            trend_window: 20,
            zscore_window: 30,
            // 2.5σ — between the 2σ ("loose") and 3σ ("strict") conventions.
            // The choice is deliberate: 3σ is too tight on small samples
            // (rarely fires, low recall), 2σ generates many FPs.
            threshold: 2.5,
        }
    }
}

/// Run the detector over a feature series and return detections.
///
/// The output detection ticks are 1-indexed to align with PSE's
/// post-`macro_step` `commit_index` (see module docs).
pub fn detect(values: &[f64], config: &StlZscoreConfig) -> Vec<Detection> {
    let warmup = config.trend_window.saturating_sub(1) + config.zscore_window;
    if values.len() <= warmup {
        return Vec::new();
    }

    let trend = trailing_moving_average(values, config.trend_window);
    let residuals: Vec<f64> = values.iter().zip(trend.iter()).map(|(v, t)| v - t).collect();

    // Detection loop starts past the warm-up. Earlier indices either have
    // a truncated trend window or insufficient residual history for the
    // z-score window — both would inject boundary artifacts.
    let mut detections = Vec::new();
    for i in warmup..residuals.len() {
        let window = &residuals[i - config.zscore_window..i];
        let mean = window.iter().sum::<f64>() / window.len() as f64;
        let var = window.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
            / (window.len() as f64 - 1.0).max(1.0);
        let std = var.sqrt().max(1e-12);
        let z = (residuals[i] - mean) / std;
        let abs_z = z.abs();
        if abs_z >= config.threshold {
            let score = (abs_z / config.threshold).min(1.0);
            detections.push(Detection::new(i as u64 + 1, score, SOURCE));
        }
    }

    detections
}

/// Trailing moving average over the most recent `window` samples.
///
/// For indices `i < window - 1` the window is truncated to `values[0..=i]`;
/// the [`detect`] loop skips these warm-up indices to avoid the
/// resulting bias.
fn trailing_moving_average(values: &[f64], window: usize) -> Vec<f64> {
    if window <= 1 || values.is_empty() {
        return values.to_vec();
    }
    let n = values.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let lo = (i + 1).saturating_sub(window);
        let hi = i + 1;
        let slice = &values[lo..hi];
        let mean = slice.iter().sum::<f64>() / slice.len() as f64;
        out.push(mean);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_no_detections() {
        let det = detect(&[], &StlZscoreConfig::default());
        assert!(det.is_empty());
    }

    #[test]
    fn short_input_yields_no_detections() {
        let cfg = StlZscoreConfig::default();
        let v: Vec<f64> = (0..5).map(|x| x as f64).collect();
        let det = detect(&v, &cfg);
        assert!(det.is_empty());
    }

    #[test]
    fn flat_signal_has_no_anomalies() {
        let cfg = StlZscoreConfig {
            trend_window: 5,
            zscore_window: 10,
            threshold: 3.0,
        };
        let v = vec![1.0_f64; 100];
        let det = detect(&v, &cfg);
        assert!(det.is_empty(), "flat signal must not trigger; got {:?}", det);
    }

    #[test]
    fn lone_spike_in_flat_signal_triggers_detection() {
        let cfg = StlZscoreConfig {
            trend_window: 5,
            zscore_window: 10,
            threshold: 3.0,
        };
        let mut v = vec![1.0_f64; 100];
        v[60] = 50.0;
        let det = detect(&v, &cfg);
        assert!(!det.is_empty(), "spike must trigger");
        // With a trailing MA, the spike at index 60 contaminates the trend
        // at indices 60..=64 (window=5). The first detection therefore lands
        // at index 60 itself (the spike's own large residual against a
        // still-flat z-score window) and follow-ups may fire in 60..=64.
        let first_idx = (det[0].tick - 1) as usize;
        assert!(
            (60..=64).contains(&first_idx),
            "expected first detection in 60..=64, got index {}",
            first_idx
        );
    }

    #[test]
    fn linear_trend_with_no_residual_anomaly_is_clean() {
        // A pure linear trend has zero residual after centered MA; no
        // anomalies should be reported.
        let cfg = StlZscoreConfig {
            trend_window: 11,
            zscore_window: 30,
            threshold: 3.0,
        };
        let v: Vec<f64> = (0..200).map(|i| 0.5 * i as f64).collect();
        let det = detect(&v, &cfg);
        assert!(det.is_empty(), "pure linear trend leaked detections: {:?}", det);
    }

    #[test]
    fn detection_score_is_clamped_to_unit_interval() {
        let cfg = StlZscoreConfig {
            trend_window: 5,
            zscore_window: 10,
            threshold: 3.0,
        };
        let mut v = vec![1.0_f64; 100];
        // Very large outlier — z-score should be huge, but the detection
        // score must stay in [0, 1].
        v[60] = 1.0e6;
        let det = detect(&v, &cfg);
        for d in &det {
            assert!((0.0..=1.0).contains(&d.score), "score out of range: {:?}", d);
        }
    }
}
