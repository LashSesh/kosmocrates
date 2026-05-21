//! `pse-traverse-lpcm` — CLI for the Fragmented 51% Condensation Layer.
//!
//! Subcommands (§17 Reference CLI):
//!
//! ```text
//! inspect   <window.json>                               → summary stats
//! partition <window.json> --out <fragments.json>        → fragmented window
//! collapse  <window.json> [--rd <rd.json>] --out <report.json>
//! replay    --rd <rd.json> --input <window.json> --report <report.json>
//! verify    <report.json>                               → replay identity check
//! ablate    <window.json> [--without-lpcm] [--with-lpcm] --out <ablation.json>
//! ```

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use pse_traverse::lpcm::{
    evidence::LpcmInputWindow,
    pipeline::run_lpcm,
    replay::verify_lpcm_replay,
    run_descriptor::{LpcmPolicies, LpcmRunDescriptor, LpcmThresholds},
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    match args[1].as_str() {
        "inspect" => run(cmd_inspect(&args[2..])),
        "partition" => run(cmd_partition(&args[2..])),
        "collapse" => run(cmd_collapse(&args[2..])),
        "replay" => run(cmd_replay(&args[2..])),
        "verify" => run(cmd_verify(&args[2..])),
        "ablate" => run(cmd_ablate(&args[2..])),
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown subcommand: {other}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run(r: Result<(), String>) -> ExitCode {
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

// ── inspect ──────────────────────────────────────────────────────────────────

fn cmd_inspect(args: &[String]) -> Result<(), String> {
    let path = args.first().ok_or("usage: inspect <window.json>")?;
    let window = load_window(path)?;
    let rd = default_rd(&window);
    let report = run_lpcm(rd, window.clone()).map_err(|e| format!("lpcm error: {e}"))?;

    let summary = serde_json::json!({
        "window_id": window.window_id,
        "evidence_refs": window.evidence_refs.len(),
        "fragment_count": report.metrics.fragment_count,
        "triangulable_patch_rate": report.metrics.triangulable_patch_rate,
        "local_condensation_rate": report.metrics.local_condensation_rate,
        "percolation_path_count": report.metrics.percolation_path_count,
        "strongest_path_score": report.metrics.strongest_path_score,
        "outcome": format!("{:?}", report.outcome),
        "report_id": report.report_id,
    });

    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
    Ok(())
}

// ── partition ─────────────────────────────────────────────────────────────────

fn cmd_partition(args: &[String]) -> Result<(), String> {
    let (window_path, out_path) =
        parse_in_out(args, "partition <window.json> --out <fragments.json>")?;
    let window = load_window(&window_path)?;
    let rd = default_rd(&window);

    let fragmented = pse_traverse::lpcm::partition::partition_window(&rd, &window)
        .map_err(|e| format!("partition error: {e}"))?;

    let summary = serde_json::json!({
        "window_id": fragmented.window_id,
        "source_window_hash": fragmented.source_window_hash,
        "fragment_count": fragmented.fragments.len(),
        "fragments": fragmented.fragments.iter().map(|f| serde_json::json!({
            "fragment_id": f.fragment_id,
            "ordinal": f.ordinal,
            "evidence_ref_count": f.evidence_refs.len(),
        })).collect::<Vec<_>>(),
        "partition_report_ref": fragmented.partition_report_ref,
    });

    write_json(&out_path, &summary)?;
    eprintln!(
        "Partition written to {out_path}  ({} fragments)",
        fragmented.fragments.len()
    );
    Ok(())
}

// ── collapse ──────────────────────────────────────────────────────────────────

fn cmd_collapse(args: &[String]) -> Result<(), String> {
    let mut window_path = String::new();
    let mut rd_path: Option<String> = None;
    let mut out_path = String::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--rd" => {
                i += 1;
                rd_path = Some(args.get(i).ok_or("--rd requires a path")?.clone());
            }
            "--out" => {
                i += 1;
                out_path = args.get(i).ok_or("--out requires a path")?.clone();
            }
            p if !p.starts_with('-') && window_path.is_empty() => window_path = p.to_string(),
            other => return Err(format!("unexpected argument: {other}")),
        }
        i += 1;
    }
    if window_path.is_empty() {
        return Err("usage: collapse <window.json> [--rd rd.json] --out report.json".into());
    }
    if out_path.is_empty() {
        return Err("--out is required".into());
    }

    let window = load_window(&window_path)?;
    let rd = if let Some(p) = rd_path {
        let text = fs::read_to_string(&p).map_err(|e| format!("cannot read {p}: {e}"))?;
        serde_json::from_str::<LpcmRunDescriptor>(&text)
            .map_err(|e| format!("invalid run descriptor: {e}"))?
    } else {
        default_rd(&window)
    };

    let report = run_lpcm(rd, window).map_err(|e| format!("lpcm error: {e}"))?;

    write_json(&out_path, &report)?;
    eprintln!(
        "Collapse report written to {out_path}  outcome={:?}  report_id={}",
        report.outcome,
        serde_json::to_string(&report.report_id).unwrap_or_default()
    );
    Ok(())
}

