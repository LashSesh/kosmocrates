//! Ensemble resonance: substrate knowledge actually crystallizes in the
//! cognition engine, end to end through the real binary.
//!
//! Singles deny resonance by construction (the engine forms graph edges
//! pairwise within a batch, so singleton batches keep the connectivity metric
//! `j` at zero). `--batch` co-observes the candidates; `--calibration
//! substrate` keeps the 8-fold conjunctive kairos gate fully armed while
//! letting it be the discriminant (the `preset_anomaly_detection` rationale
//! for carrier-physics consensus on untuned workloads). Under that explicit
//! operator choice, a polyglot workspace's certified structure commits real
//! `SemanticCrystal`s — Q3-certified, archived, and warm-starting the next
//! session.

use std::path::{Path, PathBuf};
use std::process::Command;

use pse_types::SemanticCrystal;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_kosmo-promote")
}

/// A polyglot workspace rich enough to form a resonant ensemble.
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

#[test]
fn substrate_calibration_batch_crystallizes() {
    let (root, ws) = scratch("resonance");
    let state = root.join("archive.json");
    let state_arg = state.to_string_lossy().to_string();

    let (doc, out) = run_json(
        &ws,
        &[
            "--all-kinds",
            "--offer",
            "--batch",
            "--calibration",
            "substrate",
            "--state",
            &state_arg,
        ],
    );
    assert!(out.status.success(), "run must succeed");
    assert_eq!(doc["calibration"], "substrate");

    // The gate snapshot must show live connectivity (the whole point of batch).
    let j = doc["last_gate"]["metrics"]["j"]["value"].as_f64().unwrap();
    assert!(
        j > 0.0,
        "batch co-observation must raise the j metric, got {j}"
    );

    // Substrate knowledge crystallized: accepted verdicts with Q3 certificates.
    let offers = doc["offers"].as_array().unwrap();
    let accepted: Vec<_> = offers
        .iter()
        .filter(|o| o["crystal_committed"] == true)
        .collect();
    assert!(
        !accepted.is_empty(),
        "the ensemble must commit at least one crystal, offers: {}",
        offers.len()
    );
    for offer in &accepted {
        assert_eq!(
            offer["qtic_class"], 3,
            "promotion-path crystals are Q3 (gate-passed, no IL anchor)"
        );
    }

    // The committed crystal landed in the archive — memory for the next session.
    let archive: Vec<SemanticCrystal> =
        serde_json::from_slice(&std::fs::read(&state).unwrap()).unwrap();
    assert!(
        !archive.is_empty(),
        "archive must hold the committed crystal"
    );

    // Session 2 warm-starts from it.
    let (doc2, out2) = run_json(
        &ws,
        &[
            "--all-kinds",
            "--offer",
            "--batch",
            "--calibration",
            "substrate",
            "--state",
            &state_arg,
        ],
    );
    assert!(out2.status.success());
    assert_eq!(
        doc2["memory_loaded"].as_u64().unwrap(),
        archive.len() as u64,
        "session 2 loads session 1's crystals into PatternMemory"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn default_calibration_stays_fail_closed() {
    // Without the explicit calibration choice, the sensor-tuned consensus
    // stays armed and nothing crystallizes — the conservative default holds.
    let (root, ws) = scratch("resonance-default");
    let (doc, out) = run_json(&ws, &["--all-kinds", "--offer", "--batch"]);
    assert!(out.status.success());
    assert_eq!(doc["calibration"], "default");
    let offers = doc["offers"].as_array().unwrap();
    assert!(
        offers.iter().all(|o| o["crystal_committed"] == false),
        "default calibration must not commit on this workload"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn bad_calibration_and_ticks_are_usage_errors() {
    let (root, ws) = scratch("resonance-usage");
    let out = Command::new(bin())
        .arg(&ws)
        .args(["--calibration", "yolo"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "unknown calibration → usage error"
    );
    let out2 = Command::new(bin())
        .arg(&ws)
        .args(["--ticks", "0"])
        .output()
        .unwrap();
    assert_eq!(out2.status.code(), Some(2), "--ticks 0 → usage error");
    std::fs::remove_dir_all(&root).ok();
}
