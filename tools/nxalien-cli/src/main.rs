//! nxalien — PSE agent-context exoskeleton CLI.
//!
//! Subcommands:
//!   init      Create .nxalien/ with default policy
//!   inspect   Print detected project substrate
//!   compile   Full pipeline → nxalien.manifest.json + context artifacts
//!   ground    Print [NXALIEN-CONTEXT] block to stdout
//!   handoff   Print handoff candidates JSON
//!   replay    Recompute hashes and compare to manifest
//!   verify    Run gates + replay + canon verification
//!   export    Generate CLAUDE.md / AGENTS.md / .rules

use anyhow::{bail, Context, Result};
use pse_nxalien_agent::ContextProjector;
use pse_nxalien_core::{
    canon::{compute_replay_hash, sha256_jcs},
    gate::{auto_downgrade_rules, evaluate_gate},
    scanner::ProjectScanner,
};
use pse_nxalien_cube::HypercubeHdag;
use pse_nxalien_pse::{artifact_digest as nxa_artifact_digest, build_handoff_candidate};
use pse_nxalien_types::{NxAlienBundle, NxAlienManifest, NxAlienPolicy, NxAlienRunDescriptor};
use std::path::{Path, PathBuf};

const VERSION: &str = "0.1.0";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let subcmd = args.get(1).map(String::as_str).unwrap_or("help");

    match subcmd {
        "init" => cmd_init(&args),
        "inspect" => cmd_inspect(&args),
        "compile" => cmd_compile(&args),
        "ground" => cmd_ground(&args),
        "handoff" => cmd_handoff(&args),
        "replay" => cmd_replay(&args),
        "verify" => cmd_verify(&args),
        "export" => cmd_export(&args),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        "--version" | "-V" => {
            println!("nxalien v{VERSION}");
            Ok(())
        }
        other => {
            eprintln!("Unknown subcommand: {other}");
            print_usage();
            std::process::exit(1);
        }
    }
}

// ─── Subcommands ─────────────────────────────────────────────────────────────

fn cmd_init(args: &[String]) -> Result<()> {
    let root = project_root(args);
    let nxa_dir = root.join(".nxalien");
    std::fs::create_dir_all(&nxa_dir).with_context(|| format!("creating {}", nxa_dir.display()))?;

    let policy = NxAlienPolicy::default();
    let policy_path = nxa_dir.join("policy.json");
    write_json(&policy_path, &policy)?;

    println!("nxalien v{VERSION}");
    println!("  Initialised .nxalien/policy.json");
    println!("  Run `nxalien compile` to generate the manifest.");
    Ok(())
}

fn cmd_inspect(args: &[String]) -> Result<()> {
    let root = project_root(args);
    let scanner = ProjectScanner::new(&root);
    let meta = scanner.scan();
    let rules = scanner.default_rules(&meta);
    let unknowns = scanner.extract_unknowns(&meta);

    println!("nxalien v{VERSION}  — inspect");
    println!("  Root          : {}", root.display());
    println!("  Language      : {}", meta.language);
    println!("  Package mgr   : {}", meta.package_manager);
    println!(
        "  Test cmd      : {}",
        meta.test_command.as_deref().unwrap_or("-")
    );
    println!(
        "  Build cmd     : {}",
        meta.build_command.as_deref().unwrap_or("-")
    );
    println!(
        "  Fmt cmd       : {}",
        meta.fmt_command.as_deref().unwrap_or("-")
    );
    println!(
        "  Lint cmd      : {}",
        meta.lint_command.as_deref().unwrap_or("-")
    );
    println!("  Context files : {:?}", meta.existing_context_files);
    println!("  Default rules : {}", rules.len());
    println!("  Unknowns      : {}", unknowns.len());
    Ok(())
}

