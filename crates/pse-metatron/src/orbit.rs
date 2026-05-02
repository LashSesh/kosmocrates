use std::collections::HashSet;
use std::fmt::Write;

use serde::Serialize;

use crate::group::conjugate;

#[derive(Debug, Clone, Serialize)]
pub struct OrbitResult {
    pub canonical_form: Vec<Vec<f64>>,
    pub canonical_hash: String,
    pub orbit_size: usize,
}

/// Flatten a 13x13 matrix to a Vec<u64> using f64::to_bits() for exact comparison.
fn flatten_to_bits(matrix: &[Vec<f64>]) -> Vec<u64> {
    let mut flat = Vec::with_capacity(169);
    for row in matrix {
        for &val in row {
            flat.push(val.to_bits());
        }
    }
    flat
}

/// Compute hex digest of a flattened bit representation.
fn hex_digest(flat: &[u64]) -> String {
    // Simple hash: combine all bits into a hex string via a simple hash function
    // We'll use a basic approach: hash the bytes of all u64 values
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for &val in flat {
        let bytes = val.to_le_bytes();
        for &b in &bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3); // FNV prime
        }
    }
    let mut s = String::with_capacity(16);
    let _ = write!(s, "{:016x}", hash);
    s
}

/// Compute the orbit of the embedded graph under the S7 action.
/// Returns the canonical form (lexicographically smallest), its hash, and the orbit size.
pub fn compute_orbit(
    embedded: &[Vec<f64>],
    s7_matrices: &[Vec<Vec<f64>>],
) -> OrbitResult {
    let mut seen: HashSet<Vec<u64>> = HashSet::new();
    let mut min_flat: Option<Vec<u64>> = None;
    let mut min_matrix: Option<Vec<Vec<f64>>> = None;

    for p in s7_matrices {
        let transformed = conjugate(embedded, p);
        let flat = flatten_to_bits(&transformed);

        seen.insert(flat.clone());

        match &min_flat {
            None => {
                min_flat = Some(flat);
                min_matrix = Some(transformed);
            }
            Some(current_min) => {
                if flat < *current_min {
                    min_flat = Some(flat);
                    min_matrix = Some(transformed);
                }
            }
        }
    }

    let canonical = min_matrix.unwrap_or_else(|| vec![vec![0.0; 13]; 13]);
    let hash = hex_digest(min_flat.as_ref().unwrap_or(&vec![]));

    OrbitResult {
        canonical_form: canonical,
        canonical_hash: hash,
        orbit_size: seen.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::embed;
    use crate::group::{generate_s7, permutation_matrix_13};
    use crate::ingest::InputGraph;

    fn make_s7_matrices() -> Vec<Vec<Vec<f64>>> {
        generate_s7()
            .iter()
            .map(|sigma| permutation_matrix_13(sigma))
            .collect()
    }

    #[test]
    fn empty_graph_orbit_size_1() {
        let g = InputGraph::from_edges(7, &[]).unwrap();
        let e = embed(&g);
        let matrices = make_s7_matrices();
        let orbit = compute_orbit(&e, &matrices);
        assert_eq!(orbit.orbit_size, 1);
    }

    #[test]
    fn complete_k7_orbit_size_1() {
        let edges: Vec<(usize, usize)> = (1..=7)
            .flat_map(|i| ((i + 1)..=7).map(move |j| (i, j)))
            .collect();
        let g = InputGraph::from_edges(7, &edges).unwrap();
        let e = embed(&g);
        let matrices = make_s7_matrices();
        let orbit = compute_orbit(&e, &matrices);
        assert_eq!(orbit.orbit_size, 1);
    }

    #[test]
    fn canonical_hash_is_deterministic() {
        let g = InputGraph::from_edges(3, &[(1, 2), (2, 3)]).unwrap();
        let e = embed(&g);
        let matrices = make_s7_matrices();
        let orbit1 = compute_orbit(&e, &matrices);
        let orbit2 = compute_orbit(&e, &matrices);
        assert_eq!(orbit1.canonical_hash, orbit2.canonical_hash);
    }
}
