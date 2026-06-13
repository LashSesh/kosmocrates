//! `pse-traverse-cli` — minimal CLI for the PSE Traversal Agent.
//!
//! Subcommands:
//!
//! ```text
//! inspect  --problem PATH                       → JSON dump of FieldCube + DoFGraph + excisions
//! plan     --problem PATH [--out FILE]          → CollapsePlan as canonical JSON
//!          [--signature]                        → also derive operator + signature + diagnostics
//! run      --problem PATH [--out FILE]          → TraversalRunReport (incl. PSE bridge attempt)
//!          [--signature-gate]                   → attach SignatureGate diagnostic channel
//! replay   --run FILE                           → re-derive plan + assert byte-identity
//! search   --problem PATH [--n NUM] [--out FILE]
//!                                               → generate traversal blueprints
//! ```
//!
//! Manual flag parsing — no `clap` dependency is added at this stage.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use pse_traverse::{
    blueprint_search::{BlueprintSearch, BlueprintSearchPolicy},
    bridge::{CrystalCommitter, PseMacroStepCommitter},
    canonical::{canonical_bytes, hex_address},
    dof::DoFGraph,
    excision::detect_path_excision,
    field_cube::{DefaultFieldCubeBuilder, FieldCubeBuilder},
    gate::{GateDiagnosticChannel, GateEngine},
    operator::{build_operator_from_dof_graph, OperatorBuildConfig},
    plan::{CollapsePlanner, DefaultCollapsePlanner, OrderingPolicy},
    report::{TraversalRunDescriptor, TraversalRunReport, TraversalRunReportSignatureExtension},
    signature::{derive_signature, SignatureConfig},
    signature_diag::{compute_diagnostics, DiagnosticConfig},
    signature_gate::{check_signature_gate, SignatureGateConfig},
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
        "plan" => cmd_plan(&args[2..]),
        "run" => cmd_run(&args[2..]),
        "replay" => cmd_replay(&args[2..]),
        "search" => cmd_search(&args[2..]),
        "dynamics" => cmd_dynamics(&args[2..]),
        "doors" => {
            cmd_doors(&args[2..]);
            Ok(())
        }
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
  pse-traverse-cli inspect           --problem <PATH>
  pse-traverse-cli plan              --problem <PATH> [--out <FILE>] [--signature]
  pse-traverse-cli run               --problem <PATH> [--out <FILE>] [--signature-gate]
  pse-traverse-cli replay            --run <FILE>
  pse-traverse-cli search            --problem <PATH> [--n <NUM>] [--out <FILE>]
  pse-traverse-cli dynamics init     --problem <PATH> [--out <FILE>]
  pse-traverse-cli dynamics tick     --state <FILE> [--out <FILE>]
  pse-traverse-cli dynamics run      --problem <PATH> [--ticks <N>] [--out <FILE>]
  pse-traverse-cli dynamics replay   --report <FILE>
  pse-traverse-cli dynamics inspect  --report <FILE>
  pse-traverse-cli doors     [--json]   the binary's docking surface, self-described
";

type CliResult<T> = std::result::Result<T, String>;

fn flag_value(args: &[String], flag: &str) -> CliResult<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            return iter
                .next()
                .cloned()
                .ok_or_else(|| format!("missing value for {}", flag));
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

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn load_spec(path: &str) -> CliResult<ProblemSpec> {
    let bytes = fs::read(PathBuf::from(path)).map_err(|e| format!("read {}: {}", path, e))?;
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
    let cube = DefaultFieldCubeBuilder
        .build(&spec)
        .map_err(|e| e.to_string())?;
    let graph = DoFGraph::from_field_cube(&cube);
    let excisions = detect_path_excision(&cube);
    let payload = serde_json::json!({
        "field_cube": cube,
        "dof_graph": graph,
        "path_excisions": excisions,
    });
    let bytes = serde_jcs::to_vec(&payload).map_err(|e| format!("canonical encode: {}", e))?;
    println!("{}", String::from_utf8_lossy(&bytes));
    Ok(())
}

