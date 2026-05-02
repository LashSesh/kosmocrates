//! Classification of the input graph against the five Platonic solids'
//! skeleton graphs.
//!
//! # N1 v2: algorithmic subgraph-iso detection
//!
//! Earlier revisions hard-coded edge sets of the Platonic solids on
//! specific scaffold node indices. That approach was brittle and
//! semantically wrong — it checked "is the input a subset of this
//! specific labelled placement", which almost never matched any catalog
//! entry. (At N1 we cleaned it up partially by emptying the tetrahedron
//! and cube sets; at v2 we go all the way.)
//!
//! New semantics: for each Platonic solid `P`, the classifier reports
//!
//!   is_subgraph   : the input graph G contains the skeleton of P as a
//!                   (labeled) subgraph, i.e. ∃ injection φ: V(P) ↪ V(G)
//!                   with `(u,v) ∈ E(P) ⇒ (φu, φv) ∈ E(G)`.
//!   is_isomorphic : the input graph IS the skeleton of P (same n, same
//!                   m, graph-isomorphic).
//!   edge_coverage : 1.0 if is_isomorphic, else is_subgraph as {1,0}.
//!                   (The field is kept for JSON-API back-compat.)
//!
//! The 5 Platonic solids' vertex counts: 4 / 8 / 6 / 20 / 12. Only the
//! first three fit into n ≤ 8 graphs, so `dodecahedron` and `icosahedron`
//! always report false inside the periodic-table catalog.

use std::collections::HashSet;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PlatonicClass {
    pub solid: String,
    pub is_subgraph: bool,
    pub is_isomorphic: bool,
    pub edge_coverage: f64,
}

// ---------------------------------------------------------------------------
// Edge-set models for each Platonic solid, on their canonical 0-indexed
// vertex sets. Source data, no scaffold dependency.
// ---------------------------------------------------------------------------

fn tetrahedron_edges() -> Vec<(usize, usize)> {
    vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
}

fn octahedron_edges() -> Vec<(usize, usize)> {
    // K_{2,2,2}: pairs {0,1}, {2,3}, {4,5} are the three "antipodal" classes.
    let mut e = Vec::new();
    for i in 0..6 {
        for j in (i + 1)..6 {
            let same_class = (i / 2) == (j / 2);
            if !same_class {
                e.push((i, j));
            }
        }
    }
    e
}

fn cube_edges() -> Vec<(usize, usize)> {
    // Q_3: vertices 0..7 labelled by their 3-bit binary; edges at Hamming
    // distance 1.
    let mut e = Vec::new();
    for i in 0..8 {
        for j in (i + 1)..8 {
            let diff = (i as u32) ^ (j as u32);
            if diff.count_ones() == 1 {
                e.push((i, j));
            }
        }
    }
    e
}

// ---------------------------------------------------------------------------
// Subgraph-isomorphism: does the input n-node graph contain the Platonic
// solid `P` as a labelled subgraph?
// ---------------------------------------------------------------------------

fn has_subgraph_iso(
    input_adj: &[Vec<f64>],
    input_n: usize,
    platonic_edges: &[(usize, usize)],
    platonic_n: usize,
) -> bool {
    if platonic_n > input_n {
        return false;
    }
    let mut mapping = vec![usize::MAX; platonic_n];
    let mut used = vec![false; input_n];
    search(input_adj, platonic_edges, 0, platonic_n, &mut mapping, &mut used)
}

fn search(
    adj: &[Vec<f64>],
    platonic_edges: &[(usize, usize)],
    k: usize,
    p_n: usize,
    mapping: &mut [usize],
    used: &mut [bool],
) -> bool {
    if k == p_n {
        return platonic_edges
            .iter()
            .all(|&(u, v)| adj[mapping[u]][mapping[v]] > 0.5);
    }
    for v in 0..used.len() {
        if used[v] {
            continue;
        }
        // Partial check: every platonic edge (u, k) with u < k must already
        // be present in the input when we commit vertex v to slot k.
        let ok = platonic_edges
            .iter()
            .filter(|&&(a, b)| (a == k && b < k) || (b == k && a < k))
            .all(|&(a, b)| {
                let other = if a == k { mapping[b] } else { mapping[a] };
                adj[other][v] > 0.5
            });
        if !ok {
            continue;
        }
        mapping[k] = v;
        used[v] = true;
        if search(adj, platonic_edges, k + 1, p_n, mapping, used) {
            return true;
        }
        used[v] = false;
        mapping[k] = usize::MAX;
    }
    false
}

