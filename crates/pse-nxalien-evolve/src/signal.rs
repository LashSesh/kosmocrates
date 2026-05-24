//! Epistemic signal extraction from the PSE knowledge field.
//!
//! An EpistemicSignal captures the current position of the nxalien
//! vertex in 5D state space relative to the attractor centroid.  The
//! signal drives rule evolution: a converging signal confirms the
//! current rules; a drifting signal triggers proposals to update them.

use crate::graph_state::GraphState;
use pse_graph::PersistentGraph;
use pse_types::{FiveDState, TopologySignature};
use serde::{Deserialize, Serialize};

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
    /// Stability classification.
    pub stability: SignalStability,
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
        }
    }

    /// True if the signal indicates that rules should be reviewed.
    pub fn needs_rule_review(&self) -> bool {
        matches!(self.stability, SignalStability::Drifting | SignalStability::Diverging)
    }

    /// True if the signal confirms rules are attractor-aligned.
    pub fn is_confirmed(&self) -> bool {
        matches!(self.stability, SignalStability::Stable | SignalStability::Converging)
    }
}

/// Classify stability from run count, distance, and trend.
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
        assert_eq!(
            classify_stability(3, 0.2, -0.01),
            SignalStability::Converging
        );
    }

    #[test]
    fn drifting_when_trend_positive() {
        assert_eq!(
            classify_stability(3, 0.2, 0.05),
            SignalStability::Drifting
        );
    }

    #[test]
    fn diverging_when_far() {
        assert_eq!(
            classify_stability(3, 0.9, 0.1),
            SignalStability::Diverging
        );
    }
}
