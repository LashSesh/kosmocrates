//! End-to-end CLI smoke tests for `pse-eval-matrix`. Drives every
//! subcommand against the built-in `agent-cognition` preset.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_pse-eval-matrix");

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
            p.push(format!("pse-eval-{label}-{pid}-{nonce}"));
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
fn cli_init_emits_valid_spec_for_each_preset() {
    for preset in [
        "agent-cognition",
        "streaming-event-detection",
        "post-symbolic-ablation",
    ] {
        let dir = tempdir::TempDir::new(&format!("init-{preset}"));
        let out = dir.join("spec.json");
        let (ok, _, e) = run(&["init", "--template", preset, "--out", out.to_str().unwrap()]);
        assert!(ok, "init {preset} failed: {e}");
        let bytes = std::fs::read(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            parsed.get("spec_id").is_some(),
            "spec missing spec_id ({preset})"
        );
        assert!(parsed.get("variants").is_some());
        let (ok_v, _, e_v) = run(&["validate", "--spec", out.to_str().unwrap()]);
        assert!(ok_v, "validate {preset} failed: {e_v}");
    }
}

#[test]
fn cli_full_pipeline_init_plan_run_replay_score_report() {
    let dir = tempdir::TempDir::new("full");
    let spec = dir.join("spec.json");
    let plan = dir.join("plan.json");
    let bundle = dir.join("bundle.json");
    let summary = dir.join("summary.json");
    let md = dir.join("summary.md");

    // init
    let (ok, _, e) = run(&[
        "init",
        "--template",
        "agent-cognition",
        "--out",
        spec.to_str().unwrap(),
    ]);
    assert!(ok, "init failed: {e}");
    // plan
    let (ok, _, e) = run(&[
        "plan",
        "--spec",
        spec.to_str().unwrap(),
        "--out",
        plan.to_str().unwrap(),
    ]);
    assert!(ok, "plan failed: {e}");
    // run
    let (ok, _, e) = run(&[
        "run",
        "--spec",
        spec.to_str().unwrap(),
        "--plan",
        plan.to_str().unwrap(),
        "--out",
        bundle.to_str().unwrap(),
    ]);
    assert!(ok, "run failed: {e}");
    // replay (using the bundle file)
    let (ok, _, e) = run(&["replay", "--bundle", bundle.to_str().unwrap()]);
    assert!(ok, "replay failed: {e}");
    // score
    let (ok, _, e) = run(&[
        "score",
        "--spec",
        spec.to_str().unwrap(),
        "--bundle",
        bundle.to_str().unwrap(),
        "--out",
        summary.to_str().unwrap(),
    ]);
    assert!(ok, "score failed: {e}");
    // report markdown
    let (ok, _, e) = run(&[
        "report",
        "--summary",
        summary.to_str().unwrap(),
        "--format",
        "md",
        "--out",
        md.to_str().unwrap(),
    ]);
    assert!(ok, "report failed: {e}");
    let bytes = std::fs::read(&md).unwrap();
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("Variant summaries"));
    assert!(s.contains("Capability profiles"));
}

#[test]
fn cli_ablate_emits_ladder_for_b6_cognition() {
    let dir = tempdir::TempDir::new("ablate");
    let spec = dir.join("spec.json");
    let ladder = dir.join("ladder.json");
    let (ok, _, e) = run(&[
        "init",
        "--template",
        "agent-cognition",
        "--out",
        spec.to_str().unwrap(),
    ]);
    assert!(ok, "init failed: {e}");
    let (ok, _, e) = run(&[
        "ablate",
        "--spec",
        spec.to_str().unwrap(),
        "--base",
        "B6_Cognition",
        "--out",
        ladder.to_str().unwrap(),
    ]);
    assert!(ok, "ablate failed: {e}");
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&ladder).unwrap()).unwrap();
    assert!(parsed.get("variants").is_some());
}

#[test]
fn cli_compare_runs_against_summary() {
    let dir = tempdir::TempDir::new("compare");
    let spec = dir.join("spec.json");
    let plan = dir.join("plan.json");
    let bundle = dir.join("bundle.json");
    let summary = dir.join("summary.json");
    for cmd in [
        vec![
            "init",
            "--template",
            "post-symbolic-ablation",
            "--out",
            spec.to_str().unwrap(),
        ],
        vec![
            "plan",
            "--spec",
            spec.to_str().unwrap(),
            "--out",
            plan.to_str().unwrap(),
        ],
        vec![
            "run",
            "--spec",
            spec.to_str().unwrap(),
            "--plan",
            plan.to_str().unwrap(),
            "--out",
            bundle.to_str().unwrap(),
        ],
        vec![
            "score",
            "--spec",
            spec.to_str().unwrap(),
            "--bundle",
            bundle.to_str().unwrap(),
            "--out",
            summary.to_str().unwrap(),
        ],
    ] {
        let (ok, _, e) = run(&cmd.iter().map(|s| s.as_ref()).collect::<Vec<_>>());
        assert!(ok, "step {cmd:?} failed: {e}");
    }
    let (ok, stdout, e) = run(&[
        "compare",
        "--summary",
        summary.to_str().unwrap(),
        "--baseline",
        "B0_Baseline",
        "--spec",
        spec.to_str().unwrap(),
    ]);
    assert!(ok, "compare failed: {e}");
    assert!(stdout.contains("baseline = B0_Baseline"));
    assert!(stdout.contains("LMU"));
}
