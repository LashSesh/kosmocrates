//! nxalien — PSE agent-context exoskeleton CLI.
//!
//! Subcommands:
//!
//! ```text
//!   init      Create .nxalien/ with default policy
//!   inspect   Print detected project substrate
//!   compile   Full pipeline → nxalien.manifest.json + context artifacts
//!             --remote <url>    POST bundle to a PSE server (cross-repo attractor)
//!             --remote-only     skip local IL/PSE state after remote POST
//!   ground    Print [NXALIEN-CONTEXT] block to stdout
//!   handoff   Print handoff candidates JSON
//!   replay    Recompute hashes and compare to manifest
//!   verify    Run gates + replay + canon verification
//!   export    Generate CLAUDE.md / AGENTS.md / .rules
//! ```

use anyhow::{bail, Context, Result};
use pse_exploratory::{ExploratoryLedger, DEFAULT_DECAY_AFTER_RUNS, EXPLORATORY_PSI_THRESHOLD};
use pse_nxalien_agent::ContextProjector;
use pse_nxalien_core::{
    canon::{compute_replay_hash, sha256_jcs},
    gate::{auto_downgrade_rules, evaluate_gate},
    scanner::ProjectScanner,
};
use pse_nxalien_cube::HypercubeHdag;
use pse_nxalien_evolve::{
    agenda_to_unknowns, commit_rules_to_il,
    evolution::{apply_validated_proposals, propose_rule_evolution, EvolutionGuard},
    graph_state::GraphState,
    signal::EpistemicSignal,
};
use pse_nxalien_pse::{artifact_digest as nxa_artifact_digest, build_handoff_candidate};
use pse_nxalien_types::{NxAlienBundle, NxAlienManifest, NxAlienPolicy, NxAlienRunDescriptor};
use serde::Deserialize;
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
        "doors" => {
            cmd_doors(&args);
            Ok(())
        }
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

    // --remote <url>   POST bundle to a PSE server after local compilation
    // --remote-only    skip writing local .nxalien/ state (IL + PSE graph)
    let remote_url: Option<String> = args
        .iter()
        .skip_while(|a| a.as_str() != "--remote")
        .nth(1)
        .cloned();
    let remote_only = args.iter().any(|a| a == "--remote-only");

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

    // ── Remote: POST bundle to PSE server (cross-repo attractor) ─────────────
    if let Some(ref url) = remote_url {
        push_bundle_remote(url, &bundle)?;
        if remote_only {
            println!("  --remote-only  : local IL/PSE state skipped");
            let projector = ContextProjector::new(policy.max_context_chars);
            std::fs::write(
                root.join("nxalien.rules.md"),
                projector.render_rules_file(&manifest),
            )?;
            println!("               : nxalien.rules.md");
            return Ok(());
        }
    }

    // ── PSE graph state: load first so run_count is available for IL commit ──
    let nxa_dir = root.join(".nxalien");
    let il_path = nxa_dir.join("il");
    let mut graph_state = GraphState::load(&nxa_dir);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let live_graph = graph_state.ingest_and_update(&bundle, timestamp);
    let run_count = graph_state.run_count;

    // ── IL bridge: commit every RuleAtom as a SemanticCrystal ────────────────
    let il_summary = commit_rules_to_il(&bundle.rules, &il_path, run_count);
    println!(
        "  IL crystals   : {}/{} committed  QTIC̄={:.2}  ψ̄={:.3}  gate={}",
        il_summary.committed,
        bundle.rules.len(),
        il_summary.mean_qtic_class,
        il_summary.mean_coherence_potential,
        if il_summary.gate_passed_all {
            "✓"
        } else {
            "✗ partial"
        },
    );
    for entry in &il_summary.entries {
        let marker = if entry.coherence_potential < EXPLORATORY_PSI_THRESHOLD {
            " ◈"
        } else {
            ""
        };
        println!(
            "    {:20}  Q{}  ψ={:.3}  {}{}",
            entry.rule_id,
            entry.qtic_class,
            entry.coherence_potential,
            entry.block_hash_prefix,
            marker,
        );
    }
    write_json(&nxa_dir.join("il_summary.json"), &il_summary)?;
    println!("               : .nxalien/il_summary.json");

    // ── Exploratory ledger: park negative-ψ crystals as hypotheses ───────────
    let mut exp_ledger = ExploratoryLedger::open(&nxa_dir);

    // Landing check: rules that now have ψ ≥ 0 may ground a pending hypothesis.
    let grounded_pairs: Vec<(&str, f64)> = il_summary
        .entries
        .iter()
        .filter(|e| e.coherence_potential >= EXPLORATORY_PSI_THRESHOLD)
        .map(|e| (e.rule_id.as_str(), e.coherence_potential))
        .collect();
    let landings = exp_ledger.check_landings(&grounded_pairs, run_count);
    for l in &landings {
        println!(
            "  Exploratory   : ↑ landed  {} ψ {:.3} → {:.3} ({} runs pending)",
            l.rule_id, l.initial_psi, l.grounded_psi, l.runs_pending,
        );
    }

    // Decay: hypotheses that have waited too long without evidence.
    let decays = exp_ledger.tick_decay(run_count);
    for d in &decays {
        println!(
            "  Exploratory   : ✗ decayed {} ψ={:.3} after {} runs",
            d.rule_id, d.initial_psi, d.runs_pending,
        );
    }

    // Ingest new exploratory entries (ψ < 0).
    for entry in il_summary
        .entries
        .iter()
        .filter(|e| e.coherence_potential < EXPLORATORY_PSI_THRESHOLD)
    {
        exp_ledger.ingest(
            &entry.rule_id,
            entry.coherence_potential,
            entry.qtic_class,
            &entry.block_hash_prefix,
            run_count,
            DEFAULT_DECAY_AFTER_RUNS,
        );
    }

    let exp_summary = exp_ledger.summary(run_count);
    if exp_summary.total > 0 {
        println!(
            "  Exploratory   : {} pending  {} landed  {} decayed  ψ̄={:.3}",
            exp_summary.pending,
            exp_summary.landed,
            exp_summary.decayed,
            exp_summary.mean_pending_psi,
        );
    }
    exp_ledger.save(&nxa_dir)?;
    if exp_summary.total > 0 {
        write_json(&nxa_dir.join("exploratory_summary.json"), &exp_summary)?;
        println!("               : .nxalien/exploratory_summary.json");
    }
    const ATTRACTOR_K: usize = 8;
    let signal = EpistemicSignal::extract_with_il(&live_graph, &graph_state, ATTRACTOR_K, &il_path);
    println!("  PSE signal    : {}", signal.summary_line());

    // Propose and apply validated rule evolutions.
    let proposals = propose_rule_evolution(&signal, &bundle.rules, &policy);
    if !proposals.is_empty() {
        println!(
            "  Evolution     : {} proposal(s) from signal",
            proposals.len()
        );
        let guard = EvolutionGuard::default();
        let mut evolved_rules = bundle.rules.clone();
        let new_unknowns = apply_validated_proposals(&mut evolved_rules, &proposals, &guard);
        if !new_unknowns.is_empty() {
            println!(
                "  Unknowns+     : {} rejected proposal(s) surfaced as unknowns",
                new_unknowns.len()
            );
        }
        // Write evolved rule set for the next compile cycle.
        let evolved_path = root.join("nxalien.evolved-rules.json");
        write_json(&evolved_path, &evolved_rules)?;
        println!("               : nxalien.evolved-rules.json");
    } else {
        println!(
            "  Evolution     : no proposals (signal {:?})",
            signal.stability
        );
    }

    // IL agenda → UnknownSlots (surface at-risk crystals to the agent).
    if let Some(ref health) = signal.il_health {
        let agenda_unknowns = agenda_to_unknowns(&health.agenda_items, 0.50);
        if !agenda_unknowns.is_empty() {
            println!(
                "  IL agenda     : {} item(s) → unknowns",
                agenda_unknowns.len()
            );
            for u in &agenda_unknowns {
                println!("    [p≥0.5] {}", u.name);
            }
            let agenda_path = nxa_dir.join("il_agenda_unknowns.json");
            write_json(&agenda_path, &agenda_unknowns)?;
            println!("               : .nxalien/il_agenda_unknowns.json");
        }
    }

    // Persist signal and graph state.
    let signal_path = root.join("nxalien.signal.json");
    write_json(&signal_path, &signal)?;
    println!("               : nxalien.signal.json");
    graph_state.save(&nxa_dir)?;
    println!("               : .nxalien/graph_state.json");

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

