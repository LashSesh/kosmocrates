//! `FunnelGraph` (§12).
//!
//! The funnel graph carries spatial / temporal / semantic / resonance
//! edges in parallel. Acyclicity is the default; cyclic graphs require
//! an explicit descriptor flag (§12 invariant).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::primitives::{content_address, Fixed, Hash256, PhaseError};

/// Funnel node kinds (§12.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FunnelNodeKind {
    /// Cell node.
    Cell,
    /// Pulse node.
    Pulse,
    /// Memory probe.
    Memory,
    /// Boundary marker.
    Boundary,
    /// Intent vertex.
    Intent,
    /// Trace vertex.
    Trace,
}

/// Funnel edge kinds (§12.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FunnelEdgeKind {
    /// Spatial edge (parent/child or co-membership).
    Spatial,
    /// Temporal edge (lifecycle transition).
    Temporal,
    /// Semantic edge (shared purpose / role).
    Semantic,
    /// Resonance edge (correlated pulses).
    Resonance,
    /// Evidence-bearing edge.
    Evidence,
}

/// `FunnelNode` (§12.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FunnelNode {
    /// Content-addressed node id.
    pub node_id: Hash256,
    /// Optional cell reference.
    pub cell_id: Option<Hash256>,
    /// Node kind.
    pub node_kind: FunnelNodeKind,
    /// Hash of the node's local state snapshot.
    pub state_hash: Hash256,
}

/// `FunnelEdge` (§12.2).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FunnelEdge {
    /// Content-addressed edge id.
    pub edge_id: Hash256,
    /// Source node id.
    pub source: Hash256,
    /// Target node id.
    pub target: Hash256,
    /// Edge weight.
    pub weight: Fixed,
    /// Edge kind.
    pub edge_kind: FunnelEdgeKind,
}

/// `FunnelGraph` (§12.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunnelGraph {
    /// Content-addressed graph id.
    pub graph_id: Hash256,
    /// Owning cluster id.
    pub cluster_id: Hash256,
    /// Sorted node list.
    pub nodes: Vec<FunnelNode>,
    /// Sorted spatial edges.
    pub spatial_edges: Vec<FunnelEdge>,
    /// Sorted temporal edges.
    pub temporal_edges: Vec<FunnelEdge>,
    /// Sorted semantic edges.
    pub semantic_edges: Vec<FunnelEdge>,
    /// Sorted resonance edges.
    pub resonance_edges: Vec<FunnelEdge>,
    /// Whether the graph is required to be acyclic.
    pub acyclic: bool,
    /// Hash of the trace artefact for this graph.
    pub trace_hash: Hash256,
}

