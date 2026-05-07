//! Morphodynamic compressor for the traversal dynamics layer.
//! COMPRESS-01: ALL operations deterministic via lexicographic sorting on StableId/Hash256.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::dynamic_state::{
    BaseState, CanonicalNumber, DynamicError, Hash256, LiftedState, StableId, stable_id_of,
};
use crate::dynamic_policy::DynamicPolicy;

// ───────────────────────────────────────────────────────────────────────────────
// CompressionNode
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionNode {
    pub id: StableId,
    pub state: LiftedState,
    pub mass: CanonicalNumber,
    pub variance: CanonicalNumber,
    pub born_tick: u64,
}

// ───────────────────────────────────────────────────────────────────────────────
// CompressionEdge
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CompressionEdge {
    pub from: StableId,
    pub to: StableId,
    pub weight: CanonicalNumber,
    pub decay: CanonicalNumber,
    pub coherence: CanonicalNumber,
}

// ───────────────────────────────────────────────────────────────────────────────
// CompressionGraph
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionGraph {
    pub nodes: BTreeMap<StableId, CompressionNode>,
    pub edges: Vec<CompressionEdge>,
    pub graph_id: Hash256,
}

impl CompressionGraph {
    pub fn empty() -> Self {
        let mut g = CompressionGraph {
            nodes: BTreeMap::new(),
            edges: vec![],
            graph_id: Hash256::zero(),
        };
        if let Ok(id) = stable_id_of(&(&g.nodes, &g.edges)) {
            g.graph_id = id;
        }
        g
    }