fn cmd_compile(args: &[String]) -> Result<()> {
    let root = project_root(args);
    let policy = load_policy(&root);

    println!("nxalien v{VERSION}  — compile");
    println!("  Scanning project root: {}", root.display());

    let scanner = ProjectScanner::new(&root);
    let meta = scanner.scan();
    let mut rules = scanner.default_rules(&meta);
    let unknowns = scanner.extract_unknowns(&meta);

    println!(
        "  Language      : {} ({})",
        meta.language, meta.package_manager
    );

    // Auto-downgrade rules lacking evidence.
    auto_downgrade_rules(&mut rules, &policy);

    // Build HDAG.
    let hdag = HypercubeHdag::build_with_policy(&meta, &rules, &unknowns, policy.clone());
    let cube = hdag.to_cube();
    let topo = hdag.topological_order();
    println!(
        "  Cube          : {} nodes, {} edges (acyclic {})",
        cube.nodes.len(),
        cube.edges.len(),
        if hdag.is_acyclic() { "✓" } else { "✗ WARN" }
    );
    println!("  Topo order    : {} nodes sorted", topo.len());

    // Context char count.
    let context_str = serde_json::to_string(&cube)?;
    let context_hash = sha256_jcs(&cube);

    // Gate evaluation.
    let gate = evaluate_gate(&rules, context_str.len(), &policy, true, true);
    println!("  Gate G_A      : {:?}", gate.outcome);
    for note in &gate.notes {
        println!("    ⚠ {note}");
    }

    // Build descriptor.
    let project_root_digest = pse_nxalien_core::scanner::project_root_digest(&root);
    let policy_digest = sha256_jcs(&policy);
    let descriptor = NxAlienRunDescriptor {
        schema_version: "1.0".to_string(),
        project_root_digest,
        policy_digest,
        seed: 0,
        started_at_utc: utc_now(),
    };

    // Build manifest (without hashes first).
    let mut manifest = NxAlienManifest {
        schema_version: "1.0".to_string(),
        descriptor: descriptor.clone(),
        metadata: meta.clone(),
        rules: rules.clone(),
        unknowns: unknowns.clone(),
        cube_node_count: cube.nodes.len(),
        cube_edge_count: cube.edges.len(),
        gate_report: gate.clone(),
        manifest_hash: String::new(),
        replay_hash: String::new(),
    };

    // Compute manifest hash (JCS of manifest with empty hash fields).
    let manifest_hash = sha256_jcs(&manifest);
    let replay_hash = compute_replay_hash(&manifest_hash, &context_hash, &descriptor);
    manifest.manifest_hash = manifest_hash.clone();
    manifest.replay_hash = replay_hash.clone();

    // Write outputs.
    let manifest_path = root.join("nxalien.manifest.json");
    write_json(&manifest_path, &manifest)?;

    let context_path = root.join("nxalien.context.json");
    write_json(&context_path, &cube)?;

    let replay_data = serde_json::json!({
        "manifest_hash": manifest_hash,
        "context_hash": context_hash,
        "replay_hash": replay_hash,
        "schema_version": "1.0",
        "seed": descriptor.seed,
    });
    let replay_path = root.join("nxalien.replay.json");
    write_json(&replay_path, &replay_data)?;

    // Build bundle for handoff.
    let bundle = NxAlienBundle {
        descriptor: descriptor.clone(),
        manifest_hash: manifest_hash.clone(),
        context_hash,
        replay_hash: replay_hash.clone(),
        gate_report: gate.clone(),
        artifacts: vec![
            file_digest(&manifest_path),
            file_digest(&context_path),
            file_digest(&replay_path),
        ]
        .into_iter()
        .flatten()
        .collect(),
        handoff_candidates: vec![],
        rules,
        unknowns,
        metadata: meta,
    };
    let candidate = build_handoff_candidate(&bundle);
    let handoff_path = root.join("nxalien.handoff.json");
    write_json(&handoff_path, &candidate)?;

    println!("  Replay hash   : sha256:{replay_hash}");
    println!("  Written       : {}", manifest_path.display());
    println!("               : {}", context_path.display());
    println!("               : {}", replay_path.display());
    println!("               : {}", handoff_path.display());

    // Context projections.
    let projector = ContextProjector::new(policy.max_context_chars);
    std::fs::write(
        root.join("nxalien.rules.md"),
        projector.render_rules_file(&manifest),
    )?;
    println!("               : nxalien.rules.md");
    Ok(())
}

fn cmd_ground(args: &[String]) -> Result<()> {
    let root = project_root(args);
    let manifest = load_manifest(&root)?;
    let policy = load_policy(&root);
    let projector = ContextProjector::new(policy.max_context_chars);
    print!("{}", projector.render_context_block(&manifest));
    Ok(())
}

fn cmd_handoff(args: &[String]) -> Result<()> {
    let root = project_root(args);
    let handoff_path = root.join("nxalien.handoff.json");
    if !handoff_path.exists() {
        bail!("nxalien.handoff.json not found — run `nxalien compile` first");
    }
    let content = std::fs::read_to_string(&handoff_path)?;
    println!("{content}");
    Ok(())
}

