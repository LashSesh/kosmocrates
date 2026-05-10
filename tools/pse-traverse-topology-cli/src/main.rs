//! `pse-traverse-topology` — CLI for the TPT-MTL-04 topology layer.
//!
//! Subcommands (TPT-MTL §14 Reference CLI):
//!
//! ```text
//! inspect       <state.json>                              → JSON digest summary
//! window        --state <state.json> --rd <rd.json> --out <window.json>
//! axis          --window <window.json> --out <axis_report.json>
//! triangulate   --window <window.json> --out <mesh_holo.json>
//! lift          --window <window.json> --out <micro_fibers.json>
//! weight        --mesh <mesh.json> --fibers <fibers.json> --out <weighted.json>
//! refine        --mesh <mesh.json> --depth <N> --out <refined.json>
//! carrier       --mesh <mesh.json> --window <window.json> --out <carrier.json>
//! reinterpret   --mesh <mesh.json> --carrier <carrier.json> --out <reinterp.json>
//! gate          --window <w.json> --mesh <m.json> --fibers <f.json>
//!               --reinterpretation <r.json> --out <gate.json>
//! bundle        --gate <gate.json> --out <bundle.json>
//! replay        --bundle <bundle.json> --expect same-digest
//! verify        --bundle <bundle.json>
//! run           --state <state.json> --rd <rd.json> --out <bundle.json>   (full pipeline)
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use pse_traverse::topology::{
    build_axis_bridge_report, build_phase_space_window, carrier::evaluate_carrier, evolve_mesh,
    form_topological_crystal_candidate, gate_all, lift_micro_fibers, materialize_bundle,
    refine_recursive, reinterpret_mesh_to_panoptic, replay::ReplayManifest, run_tpt_mtl,
    seed_mesh_holo, weight_mesh_with_microfibers, AxisBridgeReport, CarrierContext, MeshHolo,
    MicroFiber, PhaseSpaceWindow, ReinterpretationReport, TptMtlBundle, TptMtlGateReport,
    TptMtlInput, TptMtlRunDescriptor,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let result: CliResult<()> = match args[1].as_str() {
        "inspect" => cmd_inspect(&args[2..]),
        "window" => cmd_window(&args[2..]),
        "axis" => cmd_axis(&args[2..]),
        "triangulate" => cmd_triangulate(&args[2..]),
        "lift" => cmd_lift(&args[2..]),
        "weight" => cmd_weight(&args[2..]),
        "refine" => cmd_refine(&args[2..]),
        "carrier" => cmd_carrier(&args[2..]),
        "reinterpret" => cmd_reinterpret(&args[2..]),
        "gate" => cmd_gate(&args[2..]),
        "bundle" => cmd_bundle(&args[2..]),
        "replay" => cmd_replay(&args[2..]),
        "verify" => cmd_verify(&args[2..]),
        "run" => cmd_run(&args[2..]),
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        other => {
            eprintln!("unknown subcommand: {other}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "Usage:
  pse-traverse-topology inspect       <state.json>
  pse-traverse-topology window        --state <state.json> --rd <rd.json> --out <window.json>
  pse-traverse-topology axis          --window <window.json> --out <axis_report.json>
  pse-traverse-topology triangulate   --window <window.json> --out <mesh_holo.json>
  pse-traverse-topology lift          --window <window.json> --out <micro_fibers.json>
  pse-traverse-topology weight        --mesh <mesh.json> --fibers <fibers.json> --out <weighted.json>
  pse-traverse-topology refine        --mesh <mesh.json> --depth <N> --out <refined.json>
  pse-traverse-topology carrier       --mesh <mesh.json> --window <window.json> --out <carrier.json>
  pse-traverse-topology reinterpret   --mesh <mesh.json> --carrier <carrier.json> --out <reinterp.json>
  pse-traverse-topology gate          --window <w.json> --mesh <m.json> --fibers <f.json> \\
                                      --reinterpretation <r.json> --out <gate.json>
  pse-traverse-topology bundle        --gate <gate.json> --out <bundle.json>
  pse-traverse-topology replay        --bundle <bundle.json> [--expect same-digest]
  pse-traverse-topology verify        --bundle <bundle.json>
  pse-traverse-topology run           --state <state.json> --rd <rd.json> --out <bundle.json>
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

fn write_json<T: serde::Serialize>(path: &str, value: &T) -> CliResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| format!("serialize: {e}"))?;
    fs::write(PathBuf::from(path), bytes).map_err(|e| format!("write {path}: {e}"))
}

fn default_rd() -> TptMtlRunDescriptor {
    TptMtlRunDescriptor::default_permissive()
}

// ── Subcommand implementations ───────────────────────────────────────────────

fn cmd_inspect(args: &[String]) -> CliResult<()> {
    if args.is_empty() {
        return Err("inspect: missing <state.json>".into());
    }
    let input: TptMtlInput = read_json(&args[0])?;
    let rd = default_rd();
    let rd_hash = rd.content_hash().map_err(|e| format!("rd hash: {e}"))?;
    let summary = serde_json::json!({
        "state_digest": input.state_digest,
        "sample_points": input.sample_points.len(),
        "rd_hash": rd_hash,
        "rdver": rd.rdver,
        "domain_profile": rd.domain_profile,
    });
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
    Ok(())
}

fn cmd_window(args: &[String]) -> CliResult<()> {
    let state_path = flag_value(args, "--state")?;
    let rd_path = opt_flag_value(args, "--rd");
    let out_path = flag_value(args, "--out")?;

    let input: TptMtlInput = read_json(&state_path)?;
    let rd: TptMtlRunDescriptor = if let Some(p) = rd_path {
        read_json(&p)?
    } else {
        default_rd()
    };

    let window = build_phase_space_window(&input, &rd)
        .map_err(|e| format!("build_phase_space_window: {e}"))?;
    write_json(&out_path, &window)
}

fn cmd_axis(args: &[String]) -> CliResult<()> {
    let window_path = flag_value(args, "--window")?;
    let out_path = flag_value(args, "--out")?;
    let window: PhaseSpaceWindow = read_json(&window_path)?;
    let rd = default_rd();
    let report = build_axis_bridge_report(&window, &rd)
        .map_err(|e| format!("build_axis_bridge_report: {e}"))?;
    write_json(&out_path, &report)
}

fn cmd_triangulate(args: &[String]) -> CliResult<()> {
    let window_path = flag_value(args, "--window")?;
    let out_path = flag_value(args, "--out")?;
    let window: PhaseSpaceWindow = read_json(&window_path)?;
    let rd = default_rd();
    let mesh = seed_mesh_holo(&window, &rd).map_err(|e| format!("seed_mesh_holo: {e}"))?;
    let (evolved, _) = evolve_mesh(mesh, &rd).map_err(|e| format!("evolve_mesh: {e}"))?;
    write_json(&out_path, &evolved)
}

fn cmd_lift(args: &[String]) -> CliResult<()> {
    let window_path = flag_value(args, "--window")?;
    let out_path = flag_value(args, "--out")?;
    let window: PhaseSpaceWindow = read_json(&window_path)?;
    let rd = default_rd();
    let mesh = seed_mesh_holo(&window, &rd).map_err(|e| format!("seed_mesh: {e}"))?;
    let (evolved, _) = evolve_mesh(mesh, &rd).map_err(|e| format!("evolve: {e}"))?;
    let fibers =
        lift_micro_fibers(&window, &evolved, &rd).map_err(|e| format!("lift_micro_fibers: {e}"))?;
    write_json(&out_path, &fibers)
}

fn cmd_weight(args: &[String]) -> CliResult<()> {
    let mesh_path = flag_value(args, "--mesh")?;
    let fibers_path = flag_value(args, "--fibers")?;
    let out_path = flag_value(args, "--out")?;
    let mesh: MeshHolo = read_json(&mesh_path)?;
    let fibers: Vec<MicroFiber> = read_json(&fibers_path)?;
    let rd = default_rd();
    let weighted = weight_mesh_with_microfibers(mesh, &fibers, &rd)
        .map_err(|e| format!("weight_mesh: {e}"))?;
    write_json(&out_path, &weighted)
}

fn cmd_refine(args: &[String]) -> CliResult<()> {
    let mesh_path = flag_value(args, "--mesh")?;
    let depth_str = flag_value(args, "--depth")?;
    let out_path = flag_value(args, "--out")?;
    let mesh: MeshHolo = read_json(&mesh_path)?;
    let depth: u32 = depth_str
        .parse()
        .map_err(|_| format!("depth must be u32, got '{depth_str}'"))?;
    let rd = default_rd();
    let (refined, _) =
        refine_recursive(mesh, &rd, depth).map_err(|e| format!("refine_recursive: {e}"))?;
    write_json(&out_path, &refined)
}

fn cmd_carrier(args: &[String]) -> CliResult<()> {
    let window_path = flag_value(args, "--window")?;
    let out_path = flag_value(args, "--out")?;
    let window: PhaseSpaceWindow = read_json(&window_path)?;
    let rd = default_rd();
    let report = evaluate_carrier(&window.carrier, &rd, false)
        .map_err(|e| format!("evaluate_carrier: {e}"))?;
    write_json(&out_path, &report)
}

fn cmd_reinterpret(args: &[String]) -> CliResult<()> {
    let mesh_path = flag_value(args, "--mesh")?;
    let carrier_path = flag_value(args, "--carrier")?;
    let out_path = flag_value(args, "--out")?;
    let mesh: MeshHolo = read_json(&mesh_path)?;
    let carrier_report = read_json(&carrier_path)?;
    let rd = default_rd();
    let reinterp = reinterpret_mesh_to_panoptic(&mesh, &carrier_report, &rd)
        .map_err(|e| format!("reinterpret: {e}"))?;
    write_json(&out_path, &reinterp)
}

fn cmd_gate(args: &[String]) -> CliResult<()> {
    let window_path = flag_value(args, "--window")?;
    let mesh_path = flag_value(args, "--mesh")?;
    let fibers_path = flag_value(args, "--fibers")?;
    let reinterp_path = flag_value(args, "--reinterpretation")?;
    let out_path = flag_value(args, "--out")?;

    let window: PhaseSpaceWindow = read_json(&window_path)?;
    let mesh: MeshHolo = read_json(&mesh_path)?;
    let fibers: Vec<MicroFiber> = read_json(&fibers_path)?;
    let reinterp: ReinterpretationReport = read_json(&reinterp_path)?;
    let rd = default_rd();
    let axis_report = build_axis_bridge_report(&window, &rd).map_err(|e| format!("axis: {e}"))?;
    let carrier_report =
        evaluate_carrier(&window.carrier, &rd, false).map_err(|e| format!("carrier: {e}"))?;
    let gate_report = gate_all(
        &window,
        &axis_report,
        &mesh,
        &fibers,
        &carrier_report,
        &reinterp,
        &rd,
        None,
        None,
    )
    .map_err(|e| format!("gate_all: {e}"))?;
    write_json(&out_path, &gate_report)
}

fn cmd_bundle(args: &[String]) -> CliResult<()> {
    let gate_path = flag_value(args, "--gate")?;
    let out_path = flag_value(args, "--out")?;
    let gate_report: TptMtlGateReport = read_json(&gate_path)?;
    let outcome_kind = gate_report.outcome_kind();
    let summary = serde_json::json!({
        "outcome": outcome_kind,
        "all_passed": gate_report.all_passed,
        "gate_report_id": gate_report.report_id,
    });
    write_json(&out_path, &summary)
}

fn cmd_replay(args: &[String]) -> CliResult<()> {
    let bundle_path = flag_value(args, "--bundle")?;
    let bundle: TptMtlBundle = read_json(&bundle_path)?;
    let manifest = ReplayManifest::from_bundle(&bundle).map_err(|e| format!("manifest: {e}"))?;
    // Verify the bundle against its own manifest (identity check).
    manifest
        .verify(&bundle)
        .map_err(|e| format!("replay verification failed: {e}"))?;
    println!("replay OK: bundle_id={}", bundle.bundle_id.hex());
    Ok(())
}

fn cmd_verify(args: &[String]) -> CliResult<()> {
    let bundle_path = flag_value(args, "--bundle")?;
    let bundle: TptMtlBundle = read_json(&bundle_path)?;
    let outcome = &bundle.outcome;
    let summary = serde_json::json!({
        "bundle_id": bundle.bundle_id,
        "outcome": outcome,
        "rd_hash": bundle.rd_hash,
        "window_hash": bundle.phase_space_window_hash,
        "mesh_hash": bundle.mesh_holo_hash,
        "trace_hash": bundle.trace_hash,
    });
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
    Ok(())
}

fn cmd_run(args: &[String]) -> CliResult<()> {
    let state_path = flag_value(args, "--state")?;
    let rd_path = opt_flag_value(args, "--rd");
    let out_path = flag_value(args, "--out")?;

    let input: TptMtlInput = read_json(&state_path)?;
    let rd: TptMtlRunDescriptor = if let Some(p) = rd_path {
        read_json(&p)?
    } else {
        default_rd()
    };

    let result = run_tpt_mtl(&input, &rd).map_err(|e| format!("run_tpt_mtl: {e}"))?;
    write_json(&out_path, &result.outcome.bundle)?;

    eprintln!(
        "outcome: {:?}  bundle_id: {}",
        result.outcome.kind,
        result.outcome.bundle.bundle_id.hex()
    );
    Ok(())
}
