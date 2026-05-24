//! Epistemic memory health monitoring and uncertainty quantification.
//!
//! A PSE+IL store accumulates crystals over time, but not all crystals are
//! equally trustworthy.  This module answers two questions:
//!
//! 1. **Per-crystal**: how uncertain are we about this specific crystal?
//! 2. **Store-wide**: how healthy is the overall crystal memory?
//!
//! ## Uncertainty signal
//!
//! Crystal uncertainty `u ∈ [0, 1]` is computed as the geometric mean of
//! three deficiency signals:
//!
//! ```text
//! u = 1 − (qtic_weight · stability · coherence)^(1/3)
//!
//! where
//!   qtic_weight = qtic_class / 5           ∈ [0, 1]
//!   stability   = crystal.stability_score  ∈ [0, 1]
//!   coherence   = kuramoto_coherence        ∈ [0, 1]
//! ```
//!
//! `u = 0` means a fully trusted crystal (Q5, stability = 1, coherence = 1).
//! `u = 1` means a crystal with no QTIC certificate and collapsed coherence.
//!
//! ## Memory health
//!
//! [`MemoryHealthReport`] aggregates per-crystal metrics into a store-level
//! dashboard.  The key invariant:
//!
//! ```text
//! healthy_store  ⟺  fraction_q4_plus ≥ 0.8  ∧  mean_uncertainty ≤ 0.3
//!                ⟺  at_risk_count = 0  (for threshold 0.5)
//! ```
//!
//! This is the operational analogue of the constitutional fixpoint: not a
//! binary pass/fail, but a continuous gradient that signals when the store
//! needs reinforcement (new high-quality crystals or refinement passes).

use serde::{Deserialize, Serialize};

// ─── Per-crystal metrics ─────────────────────────────────────────────────────

/// Epistemic health metrics for a single crystal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystalHealthMetrics {
    /// First 16 hex chars of the crystal ID.
    pub crystal_id: String,
    /// QTIC conformance class (0–5); `None` if not yet classified.
    pub qtic_class: Option<u8>,
    /// PSE stability score ρ ∈ [0, 1].
    pub stability: f64,
    /// Kuramoto coherence κ ∈ [0, 1].
    pub coherence: f64,
    /// Coherence potential ψ = κ − (1 − ρ).
    pub psi: f64,
    /// Epistemic uncertainty u ∈ [0, 1].
    /// `u = 1 − (qtic_weight · stability · coherence)^(1/3)`.
    pub uncertainty: f64,
    /// IL block index at commit time (lower = older).
    pub block_index: i64,
    /// Agent that committed this crystal (empty = unattributed).
    pub agent_id: String,
}

impl CrystalHealthMetrics {
    /// True when this crystal is considered healthy:
    /// QTIC ≥ Q3, stability ≥ 0.6, uncertainty ≤ 0.4.
    pub fn is_healthy(&self) -> bool {
        self.qtic_class.map(|q| q >= 3).unwrap_or(false)
            && self.stability >= 0.6
            && self.uncertainty <= 0.4
    }

    /// True when this crystal is "at risk": uncertainty above `threshold`.
    pub fn is_at_risk(&self, threshold: f64) -> bool {
        self.uncertainty >= threshold
    }

    /// One-line summary for logging / LLM context injection.
    pub fn summary(&self) -> String {
        let qtic = self
            .qtic_class
            .map(|q| format!("Q{q}"))
            .unwrap_or_else(|| "Q?".to_string());
        format!(
            "[{}] {} stab={:.2} κ={:.2} ψ={:.2} u={:.2}{}",
            &self.crystal_id[..self.crystal_id.len().min(8)],
            qtic,
            self.stability,
            self.coherence,
            self.psi,
            self.uncertainty,
            if self.is_healthy() { "" } else { " ⚠" },
        )
    }
}

// ─── Store-wide health report ─────────────────────────────────────────────────

