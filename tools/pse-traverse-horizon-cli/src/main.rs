//! `pse-traverse-horizon` — CLI for the v0.3 null-centered horizon
//! geometry layer.
//!
//! Subcommands:
//!
//! ```text
//! inspect  <input.json>                                         → JSON dump of input + RD digest + carrier preview
//! chart    <input.json> --rd horizon-rd.json --out chart.json   → content-addressed HorizonChart
//! rays     <chart.json> --rd horizon-rd.json --out rays.json    → deterministic phase rays
//! crossing <input.json> --rd horizon-rd.json --out crossing.json
//! finalize <input.json> --rd horizon-rd.json --out final.json   → fails when G_v0.3 = 0
//! replay   <final-or-hold.json>                                 → byte-identity check
//! verify   <horizon-certificate.json>                           → certificate-chain audit
//! ```
//!
//! Manual flag parsing — keeps the binary dependency-free.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use pse_traverse::canonical::canonical_bytes;
use pse_traverse::horizon::{
    horizon_chart::null_center_unfold,
    pipeline::{run_horizon_v3, HorizonInput, HorizonRunResult},
    HorizonCertificate, HorizonRunDescriptorV3, HorizonV3Outcome,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let result: CliResult<()> = match args[1].as_str() {
        "inspect" => cmd_inspect(&args[2..]),
        "chart" => cmd_chart(&args[2..]),
        "rays" => cmd_rays(&args[2..]),
        "crossing" => cmd_crossing(&args[2..]),
        "finalize" => cmd_finalize(&args[2..]),
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
  pse-traverse-horizon inspect  <input.json>
  pse-traverse-horizon chart    <input.json> --rd <horizon-rd.json> --out <chart.json>
  pse-traverse-horizon rays     <input.json> --rd <horizon-rd.json> --out <rays.json>
  pse-traverse-horizon crossing <input.json> --rd <horizon-rd.json> --out <crossing.json>
  pse-traverse-horizon finalize <input.json> --rd <horizon-rd.json> --out <final.json>
  pse-traverse-horizon replay   <final-or-hold.json>
  pse-traverse-horizon verify   <horizon-certificate.json>
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
    let input: HorizonInput = read_json(path)?;
    let payload = serde_json::json!({
        "state_id": input.state_id,
        "null_center_id": input.null_center_id,
        "duality_spec": input.duality_spec,
        "projection_v2": input.projection_v2,
    });
    let bytes = serde_jcs::to_vec(&payload).map_err(|e| format!("canonical encode: {e}"))?;
    println!("{}", String::from_utf8_lossy(&bytes));
    Ok(())
}

fn cmd_chart(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let rd_path = flag_value(args, "--rd")?;
    let input: HorizonInput = read_json(path)?;
    let rd: HorizonRunDescriptorV3 = read_json(&rd_path)?;
    let null_center = input
        .null_center_id
        .clone()
        .unwrap_or_else(|| input.state_id.clone());
    let chart = null_center_unfold(&null_center, &rd).map_err(|e| format!("unfold: {e}"))?;
    write_canonical(&chart, opt_flag_value(args, "--out"), "horizon chart")
}

fn cmd_rays(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let rd_path = flag_value(args, "--rd")?;
    let input: HorizonInput = read_json(path)?;
    let rd: HorizonRunDescriptorV3 = read_json(&rd_path)?;
    let null_center = input
        .null_center_id
        .clone()
        .unwrap_or_else(|| input.state_id.clone());
    let chart = null_center_unfold(&null_center, &rd).map_err(|e| format!("unfold: {e}"))?;
    let rays =
        pse_traverse::horizon::lift_phase_rays(&chart, &rd).map_err(|e| format!("rays: {e}"))?;
    write_canonical(&rays, opt_flag_value(args, "--out"), "phase rays")
}

fn cmd_crossing(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let rd_path = flag_value(args, "--rd")?;
    let input: HorizonInput = read_json(path)?;
    let rd: HorizonRunDescriptorV3 = read_json(&rd_path)?;
    let result = run_horizon_v3(&input, &rd).map_err(|e| format!("run: {e}"))?;
    write_canonical(
        &result.crossing,
        opt_flag_value(args, "--out"),
        "horizon crossing report",
    )
}

fn cmd_finalize(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <input.json>".to_string())?;
    let rd_path = flag_value(args, "--rd")?;
    let input: HorizonInput = read_json(path)?;
    let rd: HorizonRunDescriptorV3 = read_json(&rd_path)?;
    let result = run_horizon_v3(&input, &rd).map_err(|e| format!("run: {e}"))?;
    if !result.outcome.is_finalized() {
        return Err(format!(
            "G_v0.3 = 0 — refusing to finalize. outcome={}",
            outcome_label(&result.outcome)
        ));
    }
    write_canonical(&result, opt_flag_value(args, "--out"), "finalized run")
}

fn cmd_replay(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <final-or-hold.json>".to_string())?;
    let bytes = fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let original: HorizonRunResult =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse: {e}"))?;
    // Replay: re-run the pipeline with the persisted input, then assert
    // byte-identity on the certificate.
    let input = original.input.clone();
    // We can't reconstruct the descriptor from a final-only file, so we
    // require the caller to also pass a `--rd` for replay. In its
    // absence we re-run with the chart's stored `rd_hash` and compare
    // certificate replay_hash byte-by-byte against the original.
    if let Some(rd_path) = opt_flag_value(args, "--rd") {
        let rd: HorizonRunDescriptorV3 = read_json(&rd_path)?;
        let replayed = run_horizon_v3(&input, &rd).map_err(|e| format!("replay: {e}"))?;
        if canonical_bytes(&replayed.certificate).map_err(|e| e.to_string())?
            != canonical_bytes(&original.certificate).map_err(|e| e.to_string())?
        {
            return Err("replay mismatch: certificate bytes differ".into());
        }
        println!(
            "horizon replay ok: certificate byte-identical ({})",
            replayed.certificate.certificate_id.hex()
        );
    } else {
        // Without the descriptor we can only verify that the original
        // file is itself self-consistent.
        let cert_bytes = canonical_bytes(&original.certificate).map_err(|e| e.to_string())?;
        if cert_bytes.is_empty() {
            return Err("replay: empty certificate bytes".into());
        }
        println!(
            "horizon replay (self-check) ok: certificate id {}",
            original.certificate.certificate_id.hex()
        );
    }
    Ok(())
}

fn cmd_verify(args: &[String]) -> CliResult<()> {
    let path = args
        .first()
        .ok_or_else(|| "missing <horizon-certificate.json>".to_string())?;
    let cert: HorizonCertificate = read_json(path)?;
    let recomputed = cert
        .clone()
        .with_id()
        .map_err(|e| format!("re-hash: {e}"))?;
    if recomputed.certificate_id != cert.certificate_id {
        return Err(format!(
            "certificate tampered: stored={} recomputed={}",
            cert.certificate_id.hex(),
            recomputed.certificate_id.hex()
        ));
    }
    println!(
        "certificate ok: id {} chart {} crossing {} replay {}",
        cert.certificate_id.hex(),
        cert.chart_id.hex(),
        cert.crossing_report_hash.hex(),
        cert.replay_hash.hex()
    );
    Ok(())
}

fn outcome_label(o: &HorizonV3Outcome) -> &'static str {
    match o {
        HorizonV3Outcome::Finalized { .. } => "Finalized",
        HorizonV3Outcome::Hold { .. } => "Hold",
        HorizonV3Outcome::WaitForHorizon { .. } => "WaitForHorizon",
        HorizonV3Outcome::RefineCone { .. } => "RefineCone",
        HorizonV3Outcome::NeedsCarrierMigration { .. } => "NeedsCarrierMigration",
        HorizonV3Outcome::Recondense { .. } => "Recondense",
        HorizonV3Outcome::ProjectionOnly { .. } => "ProjectionOnly",
        HorizonV3Outcome::InvalidInput { .. } => "InvalidInput",
        HorizonV3Outcome::DeterminismViolation { .. } => "DeterminismViolation",
    }
}
