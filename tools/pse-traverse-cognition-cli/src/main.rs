//! `pse-traverse-cognition` — CLI for the v0.1 panoptic phase cognition
//! kernel (PSE-TRAVERSE-COGNITION-01, §18).
//!
//! Subcommands (per spec):
//!
//! ```text
//! inspect       <input.json>
//! observe       <input.json>   --rd <rd.json> --out <cognition.json>
//! state5        <input.json>   --rd <rd.json> --out <state5.json>
//! memory-query  <input.json>   --rd <rd.json> --out <hits.json>
//! lattice       <input.json>   --rd <rd.json> --out <lattice.json>
//! puzzle        <input.json>   --rd <rd.json> --out <puzzle.json>
//! panorama      <input.json>   --rd <rd.json> --out <panorama.json>
//! calibrate     <bench.json>   --rd <rd.json> --out <cal.json>
//! trigger       <input.json>   --rd <rd.json> --out <trigger.json>
//! bundle        <run.json>                    --out <bundle.json>
//! replay        <run.json>     [--rd <rd.json>]
//! verify        <report.json>
//! ```
//!
//! Manual flag parsing — keeps the binary dependency-free.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use pse_traverse::canonical::canonical_bytes;
use pse_traverse::cognition::{
    pipeline::{run_cognition, CognitionInput, CognitionRunResult},
    spiral_memory::spiral_memory_query,
    CognitionRunDescriptor, CognitiveState5D, Fixed,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let result: CliResult<()> = match args[1].as_str() {
        "inspect" => cmd_inspect(&args[2..]),
        "observe" => cmd_observe(&args[2..]),
        "state5" => cmd_state5(&args[2..]),
        "memory-query" => cmd_memory_query(&args[2..]),
        "lattice" => cmd_lattice(&args[2..]),
        "puzzle" => cmd_puzzle(&args[2..]),
        "panorama" => cmd_panorama(&args[2..]),
        "calibrate" => cmd_calibrate(&args[2..]),
        "trigger" => cmd_trigger(&args[2..]),
        "bundle" => cmd_bundle(&args[2..]),
        "replay" => cmd_replay(&args[2..]),
        "verify" => cmd_verify(&args[2..]),
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
  pse-traverse-cognition inspect      <input.json>
  pse-traverse-cognition observe      <input.json> --rd <rd.json> --out <cognition.json>
  pse-traverse-cognition state5       <input.json> --rd <rd.json> --out <state5.json>
  pse-traverse-cognition memory-query <input.json> --rd <rd.json> --out <hits.json>
  pse-traverse-cognition lattice      <input.json> --rd <rd.json> --out <lattice.json>
  pse-traverse-cognition puzzle       <input.json> --rd <rd.json> --out <puzzle.json>
  pse-traverse-cognition panorama     <input.json> --rd <rd.json> --out <panorama.json>
  pse-traverse-cognition calibrate    <bench.json> --rd <rd.json> --out <cal.json>
  pse-traverse-cognition trigger      <input.json> --rd <rd.json> --out <trigger.json>
  pse-traverse-cognition bundle       <run.json> --out <bundle.json>
  pse-traverse-cognition replay       <run.json> [--rd <rd.json>]
  pse-traverse-cognition verify       <report.json>
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
    let bytes = canonical_bytes(value).map_err(|e| format!("canonical encode: {e}"))?;
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

fn cmd_inspect(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let input: CognitionInput = read_json(path)?;
    let payload = serde_json::json!({
        "null_center_id": input.null_center_id,
        "constraint_count": input.constraint_count,
        "logical_step": input.logical_step,
        "carrier_ids": input.carrier_ids,
    });
    let bytes = serde_jcs::to_vec(&payload).map_err(|e| format!("canonical encode: {e}"))?;
    println!("{}", String::from_utf8_lossy(&bytes));
    Ok(())
}

fn cmd_observe(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let rd_path = flag_value(args, "--rd")?;
    let input: CognitionInput = read_json(path)?;
    let rd: CognitionRunDescriptor = read_json(&rd_path)?;
    let result = run_cognition(&input, &rd).map_err(|e| format!("run: {e}"))?;
    write_canonical(&result, opt_flag_value(args, "--out"), "cognition run")
}

fn cmd_state5(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let rd_path = flag_value(args, "--rd")?;
    let input: CognitionInput = read_json(path)?;
    let rd: CognitionRunDescriptor = read_json(&rd_path)?;
    let result = run_cognition(&input, &rd).map_err(|e| format!("run: {e}"))?;
    write_canonical(&result.state5, opt_flag_value(args, "--out"), "state5")
}

fn cmd_memory_query(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let _rd_path = flag_value(args, "--rd")?;
    let input: CognitionInput = read_json(path)?;
    let c = &input.cognitive_components;
    let q = CognitiveState5D::from_components(
        c.psi.clone(),
        c.rho.clone(),
        c.omega.clone(),
        c.chi.clone(),
        c.tau.clone(),
    )
    .map_err(|e| e.to_string())?;
    let hs = spiral_memory_query(
        &q,
        &input.spiral_memory_candidates,
        &Fixed::quantize(2.0, 9).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    write_canonical(&hs, opt_flag_value(args, "--out"), "memory hits")
}

fn cmd_lattice(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let rd_path = flag_value(args, "--rd")?;
    let input: CognitionInput = read_json(path)?;
    let rd: CognitionRunDescriptor = read_json(&rd_path)?;
    let result = run_cognition(&input, &rd).map_err(|e| format!("run: {e}"))?;
    write_canonical(&result.lattice, opt_flag_value(args, "--out"), "lattice")
}

fn cmd_puzzle(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let rd_path = flag_value(args, "--rd")?;
    let input: CognitionInput = read_json(path)?;
    let rd: CognitionRunDescriptor = read_json(&rd_path)?;
    let result = run_cognition(&input, &rd).map_err(|e| format!("run: {e}"))?;
    write_canonical(&result.puzzle, opt_flag_value(args, "--out"), "puzzle")
}

fn cmd_panorama(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let rd_path = flag_value(args, "--rd")?;
    let input: CognitionInput = read_json(path)?;
    let rd: CognitionRunDescriptor = read_json(&rd_path)?;
    let result = run_cognition(&input, &rd).map_err(|e| format!("run: {e}"))?;
    write_canonical(&result.panorama, opt_flag_value(args, "--out"), "panorama")
}

fn cmd_calibrate(args: &[String]) -> CliResult<()> {
    use pse_traverse::cognition::{
        calibrate, evaluate_por_acceptance, FixpointCalibrationShell, PerformanceTriplet,
    };
    let path = args
        .first()
        .ok_or_else(|| "missing <bench.json>".to_string())?;
    #[derive(serde::Deserialize)]
    struct Bench {
        current: PerformanceTriplet,
        candidate: PerformanceTriplet,
        epsilon_s: Fixed,
        e_min: Fixed,
    }
    let bench: Bench = read_json(path)?;
    let _accepted = evaluate_por_acceptance(
        &bench.current,
        &bench.candidate,
        &bench.epsilon_s,
        &bench.e_min,
    );
    let shell: FixpointCalibrationShell = calibrate(
        pse_traverse::dynamic_state::Hash256::zero(),
        pse_traverse::dynamic_state::Hash256::zero(),
        bench.current,
        bench.candidate,
        &bench.epsilon_s,
        &bench.e_min,
    )
    .map_err(|e| e.to_string())?;
    write_canonical(&shell, opt_flag_value(args, "--out"), "calibration")
}

fn cmd_trigger(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let rd_path = flag_value(args, "--rd")?;
    let input: CognitionInput = read_json(path)?;
    let rd: CognitionRunDescriptor = read_json(&rd_path)?;
    let result = run_cognition(&input, &rd).map_err(|e| format!("run: {e}"))?;
    write_canonical(
        &result.handoff_gate,
        opt_flag_value(args, "--out"),
        "trigger",
    )
}

fn cmd_bundle(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <run.json>".to_string())?;
    let result: CognitionRunResult = read_json(path)?;
    let bundle = result
        .bundle
        .ok_or_else(|| "no bundle in run result (gate did not pass)".to_string())?;
    write_canonical(&bundle, opt_flag_value(args, "--out"), "bundle")
}

fn cmd_replay(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <run.json>".to_string())?;
    let original: CognitionRunResult = read_json(path)?;
    if let Some(rd_path) = opt_flag_value(args, "--rd") {
        let rd: CognitionRunDescriptor = read_json(&rd_path)?;
        let replayed = run_cognition(&original.input, &rd).map_err(|e| format!("replay: {e}"))?;
        let a = canonical_bytes(&replayed.report).map_err(|e| e.to_string())?;
        let b = canonical_bytes(&original.report).map_err(|e| e.to_string())?;
        if a != b {
            return Err("replay mismatch: report bytes differ".into());
        }
        println!(
            "cognition replay ok: report id {} byte-identical",
            replayed.report.report_id.hex()
        );
    } else {
        let cb = canonical_bytes(&original.report).map_err(|e| e.to_string())?;
        if cb.is_empty() {
            return Err("replay self-check: empty report bytes".into());
        }
        println!(
            "cognition replay (self-check) ok: report id {}",
            original.report.report_id.hex()
        );
    }
    Ok(())
}

fn cmd_verify(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <report.json>".to_string())?;
    let report: pse_traverse::cognition::CognitionReport = read_json(path)?;
    let recomputed = report.clone().with_id().map_err(|e| e.to_string())?;
    if recomputed.report_id != report.report_id {
        return Err(format!(
            "cognition report tampered: stored={} recomputed={}",
            report.report_id.hex(),
            recomputed.report_id.hex()
        ));
    }
    println!("cognition report ok: id {}", report.report_id.hex());
    Ok(())
}