fn cmd_plan(args: &[String]) -> CliResult<()> {
    let path = flag_value(args, "--problem")?;
    let with_signature = has_flag(args, "--signature");
    let spec = load_spec(&path)?;
    let cube = DefaultFieldCubeBuilder
        .build(&spec)
        .map_err(|e| e.to_string())?;
    let graph = DoFGraph::from_field_cube(&cube);
    let excisions = detect_path_excision(&cube);
    let plan = DefaultCollapsePlanner.plan(&cube, &graph, &excisions);

    if with_signature {
        let op_cfg = OperatorBuildConfig::default();
        let op = build_operator_from_dof_graph(&graph, &op_cfg).map_err(|e| e.to_string())?;
        let sig_cfg = SignatureConfig::default();
        let sig = derive_signature(&op, &sig_cfg).map_err(|e| e.to_string())?;
        let diag_cfg = DiagnosticConfig::default();
        let diag = compute_diagnostics(&sig, &op, &diag_cfg);
        let gate_cfg = SignatureGateConfig::default();
        let gate_outcome = check_signature_gate(&diag, &gate_cfg);
        let payload = serde_json::json!({
            "collapse_plan": plan,
            "operator": op,
            "signature": sig,
            "diagnostics": diag,
            "signature_gate_outcome": gate_outcome,
        });
        let bytes = serde_jcs::to_vec(&payload).map_err(|e| format!("canonical encode: {}", e))?;
        write_output(&bytes, opt_flag_value(args, "--out"), "plan+signature")
    } else {
        let bytes = canonical_bytes(&plan).map_err(|e| e.to_string())?;
        if let Some(out) = opt_flag_value(args, "--out") {
            if let Some(parent) = PathBuf::from(&out).parent() {
                fs::create_dir_all(parent).map_err(|e| format!("mkdir {:?}: {}", parent, e))?;
            }
            fs::write(&out, &bytes).map_err(|e| format!("write {}: {}", out, e))?;
            eprintln!(
                "wrote {} ({} bytes, address={})",
                out,
                bytes.len(),
                hex_address(&plan).map_err(|e| e.to_string())?
            );
        } else {
            println!("{}", String::from_utf8_lossy(&bytes));
        }
        Ok(())
    }
}

