//! CLI smoke tests for pse-validation-runner-cli (§15.3).

use std::path::Path;
use std::process::Command;

fn pse_validate_bin() -> std::path::PathBuf {
    // Use the binary built by cargo test.
    let mut path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    path.push("pse-validate");
    if !path.exists() {
        // Try one level up (deps/ subfolder in test binary location).
        path.pop();
        path.pop();
        path.push("pse-validate");
    }
    path
}

fn run_validate(args: &[&str]) -> std::process::Output {
    Command::new(pse_validate_bin())
        .args(args)
        .output()
        .expect("failed to run pse-validate")
}

#[test]
fn cli_help_exits_zero() {
    let out = run_validate(&["--help"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("pse-validate"),
        "help output missing tool name"
    );
}

#[test]
fn cli_init_writes_profile() {
    let dir = tempfile::tempdir().unwrap();
    let profile_path = dir
        .path()
        .join("default.json")
        .to_string_lossy()
        .to_string();
    let out = run_validate(&["init", "--profile", "smoke", "--out", &profile_path]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        Path::new(&profile_path).exists(),
        "profile file not written"
    );
    let content = std::fs::read_to_string(&profile_path).unwrap();
    assert!(content.contains("smoke"), "profile JSON missing name");
}

#[test]
fn cli_inspect_reads_profile() {
    let dir = tempfile::tempdir().unwrap();
    let profile_path = dir.path().join("p.json").to_string_lossy().to_string();

    // First init.
    run_validate(&["init", "--profile", "smoke", "--out", &profile_path]);

    // Then inspect.
    let out = run_validate(&["inspect", "--profile", &profile_path]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("profile_name"),
        "inspect output missing fields"
    );
}

#[test]
fn cli_run_smoke_completes() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("run_out").to_string_lossy().to_string();

    let out = run_validate(&["run", "--profile", "smoke", "--out", &out_dir, "--dry-run"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        Path::new(&out_dir).join("final/final_report.json").exists(),
        "final_report.json not created"
    );
    assert!(
        Path::new(&out_dir).join("final/final_report.md").exists(),
        "final_report.md not created"
    );
    assert!(
        Path::new(&out_dir).join("bundle_manifest.json").exists(),
        "bundle_manifest.json not created"
    );
}

#[test]
fn cli_report_writes_markdown() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("run_out").to_string_lossy().to_string();

    // Run first.
    run_validate(&["run", "--profile", "smoke", "--out", &out_dir, "--dry-run"]);

    // Report.
    let out = run_validate(&["report", "--run", &out_dir, "--format", "md"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("# PSE Validation Report"),
        "markdown missing header"
    );
    assert!(
        stdout.contains("## 9. Conclusion"),
        "markdown missing conclusion section"
    );
}

#[test]
fn cli_replay_validates_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("run_out").to_string_lossy().to_string();

    // Run first.
    run_validate(&["run", "--profile", "smoke", "--out", &out_dir, "--dry-run"]);

    // Replay.
    let out = run_validate(&["replay", "--run", &out_dir]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("PASSED"),
        "replay should pass for untampered bundle"
    );
}

#[test]
fn domain_profile_requires_manifest_flag() {
    // Running with --profile domain but no --domain-manifest must fail with a clear error.
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("run_out").to_string_lossy().to_string();

    let out = run_validate(&["run", "--profile", "domain", "--out", &out_dir]);
    assert!(
        !out.status.success(),
        "domain profile without --domain-manifest should fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("domain-manifest") || stderr.contains("domain manifest"),
        "error message should mention domain-manifest, got: {stderr}"
    );
}

#[test]
fn domain_profile_nonexistent_manifest_fails() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("run_out").to_string_lossy().to_string();

    let out = run_validate(&[
        "run",
        "--profile",
        "domain",
        "--domain-manifest",
        "/nonexistent/path/manifest.json",
        "--out",
        &out_dir,
    ]);
    assert!(
        !out.status.success(),
        "domain profile with missing manifest path should fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("manifest"),
        "error should mention missing manifest, got: {stderr}"
    );
}

#[test]
fn smoke_run_writes_verdict_json() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("run_out").to_string_lossy().to_string();

    let out = run_validate(&["run", "--profile", "smoke", "--out", &out_dir, "--dry-run"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let verdict_path = Path::new(&out_dir).join("verdict.json");
    assert!(verdict_path.exists(), "verdict.json should be written");

    let content = std::fs::read_to_string(&verdict_path).unwrap();
    assert!(
        content.contains("conclusion"),
        "verdict.json should contain 'conclusion'"
    );
}
