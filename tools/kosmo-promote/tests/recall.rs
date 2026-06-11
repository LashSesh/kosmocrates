//! Recall: the anchored memory is queryable, tested against the real binary.
//!
//! `--recall <query>` runs Pfauenthron++ retrieval (`D = ψ·ρ·ω`) over the
//! ledger's crystals — the first time promoted substrate knowledge can be
//! *found* again, ranked, QTIC-labelled, and with its causal lineage.
//! Read-only by contract: recall never creates a ledger; a missing path is a
//! hard error, never a silent empty store.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_kosmo-promote")
}

/// Same resonant polyglot workspace the resonance/ledger tests use.
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

fn run_json(args: &[&str]) -> (serde_json::Value, std::process::Output) {
    let out = Command::new(bin())
        .arg("--json")
        .args(args)
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
fn recall_finds_anchored_knowledge() {
    let (root, ws) = scratch("recall");
    let ledger = root.join("il");
    let ledger_arg = ledger.to_string_lossy().to_string();
    let ws_arg = ws.to_string_lossy().to_string();

    // Anchor: crystallize the workspace's certified structure into the ledger.
    let (_doc, out) = run_json(&[
        &ws_arg,
        "--all-kinds",
        "--offer",
        "--batch",
        "--calibration",
        "substrate",
        "--ledger",
        &ledger_arg,
    ]);
    assert!(out.status.success(), "anchoring run must succeed");

    // Recall: the memory is queryable, ranked by D = ψ·ρ·ω.
    let (doc, out2) = run_json(&[
        "--recall",
        "missing test coverage for a module",
        "--ledger",
        &ledger_arg,
        "--top",
        "3",
    ]);
    assert!(out2.status.success(), "recall must succeed");
    assert!(doc["anchored_crystals"].as_u64().unwrap() >= 1);
    let results = doc["results"].as_array().unwrap();
    assert!(!results.is_empty(), "an anchored crystal must be findable");
    let top = &results[0];
    assert!(
        top["tripolar_score"].as_f64().unwrap() > 0.0,
        "Pfauenthron D must be positive for a hit"
    );
    assert_eq!(top["qtic_class"], 5, "the anchored crystal recalls as Q5");
    assert!(
        top["question"].as_str().unwrap().contains("kosmo-promote:"),
        "provenance (the promotion scope) travels with the memory"
    );
    // The tank carries content: recall surfaces the certified claim lines,
    // not just metadata — label/kind, candidate metadata, evidence provenance.
    let claims = top["claims"].as_array().expect("claims array present");
    assert!(!claims.is_empty(), "anchored knowledge must carry claims");
    let joined = claims
        .iter()
        .map(|c| c.as_str().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("certified") && joined.contains("candidate"),
        "the first claim states what was certified: {joined}"
    );
    assert!(
        joined.contains("evidence bundle"),
        "claims carry evidence provenance: {joined}"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn recall_requires_ledger() {
    let out = Command::new(bin())
        .args(["--recall", "anything"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "--recall without --ledger is a usage error"
    );
}

#[test]
fn recall_on_missing_ledger_is_hard_error_and_creates_nothing() {
    let root = std::env::temp_dir()
        .join("kosmo-promote-tests")
        .join("recall-missing");
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();
    let ledger = root.join("does-not-exist");
    let ledger_arg = ledger.to_string_lossy().to_string();

    let out = Command::new(bin())
        .args(["--recall", "anything", "--ledger", &ledger_arg])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "missing ledger is a hard error");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no ledger"),
        "must name the failure: {stderr}"
    );
    assert!(
        !ledger.exists(),
        "recall is read-only — it must never create a ledger"
    );

    std::fs::remove_dir_all(&root).ok();
}
