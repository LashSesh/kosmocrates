//! Multi-scale state invariants (C18).
//!
//! Verifies:
//! - multi_scale_tick disabled → passthrough (empty crystals, default metrics)
//! - multi_scale_tick enabled → metrics m28/m30/m31 are populated and bounded
//! - build_universe is deterministic for the same vertex set
//! - spectral_bisection_cluster covers all input vertices
//! - kuramoto_phase_cluster covers all input vertices
//! - HyperBounds subdivision: 32 children, volume conserved

use pse_graph::PersistentGraph;
use pse_scale::{
    build_universe, kuramoto_phase_cluster, multi_scale_tick, pse_engine_types, spectral_bisection_cluster,
    HyperBounds, MultiScaleState, Scale, ScaleConfig, ScalePolicy,
};
use pse_topology::{compute_laplacian, init_kuramoto_state, kuramoto_step, spectral_decompose, TopologyConfig};
use pse_types::{FiveDState, VertexId};
use std::collections::BTreeMap;

fn make_chain(n: usize) -> PersistentGraph {
    let mut g = PersistentGraph::new();
    for i in 0..n as u64 {
        g.upsert_vertex(i, 0.0);
        g.embedding.insert(i, FiveDState {
            p: (i as f64 + 1.0) * 0.1,
            rho: 0.5,
            omega: 0.3,
            chi: 0.2,
            eta: 0.1,
        });
    }
    for i in 0..(n as u64 - 1) {
        g.upsert_edge(i, i + 1, 1.0);
    }
    g
}

fn run_kuramoto(g: &PersistentGraph, steps: usize) -> pse_topology::KuramotoState {
    let mut ks = init_kuramoto_state(g);
    let cfg = TopologyConfig::default();
    for _ in 0..steps {
        kuramoto_step(&mut ks, g, &cfg);
    }
    ks
}

// ── multi_scale_tick disabled ─────────────────────────────────────────────────

/// When ScaleConfig.enabled = false, multi_scale_tick is a passthrough.
#[test]
fn multi_scale_tick_disabled_is_passthrough() {
    let config = ScaleConfig { enabled: false, ..Default::default() };
    let g = make_chain(4);
    let lap = compute_laplacian(&g);
    let spec = spectral_decompose(&lap, 4);
    let ks = run_kuramoto(&g, 3);
    let mut state = MultiScaleState::default();
    let micro = pse_engine_types::MicroState::from_graph(&g);

    let result = multi_scale_tick(&micro, &mut state, &spec, &ks, &config, &[], 0);
    assert!(result.meso_crystals.is_empty(), "disabled: no meso crystals");
    assert!(result.macro_crystals.is_empty(), "disabled: no macro crystals");
}

// ── multi_scale_tick enabled — metrics ────────────────────────────────────────

/// M28 cluster count, M30 scale coherence, and M31 lift compression are populated.
#[test]
fn multi_scale_tick_metrics_are_populated() {
    let config = ScaleConfig::default();
    let g = make_chain(8);
    let lap = compute_laplacian(&g);
    let spec = spectral_decompose(&lap, 8);
    let ks = run_kuramoto(&g, 10);
    let mut state = MultiScaleState::default();
    let micro = pse_engine_types::MicroState::from_graph(&g);

    let result = multi_scale_tick(&micro, &mut state, &spec, &ks, &config, &[], 1);

    assert!(result.metrics.m28_cluster_count > 0, "M28 cluster count must be > 0");
    assert!(
        result.metrics.m30_scale_coherence >= 0.0 && result.metrics.m30_scale_coherence <= 1.0,
        "M30 scale coherence must be in [0,1]: {}",
        result.metrics.m30_scale_coherence
    );
    assert!(
        result.metrics.m31_lift_compression >= 0.0,
        "M31 lift compression must be non-negative"
    );
}

/// multi_scale_tick is deterministic: same inputs → same cluster count.
#[test]
fn multi_scale_tick_is_deterministic() {
    let config = ScaleConfig::default();
    let g = make_chain(6);
    let lap = compute_laplacian(&g);
    let spec = spectral_decompose(&lap, 6);
    let ks = run_kuramoto(&g, 5);
    let micro = pse_engine_types::MicroState::from_graph(&g);

    let mut s1 = MultiScaleState::default();
    let r1 = multi_scale_tick(&micro, &mut s1, &spec, &ks, &config, &[], 0);

    let mut s2 = MultiScaleState::default();
    let r2 = multi_scale_tick(&micro, &mut s2, &spec, &ks, &config, &[], 0);

    assert_eq!(
        r1.metrics.m28_cluster_count,
        r2.metrics.m28_cluster_count,
        "multi_scale_tick must be deterministic: cluster count must match"
    );
}

