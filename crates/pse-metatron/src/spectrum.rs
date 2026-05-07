use nalgebra::DMatrix;
use serde::Serialize;

use crate::complement::count_triangles;

#[derive(Debug, Clone, Serialize)]
pub struct SpectrumResult {
    pub eigenvalues: Vec<f64>,
    pub spectral_radius: f64,
    pub algebraic_connectivity: f64,
    pub triangle_count: usize,
    /// Graph energy E(G) = Σ|λᵢ|.
    pub graph_energy: f64,
    /// Sorted Laplacian eigenvalues (non-negative, smallest first).
    pub laplacian_eigenvalues: Vec<f64>,
    /// Number of spanning trees τ(G), via Kirchhoff's Matrix-Tree theorem:
    /// τ(G) = (1/n) · ∏_{i≥2} μᵢ(L).  Zero iff G is disconnected.
    pub spanning_tree_count: u64,
}

/// Compute the spectrum of the n×n active submatrix of the embedded graph.
pub fn compute_spectrum(embedded: &[Vec<f64>], n: usize) -> SpectrumResult {
    // Extract n×n submatrix (active zone)
    let mut sub = DMatrix::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            sub[(i, j)] = embedded[i][j];
        }
    }

    // Eigenvalues of the symmetric adjacency matrix
    let eigen = sub.symmetric_eigen();
    let mut eigenvalues: Vec<f64> = eigen
        .eigenvalues
        .iter()
        .map(|&v| (v * 1e10).round() / 1e10)
        .collect();
    eigenvalues.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let spectral_radius = eigenvalues.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);

    // Laplacian L = D - A
    let mut laplacian = DMatrix::zeros(n, n);
    for i in 0..n {
        let degree: f64 = (0..n).map(|j| embedded[i][j]).sum();
        laplacian[(i, i)] = degree;
        for j in 0..n {
            laplacian[(i, j)] -= embedded[i][j];
        }
    }
    let lap_eigen = laplacian.symmetric_eigen();
    let mut lap_vals: Vec<f64> = lap_eigen.eigenvalues.iter().copied().collect();
    lap_vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let algebraic_connectivity = if n >= 2 { lap_vals[1] } else { 0.0 };

    // Matrix-Tree theorem: τ(G) = (1/n) · ∏_{i≥2} μᵢ(L).  For disconnected
    // graphs the second-smallest Laplacian eigenvalue is ≈ 0, so the
    // product collapses to 0 — which matches the combinatorial definition
    // (no spanning tree exists).
    let spanning_tree_count: u64 = if n <= 1 {
        1
    } else {
        let product: f64 = lap_vals.iter().skip(1).copied().product();
        let tau = product / n as f64;
        tau.round().max(0.0) as u64
    };

    let laplacian_eigenvalues: Vec<f64> =
        lap_vals.iter().map(|v| (v * 1e10).round() / 1e10).collect();

    let triangle_count = count_triangles(embedded);
    let graph_energy = (eigenvalues.iter().map(|v| v.abs()).sum::<f64>() * 1e6).round() / 1e6;

    SpectrumResult {
        eigenvalues,
        spectral_radius,
        algebraic_connectivity,
        triangle_count,
        graph_energy,
        laplacian_eigenvalues,
        spanning_tree_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::embed;
    use crate::ingest::InputGraph;

    #[test]
    fn k3_eigenvalues() {
        let g = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)]).unwrap();
        let e = embed(&g);
        let spec = compute_spectrum(&e, 3);
        // K3 eigenvalues: [-1, -1, 2]
        assert!((spec.eigenvalues[0] - (-1.0)).abs() < 1e-8);
        assert!((spec.eigenvalues[1] - (-1.0)).abs() < 1e-8);
        assert!((spec.eigenvalues[2] - 2.0).abs() < 1e-8);
    }

    #[test]
    fn empty_graph_eigenvalues() {
        let g = InputGraph::from_edges(3, &[]).unwrap();
        let e = embed(&g);
        let spec = compute_spectrum(&e, 3);
        for &ev in &spec.eigenvalues {
            assert!(ev.abs() < 1e-8);
        }
        assert_eq!(spec.spectral_radius, 0.0);
    }

    #[test]
    fn k3_spectral_radius() {
        let g = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)]).unwrap();
        let e = embed(&g);
        let spec = compute_spectrum(&e, 3);
        assert!((spec.spectral_radius - 2.0).abs() < 1e-8);
    }
}
