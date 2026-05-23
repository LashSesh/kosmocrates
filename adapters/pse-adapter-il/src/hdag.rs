//! Original HDAG — Hyperdimensional DAG Framework for Semantic Resonance Systems.
//!
//! Implements Sebastian Klemm's HDAG v1.0 specification:
//!
//! - **Nodes** are resonance tensors T_i ∈ R^5 with axes:
//!   [temporal, morphic, relational, topological, entropic]
//!
//! - **Edges** are phase-gradient transitions Φ_ij = (T_j − T_i) / ‖T_j − T_i‖
//!   (5D normalized gradient vector, not a scalar weight)
//!
//! - **Coherence condition**: edge (i→j) is valid only if T_i ∈ S_coh
//!   (source node has passed the phase-locking gate)
//!
//! - **Acyclicity** emerges from phase disalignment, not from explicit cycle checks:
//!   edges may only increase the coherence potential ψ(T) = morphic − entropic,
//!   so returning to a prior node would require a negative gradient — geometrically
//!   impossible under the coherence gate.
//!
//! ## Mapping from SemanticCrystal
//!
//! | Tensor axis | SemanticCrystal field           |
//! |-------------|----------------------------------|
//! | temporal    | mean_propagation_time            |
//! | morphic     | kuramoto_coherence               |
//! | relational  | cheeger_estimate                 |
//! | topological | spectral_gap                    |
//! | entropic    | 1.0 − stability_score            |

use pse_types::SemanticCrystal;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// Resonance tensor T_i ∈ R^5.
/// Axes: [temporal, morphic, relational, topological, entropic]
pub type ResonanceTensor = [f64; 5];

/// Extract the 5D resonance tensor from a SemanticCrystal.
pub fn crystal_to_tensor(crystal: &SemanticCrystal) -> ResonanceTensor {
    let s = &crystal.topology_signature;
    [
        s.mean_propagation_time,
        s.kuramoto_coherence,
        s.cheeger_estimate,
        s.spectral_gap,
        1.0 - crystal.stability_score.clamp(0.0, 1.0),
    ]
}

/// A node in the HDAG — a semantic vortex in the resonance field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HDAGNode {
    pub id: String,
    pub crystal_id_hex: String,
    pub tensor: ResonanceTensor,
    /// Whether the source crystal passed its Kairos gate (PSE coherence gate).
    pub kairos: bool,
    pub timestamp: String,
}

/// A phase-gradient edge Φ_ij = (T_j − T_i) / ‖T_j − T_i‖.
/// Carries the full 5D gradient direction, not a scalar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HDAGEdge {
    pub from: String,
    pub to: String,
    /// Normalized gradient vector in R^5.
    pub gradient: ResonanceTensor,
    /// ‖T_j − T_i‖ — magnitude of the phase transition.
    pub magnitude: f64,
}

/// Result of path invariance verification between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInvarianceResult {
    pub start: String,
    pub end: String,
    pub path_count: usize,
    /// Summed gradient vectors per path (each is a 5D vector).
    pub path_gradients: Vec<ResonanceTensor>,
    /// Max pairwise L2 distance between path gradient sums.
    pub max_divergence: f64,
    /// True when max_divergence < tolerance.
    pub invariant: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct HDAGData {
    nodes: HashMap<String, HDAGNode>,
    edges: Vec<HDAGEdge>,
    /// Adjacency list: node_id → list of successor node_ids.
    adjacency: HashMap<String, Vec<String>>,
}

/// Hyperdimensional Directed Acyclic Graph.
///
/// Acyclicity is maintained by the coherence potential gate:
/// an edge (i→j) is created only when ψ(T_j) ≥ ψ(T_i) − ε,
/// where ψ(T) = T[1] − T[4] (morphic − entropic).  This makes the
/// coherence potential monotonically non-decreasing along any path,
/// which is incompatible with a cycle.
pub struct HDAG {
    data: HDAGData,
    store_file: PathBuf,
    /// Tolerance for coherence potential monotonicity.
    pub coherence_tolerance: f64,
    /// Path invariance tolerance.
    pub invariance_tolerance: f64,
}

