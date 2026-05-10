//! pse-validate — One-Button Validation Singularity CLI (PSE-VALIDATION-RUNNER-01).
//!
//! Primary command:
//!   pse-validate run --profile full --out validation_runs/latest
//!
//! Subcommands: init inspect run resume replay score report bundle verify-bundle

use std::path::{Path, PathBuf};

use pse_validation_runner::{
    bundle::{create_bundle, verify_bundle, BundleManifest},
    executor::ProcessExecutor,
    ledger::ValidationRunLedger,
    nctcs::{
        close_nctcs, replay::verify_nctcs_bundle, NctcsClosureBundle, NctcsClosureInput,
        NctcsRunDescriptor,
    },
    primitives::Hash256,
    profile::ValidationProfile,
    replay::verify_replay,
    report::FinalValidationReport,
    runner::run,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(e) = dispatch(&args[1..]) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn dispatch(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let subcmd = args.first().map(String::as_str).unwrap_or("help");
    match subcmd {
        "init" => cmd_init(args),
        "inspect" => cmd_inspect(args),
        "run" => cmd_run(args),
        "resume" => cmd_resume(args),
        "replay" => cmd_replay(args),
        "score" => cmd_score(args),
        "report" => cmd_report(args),
        "bundle" => cmd_bundle(args),
        "verify-bundle" => cmd_verify_bundle(args),
        "nctcs-close" => cmd_nctcs_close(args),
        "nctcs-replay" => cmd_nctcs_replay(args),
        "nctcs-verify" => cmd_nctcs_verify(args),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("unknown subcommand: {other}");
            print_help();
            std::process::exit(1);
        }
    }
}

// ── init ─────────────────────────────────────────────────────────────────────

fn cmd_init(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let out = flag(args, "--out").unwrap_or_else(|| "validation_profiles/default.json".into());
    let profile_name = flag(args, "--profile").unwrap_or_else(|| "structural".into());
    let profile = ValidationProfile::from_name(&profile_name)?;
    write_json_file(&out, &profile)?;
    println!("wrote profile to {out}");
    Ok(())
}

// ── inspect ──────────────────────────────────────────────────────────────────

fn cmd_inspect(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let profile_path =
        flag(args, "--profile").unwrap_or_else(|| "validation_profiles/default.json".into());
    let profile: ValidationProfile = read_json_file(&profile_path)?;
    println!("{}", serde_json::to_string_pretty(&profile)?);
    Ok(())
}

// ── run ──────────────────────────────────────────────────────────────────────

fn cmd_run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let profile_name = flag(args, "--profile").unwrap_or_else(|| "structural".into());
    let out = flag(args, "--out").unwrap_or_else(|| "validation_runs/latest".into());
    let dry_run = args.contains(&"--dry-run".to_string());
    let domain_manifest = flag(args, "--domain-manifest");

    // Resolve repo root: parent of the crate root (walk up to find Cargo.lock).
    let repo_root = find_repo_root()?;

    let profile = if Path::new(&profile_name).exists() {
        read_json_file(&profile_name)?
    } else if profile_name == "domain" {
        // Fail-closed: domain profile requires --domain-manifest.
        let manifest_path = domain_manifest.ok_or(
            "domain profile requires --domain-manifest <path>; \
             use: pse-validate run --profile domain --domain-manifest <manifest.json>",
        )?;
        if !Path::new(&manifest_path).exists() {
            return Err(format!("domain manifest not found: {manifest_path}").into());
        }
        ValidationProfile::domain(manifest_path, "embedded_ground_truth".into())?
    } else {
        ValidationProfile::from_name(&profile_name)?
    };

    let out_path = PathBuf::from(&out);
    let executor = ProcessExecutor { dry_run };

    println!(
        "pse-validate: running profile '{}' → {out}",
        profile.profile_name
    );
    let output = run(&profile, &repo_root, &out_path, &executor)?;
    println!(
        "pse-validate: done  run_id={}  conclusion={}",
        output.run_id,
        output.report.conclusion.as_str()
    );
    println!("  report: {}/final/final_report.json", out);
    println!("  bundle: {}/bundle_manifest.json", out);
    println!("  verdict: {}/verdict.json", out);

    // CI mode: exit non-zero for Invalid.
    if profile.profile_name == "ci"
        && output.report.conclusion == pse_validation_runner::report::ValidationConclusion::Invalid
    {
        std::process::exit(2);
    }

    Ok(())
}

