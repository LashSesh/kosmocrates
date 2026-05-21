//! Morphogenic constraint invariants.
//!
//! Verifies:
//! - Inv I1  (No deletion): NodeSplit, NodeMerge, SubgraphPrune deactivate vertices — never delete
//! - Inv I11 (Non-retroactive): mutation_log is append-only; past vertices remain in id_map
//! - Attractor centroid is deterministic and bounded
//! - morphogenic_update populates the mutation log

use pse_constraint::{compute_attractor_centroid, morphogenic_update, MorphState};
use pse_graph::PersistentGraph;
use pse_types::{AdaptationConfig, FiveDState};

fn make_high_pressure_graph(n: usize) -> PersistentGraph {
    // All embeddings far from origin → high pressure when attractor is near zero
    let mut g = PersistentGraph::new();
    for i in 0..n as u64 {
        g.upsert_vertex(i, 0.0);
        g.embedding.insert(
            i,
            FiveDState {
                p: (i as f64 + 1.0) * 0.5,
                rho: (i as f64 + 1.0) * 0.4,
                omega: 0.5,
                chi: 0.3,
                eta: 0.2,
            },
        );
    }
    for i in 0..(n as u64 - 1) {
        g.upsert_edge(i, i + 1, 1.0);
    }
    g
}

fn make_close_embeddings_graph() -> PersistentGraph {
    // Two vertices with nearly-identical embeddings → merge candidate
    let mut g = PersistentGraph::new();
    g.upsert_vertex(0, 0.0);
    g.upsert_vertex(1, 0.0);
    g.embedding.insert(
        0,
        FiveDState {
            p: 0.5,
            rho: 0.5,
            omega: 0.5,
            chi: 0.5,
            eta: 0.5,
        },
    );
    g.embedding.insert(
        1,
        FiveDState {
            p: 0.5001,
            rho: 0.5001,
            omega: 0.5001,
            chi: 0.5001,
            eta: 0.5001,
        },
    );
    g
}

// ── Inv I1: No deletion ───────────────────────────────────────────────────────

/// NodeSplit via morphogenic_update deactivates the split vertex but keeps it in id_map.
#[test]
fn node_split_deactivates_parent_not_deletes() {
    let mut g = make_high_pressure_graph(3);
    let mut morph = MorphState::new();
    let config = AdaptationConfig {
        split_threshold: 0.0, // fire on any positive pressure
        merge_distance: 0.0,  // no merges
        ..AdaptationConfig::default()
    };

    let vertex_ids_before: std::collections::BTreeSet<u64> = g.id_map.keys().copied().collect();

    morphogenic_update(&mut g, &mut morph, &[], &config);

    // Every vertex that existed before must still exist in id_map (Inv I1: no deletion)
    for &vid in &vertex_ids_before {
        assert!(
            g.id_map.contains_key(&vid),
            "vertex {vid} must remain in id_map after split (Inv I1: no deletion)"
        );
    }
    // At least one split should have happened given zero threshold
    let has_splits = morph
        .mutation_log
        .iter()
        .any(|m| matches!(m, pse_constraint::MorphMutation::NodeSplit { .. }));
    assert!(has_splits, "at least one NodeSplit must have fired");
}

/// NodeMerge via morphogenic_update deactivates merged vertices but keeps them in id_map.
#[test]
fn node_merge_deactivates_not_deletes() {
    let mut g = make_close_embeddings_graph();
    let mut morph = MorphState::new();
    let config = AdaptationConfig {
        merge_distance: 10.0, // very large → merges everything
        split_threshold: 1e9, // no splits
        ..AdaptationConfig::default()
    };

    let vertex_ids_before: std::collections::BTreeSet<u64> = g.id_map.keys().copied().collect();

    morphogenic_update(&mut g, &mut morph, &[], &config);

    // All original vertices must still be in id_map (Inv I1)
    for &vid in &vertex_ids_before {
        assert!(
            g.id_map.contains_key(&vid),
            "vertex {vid} must remain in id_map after merge (Inv I1: no deletion)"
        );
    }
}

