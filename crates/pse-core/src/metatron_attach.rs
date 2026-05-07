//! Strand O.3: attach a Metatron signature to every crystal whose
//! region is small enough to scan.
//!
//! The Metatron scaffold scans graphs of size ≤ 13 (1 centre + 12
//! cuboctahedron vertices), and the canonical S₇ orbit cache is
//! optimised for n ≤ 7. We adopt **n ≤ 7** as the safe upper bound:
//! every crystal whose `region.len() ≤ 7` gets its signature
//! computed; larger regions stay with `metatron_signature: None` and
//! are scored via the legacy 8-axis topology signature.
//!
//! Determinism: the scan is fully deterministic, so the signature is
//! a deterministic function of (graph state, region). Inv I4 holds
//! through the attachment.

use std::sync::OnceLock;

use pse_graph::PersistentGraph;
use pse_metatron::S7Cache;
use pse_types::{MetatronTopologySignature, SemanticCrystal, VertexId};

/// Process-global S₇ cache, lazily initialised on first use.
///
/// Building the cache materialises 5040 permutations and 5040
/// 13×13 permutation matrices (≈ 5 s on a typical workstation).
/// Once built, every `signature_for_region` call reuses it, so the
/// per-tick cost of attaching a signature drops to milliseconds.
fn global_s7_cache() -> &'static S7Cache {
    static CACHE: OnceLock<S7Cache> = OnceLock::new();
    CACHE.get_or_init(S7Cache::new)
}

/// Convenience wrapper around [`signature_for_region`] using the
/// process-global S₇ cache. The first call lazy-initialises the
/// cache; subsequent calls reuse it.
pub fn signature_for_region_global(
    graph: &PersistentGraph,
    region: &[VertexId],
) -> Option<MetatronTopologySignature> {
    signature_for_region(graph, region, global_s7_cache())
}

/// Convenience wrapper around [`attach_signature`] using the
/// process-global S₇ cache.
pub fn attach_signature_global(crystal: &mut SemanticCrystal, graph: &PersistentGraph) {
    attach_signature(crystal, graph, global_s7_cache());
}

/// Cap on `region.len()` for which we compute the signature.
/// The S₇ orbit cache covers exactly this range; larger regions
/// would need the general S_n scaling path which is significantly
/// more expensive.
pub const METATRON_REGION_CAP: usize = 7;

/// Build the induced adjacency matrix of the subgraph spanned by the
/// vertex IDs in `region`, treating any edge in either direction as
/// an undirected connection.
///
/// Returns an `n×n` 0/1 matrix where `n = region.len()`. Vertices
/// that are not in the graph (shouldn't happen but defensible) are
/// treated as isolated.
pub fn induced_adjacency(graph: &PersistentGraph, region: &[VertexId]) -> Vec<Vec<f64>> {
    let n = region.len();
    let mut adj = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let vi = region[i];
            let vj = region[j];
            let ni = graph.id_map.get(&vi);
            let nj = graph.id_map.get(&vj);
            if let (Some(&nidx_i), Some(&nidx_j)) = (ni, nj) {
                let connected = graph.graph.contains_edge(nidx_i, nidx_j)
                    || graph.graph.contains_edge(nidx_j, nidx_i);
                if connected {
                    adj[i][j] = 1.0;
                    adj[j][i] = 1.0;
                }
            }
        }
    }
    adj
}

/// Compute the Metatron signature for a crystal's region in the
/// given graph, *if* the region is small enough. Returns `None`
/// when the region exceeds [`METATRON_REGION_CAP`] or the scan
/// fails for any other reason (e.g. malformed adjacency — should
/// not happen but the API is defensive).
pub fn signature_for_region(
    graph: &PersistentGraph,
    region: &[VertexId],
    cache: &S7Cache,
) -> Option<MetatronTopologySignature> {
    if region.len() > METATRON_REGION_CAP || region.is_empty() {
        return None;
    }
    let adjacency = induced_adjacency(graph, region);
    pse_metatron::signature::compute_signature_with_cache(adjacency, None, cache).ok()
}