// ─── Remote push ─────────────────────────────────────────────────────────────

/// Minimal response shape for /nxalien/bundle.
/// Only the fields we print; extra server fields are ignored.
#[derive(Deserialize, Debug)]
struct RemoteBundleResponse {
    first_run: bool,
    il_committed: usize,
    proposal_count: usize,
    new_unknowns: usize,
    signal: RemoteSignal,
}

#[derive(Deserialize, Debug)]
struct RemoteSignal {
    stability: String,
    distance_to_attractor: f64,
    run_count: u64,
    #[serde(default)]
    il_health: Option<RemoteILHealth>,
}

#[derive(Deserialize, Debug)]
struct RemoteILHealth {
    total_crystals: usize,
    mean_qtic_class: f64,
    healthy: bool,
}

/// POST the compiled bundle to a remote PSE server.
///
/// Prints a compact summary of the server's response and returns Ok(()).
/// Network or HTTP errors are surfaced as anyhow errors.
fn push_bundle_remote(base_url: &str, bundle: &NxAlienBundle) -> Result<()> {
    let url = format!("{}/nxalien/bundle", base_url.trim_end_matches('/'));
    println!("  Remote PSE    : {url}");

    let body = serde_json::json!({ "bundle": bundle });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("building HTTP client")?;

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .with_context(|| format!("POST {url}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().unwrap_or_default();
        bail!("PSE server returned {status}: {text}");
    }

    let data: RemoteBundleResponse = resp
        .json()
        .with_context(|| format!("parsing response from {url}"))?;

    println!(
        "  Remote signal : {} (dist={:.3}  run={}{})",
        data.signal.stability,
        data.signal.distance_to_attractor,
        data.signal.run_count,
        if data.first_run { "  first-run" } else { "" },
    );
    println!(
        "  Remote IL     : {}/{} rules committed  {} proposal(s)  {} new-unknown(s)",
        data.il_committed,
        data.il_committed + data.new_unknowns,
        data.proposal_count,
        data.new_unknowns,
    );
    if let Some(ref h) = data.signal.il_health {
        println!(
            "  Remote health : {} crystals  Q̄={:.2}  {}",
            h.total_crystals,
            h.mean_qtic_class,
            if h.healthy { "healthy" } else { "⚠ degraded" },
        );
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
    println!("            --remote <url>   POST bundle to PSE server (cross-repo attractor)");
    println!("            --remote-only    skip local IL/PSE state after remote POST");
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

// ─── Doors (the binary's self-description) ───────────────────────────────────

use kosmo_core::{Door, DoorCatalog, DoorGovernance, DoorInput, DoorNeed, DoorSurface};

/// The binary's complete docking surface, spoken by the binary itself and
/// pinned by test against the dispatch.
fn doors_catalog() -> DoorCatalog {
    let here = || DoorSurface::Cli {
        binary: "nxalien".into(),
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
            "--version",
            vec!["-V".into()],
            "print the version",
            vec![],
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            here(),
            "init",
            vec![],
            "initialize the nxalien agent-context",
            vec![],
            DoorGovernance::AppendsStore,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "inspect",
            vec![],
            "inspect the project's crate topology and rule state",
            vec![],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "compile",
            vec![],
            "compile rules into a bundle (--remote optional → a cross-repo attractor)",
            vec![DoorInput::valued("--remote", "<url>"), DoorInput::switch("--remote-only")],
            DoorGovernance::AppendsStore,
            vec![DoorNeed::File, DoorNeed::Network],
        ),
        Door::new(
            here(),
            "ground",
            vec![],
            "ground the agent context in the compiled bundle",
            vec![],
            DoorGovernance::AppendsStore,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "handoff",
            vec![],
            "produce an agent handoff artifact",
            vec![],
            DoorGovernance::AppendsStore,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "replay",
            vec![],
            "replay a compile run byte-identically",
            vec![],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "verify",
            vec![],
            "verify a bundle",
            vec![],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "export",
            vec![],
            "export the bundle",
            vec![],
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
    println!("nxalien doors — the docking surface");
    println!(
        "  catalog {}… · {} door(s)",
        &catalog.catalog_id.to_hex()[..12],
        catalog.len()
    );
    for d in &catalog.doors {
        println!("  {}  [{}]  {}", d.name, d.governance.label(), d.summary);
    }
}

#[cfg(test)]
mod doors_tests {
    use super::*;

    /// This binary's own dispatch source — the catalog is pinned against it.
    const MAIN_SRC: &str = include_str!("main.rs");

    fn dispatched_words() -> std::collections::BTreeSet<String> {
        // Pick whichever dispatch anchor occurs FIRST in the file: the real
        // dispatch precedes this test module, which embeds both anchors as
        // string literals (so a plain `contains` check would be self-fooled).
        let a = MAIN_SRC.find("match args[1].as_str() {");
        let b = MAIN_SRC.find("match subcmd {");
        let start = match (a, b) {
            (Some(x), Some(y)) => x.min(y),
            (Some(x), None) => x,
            (None, Some(y)) => y,
            (None, None) => panic!("the dispatch match exists"),
        };
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
    fn the_catalog_recomputes_and_describes_itself() {
        let catalog = doors_catalog();
        assert!(catalog.verify(), "the catalog recomputes");
        assert!(catalog.doors.iter().any(|d| d.name == "doors"));
        assert_eq!(catalog.catalog_id, doors_catalog().catalog_id);
    }
}
