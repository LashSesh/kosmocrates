//! Metatron scaffold geometry — 1 center + 12 cuboctahedron vertices.
//!
//! # N1 v2 geometry correction
//!
//! Earlier revisions split the outer shell into "6 hexagon + 6 cube" — a
//! fiction that doesn't correspond to any Platonic or Archimedean solid.
//! The twelve outer nodes in fact form a single cuboctahedron (one of the
//! 13 Archimedean solids):
//!
//!   * **12 vertices** at every permutation of `(±1, ±1, 0)` — the edge
//!     midpoints of a cube with corners at `(±1, ±1, ±1)`.
//!   * **24 edges** of equal length (= √2 at circumradius √2).
//!   * **14 faces**: 8 equilateral triangles + 6 squares.
//!   * Symmetry group: O_h (order 48).
//!
//! Projected along the body diagonal (1,1,1) the cuboctahedron appears as
//! two concentric hexagons offset by 30° — the classical 2D Metatron-cube
//! symbol. The distinction between "hexagon" and "cube" nodes in earlier
//! versions was therefore a projection artefact, not a real geometric
//! split.
//!
//! The algebraic layer (S_7 action, orbit/stabilizer, spectrum, catalog
//! hashes, OEIS sequences) is coordinate-independent and remains
//! byte-for-byte unchanged under this correction.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Node {
    pub index: usize,
    pub label: &'static str,
    pub node_type: &'static str,
    pub coords: [f64; 3],
}

impl Node {
    pub fn distance_to(&self, other: &Node) -> f64 {
        let diff = [
            self.coords[0] - other.coords[0],
            self.coords[1] - other.coords[1],
            self.coords[2] - other.coords[2],
        ];
        (diff[0].powi(2) + diff[1].powi(2) + diff[2].powi(2)).sqrt()
    }
}

/// The 13 canonical nodes of the Metatron scaffold (1-indexed).
///
/// `node_type` is either `"center"` (node 1) or `"vertex"` (nodes 2..=13).
/// The old "hexagon"/"cube" split is gone: every outer node is just a
/// vertex of the cuboctahedron.
pub fn canonical_nodes() -> Vec<Node> {
    vec![
        // Center.
        Node { index: 1, label: "C", node_type: "center", coords: [0.0, 0.0, 0.0] },

        // 12 cuboctahedron vertices: all permutations of (±1, ±1, 0).
        // Grouped by the zero-coordinate plane for readability.
        //   xy-plane (z = 0)
        Node { index: 2,  label: "V1",  node_type: "vertex", coords: [ 1.0,  1.0, 0.0] },
        Node { index: 3,  label: "V2",  node_type: "vertex", coords: [ 1.0, -1.0, 0.0] },
        Node { index: 4,  label: "V3",  node_type: "vertex", coords: [-1.0,  1.0, 0.0] },
        Node { index: 5,  label: "V4",  node_type: "vertex", coords: [-1.0, -1.0, 0.0] },
        //   xz-plane (y = 0)
        Node { index: 6,  label: "V5",  node_type: "vertex", coords: [ 1.0,  0.0,  1.0] },
        Node { index: 7,  label: "V6",  node_type: "vertex", coords: [ 1.0,  0.0, -1.0] },
        Node { index: 8,  label: "V7",  node_type: "vertex", coords: [-1.0,  0.0,  1.0] },
        Node { index: 9,  label: "V8",  node_type: "vertex", coords: [-1.0,  0.0, -1.0] },
        //   yz-plane (x = 0)
        Node { index: 10, label: "V9",  node_type: "vertex", coords: [ 0.0,  1.0,  1.0] },
        Node { index: 11, label: "V10", node_type: "vertex", coords: [ 0.0,  1.0, -1.0] },
        Node { index: 12, label: "V11", node_type: "vertex", coords: [ 0.0, -1.0,  1.0] },
        Node { index: 13, label: "V12", node_type: "vertex", coords: [ 0.0, -1.0, -1.0] },
    ]
}