/// SubgraphPrune via morphogenic_update keeps dormant vertices in id_map.
#[test]
fn subgraph_prune_deactivates_not_deletes() {
    // Step 1: run a split to create dormant (deactivated) vertices
    let mut g = make_high_pressure_graph(3);
    let mut morph = MorphState::new();
    let split_config = AdaptationConfig {
        split_threshold: 0.0,
        merge_distance: 0.0,
        ..AdaptationConfig::default()
    };
    morphogenic_update(&mut g, &mut morph, &[], &split_config);

    // Capture all vertex IDs (including split children)
    let all_ids: std::collections::BTreeSet<u64> = g.id_map.keys().copied().collect();

    // Step 2: run again — dormant vertices from step 1 trigger SubgraphPrune
    let prune_config = AdaptationConfig {
        split_threshold: 1e9, // no new splits
        merge_distance: 0.0,
        ..AdaptationConfig::default()
    };
    morphogenic_update(&mut g, &mut morph, &[], &prune_config);

    // All vertices from before step 2 must still be in id_map (Inv I1)
    for &vid in &all_ids {
        assert!(
            g.id_map.contains_key(&vid),
            "vertex {vid} must remain in id_map after prune (Inv I1: no deletion)"
        );
    }
}

// ── Inv I11: Non-retroactive mutation log ─────────────────────────────────────

/// The mutation_log grows monotonically — mutations are appended, never removed.
#[test]
fn mutation_log_is_append_only() {
    let mut g = make_high_pressure_graph(6);
    let mut morph = MorphState::new();
    let config = AdaptationConfig {
        split_threshold: 0.01, // very low → many splits
        ..AdaptationConfig::default()
    };

    let before = morph.mutation_log.len();
    let mutations = morphogenic_update(&mut g, &mut morph, &[], &config);
    let after = morph.mutation_log.len();

    assert!(
        after >= before,
        "mutation_log must never shrink (Inv I11: append-only)"
    );
    assert_eq!(
        after - before,
        mutations.len(),
        "mutation_log must grow by exactly the number of mutations applied"
    );
}

/// Running morphogenic_update twice appends to the log, never resets it.
#[test]
fn mutation_log_accumulates_across_updates() {
    let mut g = make_high_pressure_graph(4);
    let mut morph = MorphState::new();
    let config = AdaptationConfig {
        split_threshold: 0.01,
        ..AdaptationConfig::default()
    };

    morphogenic_update(&mut g, &mut morph, &[], &config);
    let after_first = morph.mutation_log.len();

    // Rebuild graph so there are embeddings to apply pressure to
    let mut g2 = make_high_pressure_graph(4);
    morphogenic_update(&mut g2, &mut morph, &[], &config);
    let after_second = morph.mutation_log.len();

    assert!(
        after_second >= after_first,
        "mutation_log must be monotonically non-decreasing (Inv I11)"
    );
}

// ── Attractor Centroid ────────────────────────────────────────────────────────

/// Attractor centroid of an empty graph is the zero state.
#[test]
fn attractor_centroid_empty_graph_is_zero() {
    let g = PersistentGraph::new();
    let centroid = compute_attractor_centroid(&g, 5);
    assert_eq!(centroid, FiveDState::default());
}

/// Attractor centroid is deterministic: same graph → same centroid.
#[test]
fn attractor_centroid_is_deterministic() {
    let g = make_high_pressure_graph(5);
    let c1 = compute_attractor_centroid(&g, 3);
    let c2 = compute_attractor_centroid(&g, 3);
    assert_eq!(c1, c2, "attractor centroid must be deterministic");
}

/// Attractor centroid components are finite.
#[test]
fn attractor_centroid_is_finite() {
    let g = make_high_pressure_graph(5);
    let c = compute_attractor_centroid(&g, 3);
    assert!(c.p.is_finite(), "centroid.p must be finite");
    assert!(c.rho.is_finite(), "centroid.rho must be finite");
    assert!(c.omega.is_finite(), "centroid.omega must be finite");
    assert!(c.chi.is_finite(), "centroid.chi must be finite");
    assert!(c.eta.is_finite(), "centroid.eta must be finite");
}
