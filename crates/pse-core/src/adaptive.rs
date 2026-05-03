//! Adaptive Kairos calibration.
//!
//! Static thresholds (`Config::thresholds`) require per-workload tuning:
//! a threshold that's appropriate for batch full-graph evolution is
//! structurally unreachable for sliding-window streaming, and vice-versa.
//! That makes the engine hard to deploy on any workload its author
//! hasn't pre-calibrated.
//!
//! [`AdaptiveCalibrator`] solves this by tracking a rolling history of
//! `GateSnapshot`s and deriving thresholds from quantiles of that
//! history. The kairos gate fires on the **top `target_pass_rate`
//! fraction** of recent ticks — i.e., when current resonance is
//! exceptional relative to recent context, regardless of absolute
//! magnitude. This is the standard novelty-detection contract used by
//! production anomaly systems, applied here to all eight gate metrics
//! simultaneously.
//!
//! Semantics:
//! * Per metric: `threshold = (1 − target_pass_rate)-th quantile of history`.
//! * During warmup (first `warmup_ticks` ticks) the calibrator returns
//!   the static base thresholds unchanged.
//! * The kairos-gate composition (8-fold AND) is unchanged — adaptive
//!   thresholds only change the per-metric cut points, not the
//!   conjunction logic, so falsification, EU-AI-Act compliance proofs,
//!   and crystal verifiability all remain intact.

use std::collections::VecDeque;

use pse_types::{GateSnapshot, ThresholdConfig};

/// Rolling-history adaptive Kairos threshold calibrator.
#[derive(Clone, Debug)]
pub struct AdaptiveCalibrator {
    /// Rolling buffer of recent gate snapshots. Bounded by `window`.
    history: VecDeque<GateSnapshot>,
    /// Target fraction of ticks expected to pass the gate (e.g. 0.05 = top 5%).
    pub target_pass_rate: f64,
    /// History window size.
    pub window: usize,
    /// Number of initial ticks during which static thresholds are used.
    pub warmup_ticks: usize,
    /// Total ticks observed (for warmup gating; not the same as history.len()).
    pub seen: u64,
}

impl AdaptiveCalibrator {
    /// New calibrator. `target_pass_rate` ∈ (0, 1); typical 0.02–0.10.
    pub fn new(target_pass_rate: f64, window: usize, warmup_ticks: usize) -> Self {
        assert!(
            (0.0..1.0).contains(&target_pass_rate),
            "target_pass_rate must be in (0, 1)"
        );
        assert!(window > 0, "window must be positive");
        Self {
            history: VecDeque::with_capacity(window),
            target_pass_rate,
            window,
            warmup_ticks,
            seen: 0,
        }
    }

    /// Record a gate snapshot. The next call to `calibrate` will incorporate it.
    pub fn record(&mut self, snapshot: &GateSnapshot) {
        if self.history.len() == self.window {
            self.history.pop_front();
        }
        self.history.push_back(snapshot.clone());
        self.seen = self.seen.saturating_add(1);
    }

    /// Number of snapshots currently in the rolling buffer.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Whether warmup is still in effect.
    pub fn in_warmup(&self) -> bool {
        self.seen < self.warmup_ticks as u64
    }

    /// Compute calibrated thresholds.
    ///
    /// During warmup, returns `base` unchanged. After warmup, sets each
    /// per-metric threshold to the `(1 − target_pass_rate)`-th quantile of
    /// the history. Falls back to `base` for any metric whose quantile is
    /// non-finite.
    pub fn calibrate(&self, base: &ThresholdConfig) -> ThresholdConfig {
        if self.in_warmup() || self.history.is_empty() {
            return base.clone();
        }
        let q = (1.0 - self.target_pass_rate).clamp(0.0, 1.0);

        let extract = |g: &GateSnapshot, idx: usize| -> f64 {
            match idx {
                0 => g.d, 1 => g.q, 2 => g.r, 3 => g.g,
                4 => g.j, 5 => g.p, 6 => g.n, 7 => g.k,
                _ => unreachable!(),
            }
        };
        let mut out = base.clone();
        let bases = [
            base.d, base.q, base.r, base.g,
            base.j, base.p, base.n, base.k,
        ];
        let setters: [&mut f64; 8] = [
            &mut out.d, &mut out.q, &mut out.r, &mut out.g,
            &mut out.j, &mut out.p, &mut out.n, &mut out.k,
        ];

        for (i, slot) in setters.into_iter().enumerate() {
            let mut samples: Vec<f64> = self.history.iter().map(|g| extract(g, i)).collect();
            samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let cut = quantile(&samples, q);
            *slot = if cut.is_finite() { cut } else { bases[i] };
        }
        out
    }
}

/// Linear-interpolated quantile of a sorted slice. Empty → NaN.
fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let pos = q.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = pos - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(d: f64, q: f64, r: f64, g: f64, j: f64, p: f64, n: f64, k: f64) -> GateSnapshot {
        GateSnapshot {
            d, q, r, g, j, p, n, k,
            kairos: false,
        }
    }

    #[test]
    fn warmup_returns_base_unchanged() {
        let cal = AdaptiveCalibrator::new(0.1, 100, 50);
        let base = ThresholdConfig::default();
        let out = cal.calibrate(&base);
        assert_eq!(out.d, base.d);
        assert_eq!(out.q, base.q);
    }

    #[test]
    fn calibrate_uses_quantile_after_warmup() {
        let mut cal = AdaptiveCalibrator::new(0.1, 100, 0);
        for i in 0..100 {
            let v = i as f64 / 100.0; // 0.00, 0.01, ..., 0.99
            cal.record(&snap(v, v, v, v, v, v, v, v));
        }
        let base = ThresholdConfig::default();
        let out = cal.calibrate(&base);
        // 90th percentile of {0.00, ..., 0.99}
        // Linear-interpolated quantile at q=0.9 over 100 sorted samples
        // pos = 0.9 * 99 = 89.1, sorted[89] = 0.89, sorted[90] = 0.90
        // → 0.89 + 0.1*(0.90-0.89) = 0.891
        assert!((out.d - 0.891).abs() < 1e-6, "out.d={}", out.d);
    }

    #[test]
    fn quantile_endpoints() {
        let s = vec![0.0, 0.5, 1.0];
        assert!((quantile(&s, 0.0) - 0.0).abs() < 1e-12);
        assert!((quantile(&s, 1.0) - 1.0).abs() < 1e-12);
        assert!((quantile(&s, 0.5) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn rolling_window_evicts_oldest() {
        let mut cal = AdaptiveCalibrator::new(0.5, 3, 0);
        cal.record(&snap(0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
        cal.record(&snap(0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
        cal.record(&snap(0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
        assert_eq!(cal.len(), 3);
        cal.record(&snap(0.9, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
        // Oldest (0.1) evicted; window now {0.2, 0.3, 0.9}
        assert_eq!(cal.len(), 3);
        let base = ThresholdConfig::default();
        let out = cal.calibrate(&base);
        // q=0.5 of [0.2, 0.3, 0.9]: pos = 0.5 * 2 = 1.0 → sorted[1] = 0.3
        assert!((out.d - 0.3).abs() < 1e-9, "out.d={}", out.d);
    }
}
