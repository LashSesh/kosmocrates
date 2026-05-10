//! CLI smoke tests for pse-traverse-topology.

use std::process::Command;

fn topology_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("pse-traverse-topology");
    path
}

fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn cli_help_exits_zero() {
    let out = Command::new(topology_bin())
        .arg("--help")
        .output()
        .expect("failed to run pse-traverse-topology");
    assert!(
        out.status.success(),
        "help should exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn cli_inspect_minimal_input() {
    let input = fixtures_dir().join("input_minimal.json");
    let out = Command::new(topology_bin())
        .args(["inspect", input.to_str().unwrap()])
        .output()
        .expect("failed to run pse-traverse-topology inspect");
    assert!(
        out.status.success(),
        "inspect should succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("rdver") || stdout.contains("domain_profile"),
        "inspect should print JSON, got: {stdout}"
    );
}

#[test]
fn cli_run_pipeline_end_to_end() {
    let input = fixtures_dir().join("input_minimal.json");
    let out_path = std::env::temp_dir().join("tpt_mtl_smoke_bundle.json");

    let out = Command::new(topology_bin())
        .args([
            "run",
            "--state",
            input.to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run pse-traverse-topology run");

    assert!(
        out.status.success(),
        "run should succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let bundle_bytes = std::fs::read(&out_path).expect("bundle output should exist");
    let bundle: serde_json::Value =
        serde_json::from_slice(&bundle_bytes).expect("bundle should be valid JSON");
    assert!(
        bundle.get("bundle_id").is_some(),
        "bundle should have bundle_id"
    );
    assert!(
        bundle.get("outcome").is_some(),
        "bundle should have outcome"
    );
}

#[test]
fn cli_run_is_deterministic() {
    let input = fixtures_dir().join("input_minimal.json");
    let out1 = std::env::temp_dir().join("tpt_mtl_smoke_det1.json");
    let out2 = std::env::temp_dir().join("tpt_mtl_smoke_det2.json");

    for out in [&out1, &out2] {
        let status = Command::new(topology_bin())
            .args([
                "run",
                "--state",
                input.to_str().unwrap(),
                "--out",
                out.to_str().unwrap(),
            ])
            .status()
            .expect("failed to run");
        assert!(status.success());
    }

    let b1: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&out1).unwrap()).unwrap();
    let b2: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&out2).unwrap()).unwrap();

    assert_eq!(
        b1["bundle_id"], b2["bundle_id"],
        "CLI run must be deterministic"
    );
}

#[test]
fn cli_replay_verifies_own_bundle() {
    let input = fixtures_dir().join("input_minimal.json");
    let out = std::env::temp_dir().join("tpt_mtl_smoke_replay.json");

    let run_status = Command::new(topology_bin())
        .args([
            "run",
            "--state",
            input.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(run_status.success());

    let replay_out = Command::new(topology_bin())
        .args(["replay", "--bundle", out.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        replay_out.status.success(),
        "replay should succeed, stderr: {}",
        String::from_utf8_lossy(&replay_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&replay_out.stdout);
    assert!(stdout.contains("replay OK"));
}
