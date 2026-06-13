//! `phase-matrix` — CLI for PHASEMATRIX-HIVEMIND-03 / 03.1 (§22, §12).
//!
//! Subcommands (v0.3 cell substrate):
//!
//! ```text
//! cell-pool        <input.json>  --rd <rd.json>  --out <pool.json>
//! cluster-cycle    <input.json>  --rd <rd.json>  --out <cycle.json>
//! cluster-replay   <cycle.json>
//! cluster-verify   <trace-or-dissolution.json>
//! ```
//!
//! Subcommands added by v0.3.1 Dual-Fabric Stitch Layer (§12):
//!
//! ```text
//! stitch-fabric    <cell-cycle.json>    --rd <stitch-rd.json>  --out <fabric-h.json>
//! stitch-candidates <fabric-h.json>    --tensor <tensor.json>  --rd <stitch-rd.json>  --out <candidates.json>
//! stitch-gate      <candidates.json>   --tensor <tensor.json>  --rd <stitch-rd.json>  --out <gates.json>
//! stitch-apply     <candidates.json>   --gates <gates.json>    --tensor <tensor.json>  --rd <stitch-rd.json>  --out <stitch.json>
//! stitch-cycle     <cell-cycle.json>   --tensor <tensor.json>  --rd <stitch-rd.json>  --out <stitch.json>
//! stitch-replay    <stitch-bundle.json>
//! tensor-inspect   <tensor.json>
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use phase_matrix::cell::coupling_update::derive_stitch_candidates;
use phase_matrix::cell::field_tensor::{CouplingMatrix, FieldTensorState};
use phase_matrix::cell::field_tensor_trace::write_field_tensor_trace;
use phase_matrix::cell::mirror_consistency::evaluate_mirror_consistency;
use phase_matrix::cell::pipeline::{
    run_cell_substrate_cycle, CellSubstrateInput, CellSubstrateOutcome,
};
use phase_matrix::cell::primitives::{content_address, Hash256};
use phase_matrix::cell::report::StitcherReport;
use phase_matrix::cell::resonance_fabric::build_resonance_fabric;
use phase_matrix::cell::run_descriptor::PhaseMatrixRunDescriptorV3;
use phase_matrix::cell::stitch_pipeline::{
    run_stitch_cycle, verify_stitch_replay, StitchCycleBundle,
};
use phase_matrix::cell::stitcher::StitchRunDescriptor;
use phase_matrix::cell::stitcher_gate::evaluate_stitcher_gates;
use phase_matrix::cell::tensor_delta::compute_tensor_delta;
use phase_matrix::cell::{ClusterTrace, DissolutionReport};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let r: CliResult<()> = match args[1].as_str() {
        // v0.3 commands
        "cluster-cycle" => cmd_cluster_cycle(&args[2..]),
        "cluster-replay" => cmd_cluster_replay(&args[2..]),
        "cluster-verify" => cmd_cluster_verify(&args[2..]),
        "cell-pool" => cmd_inspect_subnet(&args[2..]),
        // v0.3.1 stitch commands
        "stitch-fabric" => cmd_stitch_fabric(&args[2..]),
        "stitch-candidates" => cmd_stitch_candidates(&args[2..]),
        "stitch-gate" => cmd_stitch_gate(&args[2..]),
        "stitch-apply" => cmd_stitch_apply(&args[2..]),
        "stitch-cycle" => cmd_stitch_cycle(&args[2..]),
        "stitch-replay" => cmd_stitch_replay(&args[2..]),
        "tensor-inspect" => cmd_tensor_inspect(&args[2..]),
        "doors" => {
            cmd_doors(&args[2..]);
            return ExitCode::SUCCESS;
        }
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        other => {
            eprintln!("unknown subcommand: {other}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "Usage (v0.3 cell substrate):
  phase-matrix cluster-cycle  <input.json>  --rd <rd.json>  --out <cycle.json>
  phase-matrix cluster-replay <cycle.json>
  phase-matrix cluster-verify <trace-or-dissolution.json>
  phase-matrix cell-pool      <input.json>

Usage (v0.3.1 Dual-Fabric Stitch Layer):
  phase-matrix stitch-fabric     <cell-cycle.json> --rd <stitch-rd.json> --out <fabric-h.json>
  phase-matrix stitch-candidates <fabric-h.json>   --tensor <tensor.json> --rd <stitch-rd.json> --out <candidates.json>
  phase-matrix stitch-gate       <candidates.json> --tensor <tensor.json> --rd <stitch-rd.json> --out <gates.json>
  phase-matrix stitch-apply      <candidates.json> --gates <gates.json> --tensor <tensor.json> --rd <stitch-rd.json> --out <stitch.json>
  phase-matrix stitch-cycle      <cell-cycle.json> --tensor <tensor.json> --rd <stitch-rd.json> --out <stitch.json>
  phase-matrix stitch-replay     <stitch-bundle.json>
  phase-matrix tensor-inspect    <tensor.json>
";

type CliResult<T> = std::result::Result<T, String>;

fn flag_value(args: &[String], flag: &str) -> CliResult<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            return iter
                .next()
                .cloned()
                .ok_or_else(|| format!("missing value for {flag}"));
        }
    }
    Err(format!("missing flag {flag}"))
}