fn is_isomorphic_to_platonic(
    input_adj: &[Vec<f64>],
    input_n: usize,
    input_m: usize,
    platonic_edges: &[(usize, usize)],
    platonic_n: usize,
) -> bool {
    input_n == platonic_n
        && input_m == platonic_edges.len()
        && has_subgraph_iso(input_adj, input_n, platonic_edges, platonic_n)
}

fn input_edge_count(embedded: &[Vec<f64>], n: usize) -> usize {
    let mut m = 0;
    for i in 0..n {
        for j in (i + 1)..n {
            if embedded[i][j] > 0.5 {
                m += 1;
            }
        }
    }
    m
}

/// Classify the input graph against each of the five Platonic solids.
pub fn classify_platonic(embedded: &[Vec<f64>], n: usize) -> Vec<PlatonicClass> {
    let tet = tetrahedron_edges();
    let oct = octahedron_edges();
    let cub = cube_edges();
    // dodecahedron / icosahedron: 20 / 12 vertices — can't fit into n ≤ 8.
    // We still emit rows so the JSON shape stays stable.
    let m = input_edge_count(embedded, n);

    let row = |name: &str, p_edges: &[(usize, usize)], p_n: usize| -> PlatonicClass {
        let is_sub = has_subgraph_iso(embedded, n, p_edges, p_n);
        let is_iso =
            is_sub && is_isomorphic_to_platonic(embedded, n, m, p_edges, p_n);
        PlatonicClass {
            solid: name.into(),
            is_subgraph: is_sub,
            is_isomorphic: is_iso,
            edge_coverage: if is_iso { 1.0 } else if is_sub { 1.0 } else { 0.0 },
        }
    };

    let mut out = vec![
        row("tetrahedron", &tet, 4),
        row("octahedron", &oct, 6),
        row("cube", &cub, 8),
        PlatonicClass {
            solid: "dodecahedron".into(),
            is_subgraph: false,
            is_isomorphic: false,
            edge_coverage: 0.0,
        },
        PlatonicClass {
            solid: "icosahedron".into(),
            is_subgraph: false,
            is_isomorphic: false,
            edge_coverage: 0.0,
        },
    ];
    out.sort_by(|a, b| a.solid.cmp(&b.solid));
    out
}

/// Does the cuboctahedron scaffold itself contain the Platonic solid as a
/// subgraph? Runs the same subgraph-iso routine against the canonical
/// scaffold edges (outer 12 nodes, 24 edges).
pub fn scaffold_contains_platonic() -> Vec<PlatonicClass> {
    // Build a 12×12 adjacency from the scaffold's cuboctahedron-only edges
    // (ignore the 12 center rays: they are not part of the Platonic
    // skeleton question).
    use crate::geometry::canonical_edges;
    let mut adj = vec![vec![0.0f64; 12]; 12];
    for (a, b) in canonical_edges() {
        if a > 1 && b > 1 {
            let i = a - 2;
            let j = b - 2;
            adj[i][j] = 1.0;
            adj[j][i] = 1.0;
        }
    }
    classify_platonic(&adj, 12)
}