// ── replay ────────────────────────────────────────────────────────────────────

fn cmd_replay(args: &[String]) -> Result<(), String> {
    let mut rd_path = String::new();
    let mut input_path = String::new();
    let mut report_path = String::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--rd" => {
                i += 1;
                rd_path = args.get(i).ok_or("--rd requires a path")?.clone();
            }
            "--input" => {
                i += 1;
                input_path = args.get(i).ok_or("--input requires a path")?.clone();
            }
            "--report" => {
                i += 1;
                report_path = args.get(i).ok_or("--report requires a path")?.clone();
            }
            other => return Err(format!("unexpected argument: {other}")),
        }
        i += 1;
    }
    if rd_path.is_empty() || input_path.is_empty() || report_path.is_empty() {
        return Err(
            "usage: replay --rd <rd.json> --input <window.json> --report <collapse_report.json>"
                .into(),
        );
    }

    let rd: LpcmRunDescriptor =
        serde_json::from_str(&fs::read_to_string(&rd_path).map_err(|e| format!("{rd_path}: {e}"))?)
            .map_err(|e| format!("invalid rd: {e}"))?;

    let window = load_window(&input_path)?;

    let expected: pse_traverse::lpcm::report::HierarchicalCollapseReport = serde_json::from_str(
        &fs::read_to_string(&report_path).map_err(|e| format!("{report_path}: {e}"))?,
    )
    .map_err(|e| format!("invalid report: {e}"))?;

    let replay =
        verify_lpcm_replay(&rd, &window, &expected).map_err(|e| format!("replay error: {e}"))?;

    println!("{}", serde_json::to_string_pretty(&replay).unwrap());

    if !replay.hash_identical {
        eprintln!("REPLAY MISMATCH — report hashes differ");
        return Err("replay mismatch".into());
    }
    eprintln!("Replay OK — byte-identical");
    Ok(())
}

// ── verify ────────────────────────────────────────────────────────────────────

fn cmd_verify(args: &[String]) -> Result<(), String> {
    let path = args.first().ok_or("usage: verify <collapse_report.json>")?;
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let report: pse_traverse::lpcm::report::HierarchicalCollapseReport =
        serde_json::from_str(&text).map_err(|e| format!("invalid report: {e}"))?;

    let summary = serde_json::json!({
        "report_id": report.report_id,
        "outcome": format!("{:?}", report.outcome),
        "fragment_count": report.metrics.fragment_count,
        "percolation_path_count": report.metrics.percolation_path_count,
        "replay_identity": report.metrics.replay_identity,
        "local_condensation_rate": report.metrics.local_condensation_rate,
        "coarse_grain_refs": report.coarse_grain_refs.len(),
        "local_candidate_refs": report.local_candidate_refs.len(),
    });

    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
    eprintln!("Report structure valid");
    Ok(())
}

// ── ablate ────────────────────────────────────────────────────────────────────

