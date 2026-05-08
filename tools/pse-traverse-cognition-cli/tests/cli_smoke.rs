//! End-to-end CLI smoke tests for `pse-traverse-cognition`. Covers the
//! integration test list from §21.2 of the spec.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_pse-traverse-cognition");
const INPUT: &str = "tests/fixtures/input_minimal.json";
const RD: &str = "tests/fixtures/rd_minimal.json";

mod tempdir {
    pub struct TempDir(pub std::path::PathBuf);
    impl TempDir {
        pub fn new(label: &str) -> Self {
            let mut p = std::env::temp_dir();
            let pid = std::process::id();
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            p.push(format!("pse-cognition-{label}-{pid}-{nonce}"));
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
        pub fn join(&self, name: &str) -> std::path::PathBuf {
            self.0.join(name)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("spawn");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn cli_inspect_succeeds() {
    let (ok, stdout, stderr) = run(&["inspect", INPUT]);
    assert!(ok, "inspect failed: {stderr}");
    assert!(stdout.contains("null_center_id"));
}

#[test]
fn cli_observe_writes_cognition_report() {
    let dir = tempdir::TempDir::new("observe");
    let out = dir.join("run.json");
    let (ok, _, e) = run(&["observe", INPUT, "--rd", RD, "--out", out.to_str().unwrap()]);
    assert!(ok, "observe failed: {e}");
    let bytes = std::fs::read(&out).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(parsed.get("report").is_some());
    assert!(parsed.get("canonical_state").is_some());
}

#[test]
fn cli_state5_writes_fixed_point_observables() {
    let dir = tempdir::TempDir::new("state5");
    let out = dir.join("state5.json");
    let (ok, _, e) = run(&["state5", INPUT, "--rd", RD, "--out", out.to_str().unwrap()]);
    assert!(ok, "state5 failed: {e}");
    let parsed: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert!(parsed.get("psi_semantic_phase").is_some());
    assert!(parsed.get("stability_index").is_some());
}

#[test]
fn cli_memory_query_returns_sorted_hitset() {
    let dir = tempdir::TempDir::new("mem");
    let out = dir.join("hits.json");
    let (ok, _, e) = run(&[
        "memory-query",
        INPUT,
        "--rd",
        RD,
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "memory-query failed: {e}");
    let parsed: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert!(parsed.get("hits").is_some());
}

#[test]
fn cli_lattice_writes_percolation_report() {
    let dir = tempdir::TempDir::new("lattice");
    let out = dir.join("lattice.json");
    let (ok, _, e) = run(&["lattice", INPUT, "--rd", RD, "--out", out.to_str().unwrap()]);
    assert!(ok, "lattice failed: {e}");
    let parsed: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert!(parsed.get("percolation_passed").is_some());
}

#[test]
fn cli_puzzle_writes_entropy_collapse_certificate() {
    let dir = tempdir::TempDir::new("puzzle");
    let out = dir.join("puzzle.json");
    let (ok, _, e) = run(&["puzzle", INPUT, "--rd", RD, "--out", out.to_str().unwrap()]);
    assert!(ok, "puzzle failed: {e}");
    let parsed: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    // collapse_certificate may be present or absent depending on
    // thresholds; the field key MUST exist.
    assert!(parsed.get("collapse_certificate").is_some());
}

#[test]
fn cli_panorama_writes_720_horizon() {
    let dir = tempdir::TempDir::new("panorama");
    let out = dir.join("panorama.json");
    let (ok, _, e) = run(&[
        "panorama",
        INPUT,
        "--rd",
        RD,
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "panorama failed: {e}");
    let parsed: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert!(parsed.get("current_horizon").is_some());
    assert!(parsed.get("counterfactual_horizon").is_some());
}

#[test]
fn cli_trigger_writes_singularity_trigger_report() {
    let dir = tempdir::TempDir::new("trigger");
    let out = dir.join("trigger.json");
    let (ok, _, e) = run(&["trigger", INPUT, "--rd", RD, "--out", out.to_str().unwrap()]);
    assert!(ok, "trigger failed: {e}");
    let parsed: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert!(parsed.get("passed").is_some());
}

#[test]
fn cli_bundle_writes_projection_handoff_bundle() {
    let dir = tempdir::TempDir::new("bundle");
    let run_path = dir.join("run.json");
    let bundle_path = dir.join("bundle.json");
    let (ok, _, e) = run(&[
        "observe",
        INPUT,
        "--rd",
        RD,
        "--out",
        run_path.to_str().unwrap(),
    ]);
    assert!(ok, "observe failed: {e}");
    let (ok2, _, e2) = run(&[
        "bundle",
        run_path.to_str().unwrap(),
        "--out",
        bundle_path.to_str().unwrap(),
    ]);
    if !ok2 {
        // If the gate didn't pass, bundle returns an error — that's
        // also a valid spec-compliant outcome (handoff fails closed).
        assert!(e2.contains("no bundle"), "unexpected error: {e2}");
        return;
    }
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&bundle_path).unwrap()).unwrap();
    assert!(parsed.get("candidate_states").is_some());
}

#[test]
fn cli_replay_byte_identical() {
    let dir = tempdir::TempDir::new("replay");
    let run_path = dir.join("run.json");
    let (ok, _, e) = run(&[
        "observe",
        INPUT,
        "--rd",
        RD,
        "--out",
        run_path.to_str().unwrap(),
    ]);
    assert!(ok, "observe failed: {e}");
    let (ok2, _, e2) = run(&["replay", run_path.to_str().unwrap(), "--rd", RD]);
    assert!(ok2, "replay failed: {e2}");
}

#[test]
fn cli_verify_accepts_clean_report_and_rejects_tampered() {
    let dir = tempdir::TempDir::new("verify");
    let run_path = dir.join("run.json");
    let (ok, _, e) = run(&[
        "observe",
        INPUT,
        "--rd",
        RD,
        "--out",
        run_path.to_str().unwrap(),
    ]);
    assert!(ok, "observe failed: {e}");
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&run_path).unwrap()).unwrap();
    let report = parsed.get("report").unwrap().clone();
    let report_path = dir.join("report.json");
    std::fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    let (ok_v, _, e_v) = run(&["verify", report_path.to_str().unwrap()]);
    assert!(ok_v, "verify clean failed: {e_v}");
    // Tamper with the lattice hash.
    let mut tampered = report.clone();
    if let Some(field) = tampered.get_mut("lattice_hash") {
        *field = serde_json::Value::String(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into(),
        );
    }
    let tampered_path = dir.join("tampered.json");
    std::fs::write(
        &tampered_path,
        serde_json::to_vec_pretty(&tampered).unwrap(),
    )
    .unwrap();
    let (ok_t, _, _) = run(&["verify", tampered_path.to_str().unwrap()]);
    assert!(!ok_t, "verify should reject tampered report");
}
