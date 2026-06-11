//! The Infinity-Ledger lift, tested against the real binary.
//!
//! `--ledger <path>` anchors every accepted crystal: a ledger block (the
//! canonical trace anchor), an IL-HDAG node (the 5D resonance tensor the
//! code-born structure acquires), and a path-invariance check. With those
//! anchors the QTIC certificate is no longer capped at Q3 — promoted
//! substrate knowledge reaches **Q5, full QTIC**, the same conformance class
//! the cognition system awards its own best memory. The flag is the
//! operator's host-write authorization; without `--offer` nothing is written.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_kosmo-promote")
}

/// Same resonant polyglot workspace the resonance tests use.
fn scratch(tag: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join("kosmo-promote-tests").join(tag);
    std::fs::remove_dir_all(&root).ok();
    let ws = root.join("ws");
    std::fs::create_dir_all(&ws).unwrap();
    let files: [(&str, &str); 5] = [
        (
            "fib.py",
            "import os\n\ndef a(x):\n    return x\n\ndef b(y):\n    return y\n",
        ),
        (
            "fib.rs",
            "use std::io;\npub fn a(x: u32) -> u32 { x }\npub fn b(y: u32) -> u32 { y }\n",
        ),
        (
            "fib.go",
            "package main\nimport \"fmt\"\nfunc a(n int) int { return n }\nfunc b(n int) int { return n }\n",
        ),
        (
            "fib.js",
            "function a(x) {\n  return x;\n}\nfunction b(y) {\n  return y;\n}\n",
        ),
        (
            "list.c",
            "#include <stdio.h>\nstruct Node { int v; };\nint sum(int a, int b) {\n    return a + b;\n}\n",
        ),
    ];
    for (name, content) in files {
        std::fs::write(ws.join(name), content).unwrap();
    }
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

fn block_files(ledger: &Path) -> usize {
    std::fs::read_dir(ledger.join("ledger"))
        .map(|d| {
            d.filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().starts_with("block_"))
                .count()
        })
        .unwrap_or(0)
}

#[test]
fn ledger_anchoring_lifts_to_full_qtic() {
    let (root, ws) = scratch("ledger-lift");
    let ledger = root.join("il");
    let ledger_arg = ledger.to_string_lossy().to_string();
    let args = [
        "--all-kinds",
        "--offer",
        "--batch",
        "--calibration",
        "substrate",
        "--ledger",
        &ledger_arg,
    ];

    let (doc, out) = run_json(&ws, &args);
    assert!(out.status.success(), "run must succeed");
    assert!(
        doc["ledger_commits"].as_u64().unwrap() >= 1,
        "at least one crystal anchored"
    );

    let offers = doc["offers"].as_array().unwrap();
    let accepted: Vec<_> = offers
        .iter()
        .filter(|o| o["crystal_committed"] == true)
        .collect();
    assert!(!accepted.is_empty(), "the ensemble must commit");
    for offer in &accepted {
        assert_eq!(
            offer["qtic_class"], 5,
            "anchored crystals reach full QTIC (Q5): block + HDAG node + path invariance"
        );
        assert!(
            offer["block_hash"].as_str().map(|h| !h.is_empty()) == Some(true),
            "every accepted offer carries its ledger block hash"
        );
    }

    // The anchors are real files: ledger block, IL-HDAG, index.
    assert_eq!(block_files(&ledger), 1, "one unique crystal → one block");
    assert!(
        ledger.join("hdag").join("hdag.json").exists(),
        "IL-HDAG persisted"
    );
    assert!(
        ledger.join("il_index.json").exists(),
        "ledger index persisted"
    );

    // Idempotent: the same crystal re-anchors to the same block, no growth.
    let first_hash = accepted[0]["block_hash"].as_str().unwrap().to_string();
    let (doc2, out2) = run_json(&ws, &args);
    assert!(out2.status.success());
    let offers2 = doc2["offers"].as_array().unwrap();
    let accepted2: Vec<_> = offers2
        .iter()
        .filter(|o| o["crystal_committed"] == true)
        .collect();
    assert_eq!(
        accepted2[0]["block_hash"].as_str().unwrap(),
        first_hash,
        "identical crystal → identical block hash (idempotent anchor)"
    );
    assert_eq!(
        block_files(&ledger),
        1,
        "re-anchoring must not grow the ledger"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn ledger_without_offer_writes_nothing() {
    let (root, ws) = scratch("ledger-report-only");
    let ledger = root.join("il");
    let ledger_arg = ledger.to_string_lossy().to_string();

    let (doc, out) = run_json(&ws, &["--all-kinds", "--ledger", &ledger_arg]);
    assert!(out.status.success());
    assert_eq!(doc["ledger_commits"], 0);
    assert!(
        !ledger.exists(),
        "no ledger directory may appear without --offer"
    );

    std::fs::remove_dir_all(&root).ok();
}