/// Attach a Metatron signature to a crystal in-place, if eligible.
/// Idempotent — calling twice is safe (overwrites, but with the same
/// value given the same graph state).
pub fn attach_signature(crystal: &mut SemanticCrystal, graph: &PersistentGraph, cache: &S7Cache) {
    crystal.metatron_signature = signature_for_region(graph, &crystal.region, cache);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pse_graph::{derive_vertex_id, PersistentGraph};
    use pse_types::PersistenceConfig;

    fn empty_graph() -> PersistentGraph {
        PersistentGraph::new()
    }

    fn add_edge_via_upsert(g: &mut PersistentGraph, a: &str, b: &str) {
        let va = derive_vertex_id(a);
        let vb = derive_vertex_id(b);
        g.upsert_edge(va, vb, 0.0);
    }

    #[test]
    fn induced_adjacency_empty_region_is_empty_matrix() {
        let g = empty_graph();
        let adj = induced_adjacency(&g, &[]);
        assert!(adj.is_empty());
    }

    #[test]
    fn induced_adjacency_disconnected_region_is_zero_matrix() {
        let mut g = empty_graph();
        g.upsert_vertex(derive_vertex_id("a"), 0.0);
        g.upsert_vertex(derive_vertex_id("b"), 0.0);
        g.upsert_vertex(derive_vertex_id("c"), 0.0);
        let region = vec![
            derive_vertex_id("a"),
            derive_vertex_id("b"),
            derive_vertex_id("c"),
        ];
        let adj = induced_adjacency(&g, &region);
        assert_eq!(adj.len(), 3);
        for row in &adj {
            for cell in row {
                assert_eq!(*cell, 0.0);
            }
        }
    }

    #[test]
    fn induced_adjacency_recovers_triangle() {
        let mut g = empty_graph();
        add_edge_via_upsert(&mut g, "a", "b");
        add_edge_via_upsert(&mut g, "b", "c");
        add_edge_via_upsert(&mut g, "c", "a");
        let region = vec![
            derive_vertex_id("a"),
            derive_vertex_id("b"),
            derive_vertex_id("c"),
        ];
        let adj = induced_adjacency(&g, &region);
        // K₃: every off-diagonal entry is 1.
        for i in 0..3 {
            for j in 0..3 {
                if i == j {
                    assert_eq!(adj[i][j], 0.0);
                } else {
                    assert_eq!(adj[i][j], 1.0, "missing edge ({}, {})", i, j);
                }
            }
        }
    }

    #[test]
    fn signature_for_region_skips_oversized() {
        let mut g = empty_graph();
        let region: Vec<VertexId> = (0..(METATRON_REGION_CAP + 1) as u64).collect();
        for v in &region {
            g.upsert_vertex(*v, 0.0);
        }
        let cache = S7Cache::new();
        let sig = signature_for_region(&g, &region, &cache);
        assert!(sig.is_none());
    }

    #[test]
    fn signature_for_triangle_has_three_vertices() {
        let mut g = empty_graph();
        add_edge_via_upsert(&mut g, "a", "b");
        add_edge_via_upsert(&mut g, "b", "c");
        add_edge_via_upsert(&mut g, "c", "a");
        let region = vec![
            derive_vertex_id("a"),
            derive_vertex_id("b"),
            derive_vertex_id("c"),
        ];
        let cache = S7Cache::new();
        let sig = signature_for_region(&g, &region, &cache).unwrap();
        assert_eq!(sig.n, 3);
        assert_eq!(sig.m, 3);
        assert_eq!(sig.triangle_count, 1);
    }

    #[test]
    fn signature_attached_to_crystal_carries_canonical_hash() {
        // Just verify the in-place attachment writes the signature
        // when eligible.
        let mut g = empty_graph();
        add_edge_via_upsert(&mut g, "a", "b");
        add_edge_via_upsert(&mut g, "b", "c");
        let region = vec![
            derive_vertex_id("a"),
            derive_vertex_id("b"),
            derive_vertex_id("c"),
        ];
        let cache = S7Cache::new();
        let mut crystal = make_test_crystal(region.clone());
        assert!(crystal.metatron_signature.is_none());
        attach_signature(&mut crystal, &g, &cache);
        let sig = crystal.metatron_signature.expect("attached");
        assert!(!sig.canonical_hash.is_empty());
    }

    fn make_test_crystal(region: Vec<VertexId>) -> SemanticCrystal {
        SemanticCrystal {
            crystal_id: [0u8; 32],
            region,
            constraint_program: Vec::new(),
            stability_score: 0.5,
            topology_signature: pse_types::TopologySignature::default(),
            betti_numbers: vec![1, 0, 0],
            evidence_chain: Vec::new(),
            commit_proof: pse_types::CommitProof::default(),
            operator_versions: std::collections::BTreeMap::new(),
            created_at: 1,
            free_energy: -0.5,
            carrier_instance_idx: 0,
            scale_tag: "test".into(),
            universe_id: String::new(),
            sub_crystal_ids: Vec::new(),
            parent_crystal_ids: Vec::new(),
            genesis_metadata: None,
            metatron_signature: None,
        }
    }

    // Suppress unused-import warning of PersistenceConfig.
    #[allow(dead_code)]
    fn _silence_unused() {
        let _ = PersistenceConfig::default();
    }
}
