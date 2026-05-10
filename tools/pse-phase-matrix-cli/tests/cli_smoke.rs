//! End-to-end CLI smoke tests for `phase-matrix`. Drives the
//! cluster-cycle / cluster-replay / cluster-verify subcommands against
//! a deterministic synthetic input.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use phase_matrix::cell::pipeline::{CellSubstrateInput, CellSubstrateOutcome};
use phase_matrix::cell::primitives::Hash256;
use phase_matrix::cell::run_descriptor::{
    CellSubstratePolicies, CellSubstrateThresholds, PhaseMatrixRunDescriptorV3,
};

const BIN: &str = env!("CARGO_BIN_EXE_phase-matrix");

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
            p.push(format!("phase-matrix-{label}-{pid}-{nonce}"));
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
    let out = Command::new(BIN).args(args).output().expect("spawn");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run_descriptor() -> PhaseMatrixRunDescriptorV3 {
    PhaseMatrixRunDescriptorV3 {
        run_id: "phase.cli.smoke".into(),
        matrix_id: Hash256::zero(),
        parent_profile_hash: None,
        operator_versions: BTreeMap::new(),
        canonicalization_version: "phase-matrix-v0.3".into(),
        thresholds: CellSubstrateThresholds::permissive(),
        policies: CellSubstratePolicies::strict(),
        feature_flags: BTreeSet::new(),
        evidence_refs: vec![],
    }
}

fn synthetic_input() -> CellSubstrateInput {
    CellSubstrateInput::synthetic(Hash256::zero(), 3, 3).unwrap()
}

#[test]
fn cli_cluster_cycle_emits_canonical_outcome() {
    let dir = tempdir::TempDir::new("cycle");
    let input_path = dir.join("input.json");
    let rd_path = dir.join("rd.json");
    let cycle_path = dir.join("cycle.json");

    std::fs::write(&input_path, serde_json::to_vec(&synthetic_input()).unwrap()).unwrap();
    std::fs::write(&rd_path, serde_json::to_vec(&run_descriptor()).unwrap()).unwrap();

    let (ok, _, e) = run(&[
        "cluster-cycle",
        input_path.to_str().unwrap(),
        "--rd",
        rd_path.to_str().unwrap(),
        "--out",
        cycle_path.to_str().unwrap(),
    ]);
    assert!(ok, "cluster-cycle failed: {e}");
    let bytes = std::fs::read(&cycle_path).unwrap();
    let outcome: CellSubstrateOutcome = serde_json::from_slice(&bytes).unwrap();
    matches!(outcome, CellSubstrateOutcome::Completed(_));
    // No commit-layer artefacts must appear in the cell-substrate
    // output bytes (Verfassung §5.4 — substrate emits handoff
    // candidates only).
    let s = String::from_utf8_lossy(&bytes);
    assert!(!s.contains("SemanticCrystal"));
    assert!(!s.contains("FinalizedEmission"));
}

#[test]
fn cli_cluster_cycle_replay_is_byte_identical() {
    let dir = tempdir::TempDir::new("replay");
    let input_path = dir.join("input.json");
    let rd_path = dir.join("rd.json");
    let cycle_a = dir.join("a.json");
    let cycle_b = dir.join("b.json");

    std::fs::write(&input_path, serde_json::to_vec(&synthetic_input()).unwrap()).unwrap();
    std::fs::write(&rd_path, serde_json::to_vec(&run_descriptor()).unwrap()).unwrap();

    for out in [&cycle_a, &cycle_b] {
        let (ok, _, e) = run(&[
            "cluster-cycle",
            input_path.to_str().unwrap(),
            "--rd",
            rd_path.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ]);
        assert!(ok, "cluster-cycle failed: {e}");
    }
    let a = std::fs::read(&cycle_a).unwrap();
    let b = std::fs::read(&cycle_b).unwrap();
    assert_eq!(
        a, b,
        "two cycles over the same (input, rd) must be byte-identical"
    );

    let (ok, stdout, e) = run(&["cluster-replay", cycle_a.to_str().unwrap()]);
    assert!(ok, "cluster-replay failed: {e}");
    assert!(stdout.contains("cluster-replay ok"));
}

#[test]
fn cli_cluster_verify_accepts_trace_from_completed_cycle() {
    let dir = tempdir::TempDir::new("verify");
    let input_path = dir.join("input.json");
    let rd_path = dir.join("rd.json");
    let cycle_path = dir.join("cycle.json");
    let trace_path = dir.join("trace.json");

    std::fs::write(&input_path, serde_json::to_vec(&synthetic_input()).unwrap()).unwrap();
    std::fs::write(&rd_path, serde_json::to_vec(&run_descriptor()).unwrap()).unwrap();

    let (ok, _, e) = run(&[
        "cluster-cycle",
        input_path.to_str().unwrap(),
        "--rd",
        rd_path.to_str().unwrap(),
        "--out",
        cycle_path.to_str().unwrap(),
    ]);
    assert!(ok, "cluster-cycle failed: {e}");

    let outcome: CellSubstrateOutcome =
        serde_json::from_slice(&std::fs::read(&cycle_path).unwrap()).unwrap();
    let trace = match outcome {
        CellSubstrateOutcome::Completed(r) => r.trace,
        other => panic!("expected Completed, got {other:?}"),
    };
    std::fs::write(&trace_path, serde_json::to_vec(&trace).unwrap()).unwrap();

    let (ok, stdout, e) = run(&["cluster-verify", trace_path.to_str().unwrap()]);
    assert!(ok, "cluster-verify failed: {e}");
    assert!(stdout.contains("cluster-verify ok: trace id"));
}