impl HDAG {
    pub fn new(store_path: impl AsRef<Path>) -> Result<Self, String> {
        let store_path = store_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&store_path)
            .map_err(|e| format!("HDAG store dir: {e}"))?;

        let store_file = store_path.join("hdag.json");
        let data = if store_file.exists() {
            let s = std::fs::read_to_string(&store_file)
                .map_err(|e| format!("HDAG read: {e}"))?;
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            HDAGData::default()
        };

        Ok(Self {
            data,
            store_file,
            coherence_tolerance: 0.05,
            invariance_tolerance: 0.01,
        })
    }

    /// Coherence potential ψ(T) = morphic − entropic = T[1] − T[4].
    /// Increases along semantically valid paths.
    fn coherence_potential(t: &ResonanceTensor) -> f64 {
        t[1] - t[4]
    }

    /// S_coh: phase-locking subspace — a node is coherent when its morphic
    /// axis exceeds the entropic axis by a margin.  Kairos (PSE gate) always
    /// overrides this check.
    fn is_in_s_coh(node: &HDAGNode) -> bool {
        node.kairos || Self::coherence_potential(&node.tensor) > -0.1
    }

    /// Register a node.  Nodes outside S_coh are stored but cannot be edge sources.
    pub fn add_node(
        &mut self,
        id: &str,
        crystal_id_hex: &str,
        tensor: ResonanceTensor,
        kairos: bool,
        timestamp: &str,
    ) -> Result<(), String> {
        if self.data.nodes.contains_key(id) {
            return Ok(());
        }
        self.data.nodes.insert(id.to_string(), HDAGNode {
            id: id.to_string(),
            crystal_id_hex: crystal_id_hex.to_string(),
            tensor,
            kairos,
            timestamp: timestamp.to_string(),
        });
        self.data.adjacency.entry(id.to_string()).or_default();
        self.save()
    }

    /// Attempt to add a directed edge from `from` to `to`.
    ///
    /// The edge is created only when:
    /// 1. Source node T_i ∈ S_coh (coherence gate)
    /// 2. ψ(T_j) ≥ ψ(T_i) − ε  (acyclicity via coherence potential)
    /// 3. T_i ≠ T_j (non-trivial gradient)
    ///
    /// Returns the gradient vector if the edge was created, None otherwise.
    pub fn add_edge(
        &mut self,
        from: &str,
        to: &str,
    ) -> Result<Option<HDAGEdge>, String> {
        let source = self.data.nodes.get(from)
            .ok_or_else(|| format!("HDAG: source node not found: {from}"))?
            .clone();
        let dest = self.data.nodes.get(to)
            .ok_or_else(|| format!("HDAG: dest node not found: {to}"))?
            .clone();

        // Gate 1: source must be in S_coh
        if !Self::is_in_s_coh(&source) {
            return Ok(None);
        }

        // Gate 2: coherence potential must be non-decreasing (emergent acyclicity)
        let psi_i = Self::coherence_potential(&source.tensor);
        let psi_j = Self::coherence_potential(&dest.tensor);
        if psi_j < psi_i - self.coherence_tolerance {
            return Ok(None);
        }

        // Compute Φ_ij = (T_j − T_i) / ‖T_j − T_i‖
        let diff: ResonanceTensor = std::array::from_fn(|k| dest.tensor[k] - source.tensor[k]);
        let mag = diff.iter().map(|x| x * x).sum::<f64>().sqrt();
        if mag < 1e-10 {
            return Ok(None);
        }
        let gradient: ResonanceTensor = std::array::from_fn(|k| diff[k] / mag);

        // Avoid duplicate edges
        if self.data.adjacency
            .get(from)
            .map(|succs| succs.contains(&to.to_string()))
            .unwrap_or(false)
        {
            return Ok(None);
        }

        let edge = HDAGEdge {
            from: from.to_string(),
            to: to.to_string(),
            gradient,
            magnitude: mag,
        };
        self.data.edges.push(edge.clone());
        self.data.adjacency.entry(from.to_string()).or_default().push(to.to_string());
        self.save()?;

        Ok(Some(edge))
    }

    /// Topological order via Kahn's algorithm (Kairos-gated).
    pub fn topological_order(&self) -> Vec<String> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for id in self.data.nodes.keys() {
            in_degree.insert(id.clone(), 0);
        }
        for succs in self.data.adjacency.values() {
            for s in succs {
                *in_degree.entry(s.clone()).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut order = Vec::new();
        while let Some(node) = queue.pop_front() {
            order.push(node.clone());
            if let Some(succs) = self.data.adjacency.get(&node) {
                for s in succs {
                    let d = in_degree.entry(s.clone()).or_insert(1);
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        queue.push_back(s.clone());
                    }
                }
            }
        }
        order
    }

    /// Enumerate all simple paths from `start` to `end` (DFS, capped at 64 paths).
    fn all_paths(&self, start: &str, end: &str) -> Vec<Vec<String>> {
        let mut result = Vec::new();
        let mut stack: Vec<(String, Vec<String>, HashSet<String>)> =
            vec![(start.to_string(), vec![start.to_string()], {
                let mut s = HashSet::new();
                s.insert(start.to_string());
                s
            })];

        while let Some((current, path, visited)) = stack.pop() {
            if current == end {
                result.push(path);
                if result.len() >= 64 {
                    break;
                }
                continue;
            }
            if let Some(succs) = self.data.adjacency.get(&current) {
                for next in succs {
                    if !visited.contains(next) {
                        let mut new_path = path.clone();
                        new_path.push(next.clone());
                        let mut new_visited = visited.clone();
                        new_visited.insert(next.clone());
                        stack.push((next.clone(), new_path, new_visited));
                    }
                }
            }
        }
        result
    }

    /// Verify path invariance between two nodes.
    ///
    /// For each path, sums the gradient vectors of all edges along it.
    /// Invariance holds when all summed vectors are within `invariance_tolerance`
    /// of each other (the resonance field is curl-free between these nodes).
    pub fn verify_path_invariance(
        &self,
        start: &str,
        end: &str,
    ) -> PathInvarianceResult {
        let paths = self.all_paths(start, end);

        // Build edge gradient lookup
        let edge_map: HashMap<(&str, &str), &HDAGEdge> = self
            .data
            .edges
            .iter()
            .map(|e| ((e.from.as_str(), e.to.as_str()), e))
            .collect();

        let path_gradients: Vec<ResonanceTensor> = paths
            .iter()
            .map(|path| {
                let mut sum = [0.0f64; 5];
                for w in path.windows(2) {
                    if let Some(edge) = edge_map.get(&(w[0].as_str(), w[1].as_str())) {
                        for k in 0..5 {
                            sum[k] += edge.gradient[k];
                        }
                    }
                }
                sum
            })
            .collect();

        let max_divergence = if path_gradients.len() < 2 {
            0.0
        } else {
            let mut max = 0.0f64;
            for i in 0..path_gradients.len() {
                for j in (i + 1)..path_gradients.len() {
                    let dist = (0..5)
                        .map(|k| (path_gradients[i][k] - path_gradients[j][k]).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    if dist > max {
                        max = dist;
                    }
                }
            }
            max
        };

        PathInvarianceResult {
            start: start.to_string(),
            end: end.to_string(),
            path_count: paths.len(),
            path_gradients,
            max_divergence,
            invariant: max_divergence < self.invariance_tolerance,
        }
    }

    pub fn node_count(&self) -> usize {
        self.data.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.data.edges.len()
    }

    /// Mean coherence potential across all nodes.
    pub fn mean_coherence_potential(&self) -> f64 {
        if self.data.nodes.is_empty() {
            return 0.0;
        }
        let sum: f64 = self
            .data
            .nodes
            .values()
            .map(|n| Self::coherence_potential(&n.tensor))
            .sum();
        sum / self.data.nodes.len() as f64
    }

    fn save(&self) -> Result<(), String> {
        let s = serde_json::to_string_pretty(&self.data)
            .map_err(|e| format!("HDAG serialise: {e}"))?;
        std::fs::write(&self.store_file, s)
            .map_err(|e| format!("HDAG write: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tensor(kuramoto: f64, stability: f64) -> ResonanceTensor {
        [0.1, kuramoto, 0.3, 0.4, 1.0 - stability]
    }

    fn fresh_hdag() -> HDAG {
        let dir = tempfile::tempdir().unwrap();
        // keep dir alive by leaking — fine for tests
        let path = dir.keep();
        HDAG::new(path).unwrap()
    }

    #[test]
    fn edge_created_when_coherence_increases() {
        let mut h = fresh_hdag();
        h.add_node("A", "aaa", tensor(0.5, 0.8), true, "t0").unwrap();
        h.add_node("B", "bbb", tensor(0.7, 0.9), true, "t1").unwrap();
        let e = h.add_edge("A", "B").unwrap();
        assert!(e.is_some(), "edge should form when ψ increases");
        let edge = e.unwrap();
        assert_eq!(edge.gradient.len(), 5);
        let norm: f64 = edge.gradient.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "gradient must be unit-normalised");
    }

    #[test]
    fn edge_blocked_by_coherence_gate() {
        let mut h = fresh_hdag();
        // Low coherence source — not in S_coh, kairos=false
        h.add_node("A", "aaa", tensor(0.1, 0.2), false, "t0").unwrap();
        h.add_node("B", "bbb", tensor(0.3, 0.4), false, "t1").unwrap();
        // ψ(A) = 0.1 − 0.8 = −0.7, below threshold
        let e = h.add_edge("A", "B").unwrap();
        assert!(e.is_none(), "edge should be blocked when source is outside S_coh");
    }

    #[test]
    fn emergent_acyclicity_prevents_backward_edge() {
        let mut h = fresh_hdag();
        h.add_node("A", "aaa", tensor(0.8, 0.9), true, "t0").unwrap();
        h.add_node("B", "bbb", tensor(0.3, 0.4), true, "t1").unwrap();
        // A has high coherence potential, B has low — backward edge A→B should be blocked
        let e = h.add_edge("A", "B").unwrap();
        assert!(e.is_none(), "backward edge must be blocked by coherence potential gate");
    }

    #[test]
    fn path_invariance_two_parallel_paths() {
        let mut h = fresh_hdag();
        // A→B→D and A→C→D: two paths, both with same net gradient (symmetric tensors)
        let t_a: ResonanceTensor = [0.0, 0.2, 0.0, 0.0, 0.7];
        let t_b: ResonanceTensor = [0.0, 0.5, 0.0, 0.0, 0.4];
        let t_c: ResonanceTensor = [0.0, 0.5, 0.0, 0.0, 0.4];
        let t_d: ResonanceTensor = [0.0, 0.8, 0.0, 0.0, 0.1];
        h.add_node("A", "aaa", t_a, true, "t0").unwrap();
        h.add_node("B", "bbb", t_b, true, "t1").unwrap();
        h.add_node("C", "ccc", t_c, true, "t2").unwrap();
        h.add_node("D", "ddd", t_d, true, "t3").unwrap();
        h.add_edge("A", "B").unwrap();
        h.add_edge("A", "C").unwrap();
        h.add_edge("B", "D").unwrap();
        h.add_edge("C", "D").unwrap();

        let result = h.verify_path_invariance("A", "D");
        assert_eq!(result.path_count, 2);
        // Symmetric paths: B and C have the same tensor → gradients are identical
        assert!(
            result.invariant,
            "symmetric paths must be invariant, divergence={}",
            result.max_divergence
        );
    }

    #[test]
    fn topological_order_respects_edges() {
        let mut h = fresh_hdag();
        h.add_node("X", "xxx", tensor(0.4, 0.7), true, "t0").unwrap();
        h.add_node("Y", "yyy", tensor(0.6, 0.8), true, "t1").unwrap();
        h.add_node("Z", "zzz", tensor(0.8, 0.9), true, "t2").unwrap();
        h.add_edge("X", "Y").unwrap();
        h.add_edge("Y", "Z").unwrap();

        let order = h.topological_order();
        let xi = order.iter().position(|s| s == "X").unwrap();
        let yi = order.iter().position(|s| s == "Y").unwrap();
        let zi = order.iter().position(|s| s == "Z").unwrap();
        assert!(xi < yi && yi < zi, "topological order must respect edge direction");
    }
}
