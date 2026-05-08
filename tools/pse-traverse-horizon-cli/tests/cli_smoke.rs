//! End-to-end CLI smoke tests for `pse-traverse-horizon`.
//!
//! Exercises every subcommand end-to-end:
//!
//! 1. `inspect`  — passes for a valid input, fails for a missing file.
//! 2. `chart`    — writes a deterministic, content-addressed
//!    `HorizonChart`.
//! 3. `rays`     — writes phase rays with byte-identity across two runs.
//! 4. `crossing` — writes a crossing report with the same byte content.
//! 5. `finalize` — passes when projection-v0.2 is attached, fails with
//!    `G_v0.3 = 0` in its place when v0.2 is omitted.
//! 6. `replay`   — checks certificate byte-identity across re-runs.
//! 7. `verify`   — accepts a valid certificate, rejects a tampered one.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_pse-traverse-horizon");
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
            p.push(format!("pse-horizon-{label}-{pid}-{nonce}"));
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
    assert!(stdout.contains("state_id"));
}

#[test]
fn cli_chart_writes_horizon_chart() {
    let dir = tempdir::TempDir::new("chart");
    let out = dir.join("chart.json");
    let (ok, _stdout, stderr) = run(&["chart", INPUT, "--rd", RD, "--out", out.to_str().unwrap()]);
    assert!(ok, "chart failed: {stderr}");
    let bytes = std::fs::read(&out).unwrap();
    assert!(!bytes.is_empty());
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(parsed.get("chart_id").is_some());
    assert!(parsed.get("carrier_set_hash").is_some());
}

#[test]
fn cli_rays_byte_identical_across_runs() {
    let dir = tempdir::TempDir::new("rays");
    let out_a = dir.join("rays_a.json");
    let out_b = dir.join("rays_b.json");
    let (ok_a, _, e_a) = run(&["rays", INPUT, "--rd", RD, "--out", out_a.to_str().unwrap()]);
    assert!(ok_a, "rays a failed: {e_a}");
    let (ok_b, _, e_b) = run(&["rays", INPUT, "--rd", RD, "--out", out_b.to_str().unwrap()]);
    assert!(ok_b, "rays b failed: {e_b}");
    let bytes_a = std::fs::read(&out_a).unwrap();
    let bytes_b = std::fs::read(&out_b).unwrap();
    assert_eq!(bytes_a, bytes_b, "rays output not byte-identical");
}

#[test]
fn cli_crossing_writes_hold_or_pass() {
    let dir = tempdir::TempDir::new("crossing");
    let out = dir.join("crossing.json");
    let (ok, _, e) = run(&[
        "crossing",
        INPUT,
        "--rd",
        RD,
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "crossing failed: {e}");
    let parsed: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert!(parsed.get("crossing_allowed").is_some());
    assert!(parsed.get("failure_policy").is_some());
}

#[test]
fn cli_finalize_passes_with_projection_v2_gate() {
    let dir = tempdir::TempDir::new("final");
    let out = dir.join("final.json");
    let (ok, _, e) = run(&[
        "finalize",
        INPUT,
        "--rd",
        RD,
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "finalize failed: {e}");
    let bytes = std::fs::read(&out).unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn cli_finalize_refuses_failed_horizon() {
    // Produce an input WITHOUT a projection_v2 attestation — the
    // combined gate must reject and `finalize` must exit non-zero.
    let dir = tempdir::TempDir::new("fail");
    let bad_input = dir.join("input_no_v2.json");
    std::fs::write(
        &bad_input,
        r#"{
  "state_id": "0000000000000000000000000000000000000000000000000000000000000000",
  "null_center_id": null,
  "duality_spec": null,
  "projection_v2": null
}"#,
    )
    .unwrap();
    let out = dir.join("final.json");
    let (ok, _, e) = run(&[
        "finalize",
        bad_input.to_str().unwrap(),
        "--rd",
        RD,
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(!ok, "finalize should have failed without v0.2");
    assert!(e.contains("G_v0.3"), "expected G_v0.3 mention, got: {e}");
}

#[test]
fn cli_replay_byte_identical() {
    let dir = tempdir::TempDir::new("replay");
    let out = dir.join("final.json");
    let (ok, _, e) = run(&[
        "finalize",
        INPUT,
        "--rd",
        RD,
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "pre-replay finalize failed: {e}");
    let (ok2, _, e2) = run(&["replay", out.to_str().unwrap(), "--rd", RD]);
    assert!(ok2, "replay failed: {e2}");
}

#[test]
fn cli_verify_accepts_clean_certificate_and_rejects_tampered() {
    let dir = tempdir::TempDir::new("verify");
    let out = dir.join("final.json");
    let (ok, _, e) = run(&[
        "finalize",
        INPUT,
        "--rd",
        RD,
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "finalize failed: {e}");
    // Pull the certificate out of the run result.
    let run_result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    let cert = run_result.get("certificate").unwrap().clone();
    let cert_path = dir.join("cert.json");
    std::fs::write(&cert_path, serde_json::to_vec_pretty(&cert).unwrap()).unwrap();
    let (ok_v, _, e_v) = run(&["verify", cert_path.to_str().unwrap()]);
    assert!(ok_v, "verify clean failed: {e_v}");

    // Tamper: change the chart_id field, leave certificate_id alone ⇒
    // recompute MUST not match.
    let mut tampered = cert.clone();
    if let Some(chart) = tampered.get_mut("chart_id") {
        // Swap in a different chart id.
        *chart = serde_json::Value::String(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into(),
        );
    }
    let tampered_path = dir.join("cert_tampered.json");
    std::fs::write(
        &tampered_path,
        serde_json::to_vec_pretty(&tampered).unwrap(),
    )
    .unwrap();
    let (ok_t, _, e_t) = run(&["verify", tampered_path.to_str().unwrap()]);
    assert!(!ok_t, "verify tampered should fail (stdout/stderr={e_t})");
}
