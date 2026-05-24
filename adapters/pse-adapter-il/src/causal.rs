//! Causal graph extraction from the HDAG and crystal genealogy.
//!
//! Every HDAG edge encodes a causal relationship: sequential commits enforce
//! temporal precedence (ψ-monotone), resonance-proximity edges link crystals
//! by structural similarity, refinement edges record IL-feedback genealogy,
//! and Metatron-isomorphic edges connect topologically equivalent regions.
//!
//! `CausalGraph::build` translates the full HDAG edge set into a typed,
//! queryable causal structure.  This makes PSE's implicit causal order
//! explicit and traversable without accessing HDAG internals.
//!
//! ## Causal strength
//! Strength ∈ [0, 1] is derived from the edge phase-gradient magnitude:
//!
//! ```text
//! strength = 1 / (1 + magnitude)    ← resonance_proximity, refinement
//! strength = (gradient[1] * magnitude).clamp(0, 1)  ← sequential_commit (Δψ)
//! strength = 1.0                    ← metatron_isomorphic (structural identity)
//! ```

use crate::hdag::HDAGEdge;
use std::collections::{HashSet, VecDeque};

/// Nature of a directed causal relationship between two crystals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CausalCause {
    /// Temporal commitment sequence: source was committed before target
    /// with ψ-monotone coherence potential (emergent acyclicity).
    SequentialCommit,
    /// Phase-resonance proximity: source and target share similar 5D
    /// resonance tensors and the coherence gate allows the connection.
    ResonanceProximity,
    /// Genealogical descent: target is an IL-refined version of source
    /// (`parent_crystal_ids` link).
    Refinement,
    /// Structural isomorphism: source and target have the same Metatron
    /// canonical hash (topologically equivalent region subgraphs).
    MetatronIsomorphic,
    /// Unknown cause label from a future HDAG version.
    Other(String),
}

impl CausalCause {
    fn from_str(s: &str) -> Self {
        match s {
            "sequential_commit" => Self::SequentialCommit,
            "resonance_proximity" => Self::ResonanceProximity,
            "refinement" => Self::Refinement,
            "metatron_isomorphic" => Self::MetatronIsomorphic,
            other => Self::Other(other.to_string()),
        }
    }

    /// Short label for display (matches the HDAG cause string).
    pub fn label(&self) -> &str {
        match self {
            Self::SequentialCommit => "sequential_commit",
            Self::ResonanceProximity => "resonance_proximity",
            Self::Refinement => "refinement",
            Self::MetatronIsomorphic => "metatron_isomorphic",
            Self::Other(s) => s.as_str(),
        }
    }
}

/// A directed causal link between two crystals.
///
/// Both `from` and `to` are 16-char crystal ID prefixes, matching the
/// `crystal_id` field in [`crate::context::CrystalSummary`].
#[derive(Debug, Clone)]
pub struct CausalLink {
    /// Source crystal (16-char hex prefix).
    pub from: String,
    /// Target crystal (16-char hex prefix).
    pub to: String,
    /// Nature of the causal relationship.
    pub cause: CausalCause,
    /// Causal strength ∈ [0, 1].
    pub strength: f64,
}

/// Complete causal structure derived from the HDAG.
///
/// Build via [`CausalGraph::from_edges`] (called internally by
/// [`ILStore::build_causal_graph`]).
#[derive(Debug, Clone, Default)]
pub struct CausalGraph {
    pub links: Vec<CausalLink>,
}

impl CausalGraph {
    /// Build a `CausalGraph` from raw HDAG edges.
    ///
    /// HDAG node IDs have the form `"N-<16-char-crystal-prefix>"`.
    /// The `"N-"` prefix is stripped to yield the crystal ID prefix used
    /// throughout the context and causal modules.
    pub fn from_edges(edges: &[HDAGEdge]) -> Self {
        let links = edges
            .iter()
            .map(|e| {
                let from = strip_node_prefix(&e.from);
                let to = strip_node_prefix(&e.to);
                let cause = CausalCause::from_str(&e.cause);
                let strength = compute_strength(&cause, &e.gradient, e.magnitude);
                CausalLink {
                    from,
                    to,
                    cause,
                    strength,
                }
            })
            .collect();
        Self { links }
    }

