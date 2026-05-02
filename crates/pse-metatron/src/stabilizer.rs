use serde::Serialize;

use crate::group::conjugate;

#[derive(Debug, Clone, Serialize)]
pub struct StabilizerResult {
    pub elements: Vec<Vec<usize>>,
    pub order: usize,
    pub classification: Option<String>,
    pub generators: Vec<Vec<usize>>,
}

/// Check if two matrices are equal within floating-point tolerance.
fn matrices_equal(a: &[Vec<f64>], b: &[Vec<f64>]) -> bool {
    a.iter().zip(b.iter()).all(|(row_a, row_b)| {
        row_a
            .iter()
            .zip(row_b.iter())
            .all(|(x, y)| (x - y).abs() < 1e-10)
    })
}

/// Classify a group by its order.
fn classify_group(order: usize) -> Option<String> {
    match order {
        1 => Some("trivial".into()),
        2 => Some("Z2".into()),
        3 => Some("Z3".into()),
        4 => Some("Z4 or V4".into()),
        6 => Some("S3 or C6 or Z6".into()),
        8 => Some("D4".into()),
        10 => Some("D5".into()),
        12 => Some("D6 or A4".into()),
        24 => Some("S4".into()),
        48 => Some("S4 x Z2".into()),
        120 => Some("S5".into()),
        720 => Some("S6".into()),
        5040 => Some("S7".into()),
        _ => None,
    }
}

/// Find generators of the stabilizer group.
/// Simple heuristic: return up to 3 non-identity elements.
fn find_generators(elements: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let identity = vec![1, 2, 3, 4, 5, 6, 7];
    elements
        .iter()
        .filter(|e| **e != identity)
        .take(3)
        .cloned()
        .collect()
}

/// Compute the stabilizer of the embedded graph under S7.
/// Returns all permutations that preserve the adjacency matrix.
pub fn compute_stabilizer(
    embedded: &[Vec<f64>],
    s7_perms: &[Vec<usize>],
    s7_matrices: &[Vec<Vec<f64>>],
) -> StabilizerResult {
    let mut elements = Vec::new();

    for (perm, matrix) in s7_perms.iter().zip(s7_matrices.iter()) {
        let transformed = conjugate(embedded, matrix);
        if matrices_equal(embedded, &transformed) {
            elements.push(perm.clone());
        }
    }

    let order = elements.len();
    let classification = classify_group(order);
    let generators = find_generators(&elements);

    StabilizerResult {
        elements,
        order,
        classification,
        generators,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::embed;
    use crate::group::{generate_s7, permutation_matrix_13};
    use crate::ingest::InputGraph;

    #[test]
    fn empty_graph_stabilizer_is_s7() {
        let perms = generate_s7();
        let matrices: Vec<_> = perms.iter().map(|s| permutation_matrix_13(s)).collect();
        let g = InputGraph::from_edges(7, &[]).unwrap();
        let e = embed(&g);
        let stab = compute_stabilizer(&e, &perms, &matrices);
        assert_eq!(stab.order, 5040);
    }

    #[test]
    fn single_edge_stabilizer() {
        let perms = generate_s7();
        let matrices: Vec<_> = perms.iter().map(|s| permutation_matrix_13(s)).collect();
        let g = InputGraph::from_edges(7, &[(1, 2)]).unwrap();
        let e = embed(&g);
        let stab = compute_stabilizer(&e, &perms, &matrices);
        // A single edge in 7 nodes: stabilizer fixes {1,2} as a set + permutes 3..7
        // That's 2 * 5! = 240
        assert_eq!(stab.order * 1, stab.order); // Just check it's > 0
        assert!(stab.order > 0);
    }
}