/// Store-level epistemic health dashboard.
///
/// Built by [`ILStore::memory_health`].  All fields are computed purely from
/// the IL index — no disk reads beyond what's already loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHealthReport {
    /// Total crystals in the store.
    pub total_crystals: usize,
    /// Mean QTIC conformance class across all crystals with a certificate.
    /// `None` when no crystals have been classified yet.
    pub mean_qtic_class: Option<f64>,
    /// Fraction of crystals at Q4 or above (auditable QTIC).
    pub fraction_q4_plus: f64,
    /// Mean PSE stability score across the store.
    pub mean_stability: f64,
    /// Mean Kuramoto coherence across the store.
    pub mean_coherence: f64,
    /// Mean epistemic uncertainty `u` across the store.
    pub mean_uncertainty: f64,
    /// Crystals passing the `is_healthy()` gate.
    pub healthy_count: usize,
    /// Crystals with uncertainty ≥ 0.5 ("at risk").
    pub at_risk_count: usize,
    /// Fraction of crystals with agent attribution.
    pub attributed_fraction: f64,
    /// Block index of the oldest crystal (first committed).
    pub oldest_block_index: Option<i64>,
    /// Block index of the newest crystal (most recently committed).
    pub newest_block_index: Option<i64>,
}

impl MemoryHealthReport {
    /// True when the store meets the baseline health criteria:
    /// - ≥ 80 % of crystals are Q4+, and
    /// - mean uncertainty ≤ 0.30.
    pub fn is_healthy(&self) -> bool {
        self.fraction_q4_plus >= 0.8 && self.mean_uncertainty <= 0.3
    }

    /// Compact one-line summary suitable for LLM context injection.
    pub fn summary(&self) -> String {
        if self.total_crystals == 0 {
            return "MemoryHealth: empty store".to_string();
        }
        let mean_q = self
            .mean_qtic_class
            .map(|q| format!("{:.1}", q))
            .unwrap_or_else(|| "?".to_string());
        format!(
            "MemoryHealth: {total} crystals | mean_Q={mq} | Q4+={pct:.0}% | \
             mean_u={u:.2} | healthy={h} | at-risk={r} | {status}",
            total = self.total_crystals,
            mq = mean_q,
            pct = self.fraction_q4_plus * 100.0,
            u = self.mean_uncertainty,
            h = self.healthy_count,
            r = self.at_risk_count,
            status = if self.is_healthy() { "OK" } else { "DEGRADED" },
        )
    }
}

// ─── Uncertainty computation ──────────────────────────────────────────────────

