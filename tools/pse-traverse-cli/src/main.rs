//! `pse-traverse-cli` — minimal CLI for the PSE Traversal Agent.
//!
//! Subcommands:
//!   inspect --problem <PATH>                  → JSON dump of FieldCube + DoFGraph + excisions
//!   plan    --problem <PATH> [--out <FILE>]   → CollapsePlan as canonical JSON
//!   run     --problem <PATH> [--out <FILE>]   → TraversalRunReport (incl. PSE bridge attempt)
//!   replay  --run <FILE>                      → re-derive plan + assert byte-identity
//!
//! Manual flag parsing — no `clap` dependency is added at this stage.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use pse_traverse::{
    bridge::{CrystalCommitter, PseMacroStepCommitter},
    canonical::{canonical_bytes, hex_address},
    dof::DoFGraph,
    excision::detect_path_excision,
    field_cube::{DefaultFieldCubeBuilder, FieldCubeBuilder},
    gate::GateEngine,
    plan::{CollapsePlanner, DefaultCollapsePlanner, OrderingPolicy},
    report::{TraversalRunDescriptor, TraversalRunReport},
    solver::{Solver, SolverContext, TemplateSolver},
    spec::ProblemSpec,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    }
    let result = match args[1].as_str() {
        "inspect" => cmd_inspect(&args[2..]),
        "plan"    => cmd_plan(&args[2..]),
        "run"     => cmd_run(&args[2..]),
        "replay"  => cmd_replay(&args[2..]),
        "--help" | "-h" | "help" => {
            println!("{}", USAGE);
            return ExitCode::SUCCESS;
        }
        other => {
            eprintln!("unknown subcommand: {}\n\n{}", other, USAGE);
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "Usage:
  pse-traverse-cli inspect --problem <PATH>
  pse-traverse-cli plan    --problem <PATH> [--out <FILE>]
  pse-traverse-cli run     --problem <PATH> [--out <FILE>]
  pse-traverse-cli replay  --run <FILE>
";

type CliResult<T> = std::result::Result<T, String>;

fn flag_value(args: &[String], flag: &str) -> CliResult<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            return iter.next().cloned().ok_or_else(|| format!("missing value for {}", flag));
        }
    }
    Err(format!("missing flag {}", flag))
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

fn load_spec(path: &str) -> CliResult<ProblemSpec> {
    let bytes = fs::read(PathBuf::from(path))
        .map_err(|e| format!("read {}: {}", path, e))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("parse {}: {}", path, e))
}

fn write_output(bytes: &[u8], opt_out: Option<String>, label: &str) -> CliResult<()> {
    if let Some(out) = opt_out {
        if let Some(parent) = PathBuf::from(&out).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir {:?}: {}", parent, e))?;
        }
        fs::write(&out, bytes).map_err(|e| format!("write {}: {}", out, e))?;
        eprintln!("wrote {} {} ({} bytes)", label, out, bytes.len());
    } else {
        println!("{}", String::from_utf8_lossy(bytes));
    }
    Ok(())
}

fn cmd_inspect(args: &[String]) -> CliResult<()> {
    let path = flag_value(args, "--problem")?;
    let spec = load_spec(&path)?;
    let cube = DefaultFieldCubeBuilder.build(&spec).map_err(|e| e.to_string())?;
    let graph = DoFGraph::from_field_cube(&cube);
    let excisions = detect_path_excision(&cube);
    let payload = serde_json::json!({
        "field_cube": cube,
        "dof_graph": graph,
        "path_excisions": excisions,
    });
    let bytes = serde_jcs::to_vec(&payload)
        .map_err(|e| format!("canonical encode: {}", e))?;
    println!("{}", String::from_utf8_lossy(&bytes));
    Ok(())
}

fn cmd_plan(args: &[String]) -> CliResult<()> {
    let path = flag_value(args, "--problem")?;
    let spec = load_spec(&path)?;
    let cube = DefaultFieldCubeBuilder.build(&spec).map_err(|e| e.to_string())?;
    let graph = DoFGraph::from_field_cube(&cube);
    let excisions = detect_path_excision(&cube);
    let plan = DefaultCollapsePlanner.plan(&cube, &graph, &excisions);
    let bytes = canonical_bytes(&plan).map_err(|e| e.to_string())?;
    if let Some(out) = opt_flag_value(args, "--out") {
        if let Some(parent) = PathBuf::from(&out).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir {:?}: {}", parent, e))?;
        }
        fs::write(&out, &bytes).map_err(|e| format!("write {}: {}", out, e))?;
        eprintln!("wrote {} ({} bytes, address={})",
                  out, bytes.len(),
                  hex_address(&plan).map_err(|e| e.to_string())?);
    } else {
        println!("{}", String::from_utf8_lossy(&bytes));
    }
    Ok(())
}