// ── build_universe ────────────────────────────────────────────────────────────

/// build_universe is deterministic: same vertex set → same universe id count.
#[test]
fn build_universe_is_deterministic() {
    let vertices: Vec<VertexId> = (0..5).collect();
    let embeddings: BTreeMap<VertexId, FiveDState> = vertices
        .iter()
        .map(|&v| (v, FiveDState { p: v as f64 * 0.1, rho: 0.5, omega: 0.3, chi: 0.2, eta: 0.1 }))
        .collect();

    let u1 = build_universe(&vertices, &embeddings, Scale::Meso, ScalePolicy::Balanced);
    let u2 = build_universe(&vertices, &embeddings, Scale::Meso, ScalePolicy::Balanced);

    assert_eq!(u1.vertex_ids.len(), u2.vertex_ids.len(), "universe vertex count must be deterministic");
}

/// build_universe contains all input vertices.
#[test]
fn build_universe_contains_all_vertices() {
    let vertices: Vec<VertexId> = (0..6).collect();
    let embeddings: BTreeMap<VertexId, FiveDState> = vertices
        .iter()
        .map(|&v| (v, FiveDState { p: v as f64 * 0.15, rho: 0.5, omega: 0.3, chi: 0.2, eta: 0.1 }))
        .collect();

    let u = build_universe(&vertices, &embeddings, Scale::Micro, ScalePolicy::Balanced);
    assert_eq!(u.vertex_ids.len(), vertices.len(), "all input vertices must be in the universe");
}

// ── spectral_bisection_cluster ────────────────────────────────────────────────

/// spectral_bisection_cluster assigns all vertices to a cluster.
#[test]
fn spectral_bisection_covers_all_vertices() {
    let g = make_chain(6);
    let lap = compute_laplacian(&g);
    let spec = spectral_decompose(&lap, 6);
    let vertex_ids: Vec<VertexId> = (0..6).collect();
    let config = ScaleConfig::default();

    let assignment = spectral_bisection_cluster(
        &spec.fiedler_vector,
        &vertex_ids,
        config.meso.spectral_gap_threshold,
        config.meso.min_cluster_size,
        config.meso.max_clusters,
    );

    assert_eq!(
        assignment.len(),
        6,
        "spectral_bisection must assign all {} vertices",
        6
    );
    for &vid in &vertex_ids {
        assert!(assignment.contains_key(&vid), "vertex {vid} must have a cluster assignment");
    }
}

// ── kuramoto_phase_cluster ────────────────────────────────────────────────────

/// kuramoto_phase_cluster assigns all vertices.
#[test]
fn kuramoto_phase_cluster_covers_all_vertices() {
    let g = make_chain(5);
    let ks = run_kuramoto(&g, 5);
    let vertex_ids: Vec<VertexId> = (0..5).collect();
    let config = ScaleConfig::default();

    let assignment = kuramoto_phase_cluster(
        &ks.phases,
        &vertex_ids,
        config.meso.phase_threshold,
        config.meso.min_cluster_size,
    );

    assert_eq!(assignment.len(), 5, "kuramoto_phase_cluster must assign all vertices");
    for &vid in &vertex_ids {
        assert!(assignment.contains_key(&vid), "vertex {vid} must have a cluster assignment");
    }
}

// ── HyperBounds subdivision ───────────────────────────────────────────────────

/// HyperBounds.split_all() produces 32 non-overlapping children whose
/// combined volume equals the parent volume.
#[test]
fn hyperbounds_subdivision_32_children_volume_conserved() {
    let s = |v: f64| FiveDState { p: v, rho: v, omega: v, chi: v, eta: v };
    let parent = HyperBounds::new(s(0.0), s(1.0));
    let children = parent.split_all();

    assert_eq!(children.len(), 32, "split_all must produce exactly 32 children");

    let total: f64 = children.iter().map(|c| c.volume()).sum();
    let parent_vol = parent.volume();
    assert!(
        (total - parent_vol).abs() < 1e-9,
        "children volume {total} must equal parent {parent_vol}"
    );
}