// Silence unused warning in case HashSet isn't needed at call sites.
#[allow(dead_code)]
fn _reserved() -> HashSet<(usize, usize)> {
    HashSet::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::embed;
    use crate::ingest::InputGraph;

    #[test]
    fn empty_graph_is_subgraph_of_no_solid() {
        // Strict semantics: only a graph that actually contains all required
        // edges is a subgraph. The empty graph on 3 nodes contains none.
        let g = InputGraph::from_edges(3, &[]).unwrap();
        let e = embed(&g);
        let classes = classify_platonic(&e, 3);
        for c in &classes {
            assert!(!c.is_subgraph, "{} should NOT contain empty graph", c.solid);
        }
    }

    #[test]
    fn k4_is_tetrahedron() {
        let g =
            InputGraph::from_edges(4, &[(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)])
                .unwrap();
        let e = embed(&g);
        let classes = classify_platonic(&e, 4);
        let tet = classes.iter().find(|c| c.solid == "tetrahedron").unwrap();
        assert!(tet.is_subgraph);
        assert!(tet.is_isomorphic);
    }

    #[test]
    fn k5_contains_tetrahedron_but_is_not() {
        let edges: Vec<(usize, usize)> =
            (1..=5).flat_map(|i| ((i + 1)..=5).map(move |j| (i, j))).collect();
        let g = InputGraph::from_edges(5, &edges).unwrap();
        let e = embed(&g);
        let classes = classify_platonic(&e, 5);
        let tet = classes.iter().find(|c| c.solid == "tetrahedron").unwrap();
        assert!(tet.is_subgraph);
        assert!(!tet.is_isomorphic);
    }

    #[test]
    fn k333_is_not_octahedron() {
        // K_{3,3}: 3-3-bipartite, 9 edges — NOT the octahedron (K_{2,2,2}).
        let edges: Vec<(usize, usize)> = (1..=3)
            .flat_map(|i| (4..=6).map(move |j| (i, j)))
            .collect();
        let g = InputGraph::from_edges(6, &edges).unwrap();
        let e = embed(&g);
        let classes = classify_platonic(&e, 6);
        let oct = classes.iter().find(|c| c.solid == "octahedron").unwrap();
        assert!(!oct.is_isomorphic);
    }

    #[test]
    fn octahedron_k222_recognized() {
        // K_{2,2,2} = K_6 minus a perfect matching. Build it: remove edges
        // (1,2), (3,4), (5,6).
        let mut edges = Vec::new();
        for i in 1..=6u32 {
            for j in (i + 1)..=6u32 {
                let forbidden = [(1u32, 2u32), (3, 4), (5, 6)];
                if !forbidden.contains(&(i, j)) {
                    edges.push((i as usize, j as usize));
                }
            }
        }
        assert_eq!(edges.len(), 12);
        let g = InputGraph::from_edges(6, &edges).unwrap();
        let e = embed(&g);
        let classes = classify_platonic(&e, 6);
        let oct = classes.iter().find(|c| c.solid == "octahedron").unwrap();
        assert!(oct.is_subgraph);
        assert!(oct.is_isomorphic);
    }

    #[test]
    fn large_platonic_never_match_in_catalog_range() {
        // dodecahedron (20 nodes) and icosahedron (12 nodes) cannot fit in
        // n ≤ 8, so they never match any catalog graph.
        let edges: Vec<(usize, usize)> = (1..=7)
            .flat_map(|i| ((i + 1)..=7).map(move |j| (i, j)))
            .collect();
        let g = InputGraph::from_edges(7, &edges).unwrap();
        let e = embed(&g);
        let classes = classify_platonic(&e, 7);
        for name in &["dodecahedron", "icosahedron"] {
            let c = classes.iter().find(|c| &c.solid == name).unwrap();
            assert!(!c.is_subgraph);
            assert!(!c.is_isomorphic);
        }
    }

    #[test]
    fn scaffold_platonic_classification_is_empty() {
        // The cuboctahedron graph is 4-regular, 12 vertices, 24 edges, girth
        // 3. Its neighbourhood structure: the 4 neighbours of any vertex
        // form a *matching* (2 disjoint edges), not a 4-clique. Hence the
        // scaffold contains NO K₄ subgraph, and therefore no tetrahedron,
        // no K_{2,2,2} (octahedron), and no 3-regular-bipartite-8-node
        // cube: each of those would force at least one additional triangle
        // structure beyond the local matching.
        //
        // Dodecahedron / icosahedron: too big anyway.
        //
        // So every Platonic subgraph-iso check on the 12-vertex cuboctahedron
        // graph returns false. Documenting that here as a invariant.
        let classes = scaffold_contains_platonic();
        for c in &classes {
            assert!(
                !c.is_subgraph,
                "cuboctahedron contains NO {} subgraph (local neighbourhood is a matching)",
                c.solid
            );
        }
    }
}
