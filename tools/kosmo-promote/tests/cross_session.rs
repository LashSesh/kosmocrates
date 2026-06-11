//! Cross-session persistence of the promotion engine state, tested against the
//! real binary.
//!
//! `--state <path>` is the operator's write authorization: session N's crystal
//! archive warm-starts session N+1's pattern memory (the `pse-core`
//! cross-session mechanism). These tests pin the mechanics fail-closed:
//! report-only never writes, a corrupt archive is a hard error (never silently
//! cold-started over), and the archive round-trips losslessly.

use std::path::{Path, PathBuf};
use std::process::Command;

use pse_types::SemanticCrystal;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_kosmo-promote")
}

/// Fresh scratch dir with a tiny polyglot workspace inside.
fn scratch(tag: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join("kosmo-promote-tests").join(tag);
    std::fs::remove_dir_all(&root).ok();
    let ws = root.join("ws");
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::write(
        ws.join("fib.py"),
        "import os\n\ndef a(x):\n    return x\n\ndef b(y):\n    return y\n",
    )
    .unwrap();
    std::fs::write(
        ws.join("fib.rs"),
        "use std::io;\npub fn a(x: u32) -> u32 { x }\npub fn b(y: u32) -> u32 { y }\n",
    )
    .unwrap();
    (root, ws)
}

fn run_json(ws: &Path, extra: &[&str]) -> (serde_json::Value, std::process::Output) {
    let out = Command::new(bin())
        .arg(ws)
        .arg("--json")
        .args(extra)
        .output()
        .expect("binary must run");
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout must be JSON ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    });
    (doc, out)
}

#[test]
fn state_archive_round_trips_across_sessions() {
    let (root, ws) = scratch("round-trip");
    let state = root.join("archive.json");
    let state_arg = state.to_string_lossy().to_string();

    // Session 1 — cold start, offer, archive written.
    let (doc1, out1) = run_json(&ws, &["--all-kinds", "--offer", "--state", &state_arg]);
    assert!(out1.status.success(), "session 1 must succeed");
    assert_eq!(doc1["memory_loaded"], 0, "cold start has no prior memory");
    assert_eq!(
        doc1["state_written"], true,
        "offer + --state writes the archive"
    );
    assert!(state.exists(), "archive file must exist after session 1");

    // The archive is a valid, lossless crystal list.
    let bytes = std::fs::read(&state).unwrap();
    let archive: Vec<SemanticCrystal> =
        serde_json::from_slice(&bytes).expect("archive must parse as Vec<SemanticCrystal>");
    assert_eq!(
        archive.len() as u64,
        doc1["new_crystals"].as_u64().unwrap(),
        "archive content matches the session's reported crystal count"
    );

    // Session 2 — warm start from the archive.
    let (doc2, out2) = run_json(&ws, &["--all-kinds", "--offer", "--state", &state_arg]);
    assert!(out2.status.success(), "session 2 must succeed");
    assert_eq!(
        doc2["memory_loaded"].as_u64().unwrap(),
        archive.len() as u64,
        "session 2 loads exactly what session 1 archived"
    );
    assert_eq!(doc2["state_written"], true);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn report_only_never_writes_state() {
    let (root, ws) = scratch("report-only");
    let state = root.join("archive.json");
    let state_arg = state.to_string_lossy().to_string();

    let (doc, out) = run_json(&ws, &["--all-kinds", "--state", &state_arg]);
    assert!(out.status.success());
    assert_eq!(doc["mode"], "report-only");
    assert_eq!(doc["state_written"], false, "report-only must not write");
    assert!(
        !state.exists(),
        "no archive file may appear without --offer"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn corrupt_state_is_a_hard_error() {
    let (root, ws) = scratch("corrupt");
    let state = root.join("archive.json");
    std::fs::write(&state, b"{ definitely not a crystal array").unwrap();
    let state_arg = state.to_string_lossy().to_string();

    let out = Command::new(bin())
        .arg(&ws)
        .arg("--offer")
        .arg("--state")
        .arg(&state_arg)
        .output()
        .expect("binary must run");
    assert!(
        !out.status.success(),
        "a corrupt archive must be a hard error, not a silent cold start"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("corrupt"),
        "error must name the corruption: {stderr}"
    );
    // The corrupt file must not be overwritten.
    let bytes = std::fs::read(&state).unwrap();
    assert!(
        bytes.starts_with(b"{ definitely"),
        "corrupt archive must be left untouched for inspection"
    );

    std::fs::remove_dir_all(&root).ok();
}
