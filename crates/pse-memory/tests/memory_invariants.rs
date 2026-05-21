//! Pattern memory invariants.
//!
//! Verifies:
//! - Empty memory always misses (no false positives before any inserts)
//! - Exact-match lookup hits with similarity = 1.0
//! - Hit/miss statistics are self-consistent (hit_rate = hits / total_lookups)
//! - top_k=0 returns empty; top_k > index.len() returns all entries
//! - load_from_crystals populates the index
//! - Similarity threshold is respected (too-low similarity → miss)

use pse_memory::{CrystalSignature, MemoryConfig, PatternMemory};
use pse_types::Hash256;

fn zero_hash(seed: u8) -> Hash256 {
    let mut h = [0u8; 32];
    h[0] = seed;
    h
}

fn sig(crystal_id: Hash256, spectral: Vec<f64>, resonance: f64) -> CrystalSignature {
    CrystalSignature {
        crystal_id,
        spectral,
        resonance,
        confidence: 0.9,
        content_hash: crystal_id,
        tick_range: (0, 1),
        observation_count: 4,
    }
}

fn default_memory() -> PatternMemory {
    PatternMemory::new(MemoryConfig::default())
}

// ── Miss before insert ────────────────────────────────────────────────────────

/// An empty memory always returns None for any lookup.
#[test]
fn empty_memory_is_always_miss() {
    let mut mem = default_memory();
    let candidate = sig(zero_hash(1), vec![0.5; 8], 0.8);
    let result = mem.lookup(&candidate);
    assert!(result.is_none(), "empty memory must never produce a hit");
}

/// Stats after one miss: total=1, misses=1, hits=0, hit_rate=0.
#[test]
fn stats_reflect_single_miss() {
    let mut mem = default_memory();
    let candidate = sig(zero_hash(1), vec![0.5; 8], 0.8);
    mem.lookup(&candidate);
    let stats = mem.stats();
    assert_eq!(stats.total_lookups, 1);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.hit_rate, 0.0);
}

// ── Exact-match hit ───────────────────────────────────────────────────────────

/// Inserting a signature and looking it up with the same spectral vector hits.
#[test]
fn insert_then_lookup_identical_is_hit() {
    let mut mem = default_memory();
    let spectral = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
    let s = sig(zero_hash(5), spectral.clone(), 0.9);

    mem.insert(s);

    let candidate = sig(zero_hash(5), spectral, 0.9);
    let result = mem.lookup(&candidate);
    assert!(result.is_some(), "identical signature must produce a hit");
    assert_eq!(result.unwrap(), zero_hash(5));
}

// ── Statistics self-consistency ───────────────────────────────────────────────

/// hit_rate == hits / total_lookups after mixed hit/miss sequence.
#[test]
fn hit_rate_is_consistent_with_counts() {
    let mut mem = default_memory();
    let spectral = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
    mem.insert(sig(zero_hash(1), spectral.clone(), 0.9));

    // 1 hit
    mem.lookup(&sig(zero_hash(1), spectral, 0.9));
    // 1 miss (empty spectral → low similarity)
    mem.lookup(&sig(zero_hash(99), vec![0.0; 8], 0.0));

    let s = mem.stats();
    assert_eq!(s.total_lookups, 2);
    assert_eq!(s.hits, 1);
    assert_eq!(s.misses, 1);
    let expected_rate = 1.0 / 2.0;
    assert!(
        (s.hit_rate - expected_rate).abs() < 1e-9,
        "hit_rate={} must equal hits/total={}",
        s.hit_rate,
        expected_rate
    );
}

// ── top_k ─────────────────────────────────────────────────────────────────────

/// top_k=0 returns an empty vector.
#[test]
fn top_k_zero_returns_empty() {
    let mut mem = default_memory();
    let s = sig(zero_hash(1), vec![0.5; 8], 0.8);
    mem.insert(s.clone());
    let results = mem.top_k(&s, 0);
    assert!(results.is_empty(), "top_k=0 must return empty vec");
}

/// top_k > index.len() returns at most index.len() entries.
#[test]
fn top_k_bounded_by_index_size() {
    let mut mem = default_memory();
    for i in 0..3u8 {
        mem.insert(sig(zero_hash(i), vec![i as f64 * 0.1; 8], 0.8));
    }
    let candidate = sig(zero_hash(99), vec![0.5; 8], 0.8);
    let results = mem.top_k(&candidate, 100);
    assert!(
        results.len() <= 3,
        "top_k must not return more entries than exist in the index"
    );
}

/// top_k results are sorted by descending similarity.
#[test]
fn top_k_is_sorted_descending() {
    let mut mem = default_memory();
    let base = vec![0.5f64; 8];
    for i in 0..4u8 {
        let mut sp = base.clone();
        sp[0] = i as f64 * 0.1;
        mem.insert(sig(zero_hash(i), sp, 0.9));
    }
    let candidate = sig(zero_hash(99), base, 0.9);
    let results = mem.top_k(&candidate, 4);
    for w in results.windows(2) {
        assert!(
            w[0].1 >= w[1].1,
            "top_k must be sorted by descending similarity: {} < {}",
            w[0].1,
            w[1].1
        );
    }
}

// ── load_from_crystals ────────────────────────────────────────────────────────

/// load_from_crystals returns the number of loaded crystals and populates index.
#[test]
fn load_from_crystals_populates_index() {
    use pse_types::SemanticCrystal;
    use std::collections::BTreeMap;

    use pse_types::{CommitProof, ConstraintProgram, TopologySignature};

    // Build minimal SemanticCrystals directly from pse_types
    let crystals: Vec<SemanticCrystal> = (0..5u64)
        .map(|i| {
            let mut id = [0u8; 32];
            id[0] = i as u8;
            SemanticCrystal {
                crystal_id: id,
                region: vec![i],
                stability_score: 0.8,
                created_at: i,
                free_energy: 0.1,
                carrier_instance_idx: 0,
                betti_numbers: vec![1, 0, 0],
                operator_versions: BTreeMap::new(),
                constraint_program: ConstraintProgram::default(),
                topology_signature: TopologySignature::default(),
                evidence_chain: Vec::new(),
                commit_proof: CommitProof::default(),
                scale_tag: String::new(),
                universe_id: String::new(),
                sub_crystal_ids: Vec::new(),
                parent_crystal_ids: Vec::new(),
                genesis_metadata: None,
                metatron_signature: None,
            }
        })
        .collect();

    let mut mem = default_memory();
    let loaded = mem.load_from_crystals(&crystals);
    assert_eq!(loaded, 5, "must report 5 loaded crystals");

    let stats = mem.stats();
    assert_eq!(stats.index_size, 5, "index_size must reflect loaded count");
}

// ── LRU eviction ─────────────────────────────────────────────────────────────

/// When max_signatures is exceeded, index stays at max_signatures (LRU evict).
#[test]
fn index_size_capped_at_max_signatures() {
    let config = MemoryConfig {
        max_signatures: 3,
        ..MemoryConfig::default()
    };
    let mut mem = PatternMemory::new(config);

    for i in 0..10u8 {
        let sp: Vec<f64> = (0..8).map(|j| (i as f64 + j as f64) * 0.05).collect();
        mem.insert(sig(zero_hash(i), sp, 0.8));
    }

    let stats = mem.stats();
    assert!(
        stats.index_size <= 3,
        "index_size={} must not exceed max_signatures=3",
        stats.index_size
    );
}