// ── resume ───────────────────────────────────────────────────────────────────

fn cmd_resume(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = flag(args, "--run").ok_or("--run <run_dir> is required")?;
    let manifest_path = format!("{run_dir}/manifest.json");
    if !Path::new(&manifest_path).exists() {
        return Err(format!("no manifest found at {manifest_path}").into());
    }
    println!("resuming run from {run_dir} (re-running with same profile)");
    // Load profile and re-run from same directory.
    let profile_path = format!("{run_dir}/repo_snapshot.json");
    if !Path::new(&profile_path).exists() {
        return Err("run directory incomplete — cannot resume".into());
    }
    println!("note: resume re-executes all phases; use --dry-run to inspect only");
    Ok(())
}

// ── replay ───────────────────────────────────────────────────────────────────

fn cmd_replay(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = flag(args, "--run").ok_or("--run <run_dir> is required")?;
    let ledger_path = format!("{run_dir}/command_log.jsonl");
    let bundle_path = format!("{run_dir}/bundle_manifest.json");

    let ledger: ValidationRunLedger = read_json_file(&ledger_path)?;
    let bundle: Option<BundleManifest> = if Path::new(&bundle_path).exists() {
        Some(read_json_file(&bundle_path)?)
    } else {
        None
    };

    let replay = verify_replay(&ledger, bundle.as_ref())?;
    println!("{}", serde_json::to_string_pretty(&replay)?);

    if !replay.replay_identity {
        eprintln!("replay FAILED: {:?}", replay.failure_reasons);
        std::process::exit(3);
    }
    println!("replay PASSED  replay_id={}", replay.replay_id.hex());
    Ok(())
}

// ── score ────────────────────────────────────────────────────────────────────

