//! Persistent graph state for nxalien — survives across compile runs.
//!
//! PersistentGraph is in-memory only, so we serialize the minimal
//! state needed for attractor computation: the point cloud (vertex → 5D
//! embedding) and a ring of recent observation timestamps.  On each
//! `nxalien compile` run this is loaded from `.nxalien/graph_state.json`,
//! updated with the new bundle, and written back.

use pse_constraint::compute_attractor_centroid;
use pse_graph::{derive_vertex_id, PersistentGraph};
use pse_nxalien_pse::ingest_bundle;
use pse_nxalien_types::NxAlienBundle;
use pse_types::{FiveDState, MeasurementContext, PersistenceConfig, VertexId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Source ID used when nxalien bundles are ingested into PSE.
pub const NXALIEN_SOURCE_ID: &str = "nxalien-v1";

/// Serialisable snapshot of the PSE point cloud for nxalien.
///
/// Stored in `.nxalien/graph_state.json` between compile runs so that
/// the attractor centroid can be computed from a history of observations,
/// not just a single run.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphState {
    /// 5D embeddings keyed by VertexId (accumulated across runs).
    pub point_cloud: BTreeMap<VertexId, SerFiveDState>,
    /// Timestamps of the last N nxalien observations (ring buffer, max 64).
    pub observation_timestamps: Vec<f64>,
    /// Number of compile runs recorded.
    pub run_count: u64,
    /// Free-energy history (one entry per run).
    pub free_energy_history: Vec<f64>,
}

/// Serde-friendly wrapper for FiveDState.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SerFiveDState {
    pub p: f64,
    pub rho: f64,
    pub omega: f64,
    pub chi: f64,
    pub eta: f64,
}

impl From<&FiveDState> for SerFiveDState {
    fn from(s: &FiveDState) -> Self {
        Self { p: s.p, rho: s.rho, omega: s.omega, chi: s.chi, eta: s.eta }
    }
}

impl From<&SerFiveDState> for FiveDState {
    fn from(s: &SerFiveDState) -> Self {
        Self { p: s.p, rho: s.rho, omega: s.omega, chi: s.chi, eta: s.eta }
    }
}

impl GraphState {
    /// Load from `.nxalien/graph_state.json`, or return a fresh default.
    pub fn load(nxa_dir: &Path) -> Self {
        let p = nxa_dir.join("graph_state.json");
        if let Ok(s) = std::fs::read_to_string(&p) {
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Ingest a new bundle into a fresh PersistentGraph seeded with the
    /// stored point cloud, update state, and return the graph for signal
    /// extraction.
    pub fn ingest_and_update(
        &mut self,
        bundle: &NxAlienBundle,
        timestamp: f64,
    ) -> PersistentGraph {
        // Build an in-memory graph seeded from stored embeddings.
        let mut graph = PersistentGraph::new();
        for (vid, emb) in &self.point_cloud {
            let state: FiveDState = emb.into();
            graph.upsert_vertex(*vid, timestamp - 1.0);
            // Restore embedding by applying a synthetic single-vertex batch.
            // The graph recomputes embeddings from topology, so we seed via
            // the stored point cloud as initial vertex state.
            graph.embedding.insert(*vid, state);
        }

        // Ingest the new bundle as a PSE Observation.
        let ctx = MeasurementContext::default();
        if let Ok(obs) = ingest_bundle(bundle, &ctx) {
            let config = PersistenceConfig::default();
            let _ = graph.apply_observations(&[obs], &config);
        }

        // Update stored point cloud from the live graph.
        self.point_cloud.clear();
        for (vid, state) in graph.point_cloud() {
            self.point_cloud.insert(vid, SerFiveDState::from(&state));
        }

        // Track observation timestamps (ring, max 64).
        self.observation_timestamps.push(timestamp);
        if self.observation_timestamps.len() > 64 {
            self.observation_timestamps.remove(0);
        }

        self.run_count += 1;
        graph
    }

    /// Compute attractor centroid using the top-k highest-norm embeddings.
    pub fn attractor_centroid(&self, graph: &PersistentGraph, k: usize) -> FiveDState {
        if self.run_count < 2 {
            return FiveDState::default();
        }
        compute_attractor_centroid(graph, k)
    }

    /// Vertex ID for the nxalien source (deterministic, same as PSE uses).
    pub fn nxalien_vertex_id() -> VertexId {
        derive_vertex_id(NXALIEN_SOURCE_ID)
    }

    /// Current 5D embedding of the nxalien vertex, or default.
    pub fn current_embedding(&self) -> FiveDState {
        let vid = Self::nxalien_vertex_id();
        self.point_cloud
            .get(&vid)
            .map(FiveDState::from)
            .unwrap_or_default()
    }

    /// Free-energy proxy: norm_sq of (embedding - centroid).
    pub fn free_energy(&self, centroid: &FiveDState) -> f64 {
        let emb = self.current_embedding();
        emb.distance(centroid).powi(2)
    }

    /// Persist state to `.nxalien/graph_state.json`.
    pub fn save(&mut self, nxa_dir: &Path) -> anyhow::Result<()> {
        // Record free energy for this run.
        let centroid = {
            // Approximate centroid from stored cloud (no live graph needed).
            let states: Vec<FiveDState> =
                self.point_cloud.values().map(FiveDState::from).collect();
            if states.is_empty() {
                FiveDState::default()
            } else {
                let n = states.len() as f64;
                let mut c = FiveDState::default();
                for s in &states {
                    c.p += s.p / n;
                    c.rho += s.rho / n;
                    c.omega += s.omega / n;
                    c.chi += s.chi / n;
                    c.eta += s.eta / n;
                }
                c
            }
        };
        self.free_energy_history.push(self.free_energy(&centroid));
        if self.free_energy_history.len() > 64 {
            self.free_energy_history.remove(0);
        }

        std::fs::create_dir_all(nxa_dir)?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(nxa_dir.join("graph_state.json"), json)?;
        Ok(())
    }

    /// Trend of free energy: positive = drifting, negative = converging.
    pub fn free_energy_trend(&self) -> f64 {
        let h = &self.free_energy_history;
        if h.len() < 2 {
            return 0.0;
        }
        let n = h.len();
        let recent = h[n - 1];
        let prior = h[n - 2];
        recent - prior
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nxalien_vertex_id_deterministic() {
        let id1 = GraphState::nxalien_vertex_id();
        let id2 = GraphState::nxalien_vertex_id();
        assert_eq!(id1, id2);
    }

    #[test]
    fn graph_state_default_has_no_cloud() {
        let gs = GraphState::default();
        assert!(gs.point_cloud.is_empty());
        assert_eq!(gs.run_count, 0);
    }

    #[test]
    fn free_energy_trend_needs_two_runs() {
        let gs = GraphState::default();
        assert_eq!(gs.free_energy_trend(), 0.0);
    }
}
