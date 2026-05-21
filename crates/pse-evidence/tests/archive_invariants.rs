//! Evidence archive invariants.
//!
//! Verifies:
//! - Inv I10 (Append-only): Archive.len() never decreases; crystals() slice is immutable
//! - Hash-chain integrity: evidence_chain links are correct
//! - Crystal content address: verify_crystal accepts valid crystals
//! - PoR monotonicity: out-of-order timestamps are rejected
//! - Operator drift: pinned-version mismatch is detected

use pse_evidence::{build_crystal_with_id, build_evidence_chain, Archive, VerifyError};
use pse_types::{CommitProof, ConsensusResult, ConstraintProgram, GateSnapshot, PoRTrace};
use std::collections::BTreeMap;

fn valid_commit_proof() -> CommitProof {
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
        operator_stack: Vec::new(),
        ..Default::default()
    }
}

fn build_valid_crystal(idx: u64) -> pse_types::SemanticCrystal {
    build_crystal_with_id(
        vec![idx, idx + 1],
        0.8,
        idx,
        0.1,
        0,
        ConstraintProgram::default(),
        valid_commit_proof(),
    )
}

// ── Inv I10: Append-only ──────────────────────────────────────────────────────

/// Archive.len() never decreases after append.
#[test]
fn archive_len_never_decreases() {
    let mut archive = Archive::new();
    assert_eq!(archive.len(), 0);

    for i in 0..5u64 {
        archive.append(build_valid_crystal(i));
        assert_eq!(archive.len(), i as usize + 1, "len must equal append count");
    }
}

/// Archive.crystals() slice grows but previously appended crystals are unchanged.
#[test]
fn archive_previously_appended_crystals_are_unchanged() {
    let mut archive = Archive::new();
    let c0 = build_valid_crystal(0);
    let c1 = build_valid_crystal(10);

    let id0 = c0.crystal_id;
    archive.append(c0);
    archive.append(c1);

    assert_eq!(
        archive.crystals()[0].crystal_id, id0,
        "first crystal must be unchanged after second append"
    );
    assert_eq!(archive.len(), 2);
}

// ── Hash-chain integrity ──────────────────────────────────────────────────────

/// An evidence chain links correctly: chain[i].prev == Some(chain[i-1].digest).
#[test]
fn evidence_chain_links_are_valid() {
    let data: Vec<Vec<u8>> = vec![
        b"entry-A".to_vec(),
        b"entry-B".to_vec(),
        b"entry-C".to_vec(),
    ];
    let chain = build_evidence_chain(&data);

    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0].prev, None, "first entry must have no predecessor");
    assert_eq!(chain[1].prev, Some(chain[0].digest), "chain[1].prev must link to chain[0]");
    assert_eq!(chain[2].prev, Some(chain[1].digest), "chain[2].prev must link to chain[1]");
}

/// An evidence chain's digests match the SHA-256 of their content.
#[test]
fn evidence_chain_digests_are_correct() {
    use pse_types::content_address_raw;
    let data: Vec<Vec<u8>> = vec![b"hello".to_vec(), b"world".to_vec()];
    let chain = build_evidence_chain(&data);

    for (i, entry) in chain.iter().enumerate() {
        let expected = content_address_raw(&data[i]);
        assert_eq!(
            entry.digest, expected,
            "chain[{i}].digest must be sha256 of its content"
        );
    }
}

// ── Crystal verification ──────────────────────────────────────────────────────

/// A correctly-built crystal passes verify_crystal with no pinned versions.
#[test]
fn valid_crystal_passes_verify() {
    let crystal = build_valid_crystal(1);
    let result = pse_evidence::verify_crystal(&crystal, &BTreeMap::new());
    assert!(
        result.is_ok(),
        "valid crystal must pass verify_crystal: {:?}",
        result
    );
}

/// archive.verify_all() returns Ok for all appended valid crystals.
#[test]
fn archive_verify_all_valid_crystals() {
    let mut archive = Archive::new();
    for i in 0..4u64 {
        archive.append(build_valid_crystal(i * 3));
    }

    let results = archive.verify_all();
    assert_eq!(results.len(), 4);
    for (i, (idx, res)) in results.iter().enumerate() {
        assert_eq!(*idx, i);
        assert!(
            res.is_ok(),
            "crystal at index {i} must pass verification: {:?}",
            res
        );
    }
}

// ── PoR monotonicity ─────────────────────────────────────────────────────────

/// A crystal with reversed PoR timestamps is rejected.
#[test]
fn por_trace_monotonicity_is_enforced() {
    let mut crystal = build_valid_crystal(5);
    // Reverse: lock < search → violation
    crystal.commit_proof.por_trace = PoRTrace {
        search_enter: 10.0,
        lock_enter: Some(5.0),  // 5 < 10 → PoR monotonicity violated
        verify_enter: Some(15.0),
        commit_enter: Some(20.0),
    };

    let result = pse_evidence::verify_crystal(&crystal, &BTreeMap::new());
    assert!(
        matches!(result, Err(VerifyError::PoRMonotonicity)),
        "reversed PoR timestamps must produce PoRMonotonicity error, got: {:?}",
        result
    );
}

// ── Operator drift detection ──────────────────────────────────────────────────

/// A crystal whose operator version differs from the pinned version is rejected.
#[test]
fn operator_drift_is_detected() {
    let mut crystal = build_valid_crystal(7);
    crystal.commit_proof.operator_stack.push(("lpcm".to_string(), "1.0.0".to_string()));

    let mut pinned = BTreeMap::new();
    pinned.insert("lpcm".to_string(), "2.0.0".to_string()); // different version

    let result = pse_evidence::verify_crystal(&crystal, &pinned);
    assert!(
        matches!(result, Err(VerifyError::OperatorDrift(_))),
        "mismatched operator version must produce OperatorDrift error, got: {:?}",
        result
    );
}

/// Operator drift is NOT raised when versions match.
#[test]
fn matching_operator_version_passes() {
    let mut crystal = build_valid_crystal(9);
    crystal.commit_proof.operator_stack.push(("lpcm".to_string(), "1.0.0".to_string()));

    let mut pinned = BTreeMap::new();
    pinned.insert("lpcm".to_string(), "1.0.0".to_string());

    let result = pse_evidence::verify_crystal(&crystal, &pinned);
    assert!(
        result.is_ok(),
        "matching operator version must not trigger drift: {:?}",
        result
    );
}

// ── Kairos gate fail-closed in verify ────────────────────────────────────────

/// A crystal with kairos=false fails verify_crystal with GateFail.
#[test]
fn crystal_with_kairos_false_fails_verification() {
    let mut crystal = build_valid_crystal(11);
    crystal.commit_proof.gate_values.kairos = false;
    // Recompute crystal_id to pass content address check
    let crystal = build_crystal_with_id(
        crystal.region.clone(),
        crystal.stability_score,
        crystal.created_at,
        crystal.free_energy,
        crystal.carrier_instance_idx,
        ConstraintProgram::default(),
        crystal.commit_proof,
    );

    let result = pse_evidence::verify_crystal(&crystal, &BTreeMap::new());
    assert!(
        matches!(result, Err(VerifyError::GateFail)),
        "kairos=false must produce GateFail, got: {:?}",
        result
    );
}