#[test]
fn cli_cell_pool_inspects_subnet_membership() {
    let dir = tempdir::TempDir::new("pool");
    let input_path = dir.join("input.json");
    std::fs::write(&input_path, serde_json::to_vec(&synthetic_input()).unwrap()).unwrap();
    let (ok, stdout, e) = run(&["cell-pool", input_path.to_str().unwrap()]);
    assert!(ok, "cell-pool failed: {e}");
    assert!(stdout.contains("seed_cell_count"));
    assert!(stdout.contains("member_nodes"));
}

// ── PHASEMATRIX-HIVEMIND-03.1 Dual-Fabric Stitch Layer CLI smoke tests ────────

use phase_matrix::cell::stitcher::StitchRunDescriptor;

fn stitch_rd() -> StitchRunDescriptor {
    StitchRunDescriptor::default_for(&run_descriptor()).unwrap()
}

#[test]
fn cli_stitch_cycle_produces_bundle() {
    let dir = tempdir::TempDir::new("stitch-cycle");
    let rd_path = dir.join("rd.json");
    let stitch_rd_path = dir.join("stitch_rd.json");
    let input_path = dir.join("input.json");
    let cycle_path = dir.join("cycle.json");
    let bundle_path = dir.join("bundle.json");

    std::fs::write(&rd_path, serde_json::to_vec(&run_descriptor()).unwrap()).unwrap();
    std::fs::write(&stitch_rd_path, serde_json::to_vec(&stitch_rd()).unwrap()).unwrap();
    std::fs::write(&input_path, serde_json::to_vec(&synthetic_input()).unwrap()).unwrap();

    // First produce a cell cycle output.
    let (ok, _, e) = run(&[
        "cluster-cycle",
        input_path.to_str().unwrap(),
        "--rd",
        rd_path.to_str().unwrap(),
        "--out",
        cycle_path.to_str().unwrap(),
    ]);
    assert!(ok, "cluster-cycle failed: {e}");

    // Now run the stitch cycle against that cell output.
    let (ok, _, e) = run(&[
        "stitch-cycle",
        cycle_path.to_str().unwrap(),
        "--rd",
        stitch_rd_path.to_str().unwrap(),
        "--out",
        bundle_path.to_str().unwrap(),
    ]);
    assert!(ok, "stitch-cycle failed: {e}");
    assert!(bundle_path.exists(), "stitch bundle must be written");
}

#[test]
fn cli_stitch_replay_passes_for_fresh_bundle() {
    let dir = tempdir::TempDir::new("stitch-replay");
    let rd_path = dir.join("rd.json");
    let stitch_rd_path = dir.join("stitch_rd.json");
    let input_path = dir.join("input.json");
    let cycle_path = dir.join("cycle.json");
    let bundle_path = dir.join("bundle.json");

    std::fs::write(&rd_path, serde_json::to_vec(&run_descriptor()).unwrap()).unwrap();
    std::fs::write(&stitch_rd_path, serde_json::to_vec(&stitch_rd()).unwrap()).unwrap();
    std::fs::write(&input_path, serde_json::to_vec(&synthetic_input()).unwrap()).unwrap();

    let (ok, _, e) = run(&[
        "cluster-cycle",
        input_path.to_str().unwrap(),
        "--rd",
        rd_path.to_str().unwrap(),
        "--out",
        cycle_path.to_str().unwrap(),
    ]);
    assert!(ok, "cluster-cycle failed: {e}");

    let (ok, _, e) = run(&[
        "stitch-cycle",
        cycle_path.to_str().unwrap(),
        "--rd",
        stitch_rd_path.to_str().unwrap(),
        "--out",
        bundle_path.to_str().unwrap(),
    ]);
    assert!(ok, "stitch-cycle failed: {e}");

    let (ok, stdout, e) = run(&["stitch-replay", bundle_path.to_str().unwrap()]);
    assert!(ok, "stitch-replay failed: {e}");
    assert!(
        stdout.contains("stitch-replay ok"),
        "expected 'stitch-replay ok' in: {stdout}"
    );
}

#[test]
fn cli_stitch_cycle_replay_is_byte_identical() {
    let dir = tempdir::TempDir::new("stitch-idem");
    let rd_path = dir.join("rd.json");
    let stitch_rd_path = dir.join("stitch_rd.json");
    let input_path = dir.join("input.json");
    let cycle_path = dir.join("cycle.json");
    let bundle_a = dir.join("bundle_a.json");
    let bundle_b = dir.join("bundle_b.json");

    std::fs::write(&rd_path, serde_json::to_vec(&run_descriptor()).unwrap()).unwrap();
    std::fs::write(&stitch_rd_path, serde_json::to_vec(&stitch_rd()).unwrap()).unwrap();
    std::fs::write(&input_path, serde_json::to_vec(&synthetic_input()).unwrap()).unwrap();

    let (ok, _, e) = run(&[
        "cluster-cycle",
        input_path.to_str().unwrap(),
        "--rd",
        rd_path.to_str().unwrap(),
        "--out",
        cycle_path.to_str().unwrap(),
    ]);
    assert!(ok, "cluster-cycle failed: {e}");

    for out in [&bundle_a, &bundle_b] {
        let (ok, _, e) = run(&[
            "stitch-cycle",
            cycle_path.to_str().unwrap(),
            "--rd",
            stitch_rd_path.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ]);
        assert!(ok, "stitch-cycle failed: {e}");
    }

    let a = std::fs::read(&bundle_a).unwrap();
    let b = std::fs::read(&bundle_b).unwrap();
    assert_eq!(
        a, b,
        "two stitch cycles over the same (cell_report, rd) must be byte-identical"
    );
}
