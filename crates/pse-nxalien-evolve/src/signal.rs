//! Epistemic signal extraction from the PSE knowledge field.
//!
//! An EpistemicSignal captures the current position of the nxalien
//! vertex in 5D state space relative to the attractor centroid, and —
//! after the IL bridge is wired — the live health of the crystal store.
//! The signal drives rule evolution: a converging signal confirms the
//! current rules; a drifting signal triggers proposals to update them.

use crate::graph_state::GraphState;
use crate::il_bridge::{load_il_health_and_agenda, ILHealthSnapshot};
use pse_graph::PersistentGraph;
use pse_types::{FiveDState, TopologySignature};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Stability classification of the nxalien knowledge vertex.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalStability {
    /// Not enough history to classify (fewer than 2 compile runs).
    Initialising,
    /// Distance to attractor decreasing — rules are working.
    Converging,
    /// Distance stable below threshold — rules are confirmed.
    Stable,
    /// Distance increasing — rules may need revision.
    Drifting,
    /// Distance far above threshold — rules likely misaligned.
    Diverging,
}

/// Snapshot of the epistemic state of the nxalien layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpistemicSignal {
    /// Current 5D embedding of the nxalien-v1 vertex in PSE.
    pub embedding: FiveDState,
    /// Attractor centroid of the top-k highest-norm vertices.
    pub attractor_centroid: FiveDState,
    /// Euclidean distance from embedding to centroid.
    pub distance_to_attractor: f64,
    /// Free-energy trend (positive = drifting away).
    pub free_energy_trend: f64,
    /// Number of compile runs observed.
    pub run_count: u64,
    /// Topological signature of the current PSE graph.
    pub topology: TopologySignature,
    /// Stability classification (PSE graph + IL health combined).
    pub stability: SignalStability,
    /// Live IL store health snapshot, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub il_health: Option<ILHealthSnapshot>,
}

impl EpistemicSignal {
    /// Extract an EpistemicSignal from the live graph and graph state.
    ///
    /// `k` is the number of top-norm vertices used for centroid computation.
    pub fn extract(graph: &PersistentGraph, state: &GraphState, k: usize) -> Self {
        let embedding = state.current_embedding();
        let centroid = state.attractor_centroid(graph, k);
        let distance = embedding.distance(&centroid);
        let trend = state.free_energy_trend();
        let topology = graph.topology_signature();
        let stability = classify_stability(state.run_count, distance, trend);

        EpistemicSignal {
            embedding,
            attractor_centroid: centroid,
            distance_to_attractor: distance,
            free_energy_trend: trend,
            run_count: state.run_count,
            topology,
            stability,
            il_health: None,
        }
    }

    /// Like `extract`, but also reads IL store health and folds it into
    /// the stability classification.
    ///
    /// If the IL store at `il_path` is empty or missing, falls back to
    /// the PSE-only classification (same as `extract`).
    pub fn extract_with_il(
        graph: &PersistentGraph,
        state: &GraphState,
        k: usize,
        il_path: &Path,
    ) -> Self {
        let mut signal = Self::extract(graph, state, k);
        let il_health = load_il_health_and_agenda(il_path);

        if let Some(ref health) = il_health {
            signal.stability = classify_stability_with_il(
                state.run_count,
                signal.distance_to_attractor,
                signal.free_energy_trend,
                health,
            );
        }

        signal.il_health = il_health;
        signal
    }

    /// True if the signal indicates that rules should be reviewed.
    pub fn needs_rule_review(&self) -> bool {
        matches!(self.stability, SignalStability::Drifting | SignalStability::Diverging)
    }

    /// True if the signal confirms rules are attractor-aligned.
    pub fn is_confirmed(&self) -> bool {
        matches!(self.stability, SignalStability::Stable | SignalStability::Converging)
    }

    /// Compact one-line summary for logging.
    pub fn summary_line(&self) -> String {
        let il = self
            .il_health
            .as_ref()
            .map(|h| {
                format!(
                    "  IL: Q̄={:.1} u={:.2} at_risk={} {}",
                    h.mean_qtic_class,
                    h.mean_uncertainty,
                    h.at_risk_count,
                    if h.healthy { "✓" } else { "⚠" },
                )
            })
            .unwrap_or_default();
        format!(
            "{:?} (dist={:.3} trend={:.4}){}",
            self.stability, self.distance_to_attractor, self.free_energy_trend, il,
        )
    }
}