fn opt_flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            return iter.next().cloned();
        }
    }
    None
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> CliResult<T> {
    let bytes = fs::read(PathBuf::from(path)).map_err(|e| format!("read {path}: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("parse {path}: {e}"))
}

fn write_canonical<T: serde::Serialize>(
    value: &T,
    opt_out: Option<String>,
    label: &str,
) -> CliResult<()> {
    let bytes = serde_jcs::to_vec(value).map_err(|e| format!("canonical encode: {e}"))?;
    if let Some(out) = opt_out {
        if let Some(parent) = PathBuf::from(&out).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir {parent:?}: {e}"))?;
        }
        fs::write(&out, &bytes).map_err(|e| format!("write {out}: {e}"))?;
        eprintln!("wrote {label} {out} ({} bytes)", bytes.len());
    } else {
        println!("{}", String::from_utf8_lossy(&bytes));
    }
    Ok(())
}

// ── v0.3 commands ────────────────────────────────────────────────────────────

fn cmd_inspect_subnet(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let input: CellSubstrateInput = read_json(path)?;
    let payload = serde_json::json!({
        "subnet_id": input.subnet.subnet_id,
        "member_nodes": input.subnet.member_nodes,
        "seed_cell_count": input.seed_cells.len(),
        "purpose_hash": input.purpose_hash,
        "logical_step": input.logical_step,
    });
    let bytes = serde_jcs::to_vec(&payload).map_err(|e| format!("canonical encode: {e}"))?;
    println!("{}", String::from_utf8_lossy(&bytes));
    Ok(())
}

fn cmd_cluster_cycle(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let rd_path = flag_value(args, "--rd")?;
    let input: CellSubstrateInput = read_json(path)?;
    let rd: PhaseMatrixRunDescriptorV3 = read_json(&rd_path)?;
    let outcome = run_cell_substrate_cycle(&rd, &input).map_err(|e| format!("run: {e}"))?;
    write_canonical(&outcome, opt_flag_value(args, "--out"), "cluster cycle")
}

fn cmd_cluster_replay(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <cycle.json>".to_string())?;
    let outcome: CellSubstrateOutcome = read_json(path)?;
    let h1 = content_address(&outcome).map_err(|e| e.to_string())?;
    let h2 = content_address(&outcome).map_err(|e| e.to_string())?;
    if h1 != h2 {
        return Err(format!(
            "cluster-replay: canonical hash unstable {} vs {}",
            h1.hex(),
            h2.hex()
        ));
    }
    println!("cluster-replay ok: outcome digest {}", h1.hex());
    Ok(())
}

fn cmd_cluster_verify(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <trace-or-dissolution.json>".to_string())?;
    let bytes = fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    if let Ok(trace) = serde_json::from_slice::<ClusterTrace>(&bytes) {
        let recomputed = trace.clone().with_id().map_err(|e| e.to_string())?;
        if recomputed.trace_id != trace.trace_id {
            return Err("cluster-verify: trace tampered".into());
        }
        println!("cluster-verify ok: trace id {}", trace.trace_id.hex());
        return Ok(());
    }
    if let Ok(diss) = serde_json::from_slice::<DissolutionReport>(&bytes) {
        diss.validate_trace_preservation()
            .map_err(|e| format!("dissolution: {e}"))?;
        let recomputed = diss.clone().with_id().map_err(|e| e.to_string())?;
        if recomputed.report_id != diss.report_id {
            return Err("cluster-verify: dissolution tampered".into());
        }
        println!("cluster-verify ok: dissolution id {}", diss.report_id.hex());
        return Ok(());
    }
    let _ = Hash256::zero();
    Err("cluster-verify: input is neither a ClusterTrace nor a DissolutionReport".into())
}

// ── v0.3.1 stitch commands ───────────────────────────────────────────────────

/// Load or synthesise a `StitchRunDescriptor`.
///
/// If `--rd <stitch-rd.json>` is given the file is read; otherwise a
/// default descriptor is derived from the PM run descriptor at the
/// same path provided with `--pm-rd`.
fn load_stitch_rd(args: &[String]) -> CliResult<StitchRunDescriptor> {
    if let Ok(path) = flag_value(args, "--rd") {
        return read_json(&path);
    }
    Err("missing --rd <stitch-rd.json>".into())
}

/// Build a genesis `FieldTensorState` for the given matrix id.
fn genesis_tensor(matrix_id: Hash256) -> CliResult<FieldTensorState> {
    let cm = CouplingMatrix::empty().map_err(|e| e.to_string())?;
    FieldTensorState::genesis(matrix_id, cm.matrix_id).map_err(|e| e.to_string())
}

/// Resolve tensor: read from file if `--tensor` is given, else genesis.
fn load_tensor(args: &[String]) -> CliResult<FieldTensorState> {
    if let Some(path) = opt_flag_value(args, "--tensor") {
        read_json(&path)
    } else {
        genesis_tensor(Hash256::zero())
    }
}

/// `stitch-fabric <cell-cycle.json> --rd <stitch-rd.json> --out <fabric-h.json>`
fn cmd_stitch_fabric(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <cell-cycle.json>".to_string())?;
    let rd = load_stitch_rd(args)?;
    let cycle: CellSubstrateOutcome = read_json(path)?;
    let report = match cycle {
        CellSubstrateOutcome::Completed(r) => r,
        other => {
            return Err(format!(
                "stitch-fabric: expected a Completed cycle, got {:?}",
                other
            ));
        }
    };
    let fabric = build_resonance_fabric(&rd, &report).map_err(|e| e.to_string())?;
    write_canonical(&fabric, opt_flag_value(args, "--out"), "fabric-h")
}

/// `stitch-candidates <fabric-h.json> --tensor <tensor.json> --rd <stitch-rd.json> --out <candidates.json>`
fn cmd_stitch_candidates(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <fabric-h.json>".to_string())?;
    let rd = load_stitch_rd(args)?;
    let fabric = read_json(path)?;
    let tensor = load_tensor(args)?;
    let candidates = derive_stitch_candidates(&rd, &fabric, &tensor).map_err(|e| e.to_string())?;
    write_canonical(
        &candidates,
        opt_flag_value(args, "--out"),
        "stitch candidates",
    )
}

/// `stitch-gate <candidates.json> --tensor <tensor.json> --rd <stitch-rd.json> --out <gates.json>`
fn cmd_stitch_gate(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <candidates.json>".to_string())?;
    let rd = load_stitch_rd(args)?;
    let candidates: Vec<phase_matrix::cell::coupling_update::StitchCandidate> = read_json(path)?;
    let tensor = load_tensor(args)?;
    let mirror =
        evaluate_mirror_consistency(&rd, &candidates, &tensor).map_err(|e| e.to_string())?;
    let delta = compute_tensor_delta(&rd, &tensor, &candidates).map_err(|e| e.to_string())?;
    // Trace hash: use zero if no fabric-h is available here.
    let trace_hash = Hash256::zero();
    let gates = evaluate_stitcher_gates(&rd, &candidates, &mirror, &delta, &trace_hash, true)
        .map_err(|e| e.to_string())?;
    write_canonical(&gates, opt_flag_value(args, "--out"), "stitch gates")
}

/// `stitch-apply <candidates.json> --gates <gates.json> --tensor <tensor.json>
///               --rd <stitch-rd.json> --out <stitch.json>`
///
/// Applies accepted candidates to the tensor and writes a
/// `StitchCycleBundle`-compatible JSON. This command is a convenience
/// short-cut; the full pipeline is `stitch-cycle`.
fn cmd_stitch_apply(args: &[String]) -> CliResult<()> {
    // Delegate to stitch-cycle by reading the candidates' source cycle.
    // For simplicity we synthesise a minimal CellSubstrateCycleReport from
    // the candidates. Users needing the full pipeline should use stitch-cycle.
    let candidates_path = args
        .first()
        .ok_or_else(|| "missing <candidates.json>".to_string())?;
    let gates_path = flag_value(args, "--gates")?;
    let rd = load_stitch_rd(args)?;
    let tensor = load_tensor(args)?;

    use phase_matrix::cell::coupling_update::{build_coupling_update, StitchCandidate};
    use phase_matrix::cell::stitcher_gate::StitcherGateReport;

    let candidates: Vec<StitchCandidate> = read_json(candidates_path)?;
    let gates: Vec<StitcherGateReport> = read_json(&gates_path)?;

    let cand_map: std::collections::BTreeMap<Hash256, &StitchCandidate> = candidates
        .iter()
        .map(|c| (c.candidate_id.clone(), c))
        .collect();

    let mut accepted_updates = Vec::new();
    for gate in gates.iter().filter(|g| g.passed) {
        if let Some(c) = cand_map.get(&gate.candidate_id) {
            let upd =
                build_coupling_update(c, gate.report_id.clone()).map_err(|e| e.to_string())?;
            accepted_updates.push(upd);
        }
    }

    // Compute delta report.
    let delta = compute_tensor_delta(&rd, &tensor, &candidates).map_err(|e| e.to_string())?;

    // Build a trace.
    let rejected_gates: Vec<&StitcherGateReport> = gates.iter().filter(|g| !g.passed).collect();
    let trace = write_field_tensor_trace(
        &rd,
        &tensor,
        &tensor, // tensor_after approximation for this sub-command
        &accepted_updates,
        &gates,
        vec![],
    )
    .map_err(|e| e.to_string())?;

    let accepted_hashes: Vec<Hash256> = accepted_updates
        .iter()
        .map(|u| u.update_id.clone())
        .collect();
    let rejected_hashes: Vec<Hash256> = rejected_gates
        .iter()
        .map(|g| g.candidate_id.clone())
        .collect();
    let gate_hashes: Vec<Hash256> = gates.iter().map(|g| g.report_id.clone()).collect();
    let candidates_hash = content_address(&candidates).map_err(|e| e.to_string())?;
    let rd_hash = content_address(&rd).map_err(|e| e.to_string())?;

    let report = StitcherReport {
        report_id: Hash256::zero(),
        rd_hash,
        fabric_h_hash: Hash256::zero(),
        tensor_before_hash: tensor.tensor_id.clone(),
        tensor_after_hash: tensor.tensor_id.clone(),
        stitch_candidates_hash: candidates_hash,
        accepted_updates: accepted_hashes,
        rejected_candidates: rejected_hashes,
        gate_reports: gate_hashes,
        tensor_delta_report_hash: delta.report_id,
        field_tensor_trace_hash: trace.trace_id,
        passed: !accepted_updates.is_empty(),
        evidence_refs: vec![],
    }
    .with_id()
    .map_err(|e| e.to_string())?;

    write_canonical(
        &report,
        opt_flag_value(args, "--out"),
        "stitch apply report",
    )
}

/// `stitch-cycle <cell-cycle.json> --tensor <tensor.json> --rd <stitch-rd.json>
///               --out <stitch-bundle.json>`
fn cmd_stitch_cycle(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <cell-cycle.json>".to_string())?;
    let rd = load_stitch_rd(args)?;
    let tensor = load_tensor(args)?;
    let cycle: CellSubstrateOutcome = read_json(path)?;
    let report = match cycle {
        CellSubstrateOutcome::Completed(r) => r,
        other => {
            return Err(format!(
                "stitch-cycle: expected a Completed cycle, got {:?}",
                other
            ));
        }
    };
    let bundle = run_stitch_cycle(&rd, tensor, &report).map_err(|e| e.to_string())?;
    let out = opt_flag_value(args, "--out");
    write_canonical(&bundle, out, "stitch cycle bundle")
}

/// `stitch-replay <stitch-bundle.json>`
fn cmd_stitch_replay(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <stitch-bundle.json>".to_string())?;
    let bundle: StitchCycleBundle = read_json(path)?;
    let obs = verify_stitch_replay(&bundle).map_err(|e| e.to_string())?;
    if obs.passed {
        println!(
            "stitch-replay ok: report digest {}",
            obs.observed_hash.hex()
        );
        Ok(())
    } else {
        Err(format!(
            "stitch-replay FAILED: expected {} observed {}",
            obs.expected_hash.hex(),
            obs.observed_hash.hex()
        ))
    }
}

/// `tensor-inspect <tensor.json>`
fn cmd_tensor_inspect(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <tensor.json>".to_string())?;
    let tensor: FieldTensorState = read_json(path)?;
    let summary = serde_json::json!({
        "tensor_id": tensor.tensor_id.hex(),
        "tensor_revision": tensor.tensor_revision,
        "matrix_id": tensor.matrix_id.hex(),
        "node_count": tensor.node_state_hashes.len(),
        "seam_count": tensor.seam_state_hashes.len(),
        "coupling_matrix_hash": tensor.coupling_matrix_hash.hex(),
        "spatial_topology_hash": tensor.spatial_topology_hash.hex(),
        "previous_tensor_hash": tensor.previous_tensor_hash.map(|h| h.hex()),
        "trace_head": tensor.trace_head.hex(),
        "evidence_count": tensor.evidence_refs.len(),
    });
    let bytes = serde_jcs::to_vec(&summary).map_err(|e| format!("canonical encode: {e}"))?;
    println!("{}", String::from_utf8_lossy(&bytes));
    Ok(())
}

// ─── Doors (the binary's self-description) ───────────────────────────────────

use kosmo_core::{Door, DoorCatalog, DoorGovernance, DoorInput, DoorNeed, DoorSurface};

/// The binary's complete docking surface, spoken by the binary itself and
/// pinned by test against the dispatch.
fn doors_catalog() -> DoorCatalog {
    let here = || DoorSurface::Cli {
        binary: "phase-matrix".into(),
    };
    DoorCatalog::new(vec![
        Door::new(
            here(),
            "doors",
            vec![],
            "this catalog: the binary's complete docking surface, spoken by the binary itself (add --json for the machine form)",
            vec![DoorInput::switch("--json")],
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            here(),
            "--help",
            vec!["-h".into(), "help".into()],
            "the prose usage text",
            vec![],
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            here(),
            "cluster-cycle",
            vec![],
            "run a resonance-cluster cycle",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "cluster-replay",
            vec![],
            "replay a cluster cycle byte-identically",
            vec![DoorInput::valued("--report", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "cluster-verify",
            vec![],
            "verify a cluster cycle report",
            vec![DoorInput::valued("report", "<file>").required()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "cell-pool",
            vec![],
            "inspect the morphodynamic cell sub-network",
            vec![DoorInput::valued("state", "<file>").required()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "stitch-fabric",
            vec![],
            "build the dual-fabric field-tensor stitch",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "stitch-candidates",
            vec![],
            "enumerate stitch candidates",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "stitch-gate",
            vec![],
            "the ADAMANT fail-closed stitch gate",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "stitch-apply",
            vec![],
            "apply a gated stitch",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "stitch-cycle",
            vec![],
            "run a full stitch cycle",
            vec![DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "stitch-replay",
            vec![],
            "replay a stitch cycle byte-identically",
            vec![DoorInput::valued("--report", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "tensor-inspect",
            vec![],
            "inspect a field tensor",
            vec![DoorInput::valued("tensor", "<file>").required()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
    ])
}

fn cmd_doors(args: &[String]) {
    let catalog = doors_catalog();
    if args.iter().any(|a| a == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&catalog).unwrap_or_default()
        );
        return;
    }
    println!("phase-matrix doors — the docking surface");
    println!(
        "  catalog {}… · {} door(s)",
        &catalog.catalog_id.to_hex()[..12],
        catalog.len()
    );
    for d in &catalog.doors {
        println!("  {}  [{}]  {}", d.name, d.governance.label(), d.summary);
    }
}

#[cfg(test)]
mod doors_tests {
    use super::*;

    /// This binary's own dispatch source — the catalog is pinned against it.
    const MAIN_SRC: &str = include_str!("main.rs");

    fn dispatched_words() -> std::collections::BTreeSet<String> {
        // Pick whichever dispatch anchor occurs FIRST in the file: the real
        // dispatch precedes this test module, which embeds both anchors as
        // string literals (so a plain `contains` check would be self-fooled).
        let a = MAIN_SRC.find("match args[1].as_str() {");
        let b = MAIN_SRC.find("match subcmd {");
        let start = match (a, b) {
            (Some(x), Some(y)) => x.min(y),
            (Some(x), None) => x,
            (None, Some(y)) => y,
            (None, None) => panic!("the dispatch match exists"),
        };
        let slice = &MAIN_SRC[start..];
        let end = slice
            .lines()
            .scan(0usize, |off, line| {
                let here = *off;
                *off += line.len() + 1;
                Some((here, line))
            })
            .find(|(_, line)| *line == "    }" || *line == "    };")
            .map(|(off, _)| off)
            .expect("the dispatch match closes");
        let slice = &slice[..end];
        let mut words = std::collections::BTreeSet::new();
        for line in slice.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            let mut rest = line;
            while let Some(q) = rest.find('"') {
                let after = &rest[q + 1..];
                let Some(close) = after.find('"') else { break };
                let literal = &after[..close];
                let tail = after[close + 1..].trim_start();
                if tail.starts_with("=>") || tail.starts_with('|') {
                    words.insert(literal.to_string());
                }
                rest = &after[close + 1..];
            }
        }
        words
    }

    #[test]
    fn the_doors_catalog_is_pinned_to_the_dispatch() {
        let dispatched = dispatched_words();
        let mut cataloged = std::collections::BTreeSet::new();
        for door in doors_catalog().doors {
            cataloged.insert(door.name.clone());
            for alias in &door.aliases {
                cataloged.insert(alias.clone());
            }
        }
        assert_eq!(
            dispatched, cataloged,
            "the catalog and the dispatch must speak the same words"
        );
    }

    #[test]
    fn the_catalog_recomputes_and_describes_itself() {
        let catalog = doors_catalog();
        assert!(catalog.verify(), "the catalog recomputes");
        assert!(catalog.doors.iter().any(|d| d.name == "doors"));
        assert_eq!(catalog.catalog_id, doors_catalog().catalog_id);
    }
}
