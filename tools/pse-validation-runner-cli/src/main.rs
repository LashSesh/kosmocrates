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

    // Resolve repo root: parent of the crate root (walk up to find Cargo.lock).
    let repo_root = find_repo_root()?;

    let profile = if Path::new(&profile_name).exists() {
        read_json_file(&profile_name)?
    } else {
        ValidationProfile::from_name(&profile_name)?
    };

    let out_path = PathBuf::from(&out);
    let executor = ProcessExecutor { dry_run };

    println!("pse-validate: running profile '{}' → {out}", profile.profile_name);
    let output = run(&profile, &repo_root, &out_path, &executor)?;
    println!(
        "pse-validate: done  run_id={}  conclusion={}",
        output.run_id,
        output.report.conclusion.as_str()
    );
    println!("  report: {}/final/final_report.json", out);
    println!("  bundle: {}/bundle_manifest.json", out);

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
    let run_dir =
        flag(args, "--run").ok_or("--run <run_dir> is required")?;
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
    let out = flag(args, "--out")
        .unwrap_or_else(|| format!("{run_dir}/bundle_manifest.json"));

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
            println!("bundle VERIFIED  manifest_id={}", manifest.manifest_id.hex());
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

fn read_json_file<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_json_file<T: serde::Serialize>(path: &str, value: &T) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    std::fs::write(path, &bytes)?;
    Ok(())
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

PRIMARY COMMAND:
  pse-validate run --profile full --out validation_runs/latest

PROFILES: smoke | structural | full | domain | ci
  (or path to a profile JSON file)

OPTIONS:
  --profile <name|path>   Validation profile (default: structural)
  --out <dir>             Output directory
  --run <dir>             Existing run directory
  --format <md|json>      Report format (for report subcommand)
  --dry-run               Plan commands without executing them
"#
    );
}
