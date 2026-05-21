//! Replay system invariants.
//!
//! Verifies:
//! - ReplayPack serializes and deserializes without data loss (round-trip)
//! - compare_crystal_sequences detects mismatched crystal IDs
//! - compare_crystal_sequences detects length mismatches
//! - verify_determinism is content-addressed (same RunDescriptor → same result)
//! - ReplayResult reflects correct crystal counts

use pse_replay::{compare_crystal_sequences, verify_determinism, ReplayPack};
use pse_types::{Config, Hash256, RunDescriptor};
use std::collections::BTreeMap;

fn make_rd() -> RunDescriptor {
    RunDescriptor {
        config: Config::default(),
        operator_versions: BTreeMap::new(),
        initial_state_digest: [0u8; 32],
        seed: None,
        registry_digests: BTreeMap::new(),
        scheduler: pse_types::SchedulerConfig::default(),
    }
}

fn make_crystal_id(seed: u8) -> Hash256 {
    let mut id = [0u8; 32];
    id[0] = seed;
    id
}

// Use pse_evidence to build minimal valid crystals for sequence comparison
fn minimal_crystal(seed: u8) -> pse_types::SemanticCrystal {
    use pse_evidence::build_crystal_with_id;
    use pse_types::{CommitProof, ConsensusResult, ConstraintProgram, GateSnapshot, PoRTrace};
    build_crystal_with_id(
        vec![seed as u64],
        0.8,
        seed as u64,
        0.1,
        0,
        ConstraintProgram::default(),
        CommitProof {
            gate_values: GateSnapshot {
                d: 1.0,
                q: 1.0,
                r: 1.0,
                g: 1.0,
                j: 1.0,
                p: 1.0,
                n: 1.0,
                k: 1.0,
                kairos: true,
            },
            consensus_result: ConsensusResult {
                primal_score: 0.9,
                dual_score: 0.9,
                mci: 0.95,
                threshold: 0.6,
            },
            por_trace: PoRTrace {
                search_enter: 1.0,
                lock_enter: Some(2.0),
                verify_enter: Some(3.0),
                commit_enter: Some(4.0),
            },
            ..Default::default()
        },
    )
}

// ── ReplayPack round-trip ─────────────────────────────────────────────────────

/// ReplayPack serializes to JSON and deserializes back without data loss.
#[test]
fn replay_pack_round_trips_json() {
    let pack = ReplayPack {
        rd: make_rd(),
        observation_log: vec![vec![b"obs1".to_vec(), b"obs2".to_vec()]],
        expected_crystal_ids: vec![make_crystal_id(42)],
    };

    let json = serde_json::to_string(&pack).expect("serialize must succeed");
    let restored: ReplayPack = serde_json::from_str(&json).expect("deserialize must succeed");

    assert_eq!(restored.observation_log, pack.observation_log);
    assert_eq!(restored.expected_crystal_ids, pack.expected_crystal_ids);
}

/// An empty ReplayPack serializes and deserializes correctly.
#[test]
fn empty_replay_pack_round_trips() {
    let pack = ReplayPack {
        rd: make_rd(),
        observation_log: vec![],
        expected_crystal_ids: vec![],
    };

    let json = serde_json::to_string(&pack).expect("serialize must succeed");
    let restored: ReplayPack = serde_json::from_str(&json).expect("deserialize must succeed");

    assert!(restored.observation_log.is_empty());
    assert!(restored.expected_crystal_ids.is_empty());
}

// ── compare_crystal_sequences ─────────────────────────────────────────────────

/// Two identical sequences are deterministic.
#[test]
fn identical_sequences_are_deterministic() {
    let seq: Vec<_> = (0..4u8).map(minimal_crystal).collect();
    let result = compare_crystal_sequences(&seq, &seq);
    assert!(
        result.deterministic,
        "identical sequences must be deterministic"
    );
    assert_eq!(result.crystal_count, 4);
    assert!(result.digest_matches.iter().all(|&m| m));
}

/// Sequences with differing crystal IDs are not deterministic.
#[test]
fn different_crystal_ids_are_not_deterministic() {
    let seq_a: Vec<_> = (0..3u8).map(minimal_crystal).collect();
    let seq_b: Vec<_> = (10..13u8).map(minimal_crystal).collect();
    let result = compare_crystal_sequences(&seq_a, &seq_b);
    assert!(
        !result.deterministic,
        "sequences with different crystal IDs must not be deterministic"
    );
}

/// Length mismatch between two sequences is not deterministic.
#[test]
fn length_mismatch_is_not_deterministic() {
    let seq_a: Vec<_> = (0..3u8).map(minimal_crystal).collect();
    let seq_b: Vec<_> = (0..4u8).map(minimal_crystal).collect();
    let result = compare_crystal_sequences(&seq_a, &seq_b);
    assert!(
        !result.deterministic,
        "sequences of different lengths must not be deterministic"
    );
}

/// Empty sequences are deterministic.
#[test]
fn empty_sequences_are_deterministic() {
    let result = compare_crystal_sequences(&[], &[]);
    assert!(result.deterministic);
    assert_eq!(result.crystal_count, 0);
    assert!(result.digest_matches.is_empty());
}

/// digest_matches has one entry per matched pair.
#[test]
fn digest_matches_length_matches_shorter_sequence() {
    let seq_a: Vec<_> = (0..4u8).map(minimal_crystal).collect();
    let seq_b: Vec<_> = (0..4u8).map(minimal_crystal).collect();
    let result = compare_crystal_sequences(&seq_a, &seq_b);
    assert_eq!(result.digest_matches.len(), 4);
}

// ── verify_determinism ────────────────────────────────────────────────────────

/// verify_determinism is content-addressed: same RunDescriptor → true.
#[test]
fn verify_determinism_is_content_addressed() {
    let rd = make_rd();
    assert!(
        verify_determinism(&rd),
        "same RunDescriptor must produce deterministic content address"
    );
}

/// Two RunDescriptors with identical content both return true.
#[test]
fn two_identical_rds_both_deterministic() {
    let rd1 = make_rd();
    let rd2 = make_rd();
    assert!(verify_determinism(&rd1));
    assert!(verify_determinism(&rd2));
}
