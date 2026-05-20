//! Metatron catalog invariants — OEIS sequence conformance and scan determinism.
//!
//! Verifies that:
//! - Catalog entry counts match OEIS A000088 (unlabeled graphs) for n ≤ 6
//! - Connected graph counts match OEIS A001349
//! - Tree counts match OEIS A000055
//! - Scan is deterministic: same graph → same canonical_hash
//! - Non-isomorphic graphs produce distinct canonical hashes
//! - find_in_catalog locates entries by canonical hash

use pse_metatron::{
    build_catalog, oeis_a000055, oeis_a000088, oeis_a001349, find_in_catalog, scan, InputGraph,
};

// ── OEIS Conformance ─────────────────────────────────────────────────────────

/// Total entry count for n=1..=4 matches OEIS A000088.
#[test]
fn catalog_counts_match_oeis_a000088_for_n1_to_4() {
    let table = build_catalog(4).expect("catalog build must not fail");

    for n in 1usize..=4 {
        let expected = oeis_a000088(n).expect("OEIS A000088 must be defined for n≤4");
        let actual = table.entries.iter().filter(|e| e.n == n).count();
        assert_eq!(
            actual, expected,
            "OEIS A000088({n}) = {expected} but catalog has {actual} entries for n={n}"
        );
    }
}

/// Connected graph count for n=1..=4 matches OEIS A001349.
#[test]
fn connected_counts_match_oeis_a001349_for_n1_to_4() {
    let table = build_catalog(4).expect("catalog build must not fail");
    let stats = &table.stats;

    for n in 1usize..=4 {
        let expected = oeis_a001349(n).expect("OEIS A001349 must be defined for n≤4");
        let actual = stats.connected_per_n.get(&n).copied().unwrap_or(0);
        assert_eq!(
            actual, expected,
            "OEIS A001349({n}) = {expected} but catalog has {actual} connected for n={n}"
        );
    }
}

/// Tree count for n=1..=5 matches OEIS A000055.
#[test]
fn tree_counts_match_oeis_a000055_for_n1_to_5() {
    let table = build_catalog(5).expect("catalog build must not fail");
    let stats = &table.stats;

    for n in 1usize..=5 {
        let expected = oeis_a000055(n).expect("OEIS A000055 must be defined for n≤5");
        let actual = stats.trees_per_n.get(&n).copied().unwrap_or(0);
        assert_eq!(
            actual, expected,
            "OEIS A000055({n}) = {expected} but catalog has {actual} trees for n={n}"
        );
    }
}

// ── Scan Determinism ─────────────────────────────────────────────────────────

/// Scanning the same adjacency matrix twice produces identical canonical_hash.
#[test]
fn scan_is_deterministic() {
    // Triangle (K3)
    let g = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)])
        .expect("K3 graph must be valid");

    let r1 = scan(g.clone()).expect("scan must succeed");
    let r2 = scan(g).expect("scan must succeed");

    assert_eq!(
        r1.canonical_hash, r2.canonical_hash,
        "canonical_hash must be deterministic for the same graph"
    );
    assert_eq!(
        r1.orbit_size, r2.orbit_size,
        "orbit_size must be deterministic"
    );
}

/// Scan report spectrum (eigenvalues) is stable across repeated runs.
#[test]
fn scan_spectrum_is_stable() {
    let g = InputGraph::from_edges(4, &[(1, 2), (2, 3), (3, 4)])
        .expect("path-4 must be valid");

    let r1 = scan(g.clone()).expect("scan 1 must succeed");
    let r2 = scan(g).expect("scan 2 must succeed");

    assert_eq!(r1.eigenvalues.len(), r2.eigenvalues.len());
    for (a, b) in r1.eigenvalues.iter().zip(r2.eigenvalues.iter()) {
        assert!(
            (a - b).abs() < 1e-10,
            "eigenvalue diverged: {a} vs {b}"
        );
    }
}

// ── Canonical Hash Distinguishes Non-Isomorphic Graphs ───────────────────────

/// Path-3 (P3) and cycle-3 (K3/C3) are non-isomorphic → different hashes.
#[test]
fn non_isomorphic_graphs_have_distinct_canonical_hashes() {
    let path3 = InputGraph::from_edges(3, &[(1, 2), (2, 3)])
        .expect("P3 must be valid");
    let cycle3 = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)])
        .expect("K3 must be valid");

    let r_path = scan(path3).expect("scan P3");
    let r_cycle = scan(cycle3).expect("scan K3");

    assert_ne!(
        r_path.canonical_hash, r_cycle.canonical_hash,
        "P3 and K3 are non-isomorphic and must have different canonical hashes"
    );
}

/// Isomorphic labelings produce the same canonical hash.
#[test]
fn isomorphic_relabelings_have_same_canonical_hash() {
    // Two labelings of a 4-path: 1-2-3-4 and 4-3-2-1
    let p4_fwd = InputGraph::from_edges(4, &[(1, 2), (2, 3), (3, 4)])
        .expect("P4 forward must be valid");
    let p4_rev = InputGraph::from_edges(4, &[(1, 4), (4, 3), (3, 2)])
        .expect("P4 reverse must be valid — undirected so same graph");

    let r_fwd = scan(p4_fwd).expect("scan P4 fwd");
    let r_rev = scan(p4_rev).expect("scan P4 rev");

    assert_eq!(
        r_fwd.canonical_hash, r_rev.canonical_hash,
        "isomorphic graphs (P4 relabeled) must have the same canonical hash"
    );
}

// ── Catalog Lookup ────────────────────────────────────────────────────────────

/// find_in_catalog locates K3 (triangle) in the n=3 catalog.
#[test]
fn find_in_catalog_locates_triangle() {
    let table = build_catalog(3).expect("catalog build must not fail");

    let triangle = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)])
        .expect("K3 must be valid");
    let found = find_in_catalog(&triangle, &table);
    assert!(
        found.is_some(),
        "K3 (triangle) must be locatable in the n=3 catalog"
    );
    let entry = found.unwrap();
    assert_eq!(entry.n, 3, "found entry must have n=3");
    assert_eq!(entry.m, 3, "K3 has 3 edges");
}

/// find_in_catalog on P3 (path-3) and K3 returns different entries.
#[test]
fn find_in_catalog_distinguishes_p3_from_k3() {
    let table = build_catalog(3).expect("catalog build must not fail");

    let p3 = InputGraph::from_edges(3, &[(1, 2), (2, 3)]).expect("P3 must be valid");
    let k3 = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)]).expect("K3 must be valid");

    let found_p3 = find_in_catalog(&p3, &table).expect("P3 must be in catalog");
    let found_k3 = find_in_catalog(&k3, &table).expect("K3 must be in catalog");

    assert_ne!(
        found_p3.id, found_k3.id,
        "P3 and K3 are non-isomorphic and must map to different catalog entries"
    );
}

/// Catalog entries all have non-empty canonical hashes.
#[test]
fn all_catalog_entries_have_non_empty_hashes() {
    let table = build_catalog(4).expect("catalog build must not fail");
    for entry in &table.entries {
        assert!(
            !entry.report.canonical_hash.is_empty(),
            "entry {} has empty canonical_hash",
            entry.id
        );
    }
}

/// Catalog IDs are unique.
#[test]
fn catalog_ids_are_unique() {
    use std::collections::BTreeSet;
    let table = build_catalog(4).expect("catalog build must not fail");
    let mut seen = BTreeSet::new();
    for entry in &table.entries {
        assert!(
            seen.insert(&entry.id),
            "duplicate catalog id: {}",
            entry.id
        );
    }
}
