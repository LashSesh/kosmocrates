use crate::ingest::InputGraph;

/// Embed an n-node graph into the 13-node Metatron scaffold.
/// Nodes 1..n map to scaffold nodes 1..n.
/// Scaffold nodes (n+1)..13 have no edges to the embedded graph.
/// The scaffold's own edges are NOT included.
pub fn embed(input: &InputGraph) -> Vec<Vec<f64>> {
    let mut embedded = vec![vec![0.0; 13]; 13];
    for i in 0..input.n {
        for j in 0..input.n {
            embedded[i][j] = input.adjacency[i][j];
        }
    }
    embedded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_3_in_13() {
        let g = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)]).unwrap();
        let e = embed(&g);
        assert_eq!(e.len(), 13);
        assert_eq!(e[0].len(), 13);

        // Upper-left 3x3 has the triangle
        assert_eq!(e[0][1], 1.0);
        assert_eq!(e[1][0], 1.0);
        assert_eq!(e[1][2], 1.0);
        assert_eq!(e[2][1], 1.0);
        assert_eq!(e[0][2], 1.0);
        assert_eq!(e[2][0], 1.0);

        // Everything else is zero
        for i in 3..13 {
            for j in 0..13 {
                assert_eq!(e[i][j], 0.0, "Non-zero at ({}, {})", i, j);
            }
        }
        for i in 0..3 {
            for j in 3..13 {
                assert_eq!(e[i][j], 0.0, "Non-zero at ({}, {})", i, j);
            }
        }
    }

    #[test]
    fn embed_preserves_adjacency() {
        let g = InputGraph::from_edges(4, &[(1, 2), (3, 4)]).unwrap();
        let e = embed(&g);
        assert_eq!(e[0][1], 1.0);
        assert_eq!(e[1][0], 1.0);
        assert_eq!(e[2][3], 1.0);
        assert_eq!(e[3][2], 1.0);
        assert_eq!(e[0][2], 0.0);
    }
}
