//! Convert a `ScanReport` to a `pse_types::MetatronTopologySignature`.
//!
//! This is the bridge that lets PSE crystals carry a Metatron-scaffold
//! canonical identity. The full ScanReport contains many fields that
//! are useful during the scan itself (canonical_form matrix, generator
//! permutations, traces) but unnecessary for the per-crystal
//! signature; this module extracts the small subset that constitutes
//! a *topological identity* and stores it in the lightweight
//! [`pse_types::MetatronTopologySignature`] form.

use pse_types::MetatronTopologySignature;

use crate::error::ScanResult;
use crate::ingest::InputGraph;
use crate::scan::{scan, scan_with_cache, S7Cache, ScanReport};

/// Project a full [`ScanReport`] into a lightweight
/// [`MetatronTopologySignature`] suitable for storage on every
/// `SemanticCrystal`.
pub fn signature_from_scan_report(report: &ScanReport) -> MetatronTopologySignature {
    let triangle_count = report.triangle_count as u32;
    let (contains_tetrahedron, contains_octahedron, contains_cube) = {
        let mut t = false;
        let mut o = false;
        let mut c = false;
        for class in &report.platonic_classes {
            match class.solid.as_str() {
                "tetrahedron" => t = class.is_subgraph,
                "octahedron" => o = class.is_subgraph,
                "cube" => c = class.is_subgraph,
                _ => {}
            }
        }
        (t, o, c)
    };

    MetatronTopologySignature {
        canonical_hash: report.canonical_hash.clone(),
        n: report.input_nodes as u32,
        m: report.input_edges as u32,
        orbit_size: report.orbit_size as u64,
        stabilizer_order: report.stabilizer_order as u64,
        spectrum: report.eigenvalues.clone(),
        spectral_radius: report.spectral_radius,
        algebraic_connectivity: report.algebraic_connectivity,
        graph_energy: report.graph_energy,
        spanning_tree_count: report.spanning_tree_count,
        triangle_count,
        contains_tetrahedron,
        contains_octahedron,
        contains_cube,
    }
}

/// Convenience: construct an `InputGraph` from a square 0/1 adjacency
/// matrix, run the full scan, and return the signature.
///
/// Returns `Err` if the adjacency is malformed (non-square, non-binary,
/// asymmetric) or larger than the scaffold (n > 13).
pub fn compute_signature(
    adjacency: Vec<Vec<f64>>,
    labels: Option<Vec<String>>,
) -> ScanResult<MetatronTopologySignature> {
    let _ = labels; // labels are not yet used by InputGraph::from_adjacency
    let input = InputGraph::from_adjacency(adjacency)?;
    let report = scan(input)?;
    Ok(signature_from_scan_report(&report))
}

/// Same as [`compute_signature`] but reusing a precomputed S₇ cache —
/// the right call inside hot loops where many crystals' signatures
/// are computed in succession.
pub fn compute_signature_with_cache(
    adjacency: Vec<Vec<f64>>,
    labels: Option<Vec<String>>,
    cache: &S7Cache,
) -> ScanResult<MetatronTopologySignature> {
    let _ = labels; // labels are not yet used by InputGraph::from_adjacency
    let input = InputGraph::from_adjacency(adjacency)?;
    let report = scan_with_cache(input, cache)?;
    Ok(signature_from_scan_report(&report))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k3_adjacency() -> Vec<Vec<f64>> {
        // Triangle: 3 vertices, 3 edges, every pair connected.
        vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
        ]
    }

    fn k4_adjacency() -> Vec<Vec<f64>> {
        // Tetrahedron skeleton: 4 vertices, 6 edges, every pair connected.
        let n = 4;
        let mut a = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                a[i][j] = 1.0;
                a[j][i] = 1.0;
            }
        }
        a
    }

    fn empty_adjacency(n: usize) -> Vec<Vec<f64>> {
        vec![vec![0.0; n]; n]
    }

    #[test]
    fn signature_of_triangle_has_correct_basics() {
        let sig = compute_signature(k3_adjacency(), None).unwrap();
        assert_eq!(sig.n, 3);
        assert_eq!(sig.m, 3);
        assert_eq!(sig.triangle_count, 1);
        assert!(!sig.canonical_hash.is_empty());
        assert!(sig.spectrum.len() <= 13); // input padded into 13-node scaffold
    }

    #[test]
    fn signature_detects_tetrahedron_subgraph() {
        let sig = compute_signature(k4_adjacency(), None).unwrap();
        assert!(
            sig.contains_tetrahedron,
            "K4 must report contains_tetrahedron=true"
        );
        assert_eq!(sig.n, 4);
        assert_eq!(sig.m, 6);
    }

    #[test]
    fn empty_graph_has_no_platonic_subgraph() {
        let sig = compute_signature(empty_adjacency(3), None).unwrap();
        assert!(!sig.contains_tetrahedron);
        assert!(!sig.contains_octahedron);
        assert!(!sig.contains_cube);
        assert_eq!(sig.m, 0);
        assert_eq!(sig.triangle_count, 0);
    }

    #[test]
    fn signature_is_deterministic() {
        let a = compute_signature(k3_adjacency(), None).unwrap();
        let b = compute_signature(k3_adjacency(), None).unwrap();
        assert_eq!(a.canonical_hash, b.canonical_hash);
        assert_eq!(a.orbit_size, b.orbit_size);
        assert_eq!(a.stabilizer_order, b.stabilizer_order);
    }

    #[test]
    fn orbit_stabilizer_product_equals_5040() {
        // For any input graph, |orbit| · |stabilizer| = |S₇| = 5040.
        // This is the orbit-stabilizer theorem applied to the action
        // of S₇ on the 13-node scaffold.
        let sig = compute_signature(k3_adjacency(), None).unwrap();
        assert_eq!(sig.orbit_size * sig.stabilizer_order, 5040);
    }

    #[test]
    fn cache_reuse_produces_identical_signature() {
        let cache = S7Cache::new();
        let with_cache = compute_signature_with_cache(k3_adjacency(), None, &cache).unwrap();
        let no_cache = compute_signature(k3_adjacency(), None).unwrap();
        assert_eq!(with_cache.canonical_hash, no_cache.canonical_hash);
        assert_eq!(with_cache.spectrum.len(), no_cache.spectrum.len());
    }
}
