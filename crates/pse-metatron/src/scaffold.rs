use serde::Serialize;

use crate::geometry::{canonical_edges, canonical_nodes, Node};

/// Immutable representation of the 13-node Metatron scaffold.
#[derive(Debug, Clone, Serialize)]
pub struct MetatronScaffold {
    pub nodes: Vec<Node>,
    pub edges: Vec<(usize, usize)>,
    adjacency: Vec<Vec<f64>>,
}

impl MetatronScaffold {
    /// Construct the scaffold from canonical geometry.
    pub fn new() -> Self {
        let nodes = canonical_nodes();
        let edges = canonical_edges();
        let mut adjacency = vec![vec![0.0; 13]; 13];

        for &(i, j) in &edges {
            adjacency[i - 1][j - 1] = 1.0;
            adjacency[j - 1][i - 1] = 1.0;
        }

        Self {
            nodes,
            edges,
            adjacency,
        }
    }

    /// Returns a reference to the 13x13 adjacency matrix.
    pub fn adjacency(&self) -> &[Vec<f64>] {
        &self.adjacency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_has_13_nodes() {
        let s = MetatronScaffold::new();
        assert_eq!(s.nodes.len(), 13);
    }

    #[test]
    fn adjacency_is_symmetric() {
        let s = MetatronScaffold::new();
        let a = s.adjacency();
        for i in 0..13 {
            for j in 0..13 {
                assert_eq!(a[i][j], a[j][i], "Not symmetric at ({}, {})", i, j);
            }
        }
    }

    #[test]
    fn no_self_loops() {
        let s = MetatronScaffold::new();
        let a = s.adjacency();
        for i in 0..13 {
            assert_eq!(a[i][i], 0.0, "Self-loop at node {}", i + 1);
        }
    }
}
