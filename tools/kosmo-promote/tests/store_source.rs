//! `--store`: the CAD library as a promotion source, tested against the real
//! binary.
//!
//! A certified `StructuralCrystalRecord` is appended to a `kosmo-store` JSONL
//! store (the artifact `kosmo-substrate --store` maintains across runs), then
//! `kosmo-promote --store` offers it to the PSE engine. Records are directly
//! evidence-bound (CROSS-006), so the store wraps without resolving candidates;
//! a tampered store is a hard error.

use std::path::PathBuf;
use std::process::Command;

use kosmo_core::{
    AuthorityLabel, Digest, EvidenceBundle, EvidenceKind, EvidenceRef, PolicyProfile, ReplayStatus,
    TaintLabel, Q16,
};
use kosmo_hyphae::{
    AssimilationDecision, CrossLanguageFingerprint, GateCascade, SourceLanguage,
    StructuralCrystalCandidate, StructuralCrystalRecord, StructuralYield, StructuralYieldKind,
};
use kosmo_store::CrystalRecordStore;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_kosmo-promote")
}

fn scratch(tag: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join("kosmo-promote-tests").join(tag);
    std::fs::remove_dir_all(&root).ok();
    let ws = root.join("ws");
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::write(ws.join("lib.rs"), "pub fn a() {}\n").unwrap();
    (root, ws)
}

/// One certified, fingerprinted record — the same construction the capstone uses.
fn certified_record(seed: &str) -> StructuralCrystalRecord {
    let policy = PolicyProfile::default_report_only();
    let ev = EvidenceBundle::seal(
        vec![EvidenceRef::new(
            Digest::of_bytes(seed.as_bytes()),
            EvidenceKind::HostScan,
            "store scan",
        )],
        policy.id,
        ReplayStatus::Replayable,
    );
    let yield_ = StructuralYield::new(
        StructuralYieldKind::DeficiencyFill,
        Some(Digest::of_bytes(format!("void-{seed}").as_bytes())),
        None,
        TaintLabel::Clean,
        AuthorityLabel::Foundry,
        ev.bundle_id,
        policy.id,
    );
    let trace = GateCascade::standard_gates(policy.clone()).apply(&yield_, &ev);
    let decision = AssimilationDecision::from_trace(&yield_, &trace, &ev, policy.id);
    let fingerprint = CrossLanguageFingerprint::from_source(
        SourceLanguage::Python,
        Digest::of_bytes(format!("src-{seed}").as_bytes()),
        "import os\ndef a(x):\n    return x\n",
    );
    let (_cert, record) = StructuralCrystalCandidate::from_decision_with_signals(
        &decision,
        Some(Digest::of_bytes(format!("void-{seed}").as_bytes())),
        Q16::HALF,
        Q16::ONE,
    )
    .with_fingerprint(fingerprint)
    .certify(ReplayStatus::Replayable)
    .expect("clean candidate must certify");
    record
}

#[test]
fn store_records_are_offered_to_the_engine() {
    let (root, ws) = scratch("store-source");
    let store_path = root.join("cadlib.jsonl");

    // Build the CAD library: two certified crystals, appended under the
    // operator-approved policy the store requires for host writes.
    let mut store = CrystalRecordStore::open(&store_path).unwrap();
    let operator = PolicyProfile::operator_approved();
    store.append(certified_record("one"), &operator).unwrap();
    store.append(certified_record("two"), &operator).unwrap();

    let out = Command::new(bin())
        .arg(&ws)
        .arg("--json")
        .arg("--offer")
        .arg("--store")
        .arg(&store_path)
        .output()
        .expect("binary must run");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    assert_eq!(doc["store_loaded"], 2, "both store records loaded");
    let offers = doc["offers"].as_array().unwrap();
    let crystal_offers: Vec<_> = offers
        .iter()
        .filter(|o| o["kind"] == "CertifiedCrystal")
        .collect();
    assert_eq!(
        crystal_offers.len(),
        2,
        "both CAD-library crystals must be offered"
    );
    for offer in &crystal_offers {
        let label = offer["label"].as_str().unwrap();
        assert!(label.starts_with("crystal:python:"), "label: {label}");
        // Engine verdicts only — never a skip, never a CROSS-006 rejection.
        let outcome = &offer["outcome"];
        let verdict = outcome
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| outcome.to_string());
        assert!(
            verdict.contains("Accepted") || verdict.contains("Deferred"),
            "store crystal must reach the engine cleanly, got {verdict}"
        );
    }

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn tampered_store_is_a_hard_error() {
    let (root, ws) = scratch("store-tampered");
    let store_path = root.join("cadlib.jsonl");
    let mut store = CrystalRecordStore::open(&store_path).unwrap();
    store
        .append(certified_record("x"), &PolicyProfile::operator_approved())
        .unwrap();

    // Flip a byte inside the stored record — content no longer matches record_id.
    let mut text = std::fs::read_to_string(&store_path).unwrap();
    text = text.replace("\"rho_coherence\":32768", "\"rho_coherence\":32769");
    std::fs::write(&store_path, text).unwrap();

    let out = Command::new(bin())
        .arg(&ws)
        .arg("--offer")
        .arg("--store")
        .arg(&store_path)
        .output()
        .expect("binary must run");
    assert!(
        !out.status.success(),
        "a tampered store must be a hard error"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("integrity"),
        "must name the failure: {stderr}"
    );

    std::fs::remove_dir_all(&root).ok();
}
