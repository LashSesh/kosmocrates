/// Complement graph: `Ā[i][j] = 1 - A[i][j]` for `i ≠ j`, `Ā[i][i] = 0`.
/// Only operates on the n×n active zone within the 13×13 matrix.
pub fn complement(a: &[Vec<f64>], n: usize) -> Vec<Vec<f64>> {
    let mut result = vec![vec![0.0; 13]; 13];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                result[i][j] = 1.0 - a[i][j];
            }
        }
    }
    result
}

/// Square-matrix multiplication (size inferred from input). Works for any
/// dimension — originally hard-coded to 13×13 for the Metatron embedding,
/// now generalized so the n = 8 catalog path can reuse it.
pub fn mat_mul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let mut result = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}

/// A^k via repeated multiplication.
pub fn mat_pow(a: &[Vec<f64>], k: u32) -> Vec<Vec<f64>> {
    let n = a.len();
    if k == 0 {
        let mut id = vec![vec![0.0; n]; n];
        for i in 0..n {
            id[i][i] = 1.0;
        }
        return id;
    }
    let mut result = a.to_vec();
    for _ in 1..k {
        result = mat_mul(&result, a);
    }
    result
}

/// Count triangles: tr(A³) / 6
pub fn count_triangles(a: &[Vec<f64>]) -> usize {
    let n = a.len();
    let a3 = mat_pow(a, 3);
    let tr: f64 = (0..n).map(|i| a3[i][i]).sum();
    (tr / 6.0).round() as usize
}

/// Trace of a matrix.
pub fn trace(a: &[Vec<f64>]) -> f64 {
    (0..a.len()).map(|i| a[i][i]).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::embed;
    use crate::ingest::InputGraph;

    #[test]
    fn complement_of_k3_is_empty() {
        let g = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)]).unwrap();
        let e = embed(&g);
        let c = complement(&e, 3);
        // All edges in 3x3 active zone should be 0
        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(c[i][j], 0.0, "Non-zero at ({}, {})", i, j);
            }
        }
    }

    #[test]
    fn triangle_count_k3() {
        let g = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)]).unwrap();
        let e = embed(&g);
        assert_eq!(count_triangles(&e), 1);
    }

    #[test]
    fn triangle_count_path() {
        let g = InputGraph::from_edges(3, &[(1, 2), (2, 3)]).unwrap();
        let e = embed(&g);
        assert_eq!(count_triangles(&e), 0);
    }

    #[test]
    fn trace_of_identity() {
        let mut id = vec![vec![0.0; 13]; 13];
        for i in 0..13 {
            id[i][i] = 1.0;
        }
        assert_eq!(trace(&id), 13.0);
    }

    #[test]
    fn mat_pow_0_is_identity() {
        let a = vec![vec![0.0; 13]; 13];
        let result = mat_pow(&a, 0);
        for i in 0..13 {
            assert_eq!(result[i][i], 1.0);
        }
    }
}