    /// Direct causal predecessors of `crystal_id`.
    pub fn direct_causes(&self, crystal_id: &str) -> Vec<&CausalLink> {
        self.links.iter().filter(|l| l.to == crystal_id).collect()
    }

    /// Direct causal successors of `crystal_id`.
    pub fn direct_effects(&self, crystal_id: &str) -> Vec<&CausalLink> {
        self.links.iter().filter(|l| l.from == crystal_id).collect()
    }

    /// All causal ancestors of `crystal_id` (BFS over incoming edges).
    /// Returns crystal ID prefixes in BFS order (nearest ancestors first).
    pub fn ancestors(&self, crystal_id: &str) -> Vec<String> {
        bfs_backward(&self.links, crystal_id)
    }

    /// All causal descendants of `crystal_id` (BFS over outgoing edges).
    pub fn descendants(&self, crystal_id: &str) -> Vec<String> {
        bfs_forward(&self.links, crystal_id)
    }

    /// Shortest causal path from `from` to `to` (BFS).
    /// Returns `None` if no path exists.
    pub fn causal_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut queue: VecDeque<Vec<String>> = VecDeque::new();
        queue.push_back(vec![from.to_string()]);
        visited.insert(from.to_string());

        while let Some(path) = queue.pop_front() {
            let current = path.last().unwrap().as_str();
            if current == to {
                return Some(path);
            }
            for link in &self.links {
                if link.from == current && !visited.contains(&link.to) {
                    visited.insert(link.to.clone());
                    let mut new_path = path.clone();
                    new_path.push(link.to.clone());
                    queue.push_back(new_path);
                }
            }
        }
        None
    }

    /// Crystals with no causal predecessors (primordial crystals).
    pub fn roots(&self) -> Vec<String> {
        let all_targets: HashSet<&str> = self.links.iter().map(|l| l.to.as_str()).collect();
        let all_sources: HashSet<&str> = self.links.iter().map(|l| l.from.as_str()).collect();
        all_sources
            .into_iter()
            .filter(|s| !all_targets.contains(s))
            .map(|s| s.to_string())
            .collect()
    }

    /// Crystals with no causal successors (frontier crystals).
    pub fn leaves(&self) -> Vec<String> {
        let all_sources: HashSet<&str> = self.links.iter().map(|l| l.from.as_str()).collect();
        let all_targets: HashSet<&str> = self.links.iter().map(|l| l.to.as_str()).collect();
        all_targets
            .into_iter()
            .filter(|t| !all_sources.contains(t))
            .map(|s| s.to_string())
            .collect()
    }

    /// Human-readable summary of the full causal graph.
    pub fn summary(&self) -> String {
        if self.links.is_empty() {
            return "CausalGraph: empty (no edges yet)".to_string();
        }
        let roots = self.roots();
        let leaves = self.leaves();
        let seq = self
            .links
            .iter()
            .filter(|l| l.cause == CausalCause::SequentialCommit)
            .count();
        let res = self
            .links
            .iter()
            .filter(|l| l.cause == CausalCause::ResonanceProximity)
            .count();
        let refin = self
            .links
            .iter()
            .filter(|l| l.cause == CausalCause::Refinement)
            .count();
        let iso = self
            .links
            .iter()
            .filter(|l| l.cause == CausalCause::MetatronIsomorphic)
            .count();
        format!(
            "CausalGraph: {} links ({} sequential, {} resonance, {} refinement, {} isomorphic)\n\
             Roots: {} | Leaves: {}",
            self.links.len(),
            seq,
            res,
            refin,
            iso,
            roots.len(),
            leaves.len()
        )
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Strip the "N-" HDAG node prefix to obtain the 16-char crystal ID prefix.
fn strip_node_prefix(node_id: &str) -> String {
    node_id.strip_prefix("N-").unwrap_or(node_id).to_string()
}

/// Compute causal strength ∈ [0, 1] from the HDAG edge.
fn compute_strength(cause: &CausalCause, gradient: &[f64; 5], magnitude: f64) -> f64 {
    match cause {
        // Δψ along the morphic axis — larger morphic gradient = stronger sequential pull.
        CausalCause::SequentialCommit => (gradient[1] * magnitude).clamp(0.0, 1.0),
        // Inverse tensor distance — closer resonance = stronger proximity link.
        CausalCause::ResonanceProximity | CausalCause::Refinement => 1.0 / (1.0 + magnitude),
        // Structural identity is always maximal strength.
        CausalCause::MetatronIsomorphic => 1.0,
        CausalCause::Other(_) => 0.5,
    }
}

fn bfs_backward(links: &[CausalLink], start: &str) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start.to_string());
    visited.insert(start.to_string());
    let mut result = Vec::new();
    while let Some(current) = queue.pop_front() {
        for link in links {
            if link.to == current && !visited.contains(&link.from) {
                visited.insert(link.from.clone());
                queue.push_back(link.from.clone());
                result.push(link.from.clone());
            }
        }
    }
    result
}