fn cmd_run(args: &[String]) -> CliResult<()> {
    let path = flag_value(args, "--problem")?;
    let spec = load_spec(&path)?;
    let problem_hash = hex_address(&spec).map_err(|e| e.to_string())?;
    let cube = DefaultFieldCubeBuilder.build(&spec).map_err(|e| e.to_string())?;
    let graph = DoFGraph::from_field_cube(&cube);
    let excisions = detect_path_excision(&cube);
    let plan = DefaultCollapsePlanner.plan(&cube, &graph, &excisions);

    // Build a SolverContext from the run descriptor fields.
    let solver_ctx = SolverContext {
        run_id: format!("run.{}.0", &problem_hash[..16]),
        problem_spec_hash: problem_hash.clone(),
        seed: spec.replay.seed,
        allowed_oracle: spec.risk_policy.allow_oracle,
        metadata: BTreeMap::new(),
    };

    // TemplateSolver: deterministic, assigns first admissible value per
    // required dimension. Replace with a richer solver in Phase 5+.
    let candidates = TemplateSolver
        .solve(&cube, &plan, &solver_ctx)
        .map_err(|e| e.to_string())?;

    let mut gate_reports = Vec::new();
    let mut commit_outcomes = Vec::new();
    let gate = GateEngine::default();
    let mut committer = PseMacroStepCommitter::new(
        pse_types::Config::default(),
        pse_graph::PassthroughAdapter::new("traverse_run"),
    );

    for candidate in &candidates {
        let report = gate.check(&cube, candidate);
        if !report.passed {
            let outcome = pse_traverse::bridge::gate_failed(candidate, &report)
                .map_err(|e| e.to_string())?;
            gate_reports.push(report);
            commit_outcomes.push(outcome);
            continue;
        }
        gate_reports.push(report);
        let outcome = committer
            .commit_candidate(candidate, &[])
            .map_err(|e| e.to_string())?;
        commit_outcomes.push(outcome);
    }

    let descriptor = TraversalRunDescriptor {
        run_id: solver_ctx.run_id.clone(),
        problem_spec_hash: problem_hash,
        operator_versions: {
            let mut m = BTreeMap::new();
            m.insert("FieldCubeBuilder".to_string(),  "default-1".to_string());
            m.insert("CollapsePlanner".to_string(),   "default-1".to_string());
            m.insert("GateEngine".to_string(),        "default-1".to_string());
            m.insert("Solver".to_string(),            "template-solver-v1".to_string());
            m.insert("CrystalCommitter".to_string(),  "pse-macro-step-1".to_string());
            m
        },
        seed: spec.replay.seed,
        ordering_policy: OrderingPolicy::DeterministicLexCoupling,
        pse_config_hash: None,
        started_at_logical: 0,
    };
    let run_report = TraversalRunReport {
        descriptor,
        problem_spec: spec,
        field_cube: cube,
        dof_graph: graph,
        collapse_plan: plan,
        path_excisions: excisions,
        gate_reports,
        commit_outcomes,
    };
    let bytes = canonical_bytes(&run_report).map_err(|e| e.to_string())?;
    write_output(&bytes, opt_flag_value(args, "--out"), "run report")
}

fn cmd_replay(args: &[String]) -> CliResult<()> {
    let path = flag_value(args, "--run")?;
    let bytes = fs::read(&path).map_err(|e| format!("read {}: {}", path, e))?;
    let report: TraversalRunReport = serde_json::from_slice(&bytes)
        .map_err(|e| format!("parse {}: {}", path, e))?;

    // Reconstruct from the embedded ProblemSpec. Replay is self-contained:
    // the report carries everything a verifier needs.
    let cube_again   = DefaultFieldCubeBuilder.build(&report.problem_spec).map_err(|e| e.to_string())?;
    let graph_again  = DoFGraph::from_field_cube(&cube_again);
    let exc_again    = detect_path_excision(&cube_again);
    let plan_again   = DefaultCollapsePlanner.plan(&cube_again, &graph_again, &exc_again);

    let plan_orig = canonical_bytes(&report.collapse_plan).map_err(|e| e.to_string())?;
    let plan_repl = canonical_bytes(&plan_again).map_err(|e| e.to_string())?;
    if plan_orig != plan_repl {
        return Err("replay mismatch: collapse plan differs".into());
    }
    let cube_orig = canonical_bytes(&report.field_cube).map_err(|e| e.to_string())?;
    let cube_repl = canonical_bytes(&cube_again).map_err(|e| e.to_string())?;
    if cube_orig != cube_repl {
        return Err("replay mismatch: field cube differs".into());
    }
    let exc_orig = canonical_bytes(&report.path_excisions).map_err(|e| e.to_string())?;
    let exc_repl = canonical_bytes(&exc_again).map_err(|e| e.to_string())?;
    if exc_orig != exc_repl {
        return Err("replay mismatch: path excisions differ".into());
    }
    println!("replay ok: cube, plan, excisions byte-identical");
    Ok(())
}
