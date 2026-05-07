use serde::{Deserialize, Serialize};

use crate::error::{ScanError, ScanResult};

/// A graph with n <= 7 nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputGraph {
    pub n: usize,
    pub adjacency: Vec<Vec<f64>>,
    pub labels: Option<Vec<String>>,
}

impl InputGraph {
    /// Create an InputGraph from an adjacency matrix.
    pub fn from_adjacency(matrix: Vec<Vec<f64>>) -> ScanResult<Self> {
        let n = matrix.len();
        let graph = Self {
            n,
            adjacency: matrix,
            labels: None,
        };
        graph.validate()?;
        Ok(graph)
    }

    /// Create an InputGraph from an edge list (1-indexed).
    pub fn from_edges(n: usize, edges: &[(usize, usize)]) -> ScanResult<Self> {
        if n == 0 {
            return Err(ScanError::EmptyGraph);
        }
        if n > 8 {
            return Err(ScanError::TooManyNodes { n });
        }

        let mut adj = vec![vec![0.0; n]; n];
        for &(i, j) in edges {
            if i < 1 || i > n {
                return Err(ScanError::InvalidIndex { index: i, n });
            }
            if j < 1 || j > n {
                return Err(ScanError::InvalidIndex { index: j, n });
            }
            if i == j {
                return Err(ScanError::SelfLoop { node: i });
            }
            adj[i - 1][j - 1] = 1.0;
            adj[j - 1][i - 1] = 1.0;
        }
        Self::from_adjacency(adj)
    }

    /// Create an InputGraph from an adjacency list (1-indexed neighbors).
    pub fn from_adj_list(adj_list: &[Vec<usize>]) -> ScanResult<Self> {
        let n = adj_list.len();
        if n == 0 {
            return Err(ScanError::EmptyGraph);
        }
        if n > 8 {
            return Err(ScanError::TooManyNodes { n });
        }

        let mut adj = vec![vec![0.0; n]; n];
        for (i, neighbors) in adj_list.iter().enumerate() {
            for &j in neighbors {
                if j < 1 || j > n {
                    return Err(ScanError::InvalidIndex { index: j, n });
                }
                adj[i][j - 1] = 1.0;
                adj[j - 1][i] = 1.0;
            }
        }
        Self::from_adjacency(adj)
    }

    /// Validate the graph:
    /// 1. n > 7 -> TooManyNodes
    /// 2. n == 0 -> EmptyGraph
    /// 3. Matrix not square -> DimensionMismatch
    /// 4. Not symmetric -> NotSymmetric
    /// 5. Self-loops -> SelfLoop
    /// 6. Non-binary values -> NonBinaryWeight
    pub fn validate(&self) -> ScanResult<()> {
        if self.n == 0 {
            return Err(ScanError::EmptyGraph);
        }
        if self.n > 8 {
            return Err(ScanError::TooManyNodes { n: self.n });
        }

        // Check dimensions
        if self.adjacency.len() != self.n {
            return Err(ScanError::DimensionMismatch {
                rows: self.adjacency.len(),
                cols: self.n,
            });
        }
        for (i, row) in self.adjacency.iter().enumerate() {
            if row.len() != self.n {
                return Err(ScanError::DimensionMismatch {
                    rows: self.n,
                    cols: row.len(),
                });
            }
            // Check self-loops
            if row[i] != 0.0 {
                return Err(ScanError::SelfLoop { node: i + 1 });
            }
        }

        // Check symmetry and binary weights
        for i in 0..self.n {
            for j in 0..self.n {
                let val = self.adjacency[i][j];
                if val != 0.0 && val != 1.0 {
                    return Err(ScanError::NonBinaryWeight {
                        i: i + 1,
                        j: j + 1,
                        weight: val,
                    });
                }
                if (self.adjacency[i][j] - self.adjacency[j][i]).abs() > 1e-10 {
                    return Err(ScanError::NotSymmetric { i: i + 1, j: j + 1 });
                }
            }
        }

        Ok(())
    }

    /// Count the number of edges (upper triangle only).
    pub fn edge_count(&self) -> usize {
        let mut count = 0;
        for i in 0..self.n {
            for j in (i + 1)..self.n {
                if self.adjacency[i][j] > 0.5 {
                    count += 1;
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_n_gt_8() {
        let matrix = vec![vec![0.0; 9]; 9];
        let result = InputGraph::from_adjacency(matrix);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ScanError::TooManyNodes { n: 9 }
        ));
    }

    #[test]
    fn accept_n_eq_8() {
        let matrix = vec![vec![0.0; 8]; 8];
        let result = InputGraph::from_adjacency(matrix);
        assert!(result.is_ok());
    }

    #[test]
    fn reject_empty() {
        let matrix: Vec<Vec<f64>> = vec![];
        let result = InputGraph::from_adjacency(matrix);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScanError::EmptyGraph));
    }

    #[test]
    fn reject_non_symmetric() {
        let matrix = vec![
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
            vec![0.0, 1.0, 0.0],
        ];
        let result = InputGraph::from_adjacency(matrix);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ScanError::NotSymmetric { .. }
        ));
    }

    #[test]
    fn reject_self_loop() {
        let matrix = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0],
        ];
        let result = InputGraph::from_adjacency(matrix);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScanError::SelfLoop { .. }));
    }

    #[test]
    fn reject_non_binary() {
        let matrix = vec![vec![0.0, 0.5], vec![0.5, 0.0]];
        let result = InputGraph::from_adjacency(matrix);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ScanError::NonBinaryWeight { .. }
        ));
    }

    #[test]
    fn accept_valid_triangle() {
        let g = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)]);
        assert!(g.is_ok());
        let g = g.unwrap();
        assert_eq!(g.n, 3);
        assert_eq!(g.edge_count(), 3);
    }

    #[test]
    fn from_adj_list_works() {
        let adj_list = vec![vec![2, 3], vec![1, 3], vec![1, 2]];
        let g = InputGraph::from_adj_list(&adj_list).unwrap();
        assert_eq!(g.n, 3);
        assert_eq!(g.edge_count(), 3);
    }

    #[test]
    fn edge_count_empty() {
        let g = InputGraph::from_edges(3, &[]).unwrap();
        assert_eq!(g.edge_count(), 0);
    }
}