fn cmd_replay(args: &[String]) -> Result<()> {
    let root = project_root(args);
    let manifest = load_manifest(&root)?;

    // Recompute manifest hash.
    let stored_replay = manifest.replay_hash.clone();
    let mut check = manifest.clone();
    check.manifest_hash = String::new();
    check.replay_hash = String::new();
    let computed_manifest_hash = sha256_jcs(&check);

    // Load context hash from replay file if present.
    let replay_path = root.join("nxalien.replay.json");
    let context_hash = if replay_path.exists() {
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&replay_path)?)?;
        v["context_hash"].as_str().unwrap_or("").to_string()
    } else {
        String::new()
    };

    let computed_replay =
        compute_replay_hash(&computed_manifest_hash, &context_hash, &manifest.descriptor);

    println!("nxalien v{VERSION}  — replay");
    println!("  Stored replay hash  : sha256:{stored_replay}");
    println!("  Computed replay hash: sha256:{computed_replay}");
    if stored_replay == computed_replay {
        println!("  Replay              : ✓ MATCH");
    } else {
        println!("  Replay              : ✗ MISMATCH");
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_verify(args: &[String]) -> Result<()> {
    let root = project_root(args);
    let manifest = load_manifest(&root)?;
    let policy = load_policy(&root);

    println!("nxalien v{VERSION}  — verify");

    // Canon check: all rule hashes valid.
    let canon_ok = manifest.rules.iter().all(|r| r.verify_hash());
    println!("  Canon hashes : {}", if canon_ok { "✓" } else { "✗" });

    // Replay check.
    let mut check = manifest.clone();
    check.manifest_hash = String::new();
    check.replay_hash = String::new();
    let computed_manifest_hash = sha256_jcs(&check);
    let replay_path = root.join("nxalien.replay.json");
    let context_hash = if replay_path.exists() {
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&replay_path)?)?;
        v["context_hash"].as_str().unwrap_or("").to_string()
    } else {
        String::new()
    };
    let computed_replay =
        compute_replay_hash(&computed_manifest_hash, &context_hash, &manifest.descriptor);
    let replay_ok = manifest.replay_hash == computed_replay;
    println!("  Replay hash  : {}", if replay_ok { "✓" } else { "✗" });

    // Gate evaluation.
    let projector = ContextProjector::new(policy.max_context_chars);
    let context_len = projector.render_context_block(&manifest).len();
    let gate = evaluate_gate(&manifest.rules, context_len, &policy, replay_ok, canon_ok);
    println!("  Gate outcome : {:?}", gate.outcome);
    for note in &gate.notes {
        println!("    ⚠ {note}");
    }

    if !canon_ok || !replay_ok {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_export(args: &[String]) -> Result<()> {
    let root = project_root(args);
    let manifest = load_manifest(&root)?;
    let policy = load_policy(&root);
    let projector = ContextProjector::new(policy.max_context_chars);

    let target = args
        .iter()
        .skip_while(|a| a.as_str() != "--target")
        .nth(1)
        .map(String::as_str)
        .unwrap_or("claude");

    match target {
        "claude" => {
            let path = root.join("CLAUDE.md");
            std::fs::write(&path, projector.render_claude_md(&manifest))?;
            println!("Written: {}", path.display());
        }
        "agents" => {
            let path = root.join("AGENTS.md");
            std::fs::write(&path, projector.render_agents_md(&manifest))?;
            println!("Written: {}", path.display());
        }
        "rules" => {
            let path = root.join(".rules");
            std::fs::write(&path, projector.render_rules_file(&manifest))?;
            println!("Written: {}", path.display());
        }
        other => bail!("Unknown export target: {other}. Use claude|agents|rules"),
    }
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn print_usage() {
    println!("nxalien v{VERSION}  — PSE agent-context exoskeleton");
    println!();
    println!("USAGE:");
    println!("  nxalien <SUBCOMMAND> [--root <dir>]");
    println!();
    println!("SUBCOMMANDS:");
    println!("  init      Create .nxalien/ with default policy");
    println!("  inspect   Print detected project substrate");
    println!("  compile   Full pipeline → nxalien.manifest.json + context files");
    println!("  ground    Print [NXALIEN-CONTEXT] block to stdout");
    println!("  handoff   Print handoff candidates JSON");
    println!("  replay    Verify replay hash chain");
    println!("  verify    Run gates + replay + canon verification");
    println!("  export    --target claude|agents|rules");
}

fn project_root(args: &[String]) -> PathBuf {
    args.iter()
        .skip_while(|a| a.as_str() != "--root")
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn load_policy(root: &Path) -> NxAlienPolicy {
    let p = root.join(".nxalien").join("policy.json");
    if p.exists() {
        if let Ok(s) = std::fs::read_to_string(&p) {
            if let Ok(pol) = serde_json::from_str(&s) {
                return pol;
            }
        }
    }
    NxAlienPolicy::default()
}

fn load_manifest(root: &Path) -> Result<NxAlienManifest> {
    let p = root.join("nxalien.manifest.json");
    if !p.exists() {
        bail!("nxalien.manifest.json not found — run `nxalien compile` first");
    }
    let s = std::fs::read_to_string(&p)?;
    Ok(serde_json::from_str(&s)?)
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let s = serde_json::to_string_pretty(value)?;
    std::fs::write(path, s).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn file_digest(path: &Path) -> Option<pse_nxalien_types::ArtifactDigest> {
    nxa_artifact_digest(path.to_str()?)
}

fn utc_now() -> String {
    // Deterministic: always use seed=0 for reproducibility.
    // A real implementation would use chrono or std::time.
    "2026-01-01T00:00:00Z".to_string()
}