fn cmd_run(args: &[String]) -> CliResult<()> {
    let path = flag_value(args, "--problem")?;
    let with_sig_gate = has_flag(args, "--signature-gate");
    let spec = load_spec(&path)?;
    let problem_hash = hex_address(&spec).map_err(|e| e.to_string())?;
    let cube = DefaultFieldCubeBuilder
        .build(&spec)
        .map_err(|e| e.to_string())?;
    let graph = DoFGraph::from_field_cube(&cube);
    let excisions = detect_path_excision(&cube);
    let plan = DefaultCollapsePlanner.plan(&cube, &graph, &excisions);

    // Build signature artefacts when --signature-gate is requested.
    let sig_extension = if with_sig_gate {
        let op_cfg = OperatorBuildConfig::default();
        let op = build_operator_from_dof_graph(&graph, &op_cfg).map_err(|e| e.to_string())?;
        let sig_cfg = SignatureConfig::default();
        let sig = derive_signature(&op, &sig_cfg).map_err(|e| e.to_string())?;
        let diag_cfg = DiagnosticConfig::default();
        let diag = compute_diagnostics(&sig, &op, &diag_cfg);
        let gate_cfg = SignatureGateConfig::default();
        let gate_outcome = check_signature_gate(&diag, &gate_cfg);
        Some((op, sig, diag, gate_outcome))
    } else {
        None
    };

    let solver_ctx = SolverContext {
        run_id: format!("run.{}.0", &problem_hash[..16]),
        problem_spec_hash: problem_hash.clone(),
        seed: spec.replay.seed,
        allowed_oracle: spec.risk_policy.allow_oracle,
        metadata: BTreeMap::new(),
    };

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
        let mut report = gate.check(&cube, candidate);

        // Attach signature diagnostic channel when requested.
        if let Some((_, _, ref diag, ref gate_outcome)) = sig_extension {
            report.diagnostic_channels.push(GateDiagnosticChannel {
                channel_name: "signature_gate".into(),
                passed: gate_outcome.passed,
                address: gate_outcome.outcome_id.clone(),
                reasons: diag.messages.clone(),
            });
        }

        if !report.passed {
            let outcome =
                pse_traverse::bridge::gate_failed(candidate, &report).map_err(|e| e.to_string())?;
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

    let signature_extension =
        sig_extension.map(
            |(op, sig, diag, gate_outcome)| TraversalRunReportSignatureExtension {
                blueprint_id: None,
                operator_id: Some(op.operator_id),
                signature_id: Some(sig.signature_id),
                diagnostics_id: Some(diag.diagnostics_id),
                signature_gate_outcome_id: Some(gate_outcome.outcome_id),
                search_ledger_entry_id: None,
            },
        );

    let descriptor = TraversalRunDescriptor {
        run_id: solver_ctx.run_id.clone(),
        problem_spec_hash: problem_hash,
        operator_versions: {
            let mut m = BTreeMap::new();
            m.insert("FieldCubeBuilder".to_string(), "default-1".to_string());
            m.insert("CollapsePlanner".to_string(), "default-1".to_string());
            m.insert("GateEngine".to_string(), "default-1".to_string());
            m.insert("Solver".to_string(), "template-solver-v1".to_string());
            m.insert(
                "CrystalCommitter".to_string(),
                "pse-macro-step-1".to_string(),
            );
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
        signature_extension,
        dynamic_run: None,
    };
    let bytes = canonical_bytes(&run_report).map_err(|e| e.to_string())?;
    write_output(&bytes, opt_flag_value(args, "--out"), "run report")
}

fn cmd_replay(args: &[String]) -> CliResult<()> {
    let path = flag_value(args, "--run")?;
    let bytes = fs::read(&path).map_err(|e| format!("read {}: {}", path, e))?;
    let report: TraversalRunReport =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse {}: {}", path, e))?;

    // Reconstruct from the embedded ProblemSpec.
    let cube_again = DefaultFieldCubeBuilder
        .build(&report.problem_spec)
        .map_err(|e| e.to_string())?;
    let graph_again = DoFGraph::from_field_cube(&cube_again);
    let exc_again = detect_path_excision(&cube_again);
    let plan_again = DefaultCollapsePlanner.plan(&cube_again, &graph_again, &exc_again);

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

fn cmd_search(args: &[String]) -> CliResult<()> {
    let path = flag_value(args, "--problem")?;
    let n: usize = opt_flag_value(args, "--n")
        .map(|s| s.parse::<usize>().map_err(|e| format!("--n: {}", e)))
        .transpose()?
        .unwrap_or(8);
    let spec = load_spec(&path)?;
    let problem_hash = hex_address(&spec).map_err(|e| e.to_string())?;
    let search = BlueprintSearch {
        policy: BlueprintSearchPolicy::DeterministicGrid,
        seed: spec.replay.seed.unwrap_or(0),
    };
    let blueprints = search.generate_blueprints(&problem_hash, n);
    let payload = serde_json::json!({
        "problem_spec_hash": problem_hash,
        "n_requested": n,
        "n_generated": blueprints.len(),
        "blueprints": blueprints,
    });
    let bytes = serde_jcs::to_vec(&payload).map_err(|e| format!("canonical encode: {}", e))?;
    write_output(&bytes, opt_flag_value(args, "--out"), "search blueprints")
}

// ─────────────────────────────────────────────────────────────────────────────
// dynamics subcommand dispatcher
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_dynamics(args: &[String]) -> CliResult<()> {
    if args.is_empty() {
        return Err(format!(
            "dynamics requires a subcommand (init|tick|run|replay|inspect)\n\n{}",
            USAGE
        ));
    }
    match args[0].as_str() {
        "init" => cmd_dynamics_init(&args[1..]),
        "tick" => cmd_dynamics_tick(&args[1..]),
        "run" => cmd_dynamics_run(&args[1..]),
        "replay" => cmd_dynamics_replay(&args[1..]),
        "inspect" => cmd_dynamics_inspect(&args[1..]),
        other => Err(format!("unknown dynamics subcommand: {}", other)),
    }
}

/// `dynamics init --problem <PATH> [--out <FILE>]`
/// Load ProblemSpec, build empty DynamicTraversalState, write JSON.
fn cmd_dynamics_init(args: &[String]) -> CliResult<()> {
    use pse_traverse::{
        dynamic_policy::DynamicPolicy, dynamic_report::DynamicRunDescriptor,
        dynamic_tick::DynamicTraversalState,
    };

    let path = flag_value(args, "--problem")?;
    let spec = load_spec(&path)?;

    let policy = DynamicPolicy::default();
    let descriptor = DynamicRunDescriptor::default();
    let state =
        DynamicTraversalState::init(vec![], policy, descriptor).map_err(|e| e.to_string())?;

    let bytes = serde_json::to_vec_pretty(&state).map_err(|e| format!("serialize state: {}", e))?;
    let _ = spec; // loaded for future use (dimension extraction)
    write_output(&bytes, opt_flag_value(args, "--out"), "dynamics state")
}

/// `dynamics tick --state <FILE> [--out <FILE>]`
/// Load DynamicTraversalState, run one dynamic_tick with empty base_states, write updated state.
fn cmd_dynamics_tick(args: &[String]) -> CliResult<()> {
    use pse_traverse::dynamic_tick::{dynamic_tick, DynamicTickInput, DynamicTraversalState};

    let state_path = flag_value(args, "--state")?;
    let bytes = fs::read(&state_path).map_err(|e| format!("read {}: {}", state_path, e))?;
    let mut state: DynamicTraversalState =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse state {}: {}", state_path, e))?;

    let next_tick = state.tick + 1;
    let policy = state.policy.clone();
    let gate_config = state.descriptor.gate_config.clone();

    let input = DynamicTickInput {
        tick: next_tick,
        base_states: vec![],
        policy,
        gate_config,
        compute_proof: true,
    };

    let tick_report = dynamic_tick(&input, &mut state).map_err(|e| e.to_string())?;

    let out_val = serde_json::json!({
        "updated_state": state,
        "tick_report": tick_report,
    });
    let out_bytes = serde_json::to_vec_pretty(&out_val).map_err(|e| format!("serialize: {}", e))?;
    write_output(&out_bytes, opt_flag_value(args, "--out"), "dynamics tick")
}

/// `dynamics run --problem <PATH> [--ticks <N>] [--out <FILE>]`
/// Load ProblemSpec, run dynamic_run for N ticks, write DynamicRunReport.
fn cmd_dynamics_run(args: &[String]) -> CliResult<()> {
    use pse_traverse::{
        dynamic_policy::DynamicPolicy,
        dynamic_report::{DynamicRunDescriptor, DynamicStopCondition},
        dynamic_tick::dynamic_run,
    };

    let path = flag_value(args, "--problem")?;
    let ticks: u64 = opt_flag_value(args, "--ticks")
        .map(|s| s.parse::<u64>().map_err(|e| format!("--ticks: {}", e)))
        .transpose()?
        .unwrap_or(10);

    let spec = load_spec(&path)?;
    let _ = spec; // loaded to validate; for MVP we run with empty initial states

    let policy = DynamicPolicy::default();
    let descriptor = DynamicRunDescriptor {
        max_ticks: ticks,
        stop_conditions: vec![DynamicStopCondition::MaxTicks(ticks)],
        ..DynamicRunDescriptor::default()
    };
    let descriptor = descriptor.with_id().map_err(|e| e.to_string())?;

    let report = dynamic_run(&descriptor, vec![], policy).map_err(|e| e.to_string())?;

    let bytes = serde_json::to_vec_pretty(&report).map_err(|e| format!("serialize: {}", e))?;
    write_output(&bytes, opt_flag_value(args, "--out"), "dynamics run report")
}

/// `dynamics replay --report <FILE>`
/// Load DynamicRunReport, re-run ticks, compare report_ids byte-for-byte.
fn cmd_dynamics_replay(args: &[String]) -> CliResult<()> {
    use pse_traverse::{
        dynamic_policy::DynamicPolicy, dynamic_report::DynamicRunReport, dynamic_tick::dynamic_run,
    };

    let path = flag_value(args, "--report")?;
    let bytes = fs::read(&path).map_err(|e| format!("read {}: {}", path, e))?;
    let original: DynamicRunReport =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse report: {}", e))?;

    // Re-run with same descriptor and empty initial states
    let policy = DynamicPolicy::default();
    let replayed = dynamic_run(&original.descriptor, vec![], policy).map_err(|e| e.to_string())?;

    if original.run_id != replayed.run_id {
        return Err(format!(
            "replay mismatch: original run_id={}, replayed run_id={}",
            original.run_id.hex(),
            replayed.run_id.hex()
        ));
    }
    println!(
        "dynamics replay ok: run_id byte-identical ({})",
        original.run_id.hex()
    );
    Ok(())
}

/// `dynamics inspect --report <FILE>`
/// Load DynamicRunReport, print human-readable summary (NOT canonical JSON).
fn cmd_dynamics_inspect(args: &[String]) -> CliResult<()> {
    use pse_traverse::dynamic_report::DynamicRunReport;

    let path = flag_value(args, "--report")?;
    let bytes = fs::read(&path).map_err(|e| format!("read {}: {}", path, e))?;
    let report: DynamicRunReport =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse report: {}", e))?;

    println!("=== Dynamic Run Report ===");
    println!("run_id:              {}", report.run_id.hex());
    println!("schema_version:      {}", report.descriptor.schema_version);
    println!("max_ticks:           {}", report.descriptor.max_ticks);
    println!("ticks_executed:      {}", report.ticks.len());
    println!("final_state_id:      {}", report.final_state_id.hex());
    println!("final_gate_decision: {:?}", report.final_gate_decision);
    println!("replay_address:      {}", report.replay_address.hex());

    if !report.ticks.is_empty() {
        println!("\n--- Tick Summary ---");
        for tick_r in &report.ticks {
            println!(
                "  tick {:3}: gate={:?} states={} density={}",
                tick_r.tick,
                tick_r.gate_report.decision,
                tick_r.field_signal.state_count,
                tick_r.compressor_stats.density.to_f64(),
            );
        }
    }
    Ok(())
}

// ─── Doors (the binary's self-description) ───────────────────────────────────

use kosmo_core::{Door, DoorCatalog, DoorGovernance, DoorInput, DoorNeed, DoorSurface};

/// The binary's complete docking surface, spoken by the binary itself.
/// Every subcommand observes artifact files; `--out` writes only the file
/// the operator names. A test pins this catalog against the dispatch.
fn doors_catalog() -> DoorCatalog {
    let here = || DoorSurface::Cli {
        binary: "pse-traverse-cli".into(),
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
            "help",
            vec!["--help".into(), "-h".into()],
            "the prose usage text",
            vec![],
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            here(),
            "inspect",
            vec![],
            "inspect a problem spec",
            vec![DoorInput::valued("--problem", "<path>").required()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "plan",
            vec![],
            "build the collapse plan for a problem",
            vec![DoorInput::valued("--problem", "<path>").required(), DoorInput::valued("--out", "<file>"), DoorInput::switch("--signature")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "run",
            vec![],
            "run the traversal for a problem (optionally under the signature gate)",
            vec![DoorInput::valued("--problem", "<path>").required(), DoorInput::valued("--out", "<file>"), DoorInput::switch("--signature-gate")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "replay",
            vec![],
            "replay a recorded run byte-identically",
            vec![DoorInput::valued("--run", "<file>").required()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "search",
            vec![],
            "TRITON search over the problem space",
            vec![DoorInput::valued("--problem", "<path>").required(), DoorInput::valued("--n", "<num>"), DoorInput::valued("--out", "<file>")],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "dynamics",
            vec![],
            "the dynamics suite over states and reports",
            vec![DoorInput::valued("subcommand", "init|tick|run|replay|inspect").required(), DoorInput::valued("--problem", "<path>"), DoorInput::valued("--state", "<file>"), DoorInput::valued("--ticks", "<n>"), DoorInput::valued("--report", "<file>"), DoorInput::valued("--out", "<file>")],
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
    println!("pse-traverse-cli doors — the docking surface");
    println!(
        "  catalog {}… · {} door(s)",
        &catalog.catalog_id.to_hex()[..12],
        catalog.len()
    );
    for door in &catalog.doors {
        println!(
            "  {}  [{}]  {}",
            door.name,
            door.governance.label(),
            door.summary
        );
    }
}

#[cfg(test)]
mod doors_tests {
    use super::*;

    /// This binary's own dispatch source — the catalog is pinned against it.
    const MAIN_SRC: &str = include_str!("main.rs");

    fn dispatched_words() -> std::collections::BTreeSet<String> {
        let start = MAIN_SRC
            .find("match args[1].as_str() {")
            .expect("the dispatch match exists");
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
    fn every_door_is_read_only_and_the_catalog_recomputes() {
        let catalog = doors_catalog();
        assert!(catalog.verify(), "the catalog recomputes");
        for door in &catalog.doors {
            assert_eq!(
                door.governance,
                DoorGovernance::ReadOnly,
                "{}: traversal analyses observe artifacts; --out writes only \
                 operator-named files",
                door.name
            );
        }
        assert!(catalog.doors.iter().any(|d| d.name == "doors"));
    }
}
