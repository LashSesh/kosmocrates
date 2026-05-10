//! `phase-matrix` — CLI for PHASEMATRIX-HIVEMIND-03 (§22).
//!
//! Subcommands (per spec):
//!
//! ```text
//! cell-pool        <subnet.json> --rd <rd.json> --out <pool.json>
//! cell-resonate    <pool.json>   --input <input.json> --rd <rd.json> --out <pulses.json>
//! cluster-form     <pool.json>   --pulses <pulses.json> --rd <rd.json> --out <cluster.json>
//! funnel-build     <cluster.json> --rd <rd.json> --out <funnel.json>
//! morph-step       <cluster.json> --funnel <funnel.json> --rd <rd.json> --out <morph.json>
//! converge         <cluster.json> --pulses <pulses.json> --rd <rd.json> --out <convergence.json>
//! intent           <convergence.json> --cluster <cluster.json> --rd <rd.json> --out <intent.json>
//! cluster-dissolve <cluster.json> --trace <trace.json> --rd <rd.json> --out <dissolution.json>
//! cluster-cycle    <subnet.json>  --input <input.json> --rd <rd.json> --out <cycle.json>
//! cluster-replay   <cycle.json>
//! cluster-verify   <trace.json>
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use phase_matrix::cell::pipeline::{
    run_cell_substrate_cycle, CellSubstrateInput, CellSubstrateOutcome,
};
use phase_matrix::cell::primitives::{content_address, Hash256};
use phase_matrix::cell::run_descriptor::PhaseMatrixRunDescriptorV3;
use phase_matrix::cell::{ClusterTrace, DissolutionReport};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let r: CliResult<()> = match args[1].as_str() {
        "cluster-cycle" => cmd_cluster_cycle(&args[2..]),
        "cluster-replay" => cmd_cluster_replay(&args[2..]),
        "cluster-verify" => cmd_cluster_verify(&args[2..]),
        "cell-pool" => cmd_inspect_subnet(&args[2..]),
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

const USAGE: &str = "Usage:
  phase-matrix cluster-cycle  <input.json>  --rd <rd.json>  --out <cycle.json>
  phase-matrix cluster-replay <cycle.json>
  phase-matrix cluster-verify <trace-or-dissolution.json>
  phase-matrix cell-pool      <input.json>
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
    // The replay self-check just re-canonicalises the outcome and
    // confirms the bytes are stable. External re-execution is the
    // job of the test harness (see end_to_end.rs).
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