impl FunnelGraph {
    /// Recompute `graph_id` after sorting node + edge lists. When
    /// `acyclic == true` the graph is topologically validated; cycles
    /// raise [`PhaseError::FunnelTopology`].
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.nodes.sort();
        self.spatial_edges.sort();
        self.temporal_edges.sort();
        self.semantic_edges.sort();
        self.resonance_edges.sort();
        if self.acyclic {
            self.validate_acyclic()?;
        }
        let mut probe = self.clone();
        probe.graph_id = Hash256::zero();
        self.graph_id = content_address(&probe)?;
        Ok(self)
    }

    fn validate_acyclic(&self) -> Result<(), PhaseError> {
        // Build adjacency over every edge family.
        let mut adj: BTreeMap<Hash256, BTreeSet<Hash256>> = BTreeMap::new();
        for n in &self.nodes {
            adj.entry(n.node_id.clone()).or_default();
        }
        for edges in [
            &self.spatial_edges,
            &self.temporal_edges,
            &self.semantic_edges,
            &self.resonance_edges,
        ] {
            for e in edges.iter() {
                adj.entry(e.source.clone())
                    .or_default()
                    .insert(e.target.clone());
            }
        }
        // Standard DFS cycle detection with WHITE/GRAY/BLACK colours.
        #[derive(Clone, Copy, PartialEq)]
        enum Color {
            White,
            Gray,
            Black,
        }
        let mut color: BTreeMap<Hash256, Color> =
            adj.keys().map(|k| (k.clone(), Color::White)).collect();
        for start in adj.keys().cloned().collect::<Vec<_>>() {
            if color[&start] != Color::White {
                continue;
            }
            let mut stack = vec![(start.clone(), false)];
            while let Some((node, finished)) = stack.pop() {
                if finished {
                    color.insert(node, Color::Black);
                    continue;
                }
                if color.get(&node).copied() == Some(Color::Gray) {
                    return Err(PhaseError::FunnelTopology {
                        reason: format!(
                            "cycle through node {} (acyclic=true required)",
                            node.hex()
                        ),
                    });
                }
                color.insert(node.clone(), Color::Gray);
                stack.push((node.clone(), true));
                if let Some(succs) = adj.get(&node) {
                    for s in succs {
                        if color.get(s).copied() != Some(Color::Black) {
                            stack.push((s.clone(), false));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Build a minimal acyclic funnel graph for a cluster (one Cell
    /// node per member + one outgoing spatial edge to a Trace anchor).
    pub fn minimal_for_cluster(
        cluster_id: Hash256,
        member_cells: &[Hash256],
    ) -> Result<Self, PhaseError> {
        let trace_anchor = FunnelNode {
            node_id: content_address(&("trace.anchor", &cluster_id))?,
            cell_id: None,
            node_kind: FunnelNodeKind::Trace,
            state_hash: cluster_id.clone(),
        };
        let mut nodes = vec![trace_anchor.clone()];
        let mut spatial = Vec::with_capacity(member_cells.len());
        for cell in member_cells {
            let cell_node = FunnelNode {
                node_id: content_address(&("cell.node", cell))?,
                cell_id: Some(cell.clone()),
                node_kind: FunnelNodeKind::Cell,
                state_hash: cell.clone(),
            };
            spatial.push(FunnelEdge {
                edge_id: content_address(&("spatial", &cell_node.node_id, &trace_anchor.node_id))?,
                source: cell_node.node_id.clone(),
                target: trace_anchor.node_id.clone(),
                weight: Fixed::Rational { num: 1, den: 1 },
                edge_kind: FunnelEdgeKind::Spatial,
            });
            nodes.push(cell_node);
        }
        FunnelGraph {
            graph_id: Hash256::zero(),
            cluster_id,
            nodes,
            spatial_edges: spatial,
            temporal_edges: Vec::new(),
            semantic_edges: Vec::new(),
            resonance_edges: Vec::new(),
            acyclic: true,
            trace_hash: Hash256::zero(),
        }
        .with_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acyclic_validation_passes_on_minimal_graph() {
        let g = FunnelGraph::minimal_for_cluster(Hash256::zero(), &[]).unwrap();
        assert!(g.acyclic);
    }

    #[test]
    fn cycle_is_rejected_when_acyclic_is_set() {
        let n0 = FunnelNode {
            node_id: {
                let mut x = [0u8; 32];
                x[0] = 1;
                Hash256(x)
            },
            cell_id: None,
            node_kind: FunnelNodeKind::Cell,
            state_hash: Hash256::zero(),
        };
        let n1 = FunnelNode {
            node_id: {
                let mut x = [0u8; 32];
                x[0] = 2;
                Hash256(x)
            },
            cell_id: None,
            node_kind: FunnelNodeKind::Cell,
            state_hash: Hash256::zero(),
        };
        let e01 = FunnelEdge {
            edge_id: {
                let mut x = [0u8; 32];
                x[0] = 11;
                Hash256(x)
            },
            source: n0.node_id.clone(),
            target: n1.node_id.clone(),
            weight: Fixed::Rational { num: 1, den: 1 },
            edge_kind: FunnelEdgeKind::Spatial,
        };
        let e10 = FunnelEdge {
            edge_id: {
                let mut x = [0u8; 32];
                x[0] = 12;
                Hash256(x)
            },
            source: n1.node_id.clone(),
            target: n0.node_id.clone(),
            weight: Fixed::Rational { num: 1, den: 1 },
            edge_kind: FunnelEdgeKind::Spatial,
        };
        let g = FunnelGraph {
            graph_id: Hash256::zero(),
            cluster_id: Hash256::zero(),
            nodes: vec![n0, n1],
            spatial_edges: vec![e01, e10],
            temporal_edges: vec![],
            semantic_edges: vec![],
            resonance_edges: vec![],
            acyclic: true,
            trace_hash: Hash256::zero(),
        };
        assert!(g.with_id().is_err());
    }
}