fn cmd_ablate(args: &[String]) -> Result<(), String> {
    let (window_path, out_path) = parse_in_out(args, "ablate <window.json> --out <ablation.json>")?;
    let window = load_window(&window_path)?;

    // Run with LPCM.
    let rd_lpcm = default_rd(&window);
    let report_lpcm =
        run_lpcm(rd_lpcm, window.clone()).map_err(|e| format!("lpcm run failed: {e}"))?;

    // Baseline: run with topology-guard disabled (no candidates → DiagnosticHold).
    let mut rd_base = default_rd(&window);
    rd_base.policies.topology_policy.require_triangulable = false;
    rd_base.thresholds.theta_activate = pse_traverse::lpcm::primitives::fixed_from_f64(2.0); // unreachable → no condensations
    let report_base =
        run_lpcm(rd_base, window.clone()).map_err(|e| format!("baseline run failed: {e}"))?;

    let ablation = serde_json::json!({
        "window": window.window_id,
        "without_lpcm": {
            "outcome": format!("{:?}", report_base.outcome),
            "local_condensation_rate": report_base.metrics.local_condensation_rate,
            "percolation_path_count": report_base.metrics.percolation_path_count,
        },
        "with_lpcm": {
            "outcome": format!("{:?}", report_lpcm.outcome),
            "local_condensation_rate": report_lpcm.metrics.local_condensation_rate,
            "percolation_path_count": report_lpcm.metrics.percolation_path_count,
            "report_id": report_lpcm.report_id,
        },
        "delta": {
            "percolation_paths_gained": report_lpcm.metrics.percolation_path_count as i64
                - report_base.metrics.percolation_path_count as i64,
        },
    });

    write_json(&out_path, &ablation)?;
    eprintln!("Ablation written to {out_path}");
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn load_window(path: &str) -> Result<LpcmInputWindow, String> {
    if path == "-" {
        // Read from stdin.
        let mut buf = String::new();
        use std::io::Read;
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("stdin: {e}"))?;
        return serde_json::from_str(&buf).map_err(|e| format!("invalid window JSON: {e}"));
    }
    if !Path::new(path).exists() {
        return Err(format!("file not found: {path}"));
    }
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("invalid window JSON in {path}: {e}"))
}

fn default_rd(window: &LpcmInputWindow) -> LpcmRunDescriptor {
    LpcmRunDescriptor {
        run_id: format!("cli-run-{}", &window.window_id),
        lpcm_version: LpcmRunDescriptor::lpcm_version_tag().to_string(),
        source_window_hash: window.window_hash.clone(),
        source_topology_hash: window.source_topology_hash.clone(),
        seed: 0,
        thresholds: LpcmThresholds::default(),
        policies: LpcmPolicies::default(),
        operator_versions: BTreeMap::new(),
        evidence_refs: vec![],
    }
}

fn parse_in_out(args: &[String], usage: &str) -> Result<(String, String), String> {
    let mut input = String::new();
    let mut output = String::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                output = args.get(i).ok_or("--out requires a path")?.clone();
            }
            p if !p.starts_with('-') && input.is_empty() => input = p.to_string(),
            other => return Err(format!("unexpected argument: {other}")),
        }
        i += 1;
    }
    if input.is_empty() || output.is_empty() {
        return Err(format!("usage: {usage}"));
    }
    Ok((input, output))
}

fn write_json<T: serde::Serialize>(path: &str, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| format!("serialization: {e}"))?;
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
    }
    fs::write(path, text).map_err(|e| format!("write {path}: {e}"))?;
    Ok(())
}

const USAGE: &str = "\
Usage:
  pse-traverse-lpcm inspect   <window.json>
  pse-traverse-lpcm partition <window.json> --out <fragments.json>
  pse-traverse-lpcm collapse  <window.json> [--rd <rd.json>] --out <collapse_report.json>
  pse-traverse-lpcm replay    --rd <rd.json> --input <window.json> --report <collapse_report.json>
  pse-traverse-lpcm verify    <collapse_report.json>
  pse-traverse-lpcm ablate    <window.json> --out <ablation.json>

PSE-LPCM-IMPLEMENTATION-01 — Fragmented 51% Condensation Layer
Primary invariant: Local51(U) => Candidate(U), Candidate(U) != Truth
";
