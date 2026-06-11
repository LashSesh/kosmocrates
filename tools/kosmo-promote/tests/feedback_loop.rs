//! The memory→action loop, tested against the real binary.
//!
//! `--feedback <path>` carries the engine's verdicts back into the substrate:
//! session N's `PromotionFeedback` is loaded by session N+1 and fed into the
//! pipeline's `prior_feedback` (norm fitness, Step 5c — the "Wissen zurück ins
//! Substrat" path). Same authorization discipline as `--state`: the flag is
//! the write authorization, report-only never writes, corruption is a hard
//! error.

use std::path::{Path, PathBuf};
use std::process::Command;

use kosmo_core::{FeedbackOutcome, PromotionFeedback, Q16};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_kosmo-promote")
}

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
fn feedback_round_trips_and_feeds_the_next_run() {
    let (root, ws) = scratch("feedback-loop");
    let fb_path = root.join("feedback.json");
    let fb_arg = fb_path.to_string_lossy().to_string();

    // Session 1 — offer; engine verdicts become feedback.
    let (doc1, out1) = run_json(&ws, &["--all-kinds", "--offer", "--feedback", &fb_arg]);
    assert!(out1.status.success(), "session 1 must succeed");
    assert_eq!(
        doc1["feedback_loaded"], 0,
        "cold start has no prior feedback"
    );
    assert_eq!(doc1["feedback_written"], true);
    let offered = doc1["offered"].as_u64().unwrap();
    assert!(offered > 0, "the polyglot workspace must yield candidates");

    // One verdict per offered candidate, content-addressed and honest:
    // engine-deferred candidates carry the ¼ fitness signal, never more.
    let bytes = std::fs::read(&fb_path).unwrap();
    let pool: Vec<PromotionFeedback> =
        serde_json::from_slice(&bytes).expect("must parse as Vec<PromotionFeedback>");
    assert_eq!(pool.len() as u64, offered, "one feedback per offer");
    for fb in &pool {
        assert_eq!(fb.outcome, FeedbackOutcome::Deferred, "engine deferred");
        assert_eq!(
            fb.fitness_signal,
            Q16::ratio(1, 4).unwrap(),
            "Deferred → ¼ fitness, never full credit"
        );
        assert_ne!(
            fb.evidence_bundle_id.to_hex(),
            kosmo_core::Digest::ZERO.to_hex()
        );
    }

    // Session 2 — the verdicts feed the next run's pipeline.
    let (doc2, out2) = run_json(&ws, &["--all-kinds", "--offer", "--feedback", &fb_arg]);
    assert!(out2.status.success(), "session 2 must succeed");
    assert_eq!(
        doc2["feedback_loaded"].as_u64().unwrap(),
        pool.len() as u64,
        "session 2 loads exactly what session 1 wrote"
    );
    // Idempotent merge: same candidates, same verdicts → same pool size.
    let pool2: Vec<PromotionFeedback> =
        serde_json::from_slice(&std::fs::read(&fb_path).unwrap()).unwrap();
    assert_eq!(
        pool2.len(),
        pool.len(),
        "re-offering identical verdicts must not grow the pool"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn report_only_never_writes_feedback() {
    let (root, ws) = scratch("feedback-report-only");
    let fb_path = root.join("feedback.json");
    let fb_arg = fb_path.to_string_lossy().to_string();

    let (doc, out) = run_json(&ws, &["--all-kinds", "--feedback", &fb_arg]);
    assert!(out.status.success());
    assert_eq!(doc["feedback_written"], false, "report-only must not write");
    assert!(
        !fb_path.exists(),
        "no feedback file may appear without --offer"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn corrupt_feedback_is_a_hard_error() {
    let (root, ws) = scratch("feedback-corrupt");
    let fb_path = root.join("feedback.json");
    std::fs::write(&fb_path, b"[ not feedback").unwrap();
    let fb_arg = fb_path.to_string_lossy().to_string();

    let out = Command::new(bin())
        .arg(&ws)
        .arg("--offer")
        .arg("--feedback")
        .arg(&fb_arg)
        .output()
        .expect("binary must run");
    assert!(
        !out.status.success(),
        "corrupt feedback must be a hard error"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("corrupt"),
        "must name the corruption: {stderr}"
    );

    std::fs::remove_dir_all(&root).ok();
}