/// PSE-only stability classification from run count, distance, and trend.
fn classify_stability(run_count: u64, distance: f64, trend: f64) -> SignalStability {
    const STABLE_THRESHOLD: f64 = 0.15;
    const DIVERGING_THRESHOLD: f64 = 0.60;
    const TREND_EPS: f64 = 1e-6;

    if run_count < 2 {
        return SignalStability::Initialising;
    }
    if distance > DIVERGING_THRESHOLD {
        return SignalStability::Diverging;
    }
    if distance < STABLE_THRESHOLD && trend.abs() < TREND_EPS {
        return SignalStability::Stable;
    }
    if trend < -TREND_EPS {
        return SignalStability::Converging;
    }
    if trend > TREND_EPS {
        return SignalStability::Drifting;
    }
    SignalStability::Stable
}

/// IL-aware stability: takes the WORSE of PSE signal and IL health.
///
/// Rules:
///   - at_risk_count > 0 → at minimum Drifting (crystals are degrading)
///   - mean_qtic_class < 2.0 → Diverging (store is structurally poor)
///   - healthy IL + Converging → upgrade to Stable (confirmed by both layers)
fn classify_stability_with_il(
    run_count: u64,
    distance: f64,
    trend: f64,
    il: &ILHealthSnapshot,
) -> SignalStability {
    let base = classify_stability(run_count, distance, trend);

    // Poor QTIC across the board → Diverging regardless of PSE signal.
    if il.mean_qtic_class < 2.0 && il.total_crystals > 0 {
        return SignalStability::Diverging;
    }

    // At-risk crystals → PSE Stable becomes Drifting.
    if il.at_risk_count > 0 {
        return match base {
            SignalStability::Stable => SignalStability::Drifting,
            other => other,
        };
    }

    // IL confirmed healthy + PSE converging → promote to Stable.
    if il.healthy && matches!(base, SignalStability::Converging) {
        return SignalStability::Stable;
    }

    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialising_when_few_runs() {
        assert_eq!(classify_stability(0, 0.0, 0.0), SignalStability::Initialising);
        assert_eq!(classify_stability(1, 0.0, 0.0), SignalStability::Initialising);
    }

    #[test]
    fn stable_when_close_and_flat() {
        assert_eq!(classify_stability(3, 0.05, 0.0), SignalStability::Stable);
    }

    #[test]
    fn converging_when_trend_negative() {
        assert_eq!(classify_stability(3, 0.2, -0.01), SignalStability::Converging);
    }

    #[test]
    fn drifting_when_trend_positive() {
        assert_eq!(classify_stability(3, 0.2, 0.05), SignalStability::Drifting);
    }

    #[test]
    fn diverging_when_far() {
        assert_eq!(classify_stability(3, 0.9, 0.1), SignalStability::Diverging);
    }

    #[test]
    fn il_at_risk_overrides_stable() {
        let il = ILHealthSnapshot {
            total_crystals: 3,
            at_risk_count: 1,
            healthy: false,
            mean_qtic_class: 3.5,
            fraction_q4_plus: 0.5,
            mean_uncertainty: 0.55,
            agenda_items: vec![],
        };
        assert_eq!(
            classify_stability_with_il(5, 0.05, 0.0, &il),
            SignalStability::Drifting,
            "at-risk crystals should override PSE Stable"
        );
    }

    #[test]
    fn il_poor_qtic_causes_diverging() {
        let il = ILHealthSnapshot {
            total_crystals: 3,
            at_risk_count: 0,
            healthy: false,
            mean_qtic_class: 1.2,
            fraction_q4_plus: 0.0,
            mean_uncertainty: 0.7,
            agenda_items: vec![],
        };
        assert_eq!(
            classify_stability_with_il(5, 0.05, 0.0, &il),
            SignalStability::Diverging,
            "mean QTIC < 2.0 should cause Diverging"
        );
    }

    #[test]
    fn il_healthy_promotes_converging_to_stable() {
        let il = ILHealthSnapshot {
            total_crystals: 5,
            at_risk_count: 0,
            healthy: true,
            mean_qtic_class: 4.2,
            fraction_q4_plus: 0.9,
            mean_uncertainty: 0.15,
            agenda_items: vec![],
        };
        assert_eq!(
            classify_stability_with_il(5, 0.2, -0.01, &il),
            SignalStability::Stable,
            "healthy IL + Converging PSE should promote to Stable"
        );
    }
}