fn bfs_forward(links: &[CausalLink], start: &str) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start.to_string());
    visited.insert(start.to_string());
    let mut result = Vec::new();
    while let Some(current) = queue.pop_front() {
        for link in links {
            if link.from == current && !visited.contains(&link.to) {
                visited.insert(link.to.clone());
                queue.push_back(link.to.clone());
                result.push(link.to.clone());
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hdag::{HDAGEdge, ResonanceTensor};

    fn make_edge(from: &str, to: &str, cause: &str, magnitude: f64) -> HDAGEdge {
        // Gradient: morphic axis (index 1) positive to simulate ψ increase
        let gradient: ResonanceTensor = [0.0, 1.0 / magnitude.max(1.0), 0.0, 0.0, 0.0];
        HDAGEdge {
            from: format!("N-{}", from),
            to: format!("N-{}", to),
            gradient,
            magnitude,
            cause: cause.to_string(),
        }
    }

    #[test]
    fn from_edges_strips_node_prefix() {
        let edges = vec![make_edge("aaa", "bbb", "sequential_commit", 0.3)];
        let g = CausalGraph::from_edges(&edges);
        assert_eq!(g.links.len(), 1);
        assert_eq!(g.links[0].from, "aaa");
        assert_eq!(g.links[0].to, "bbb");
    }

    #[test]
    fn cause_enum_maps_correctly() {
        let edges = vec![
            make_edge("a", "b", "sequential_commit", 0.3),
            make_edge("b", "c", "resonance_proximity", 0.5),
            make_edge("c", "d", "refinement", 0.2),
            make_edge("d", "e", "metatron_isomorphic", 0.1),
            make_edge("e", "f", "unknown_future", 0.4),
        ];
        let g = CausalGraph::from_edges(&edges);
        assert_eq!(g.links[0].cause, CausalCause::SequentialCommit);
        assert_eq!(g.links[1].cause, CausalCause::ResonanceProximity);
        assert_eq!(g.links[2].cause, CausalCause::Refinement);
        assert_eq!(g.links[3].cause, CausalCause::MetatronIsomorphic);
        assert!(matches!(&g.links[4].cause, CausalCause::Other(s) if s == "unknown_future"));
    }

    #[test]
    fn strength_metatron_is_maximal() {
        let edges = vec![make_edge("a", "b", "metatron_isomorphic", 2.0)];
        let g = CausalGraph::from_edges(&edges);
        assert_eq!(g.links[0].strength, 1.0);
    }

    #[test]
    fn strength_resonance_inversely_proportional_to_magnitude() {
        let edges_near = vec![make_edge("a", "b", "resonance_proximity", 0.1)];
        let edges_far = vec![make_edge("a", "b", "resonance_proximity", 2.0)];
        let g_near = CausalGraph::from_edges(&edges_near);
        let g_far = CausalGraph::from_edges(&edges_far);
        assert!(
            g_near.links[0].strength > g_far.links[0].strength,
            "closer resonance must yield higher causal strength"
        );
    }

    #[test]
    fn ancestors_linear_chain() {
        // A → B → C
        let edges = vec![
            make_edge("aaa", "bbb", "sequential_commit", 0.3),
            make_edge("bbb", "ccc", "sequential_commit", 0.3),
        ];
        let g = CausalGraph::from_edges(&edges);
        let ancs = g.ancestors("ccc");
        assert!(
            ancs.contains(&"bbb".to_string()),
            "bbb must be an ancestor of ccc"
        );
        assert!(
            ancs.contains(&"aaa".to_string()),
            "aaa must be an ancestor of ccc"
        );
    }

    #[test]
    fn descendants_linear_chain() {
        let edges = vec![
            make_edge("aaa", "bbb", "sequential_commit", 0.3),
            make_edge("bbb", "ccc", "sequential_commit", 0.3),
        ];
        let g = CausalGraph::from_edges(&edges);
        let descs = g.descendants("aaa");
        assert!(descs.contains(&"bbb".to_string()));
        assert!(descs.contains(&"ccc".to_string()));
    }

    #[test]
    fn roots_and_leaves_linear_chain() {
        let edges = vec![
            make_edge("aaa", "bbb", "sequential_commit", 0.3),
            make_edge("bbb", "ccc", "sequential_commit", 0.3),
        ];
        let g = CausalGraph::from_edges(&edges);
        let roots = g.roots();
        let leaves = g.leaves();
        assert!(
            roots.contains(&"aaa".to_string()),
            "first crystal must be a root"
        );
        assert!(
            leaves.contains(&"ccc".to_string()),
            "last crystal must be a leaf"
        );
        assert!(
            !roots.contains(&"ccc".to_string()),
            "last crystal must not be a root"
        );
    }

    #[test]
    fn causal_path_found() {
        let edges = vec![
            make_edge("aaa", "bbb", "sequential_commit", 0.3),
            make_edge("bbb", "ccc", "sequential_commit", 0.3),
        ];
        let g = CausalGraph::from_edges(&edges);
        let path = g.causal_path("aaa", "ccc");
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path, vec!["aaa", "bbb", "ccc"]);
    }

    #[test]
    fn causal_path_not_found_when_disconnected() {
        let edges = vec![make_edge("aaa", "bbb", "sequential_commit", 0.3)];
        let g = CausalGraph::from_edges(&edges);
        assert!(
            g.causal_path("bbb", "aaa").is_none(),
            "no backward path in a DAG"
        );
        assert!(
            g.causal_path("aaa", "zzz").is_none(),
            "no path to unknown crystal"
        );
    }

    #[test]
    fn summary_includes_link_counts() {
        let edges = vec![
            make_edge("a", "b", "sequential_commit", 0.3),
            make_edge("b", "c", "resonance_proximity", 0.5),
            make_edge("c", "d", "refinement", 0.2),
        ];
        let g = CausalGraph::from_edges(&edges);
        let s = g.summary();
        assert!(
            s.contains("3 links"),
            "summary must report total link count"
        );
        assert!(s.contains("sequential"), "summary must mention sequential");
        assert!(s.contains("resonance"), "summary must mention resonance");
        assert!(s.contains("refinement"), "summary must mention refinement");
    }

    #[test]
    fn direct_causes_and_effects() {
        let edges = vec![
            make_edge("aaa", "bbb", "sequential_commit", 0.3),
            make_edge("aaa", "ccc", "resonance_proximity", 0.5),
        ];
        let g = CausalGraph::from_edges(&edges);
        assert_eq!(g.direct_effects("aaa").len(), 2, "aaa causes bbb and ccc");
        assert_eq!(g.direct_causes("bbb").len(), 1, "bbb is caused by aaa");
        assert_eq!(g.direct_causes("aaa").len(), 0, "aaa has no causes");
    }

    #[test]
    fn empty_graph_summary() {
        let g = CausalGraph::from_edges(&[]);
        assert!(g.summary().contains("empty"));
    }
}