    fn compute_id(&mut self) -> Result<(), DynamicError> {
        let id = stable_id_of(&(&self.nodes, &self.edges))?;
        self.graph_id = id;
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// CompressorStats
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressorStats {
    pub stats_id: Hash256,
    pub nodes_created: usize,
    pub nodes_merged: usize,
    pub nodes_split: usize,
    pub edges_created: usize,
    pub edges_pruned: usize,
    pub density: CanonicalNumber,
    pub graph_before: Hash256,
    pub graph_after: Hash256,
}

impl CompressorStats {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.stats_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.stats_id = id;
        Ok(self)
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// MorphodynamicCompressor trait
// ───────────────────────────────────────────────────────────────────────────────

pub trait MorphodynamicCompressor {
    fn add_state(&mut self, state: LiftedState) -> Result<StableId, DynamicError>;
    fn advect(
        &mut self,
        guidance: &BaseState,
        policy: &DynamicPolicy,
        tick: u64,
    ) -> Result<CompressorStats, DynamicError>;
    fn snapshot(&self) -> CompressionGraph;
}

// ───────────────────────────────────────────────────────────────────────────────
// DefaultMorphodynamicCompressor
// ───────────────────────────────────────────────────────────────────────────────

pub struct DefaultMorphodynamicCompressor {
    nodes: BTreeMap<StableId, CompressionNode>,
    edges: Vec<CompressionEdge>,
}

impl DefaultMorphodynamicCompressor {
    pub fn new() -> Self {
        DefaultMorphodynamicCompressor {
            nodes: BTreeMap::new(),
            edges: vec![],
        }
    }
}

impl Default for DefaultMorphodynamicCompressor {
    fn default() -> Self {
        Self::new()
    }
}

impl MorphodynamicCompressor for DefaultMorphodynamicCompressor {
    fn add_state(&mut self, state: LiftedState) -> Result<StableId, DynamicError> {
        let id = StableId(stable_id_of(&state)?);
        let node = CompressionNode {
            id: id.clone(),
            state,
            mass: CanonicalNumber::quantize_default(1.0)?,
            variance: CanonicalNumber::zero(),
            born_tick: 0,
        };
        self.nodes.insert(id.clone(), node);
        Ok(id)
    }

    fn advect(
        &mut self,
        _guidance: &BaseState,
        policy: &DynamicPolicy,
        tick: u64,
    ) -> Result<CompressorStats, DynamicError> {
        // 1. Record graph_before
        let graph_before = self.snapshot().graph_id;

        let alpha = policy.learning_rate.to_f64();
        let d = policy.decay.to_f64();
        let prune_threshold = policy.prune_threshold.to_f64();
        let split_threshold = policy.split_threshold.to_f64();
        let variance_threshold = policy.variance_threshold.to_f64();
        let merge_threshold = policy.merge_threshold.to_f64();

        // 2. Hebbian update on edges
        // COMPRESS-01: sort edges before updating
        self.edges.sort();
        for edge in self.edges.iter_mut() {
            let mass_i = self.nodes.get(&edge.from)
                .map(|n| n.mass.to_f64()).unwrap_or(0.0);
            let mass_j = self.nodes.get(&edge.to)
                .map(|n| n.mass.to_f64()).unwrap_or(0.0);
            let c_ij = edge.coherence.to_f64();
            let u_ij = mass_i.min(mass_j);
            let w = edge.weight.to_f64();
            let w_new = (w + alpha * c_ij * u_ij - d).max(0.0);
            edge.weight = CanonicalNumber::quantize_default(w_new)?;
        }

        // 3. Prune edges where weight < prune_threshold
        // COMPRESS-01: sort by (from, to) before deciding
        self.edges.sort();
        let edges_before = self.edges.len();
        self.edges.retain(|e| e.weight.to_f64() >= prune_threshold);
        let edges_pruned = edges_before - self.edges.len();

        // 4. Split: for each node where mass > split_threshold AND variance > variance_threshold
        // Sort candidates lexicographically by node id
        let split_candidates: Vec<StableId> = {
            let mut cands: Vec<StableId> = self.nodes.iter()
                .filter(|(_, n)| {
                    n.mass.to_f64() > split_threshold
                        && n.variance.to_f64() > variance_threshold
                })
                .map(|(id, _)| id.clone())
                .collect();
            cands.sort();
            cands
        };

        let mut nodes_split = 0usize;
        for sid in split_candidates {
            if let Some(node) = self.nodes.remove(&sid) {
                let var_half = node.variance.to_f64() / 2.0;
                // Create two new nodes split along coord index 0 ± variance/2
                let mut coords_a = node.state.coords.clone();
                let mut coords_b = node.state.coords.clone();
                if !coords_a.is_empty() {
                    let c0 = coords_a[0].to_f64();
                    coords_a[0] = CanonicalNumber::quantize_default(c0 + var_half)?;
                    coords_b[0] = CanonicalNumber::quantize_default(c0 - var_half)?;
                }
                let half_mass = CanonicalNumber::quantize_default(node.mass.to_f64() / 2.0)?;

                let state_a = LiftedState {
                    state_id: Hash256::zero(),
                    base_state_id: node.state.base_state_id.clone(),
                    coords: coords_a,
                    auxiliary: node.state.auxiliary.clone(),
                    lift_profile: node.state.lift_profile.clone(),
                };
                let state_a = state_a.with_id()?;
                let id_a = StableId(stable_id_of(&state_a)?);
                self.nodes.insert(id_a.clone(), CompressionNode {
                    id: id_a,
                    state: state_a,
                    mass: half_mass.clone(),
                    variance: CanonicalNumber::zero(),
                    born_tick: tick,
                });

                let state_b = LiftedState {
                    state_id: Hash256::zero(),
                    base_state_id: node.state.base_state_id.clone(),
                    coords: coords_b,
                    auxiliary: node.state.auxiliary.clone(),
                    lift_profile: node.state.lift_profile.clone(),
                };
                let state_b = state_b.with_id()?;
                let id_b = StableId(stable_id_of(&state_b)?);
                self.nodes.insert(id_b.clone(), CompressionNode {
                    id: id_b,
                    state: state_b,
                    mass: half_mass,
                    variance: CanonicalNumber::zero(),
                    born_tick: tick,
                });

                nodes_split += 1;
            }
        }

        // 5. Merge: for nodes i,j where L2(coords_i, coords_j) ≤ merge_threshold
        // Sort candidates by (id_i, id_j)
        let node_ids: Vec<StableId> = self.nodes.keys().cloned().collect();
        let mut to_merge: Vec<(StableId, StableId)> = Vec::new();
        for i in 0..node_ids.len() {
            for j in (i + 1)..node_ids.len() {
                let id_i = &node_ids[i];
                let id_j = &node_ids[j];
                if let (Some(ni), Some(nj)) = (self.nodes.get(id_i), self.nodes.get(id_j)) {
                    let dist = l2_coord_distance(&ni.state.coords, &nj.state.coords);
                    if dist <= merge_threshold {
                        to_merge.push((id_i.clone(), id_j.clone()));
                    }
                }
            }
        }
        // Sort by (id_i, id_j) — deterministic
        to_merge.sort();

        let mut nodes_merged = 0usize;
        let mut merged_out: std::collections::BTreeSet<StableId> = std::collections::BTreeSet::new();
        for (id_i, id_j) in to_merge {
            if merged_out.contains(&id_i) || merged_out.contains(&id_j) {
                continue;
            }
            let (mass_i, mass_j, coords_i, coords_j, base_state_id, auxiliary, lift_profile) = {
                let ni = match self.nodes.get(&id_i) { Some(n) => n.clone(), None => continue };
                let nj = match self.nodes.get(&id_j) { Some(n) => n.clone(), None => continue };
                (ni.mass.to_f64(), nj.mass.to_f64(), ni.state.coords.clone(), nj.state.coords.clone(),
                 ni.state.base_state_id.clone(), ni.state.auxiliary.clone(), ni.state.lift_profile.clone())
            };

            let total_mass = mass_i + mass_j;
            // Mass-weighted mean of coords
            let merged_coords: Result<Vec<CanonicalNumber>, DynamicError> = coords_i.iter().zip(coords_j.iter())
                .map(|(a, b)| {
                    let v = (a.to_f64() * mass_i + b.to_f64() * mass_j) / total_mass;
                    CanonicalNumber::quantize_default(v)
                })
                .collect();
            let merged_coords = merged_coords?;

            let merged_state = LiftedState {
                state_id: Hash256::zero(),
                base_state_id,
                coords: merged_coords,
                auxiliary,
                lift_profile,
            };
            let merged_state = merged_state.with_id()?;
            let new_id = StableId(stable_id_of(&merged_state)?);
            let merged_mass = CanonicalNumber::quantize_default(total_mass)?;

            self.nodes.remove(&id_i);
            self.nodes.remove(&id_j);
            merged_out.insert(id_i);
            merged_out.insert(id_j);

            self.nodes.insert(new_id.clone(), CompressionNode {
                id: new_id,
                state: merged_state,
                mass: merged_mass,
                variance: CanonicalNumber::zero(),
                born_tick: tick,
            });
            nodes_merged += 1;
        }

        // 6. density = nodes.len() / edges.len().max(1), quantized
        let density_raw = self.nodes.len() as f64 / self.edges.len().max(1) as f64;
        let density = CanonicalNumber::quantize_default(density_raw)?;

        // 7. Record graph_after
        let mut graph = self.snapshot();
        graph.compute_id().ok();
        let graph_after = graph.graph_id;

        let stats = CompressorStats {
            stats_id: Hash256::zero(),
            nodes_created: 0, // tracking not needed for MVP
            nodes_merged,
            nodes_split,
            edges_created: 0,
            edges_pruned,
            density,
            graph_before,
            graph_after,
        };
        stats.with_id()
    }

    fn snapshot(&self) -> CompressionGraph {
        let mut g = CompressionGraph {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            graph_id: Hash256::zero(),
        };
        g.compute_id().ok();
        g
    }
}

fn l2_coord_distance(a: &[CanonicalNumber], b: &[CanonicalNumber]) -> f64 {
    let mut sum = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        let d = x.to_f64() - y.to_f64();
        sum += d * d;
    }
    sum.sqrt()
}

// ───────────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic_state::{AuxiliaryKind, BaseState, DefaultStateLift, LiftProfile, StateLift};
    use crate::dynamic_policy::DynamicPolicy;

    fn make_lifted(tick: u64, coords: Vec<f64>) -> LiftedState {
        let base = BaseState {
            state_id: Hash256::zero(),
            coords: coords.iter().map(|&v| CanonicalNumber::quantize_default(v).unwrap()).collect(),
            weight: CanonicalNumber::quantize_default(1.0).unwrap(),
            evidence_refs: vec![],
        };
        let base = base.with_id().unwrap();
        DefaultStateLift.lift(&base, tick).unwrap()
    }

    #[test]
    fn compressor_split_deterministic() {
        // Create two compressors with same state, run advect, check same stats_id
        let state = make_lifted(1, vec![1.0, 2.0]);

        let mut c1 = DefaultMorphodynamicCompressor::new();
        let mut c2 = DefaultMorphodynamicCompressor::new();

        c1.add_state(state.clone()).unwrap();
        c2.add_state(state.clone()).unwrap();

        // Set mass high enough to trigger split by setting the node manually
        let mut policy = DynamicPolicy::default();
        // Use a low split_threshold to force a split
        policy.split_threshold = CanonicalNumber::quantize_default(0.5).unwrap();
        policy.variance_threshold = CanonicalNumber::quantize_default(0.0).unwrap();

        // Manually set mass on nodes
        for node in c1.nodes.values_mut() {
            node.mass = CanonicalNumber::quantize_default(2.0).unwrap();
            node.variance = CanonicalNumber::quantize_default(1.0).unwrap();
        }
        for node in c2.nodes.values_mut() {
            node.mass = CanonicalNumber::quantize_default(2.0).unwrap();
            node.variance = CanonicalNumber::quantize_default(1.0).unwrap();
        }

        let guidance = BaseState::zero(2).unwrap();
        let s1 = c1.advect(&guidance, &policy, 1).unwrap();
        let s2 = c2.advect(&guidance, &policy, 1).unwrap();

        assert_eq!(s1.stats_id, s2.stats_id, "split must be deterministic");
        assert_eq!(s1.nodes_split, s2.nodes_split);
    }

    #[test]
    fn compressor_merge_deterministic() {
        // Two identical compressors with close-enough states should merge deterministically
        let state_a = make_lifted(1, vec![1.0, 0.0]);
        let state_b = make_lifted(1, vec![1.0001, 0.0]);

        let mut c1 = DefaultMorphodynamicCompressor::new();
        let mut c2 = DefaultMorphodynamicCompressor::new();

        c1.add_state(state_a.clone()).unwrap();
        c1.add_state(state_b.clone()).unwrap();
        c2.add_state(state_a.clone()).unwrap();
        c2.add_state(state_b.clone()).unwrap();

        let mut policy = DynamicPolicy::default();
        // Set merge_threshold high enough to merge
        policy.merge_threshold = CanonicalNumber::quantize_default(10.0).unwrap();

        let guidance = BaseState::zero(2).unwrap();
        let s1 = c1.advect(&guidance, &policy, 1).unwrap();
        let s2 = c2.advect(&guidance, &policy, 1).unwrap();

        assert_eq!(s1.stats_id, s2.stats_id, "merge must be deterministic");
        assert_eq!(s1.nodes_merged, s2.nodes_merged);
    }

    #[test]
    fn compressor_add_state_and_snapshot() {
        let mut c = DefaultMorphodynamicCompressor::new();
        let state = make_lifted(0, vec![0.5, 0.5]);
        let id = c.add_state(state).unwrap();
        let snap = c.snapshot();
        assert!(snap.nodes.contains_key(&id));
    }
}