/// Canonical edges of the Metatron scaffold (1-indexed).
///
/// Computed from the canonical nodes — no hand-coded edge list:
///   * 12 center→vertex rays (center → every cuboctahedron vertex).
///   * The cuboctahedron's 24 edges, obtained by keeping exactly those
///     pairs of vertices whose Euclidean distance equals the cuboctahedron
///     edge length = √2 (= sqrt of the smallest positive pairwise
///     squared-distance among the outer 12, which is 2).
///
/// Total: 12 + 24 = 36 edges.
pub fn canonical_edges() -> Vec<(usize, usize)> {
    let nodes = canonical_nodes();
    let mut edges: Vec<(usize, usize)> = Vec::with_capacity(36);

    // 12 center → vertex rays.
    for v in 2..=13 {
        edges.push((1, v));
    }

    // Cuboctahedron edges: pick every pair of outer vertices whose squared
    // distance is exactly 2 (= √2 edge length at circumradius √2).
    for i in 2..=13 {
        for j in (i + 1)..=13 {
            let a = &nodes[i - 1];
            let b = &nodes[j - 1];
            let d2 = (a.coords[0] - b.coords[0]).powi(2)
                + (a.coords[1] - b.coords[1]).powi(2)
                + (a.coords[2] - b.coords[2]).powi(2);
            if (d2 - 2.0).abs() < 1e-10 {
                edges.push((i, j));
            }
        }
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- basic shape ----

    #[test]
    fn canonical_nodes_count() {
        assert_eq!(canonical_nodes().len(), 13);
    }

    #[test]
    fn canonical_nodes_indices() {
        let nodes = canonical_nodes();
        for (i, node) in nodes.iter().enumerate() {
            assert_eq!(node.index, i + 1);
        }
    }

    #[test]
    fn center_at_origin() {
        let nodes = canonical_nodes();
        assert_eq!(nodes[0].coords, [0.0, 0.0, 0.0]);
        assert_eq!(nodes[0].label, "C");
        assert_eq!(nodes[0].node_type, "center");
    }

    // ---- N1 v2: cuboctahedron invariants ----

    #[test]
    fn cuboctahedron_vertex_count() {
        let nodes = canonical_nodes();
        let vertices: Vec<_> = nodes
            .iter()
            .filter(|n| n.node_type == "vertex")
            .collect();
        assert_eq!(vertices.len(), 12);
    }

    #[test]
    fn cuboctahedron_all_on_sphere() {
        let nodes = canonical_nodes();
        let expected_r = (2.0_f64).sqrt();
        for node in nodes.iter().filter(|n| n.node_type == "vertex") {
            let r =
                (node.coords[0].powi(2) + node.coords[1].powi(2) + node.coords[2].powi(2)).sqrt();
            assert!(
                (r - expected_r).abs() < 1e-10,
                "{} has radius {} instead of {}",
                node.label,
                r,
                expected_r
            );
        }
    }

    #[test]
    fn each_vertex_has_one_zero_coordinate() {
        // Permutations of (±1, ±1, 0) ⇒ exactly one zero coordinate each.
        let nodes = canonical_nodes();
        for node in nodes.iter().filter(|n| n.node_type == "vertex") {
            let zeros = node.coords.iter().filter(|&&c| c.abs() < 1e-10).count();
            assert_eq!(
                zeros, 1,
                "{} has {} zero coordinates instead of 1",
                node.label, zeros
            );
        }
    }

    #[test]
    fn cuboctahedron_edge_count() {
        let edges = canonical_edges();
        let center_edges = edges.iter().filter(|&&(a, b)| a == 1 || b == 1).count();
        let body_edges = edges.iter().filter(|&&(a, b)| a > 1 && b > 1).count();
        assert_eq!(center_edges, 12, "center → 12 vertex rays");
        assert_eq!(body_edges, 24, "cuboctahedron has 24 edges");
        assert_eq!(edges.len(), 36, "total = 36");
    }

    #[test]
    fn cuboctahedron_equal_edge_length() {
        // The N1 v2 spec text mentions "edge length 2" but at the stated
        // coordinates (±1, ±1, 0) the circumradius is √2 and the edge
        // length is √2 (not 2). We implement the correct geometry and
        // assert the correct √2.
        let nodes = canonical_nodes();
        let edges = canonical_edges();
        let body_edges: Vec<_> = edges
            .iter()
            .filter(|&&(a, b)| a > 1 && b > 1)
            .copied()
            .collect();
        let expected = (2.0_f64).sqrt();
        for (a, b) in body_edges {
            let len = nodes[a - 1].distance_to(&nodes[b - 1]);
            assert!(
                (len - expected).abs() < 1e-10,
                "edge ({},{}) has length {} instead of {}",
                a,
                b,
                len,
                expected
            );
        }
    }

    #[test]
    fn cuboctahedron_vertex_degree_4() {
        let edges = canonical_edges();
        for idx in 2..=13 {
            let degree = edges
                .iter()
                .filter(|&&(a, b)| a > 1 && b > 1)
                .filter(|&&(a, b)| a == idx || b == idx)
                .count();
            assert_eq!(
                degree, 4,
                "vertex {} has cuboctahedron-degree {} instead of 4",
                idx, degree
            );
        }
    }

    #[test]
    fn no_coordinate_coincidence() {
        let nodes = canonical_nodes();
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let d = nodes[i].distance_to(&nodes[j]);
                assert!(
                    d > 0.01,
                    "nodes {} and {} coincide at distance {}",
                    nodes[i].label,
                    nodes[j].label,
                    d
                );
            }
        }
    }

    #[test]
    fn cuboctahedron_faces_eight_triangles_six_squares() {
        // The cuboctahedron has 8 triangular faces and 6 square faces.
        // Triangle face = a 3-clique in the outer-vertex subgraph.
        // Square  face = a chord-less 4-cycle in that subgraph.
        let edges = canonical_edges();
        let body: std::collections::HashSet<(usize, usize)> = edges
            .iter()
            .copied()
            .filter(|&(a, b)| a > 1 && b > 1)
            .map(|(a, b)| if a < b { (a, b) } else { (b, a) })
            .collect();
        let has_edge = |a: usize, b: usize| -> bool {
            let p = if a < b { (a, b) } else { (b, a) };
            body.contains(&p)
        };

        // Triangles
        let mut triangles = 0usize;
        for i in 2..=13 {
            for j in (i + 1)..=13 {
                for k in (j + 1)..=13 {
                    if has_edge(i, j) && has_edge(j, k) && has_edge(i, k) {
                        triangles += 1;
                    }
                }
            }
        }
        assert_eq!(triangles, 8, "cuboctahedron has 8 triangle faces");

        // Squares: 4-cycles without chord.
        let mut squares = 0usize;
        for a in 2..=13 {
            for b in 2..=13 {
                if b == a || !has_edge(a, b) {
                    continue;
                }
                for c in 2..=13 {
                    if c == a || c == b || !has_edge(b, c) || has_edge(a, c) {
                        continue;
                    }
                    for d in 2..=13 {
                        if d == a || d == b || d == c {
                            continue;
                        }
                        if has_edge(c, d) && has_edge(d, a) && !has_edge(b, d) {
                            squares += 1;
                        }
                    }
                }
            }
        }
        // Each square is counted 8× (4 rotations × 2 directions).
        assert_eq!(squares / 8, 6, "cuboctahedron has 6 square faces");
    }
}