/// Compute epistemic uncertainty for a single crystal.
///
/// `u = 1 − (qtic_weight · stability · coherence)^(1/3)`
///
/// - `qtic_class`: None → weight = 0.0, Some(n) → weight = n / 5.0
/// - All inputs are clamped to `[0, 1]` before the geometric mean.
pub fn crystal_uncertainty(qtic_class: Option<u8>, stability: f64, coherence: f64) -> f64 {
    let q_weight = qtic_class.map(|q| q as f64 / 5.0).unwrap_or(0.0);
    let s = stability.clamp(0.0, 1.0);
    let k = coherence.clamp(0.0, 1.0);
    let product = q_weight * s * k;
    1.0 - product.cbrt()
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncertainty_is_zero_for_perfect_crystal() {
        // Q5, stability=1.0, coherence=1.0 → u = 1 − 1 = 0
        let u = crystal_uncertainty(Some(5), 1.0, 1.0);
        assert!(u.abs() < 1e-9, "perfect crystal must have u=0, got {u}");
    }

    #[test]
    fn uncertainty_is_one_for_unclassified_crystal() {
        // No QTIC → q_weight=0 → product=0 → u = 1 − 0 = 1
        let u = crystal_uncertainty(None, 1.0, 1.0);
        assert!(
            (u - 1.0).abs() < 1e-9,
            "unclassified crystal must have u=1, got {u}"
        );
    }

    #[test]
    fn uncertainty_increases_as_qtic_decreases() {
        let u_q5 = crystal_uncertainty(Some(5), 0.8, 0.7);
        let u_q3 = crystal_uncertainty(Some(3), 0.8, 0.7);
        let u_q1 = crystal_uncertainty(Some(1), 0.8, 0.7);
        assert!(u_q1 > u_q3, "lower QTIC → higher uncertainty");
        assert!(u_q3 > u_q5, "lower QTIC → higher uncertainty");
    }

    #[test]
    fn uncertainty_increases_as_stability_decreases() {
        let u_high = crystal_uncertainty(Some(4), 0.9, 0.7);
        let u_low = crystal_uncertainty(Some(4), 0.3, 0.7);
        assert!(u_low > u_high, "lower stability → higher uncertainty");
    }

    #[test]
    fn uncertainty_increases_as_coherence_decreases() {
        let u_high = crystal_uncertainty(Some(4), 0.8, 0.8);
        let u_low = crystal_uncertainty(Some(4), 0.8, 0.1);
        assert!(u_low > u_high, "lower coherence → higher uncertainty");
    }

    #[test]
    fn uncertainty_is_bounded_in_0_1() {
        for (q, s, k) in [
            (None, 0.0, 0.0),
            (None, 1.0, 1.0),
            (Some(0u8), 0.5, 0.5),
            (Some(5u8), 1.0, 1.0),
            (Some(3u8), 0.0, 1.0),
        ] {
            let u = crystal_uncertainty(q, s, k);
            assert!(
                (0.0..=1.0).contains(&u),
                "u must be in [0,1] for q={q:?} s={s} k={k}; got {u}"
            );
        }
    }

    fn make_metrics(
        qtic: Option<u8>,
        stability: f64,
        coherence: f64,
        idx: i64,
    ) -> CrystalHealthMetrics {
        let uncertainty = crystal_uncertainty(qtic, stability, coherence);
        let psi = coherence - (1.0 - stability.clamp(0.0, 1.0));
        CrystalHealthMetrics {
            crystal_id: format!("{:016x}", idx),
            qtic_class: qtic,
            stability,
            coherence,
            psi,
            uncertainty,
            block_index: idx,
            agent_id: String::new(),
        }
    }

    #[test]
    fn healthy_crystal_passes_gate() {
        let m = make_metrics(Some(4), 0.80, 0.70, 0);
        assert!(
            m.is_healthy(),
            "Q4, high stability/coherence must be healthy"
        );
    }

    #[test]
    fn low_qtic_crystal_not_healthy() {
        let m = make_metrics(Some(2), 0.70, 0.60, 0);
        assert!(
            !m.is_healthy(),
            "Q2 crystal must not be healthy (requires Q3+)"
        );
    }

    #[test]
    fn at_risk_crystal_detected() {
        let m = make_metrics(None, 0.30, 0.10, 0);
        assert!(
            m.is_at_risk(0.5),
            "unclassified, low-stability crystal must be at-risk"
        );
    }

    #[test]
    fn healthy_store_passes_criteria() {
        let report = MemoryHealthReport {
            total_crystals: 10,
            mean_qtic_class: Some(4.2),
            fraction_q4_plus: 0.85,
            mean_stability: 0.80,
            mean_coherence: 0.72,
            mean_uncertainty: 0.20,
            healthy_count: 9,
            at_risk_count: 0,
            attributed_fraction: 0.80,
            oldest_block_index: Some(0),
            newest_block_index: Some(9),
        };
        assert!(report.is_healthy());
        assert!(report.summary().contains("OK"));
    }

    #[test]
    fn degraded_store_fails_criteria() {
        let report = MemoryHealthReport {
            total_crystals: 5,
            mean_qtic_class: Some(1.5),
            fraction_q4_plus: 0.20,
            mean_stability: 0.45,
            mean_coherence: 0.30,
            mean_uncertainty: 0.65,
            healthy_count: 1,
            at_risk_count: 3,
            attributed_fraction: 0.0,
            oldest_block_index: Some(0),
            newest_block_index: Some(4),
        };
        assert!(!report.is_healthy());
        assert!(report.summary().contains("DEGRADED"));
    }

    #[test]
    fn empty_store_summary() {
        let report = MemoryHealthReport {
            total_crystals: 0,
            mean_qtic_class: None,
            fraction_q4_plus: 0.0,
            mean_stability: 0.0,
            mean_coherence: 0.0,
            mean_uncertainty: 0.0,
            healthy_count: 0,
            at_risk_count: 0,
            attributed_fraction: 0.0,
            oldest_block_index: None,
            newest_block_index: None,
        };
        assert!(report.summary().contains("empty"));
    }
}
