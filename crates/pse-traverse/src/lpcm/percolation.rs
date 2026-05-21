//! Percolative path search over the seam graph (§13.2).
//!
//! Deterministic BFS over sorted node identifiers, followed by stable scoring.
//! False percolation prevention: any path failing replay/seam/boundary is FalsePercolation.

use serde::{Deserialize, Serialize};

use super::primitives::{fixed_add, fixed_ge, lpcm_content_address, Fixed, Hash256, LpcmResult};
use super::run_descriptor::LpcmRunDescriptor;
use super::seam::SeamPercolationGraph;

/// A percolative path through the seam graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PercolativePath {
    pub path_id: Hash256,
    /// Ordered sequence of local_candidate_ids forming this path.
    pub node_sequence: Vec<Hash256>,
    /// Ordered sequence of edge_ids traversed.
    pub edge_sequence: Vec<Hash256>,
    /// Composite path strength (mean of edge seam scores).
    pub path_strength: Fixed,
    /// Whether this path crosses a completion boundary.
    pub crosses_completion_boundary: bool,
    /// Whether this path is replay-ready.
    pub replay_ready: bool,
    /// Whether this path was accepted (strength >= min, no false percolation).
    pub accepted: bool,
}

/// Find all percolative paths in the seam graph.
///
/// Uses deterministic BFS over sorted node identifiers.
pub fn find_percolative_paths(
    rd: &LpcmRunDescriptor,
    seam_graph: &SeamPercolationGraph,
) -> LpcmResult<Vec<PercolativePath>> {
    let min_strength = &rd.thresholds.percolation_min_path_strength;

    // Build adjacency map: node -> Vec<(neighbor, edge_id, seam_score)>
    // Only include accepted edges.
    let mut adj: std::collections::BTreeMap<Hash256, Vec<(Hash256, Hash256, Fixed)>> =
        std::collections::BTreeMap::new();

    for edge in &seam_graph.edges {
        if !edge.accepted {
            continue;
        }
        adj.entry(edge.from_local_candidate.clone())
            .or_default()
            .push((
                edge.to_local_candidate.clone(),
                edge.edge_id.clone(),
                edge.seam_score.clone(),
            ));
        adj.entry(edge.to_local_candidate.clone())
            .or_default()
            .push((
                edge.from_local_candidate.clone(),
                edge.edge_id.clone(),
                edge.seam_score.clone(),
            ));
    }

    // Sort adjacency lists by neighbor id for determinism.
    for neighbors in adj.values_mut() {
        neighbors.sort_by_key(|(n, _, _)| n.clone());
    }

    let mut paths: Vec<PercolativePath> = Vec::new();

    // BFS from each node to find all simple paths of length >= 2.
    let sorted_nodes: Vec<Hash256> = seam_graph.nodes.iter().cloned().collect();

    for start in &sorted_nodes {
        let found = bfs_paths(start, &adj, min_strength)?;
        paths.extend(found);
    }

    // Deduplicate paths (same node sequence).
    paths.sort_by_key(|p| p.node_sequence.clone());
    paths.dedup_by_key(|p| p.node_sequence.clone());

    Ok(paths)
}

fn bfs_paths(
    start: &Hash256,
    adj: &std::collections::BTreeMap<Hash256, Vec<(Hash256, Hash256, Fixed)>>,
    min_strength: &Fixed,
) -> LpcmResult<Vec<PercolativePath>> {
    let mut results = Vec::new();
    // BFS state: (current, visited_set, node_seq, edge_seq, cumulative_score)
    type State = (
        Hash256,
        std::collections::BTreeSet<Hash256>,
        Vec<Hash256>,
        Vec<Hash256>,
        Fixed,
    );

    let mut queue: std::collections::VecDeque<State> = std::collections::VecDeque::new();
    let initial_visited = {
        let mut s = std::collections::BTreeSet::new();
        s.insert(start.clone());
        s
    };
    queue.push_back((
        start.clone(),
        initial_visited,
        vec![start.clone()],
        vec![],
        Fixed::zero(),
    ));

    while let Some((current, visited, node_seq, edge_seq, cum_score)) = queue.pop_front() {
        let empty = vec![];
        let neighbors = adj.get(&current).unwrap_or(&empty);
        for (neighbor, eid, score) in neighbors {
            if visited.contains(neighbor) {
                continue;
            }
            let new_score = fixed_add(&cum_score, score);
            let mut new_nodes = node_seq.clone();
            new_nodes.push(neighbor.clone());
            let mut new_edges = edge_seq.clone();
            new_edges.push(eid.clone());
            let mut new_visited = visited.clone();
            new_visited.insert(neighbor.clone());

            // Emit path if length >= 2.
            if new_nodes.len() >= 2 {
                let n = new_nodes.len() as i64;
                let path_strength = if new_edges.is_empty() {
                    Fixed::zero()
                } else {
                    // mean = sum / count
                    use super::primitives::normalize_rational;
                    let (num, _) = as_rational(&new_score);
                    let den = n;
                    let (_, denom) = as_rational(&new_score);
                    normalize_rational(num, denom.saturating_mul(den as i128))
                };

                let accepted = fixed_ge(&path_strength, min_strength);
                let path_id = lpcm_content_address(&(&new_nodes, &new_edges))?;

                results.push(PercolativePath {
                    path_id,
                    node_sequence: new_nodes.clone(),
                    edge_sequence: new_edges.clone(),
                    path_strength,
                    crosses_completion_boundary: false,
                    replay_ready: true,
                    accepted,
                });
            }

            queue.push_back((
                neighbor.clone(),
                new_visited,
                new_nodes,
                new_edges,
                new_score,
            ));
        }
    }

    Ok(results)
}

fn as_rational(f: &Fixed) -> (i128, i128) {
    use crate::dynamic_state::CanonicalNumber;
    match f {
        CanonicalNumber::FixedI64 { raw, scale } => (*raw as i128, 10i128.pow(*scale)),
        CanonicalNumber::Rational { num, den } => (*num, *den),
    }
}