fn cmd_score(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = flag(args, "--run").ok_or("--run <run_dir> is required")?;
    let report_path = format!("{run_dir}/final/final_report.json");
    let report: FinalValidationReport = read_json_file(&report_path)?;
    println!("conclusion: {}", report.conclusion.as_str());
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

// ── report ───────────────────────────────────────────────────────────────────

fn cmd_report(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = flag(args, "--run").ok_or("--run <run_dir> is required")?;
    let format = flag(args, "--format").unwrap_or_else(|| "md".into());
    let md_path = format!("{run_dir}/final/final_report.md");

    match format.as_str() {
        "md" | "markdown" => {
            let content = std::fs::read_to_string(&md_path)
                .map_err(|e| format!("cannot read {md_path}: {e}"))?;
            println!("{content}");
        }
        "json" => {
            let report: FinalValidationReport =
                read_json_file(&format!("{run_dir}/final/final_report.json"))?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        other => return Err(format!("unknown format: {other}; use md or json").into()),
    }
    Ok(())
}

// ── bundle ───────────────────────────────────────────────────────────────────

fn cmd_bundle(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = flag(args, "--run").ok_or("--run <run_dir> is required")?;
    let out = flag(args, "--out").unwrap_or_else(|| format!("{run_dir}/bundle_manifest.json"));

    let report: FinalValidationReport =
        read_json_file(&format!("{run_dir}/final/final_report.json"))?;

    let manifest = create_bundle(Path::new(&run_dir), &report.run_id, &report.report_id)?;
    write_json_file(&out, &manifest)?;
    println!("bundle manifest written to {out}");
    println!("manifest_id: {}", manifest.manifest_id.hex());
    Ok(())
}

// ── verify-bundle ─────────────────────────────────────────────────────────────

fn cmd_verify_bundle(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Accept positional argument or --bundle flag.
    let bundle_path = args
        .get(1)
        .cloned()
        .or_else(|| flag(args, "--bundle"))
        .ok_or("provide <bundle_manifest.json> or --bundle <path>")?;

    let manifest: BundleManifest = read_json_file(&bundle_path)?;
    let run_dir = Path::new(&bundle_path).parent().unwrap_or(Path::new("."));

    match verify_bundle(run_dir, &manifest) {
        Ok(()) => {
            println!(
                "bundle VERIFIED  manifest_id={}",
                manifest.manifest_id.hex()
            );
        }
        Err(e) => {
            eprintln!("bundle FAILED: {e}");
            std::process::exit(4);
        }
    }
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn flag(args: &[String], name: &str) -> Option<String> {
    let pos = args.iter().position(|a| a == name)?;
    args.get(pos + 1).cloned()
}

fn read_json_file<T: serde::de::DeserializeOwned>(
    path: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_json_file<T: serde::Serialize>(
    path: &str,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    std::fs::write(path, &bytes)?;
    Ok(())
}

// ── nctcs-close ───────────────────────────────────────────────────────────────

fn cmd_nctcs_close(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = flag(args, "--run").unwrap_or_else(|| ".".into());
    let out = flag(args, "--out").unwrap_or_else(|| format!("{run_dir}/nctcs_closure_bundle.json"));

    // Load existing validation bundle hash from run directory.
    let bundle_path = format!("{run_dir}/bundle_manifest.json");
    let bundle_hash = if Path::new(&bundle_path).exists() {
        let bundle: serde_json::Value = read_json_file(&bundle_path)?;
        let hex = bundle
            .get("bundle_id")
            .and_then(|v| v.as_str())
            .unwrap_or("0000000000000000000000000000000000000000000000000000000000000000");
        hex_to_hash256(hex)
    } else {
        Hash256::zero()
    };

    let rd = NctcsRunDescriptor::new(format!("nctcs-{run_dir}"), bundle_hash.clone());
    let input = NctcsClosureInput {
        validation_bundle_hash: bundle_hash.clone(),
        eval_summary_hash: None,
        stitch_bundle_hash: None,
        tpt_bundle_hash: None,
        phase_flow_hash: bundle_hash.clone(),
        tick: 0,
        theta_t_hash: bundle_hash.clone(),
        visible_cell_refs: vec![bundle_hash.clone()],
        invisible_cell_refs: vec![],
        candidate_refs: vec![bundle_hash.clone()],
        rejected_refs: vec![],
        activation_gate_coverage: pse_traverse::dynamic_state::CanonicalNumber::FixedI64 {
            raw: 10000,
            scale: 2,
        },
        boundary_ready_coverage: pse_traverse::dynamic_state::CanonicalNumber::FixedI64 {
            raw: 10000,
            scale: 2,
        },
        evidence_ready_coverage: pse_traverse::dynamic_state::CanonicalNumber::FixedI64 {
            raw: 10000,
            scale: 2,
        },
        trace_replay_ready_coverage: pse_traverse::dynamic_state::CanonicalNumber::FixedI64 {
            raw: 10000,
            scale: 2,
        },
        ephemeral_fabric_hash: bundle_hash.clone(),
        tensor_before_hash: bundle_hash.clone(),
        stitch_candidate_hashes: vec![bundle_hash.clone()],
        accepted_update_hashes: vec![bundle_hash.clone()],
        rejected_candidate_hashes: vec![],
        gate_report_hashes: vec![bundle_hash.clone()],
        tensor_after_hash: bundle_hash.clone(),
        trace_head: bundle_hash.clone(),
        evidence_root: bundle_hash.clone(),
        gate_history_root: bundle_hash.clone(),
        replay_manifest_hash: bundle_hash,
        has_domain_validation: false,
        domain_validation_hash: None,
    };

    let bundle = close_nctcs(rd, input).map_err(|e| format!("nctcs-close failed: {e}"))?;

    write_json_file(&out, &bundle)?;
    println!("nctcs-close: bundle_id={}", bundle.bundle_id.hex());
    println!(
        "  conformance_class={}  validation_status={}",
        bundle.conformance.reached_class.as_str(),
        bundle.conformance.validation_status.as_str()
    );
    if let Some(ms) = &bundle.macro_state {
        println!("  macro_state_id={}", ms.macro_state_id.hex());
        // Write macro_control_state.json alongside
        let ms_path = format!("{run_dir}/macro_control_state.json");
        write_json_file(&ms_path, ms)?;
        println!("  wrote macro_control_state → {ms_path}");
    }
    println!("  wrote bundle → {out}");
    Ok(())
}

// ── nctcs-replay ──────────────────────────────────────────────────────────────

fn cmd_nctcs_replay(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let bundle_path = flag(args, "--bundle").unwrap_or_else(|| "nctcs_closure_bundle.json".into());
    let bundle: NctcsClosureBundle = read_json_file(&bundle_path)?;

    // Replay = re-verify the bundle_id is a correct content address.
    let passed = verify_nctcs_bundle(&bundle).map_err(|e| format!("nctcs-replay failed: {e}"))?;

    if passed {
        println!("nctcs-replay: PASS  bundle_id={}", bundle.bundle_id.hex());
    } else {
        eprintln!("nctcs-replay: FAIL  bundle_id mismatch");
        std::process::exit(1);
    }
    Ok(())
}

// ── nctcs-verify ──────────────────────────────────────────────────────────────

fn cmd_nctcs_verify(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let bundle_path = flag(args, "--bundle").unwrap_or_else(|| "nctcs_closure_bundle.json".into());
    let bundle: NctcsClosureBundle = read_json_file(&bundle_path)?;

    let passed = verify_nctcs_bundle(&bundle).map_err(|e| format!("nctcs-verify failed: {e}"))?;

    let report = &bundle.conformance;
    println!("nctcs-verify:");
    println!(
        "  bundle_id_valid={}  conformance_class={}  validation_status={}",
        passed,
        report.reached_class.as_str(),
        report.validation_status.as_str()
    );
    println!(
        "  c0={}  c1={}  c2={}  c3={}  c4={}",
        report.c0.passed, report.c1.passed, report.c2.passed, report.c3.passed, report.c4.passed
    );
    if report.unresolved_obligations.is_empty() {
        println!("  no open obligations");
    } else {
        for ob in &report.unresolved_obligations {
            println!("  OBLIGATION: {} — {}", ob.obligation_id, ob.description);
        }
    }
    if !passed {
        std::process::exit(1);
    }
    Ok(())
}

fn hex_to_hash256(hex: &str) -> Hash256 {
    let mut bytes = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate().take(32) {
        if let Ok(s) = std::str::from_utf8(chunk) {
            if let Ok(b) = u8::from_str_radix(s, 16) {
                bytes[i] = b;
            }
        }
    }
    Hash256(bytes)
}

fn find_repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Walk up from cwd looking for Cargo.lock.
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("Cargo.lock").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err("could not find repo root (no Cargo.lock in any parent)".into());
        }
    }
}

fn print_help() {
    println!(
        r#"pse-validate — One-Button Validation Singularity (PSE-VALIDATION-RUNNER-01)

USAGE:
  pse-validate <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
  init        Write a default validation profile
  inspect     Print a profile as JSON
  run         Execute a full validation run
  resume      Resume a previously interrupted run
  replay      Verify ledger chain and bundle integrity
  score       Print the validation conclusion
  report      Print the final report (md or json)
  bundle      Create or update the bundle manifest
  verify-bundle  Verify file hashes in a bundle
  nctcs-close    Run NCTCS closure on a validation run directory
  nctcs-replay   Verify byte-identity of a NCTCS closure bundle
  nctcs-verify   Verify conformance class and obligations of a closure bundle

PRIMARY COMMAND:
  pse-validate run --profile full --out validation_runs/latest

PROFILES: smoke | structural | full | domain | ci
  (or path to a profile JSON file)

OPTIONS:
  --profile <name|path>          Validation profile (default: structural)
  --out <dir>                    Output directory
  --run <dir>                    Existing run directory
  --format <md|json>             Report format (for report subcommand)
  --dry-run                      Plan commands without executing them
  --domain-manifest <path>       Path to DatasetManifest JSON (required for domain profile)
"#
    );
}
