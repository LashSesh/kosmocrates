//! Integration tests for `pse-traverse-cli`.
//!
//! These tests spawn the compiled binary and verify end-to-end behaviour.
//! `CARGO_BIN_EXE_pse-traverse-cli` is set by Cargo's test harness and points
//! to the debug build of the binary.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pse-traverse-cli"))
}

fn problem_json() -> PathBuf {
    // Use the canonical minimal problem spec shipped with pse-traverse.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("..") // tools/
        .join("..") // workspace root
        .join("crates")
        .join("pse-traverse")
        .join("examples")
        .join("problem_minimal.json")
}

fn tmp_out(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    p.push(name);
    p
}

/// `plan` writes a valid JSON file containing a `steps` array.
#[test]
fn cli_plan_writes_report() {
    let out = tmp_out("plan_cli_test.json");
    let status = Command::new(bin())
        .args(["plan", "--problem", problem_json().to_str().unwrap(), "--out", out.to_str().unwrap()])
        .status()
        .expect("failed to spawn pse-traverse-cli plan");
    assert!(status.success(), "plan subcommand exited with {:?}", status.code());
    let bytes = fs::read(&out).expect("plan output file was not written");
    let v: serde_json::Value =
        serde_json::from_slice(&bytes).expect("plan output is not valid JSON");
    assert!(v.get("steps").is_some(), "plan JSON missing 'steps' field: {}", v);
}

/// Two consecutive `plan` runs on the same input produce byte-identical output.
#[test]
fn cli_plan_is_byte_identical_across_two_runs() {
    let out1 = tmp_out("plan_run1.json");
    let out2 = tmp_out("plan_run2.json");
    let problem = problem_json();

    for out in [&out1, &out2] {
        let status = Command::new(bin())
            .args(["plan", "--problem", problem.to_str().unwrap(), "--out", out.to_str().unwrap()])
            .status()
            .expect("failed to spawn pse-traverse-cli plan");
        assert!(status.success());
    }
    let b1 = fs::read(&out1).unwrap();
    let b2 = fs::read(&out2).unwrap();
    assert_eq!(b1, b2, "plan output differs between two runs");
}

/// `run` produces a valid JSON file that round-trips through `replay` without error.
#[test]
fn cli_run_then_replay_ok() {
    let run_out = tmp_out("run_replay_test.json");
    let problem = problem_json();

    let status = Command::new(bin())
        .args(["run", "--problem", problem.to_str().unwrap(), "--out", run_out.to_str().unwrap()])
        .status()
        .expect("failed to spawn pse-traverse-cli run");
    assert!(status.success(), "run subcommand failed");

    let replay_out = Command::new(bin())
        .args(["replay", "--run", run_out.to_str().unwrap()])
        .output()
        .expect("failed to spawn pse-traverse-cli replay");
    assert!(
        replay_out.status.success(),
        "replay failed: {}",
        String::from_utf8_lossy(&replay_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&replay_out.stdout);
    assert!(stdout.contains("replay ok"), "unexpected replay output: {}", stdout);
}
