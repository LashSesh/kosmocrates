//! Kosmocrates promotion CLI — the operator entry point for substrate→core
//! promotion.
//!
//! Runs the full substrate pipeline on a workspace, takes the gated
//! `PseBridgeCandidate`s the pipeline emitted, and offers them to the PSE
//! crystallization engine through `pse-adapter-kosmo` — the sanctioned
//! crossing. PSE alone decides crystallization.
//!
//! ```text
//! USAGE:
//!     kosmo-promote [OPTIONS] [PATH]
//!
//! OPTIONS:
//!     --offer         Actually feed the engine (DryRun profile; in-memory, no
//!                     host writes). Default is report-only: candidates are
//!                     listed and every outcome is SkippedByReportOnly.
//!     --all-kinds     Offer all pipeline candidate kinds, not only
//!                     CertifiedCrystal (adds Structural/Topology observations).
//!     --state <path>  Persist the engine's crystal archive across sessions
//!                     (JSON array of SemanticCrystal). Loaded on start when
//!                     present; written back only in --offer mode. The explicit
//!                     flag is the operator's write authorization — without it
//!                     nothing touches the disk.
//!     --json          Machine-readable output.
//!     -h, --help      This help.
//! ```
//!
//! Fail-closed defaults: without `--offer` the engine is never touched; the
//! adapter allowlist starts at CertifiedCrystal only; without `--state`
//! nothing is persisted.
//!
//! With `--state`, repeated `--offer` runs accumulate engine pattern memory
//! across sessions (the `pse-core` cross-session mechanism): prior crystals
//! warm-start `PatternMemory`, so recurring substrate output can build the
//! resonance that eventually flips `Deferred` into `Accepted`.

use std::path::PathBuf;
use std::process;

use kosmo_core::{ImplementationMode, PolicyProfile};
use kosmo_pipeline::{run_workspace_pipeline, IntegrationRunOptions};
use kosmo_pse_bridge::{PseBridgeCandidateKind, PseBridgePolicy, PseBridgeRateLimit};
use pse_adapter_kosmo::{describe_crystal, offer_candidates, KosmoBridgeAdapter};
use pse_core::{load_memory_from_crystals, GlobalState};
use pse_types::{Config, SemanticCrystal};

struct Args {
    path: String,
    offer: bool,
    all_kinds: bool,
    json: bool,
    state: Option<PathBuf>,
}

const HELP: &str = "kosmo-promote — substrate→core promotion (PSE decides crystallization)\n\
\n\
USAGE:\n\
    kosmo-promote [OPTIONS] [PATH]\n\
\n\
OPTIONS:\n\
    --offer         Feed the PSE engine (DryRun profile; in-memory, no host\n\
                    writes). Default is report-only: every outcome is\n\
                    SkippedByReportOnly and the engine is never touched.\n\
    --all-kinds     Offer all pipeline candidate kinds, not only\n\
                    CertifiedCrystal.\n\
    --state <path>  Persist the crystal archive across sessions (JSON).\n\
                    Loaded on start when present; written back only in\n\
                    --offer mode. The flag is the write authorization.\n\
    --json          Machine-readable output.\n\
    -h, --help      This help.\n";

fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut args = Args {
        path: ".".into(),
        offer: false,
        all_kinds: false,
        json: false,
        state: None,
    };
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                process::exit(0);
            }
            "--offer" => args.offer = true,
            "--all-kinds" => args.all_kinds = true,
            "--json" => args.json = true,
            "--state" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--state requires a file path".into());
                }
                args.state = Some(PathBuf::from(&raw[i]));
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown flag '{flag}'; run --help for usage"));
            }
            path => args.path = path.to_string(),
        }
        i += 1;
    }
    Ok(args)
}

/// Load the persisted crystal archive, or an empty one when the file does not
/// exist yet (cold start). A present-but-unreadable file is a hard error —
/// silently cold-starting over a corrupt archive would discard memory.
fn load_state(path: &PathBuf) -> Result<Vec<SemanticCrystal>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("corrupt state {}: {e}", path.display()))
}

/// Persist the merged crystal archive. Only called in `--offer` mode with an
/// explicit `--state` path — the flag is the operator's write authorization.
fn save_state(path: &PathBuf, crystals: &[SemanticCrystal]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
    }
    let json = serde_json::to_vec(crystals).map_err(|e| format!("serialize state: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(2);
        }
    };

    // 1. Substrate pipeline — always a passive, report-only scan.
    let pipeline_policy = PolicyProfile::default_report_only();
    let options = IntegrationRunOptions::all_layers(8);
    let report = match run_workspace_pipeline(&args.path, &options, &pipeline_policy) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: pipeline failed: {e}");
            process::exit(1);
        }
    };

    // 2. Select the candidate kinds to offer (fail-closed default: certified only).
    let kinds = if args.all_kinds {
        vec![
            PseBridgeCandidateKind::CertifiedCrystal,
            PseBridgeCandidateKind::StructuralObservation,
            PseBridgeCandidateKind::TopologyObservation,
        ]
    } else {
        vec![PseBridgeCandidateKind::CertifiedCrystal]
    };
    let candidates: Vec<_> = report
        .pse_candidates
        .iter()
        .filter(|c| kinds.contains(&c.kind))
        .cloned()
        .collect();

    // 3. Promotion profile — the gate that decides whether the engine runs.
    //    Default ReportOnly: validate_candidate short-circuits, engine untouched.
    let profile = if args.offer {
        PolicyProfile {
            mode: ImplementationMode::DryRun,
            ..PolicyProfile::default()
        }
    } else {
        PolicyProfile::default() // ReportOnly
    };
    let bridge_policy =
        PseBridgePolicy::allow(pipeline_policy.id, kinds, PseBridgeRateLimit::strict());
    let adapter = KosmoBridgeAdapter::new(&args.path)
        .with_allowed_kinds(bridge_policy.allowed_candidate_kinds.clone());

    // 4. Engine state — warm-start pattern memory from a prior session when an
    //    archive is provided (pse-core cross-session mechanism).
    let config = Config::default();
    let mut state = GlobalState::new(&config);
    let prior_crystals: Vec<SemanticCrystal> = match &args.state {
        Some(path) => match load_state(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(1);
            }
        },
        None => Vec::new(),
    };
    let memory_loaded = load_memory_from_crystals(&mut state, &prior_crystals);

    // 5. Offer — one engine step per candidate, per-candidate verdicts.
    let offers = offer_candidates(
        &mut state,
        &config,
        &profile,
        &bridge_policy,
        &candidates,
        &adapter,
    );

    // 6. Persist the merged archive — only in --offer mode and only when the
    //    operator authorized the write with an explicit --state path.
    let session_crystals = state.archive.crystals().to_vec();
    let state_written = match (&args.state, args.offer) {
        (Some(path), true) => {
            let mut merged = prior_crystals.clone();
            merged.extend(session_crystals.iter().cloned());
            match save_state(path, &merged) {
                Ok(()) => true,
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            }
        }
        _ => false,
    };

    // 7. Report.
    if args.json {
        let rows: Vec<serde_json::Value> = offers
            .iter()
            .zip(candidates.iter())
            .map(|(o, c)| {
                serde_json::json!({
                    "candidate_id": c.id.to_hex(),
                    "kind": format!("{:?}", c.kind),
                    "label": c.label,
                    "confidence_raw": c.confidence.raw(),
                    "outcome": o.outcome,
                    "crystal_committed": o.crystal.is_some(),
                })
            })
            .collect();
        let doc = serde_json::json!({
            "path": args.path,
            "report_id": report.report_id.to_hex(),
            "offered": candidates.len(),
            "engine_commit_index": state.commit_index,
            "mode": if args.offer { "offer" } else { "report-only" },
            "memory_loaded": memory_loaded,
            "pattern_hits": state.pattern_hits,
            "new_crystals": session_crystals.len(),
            "state_written": state_written,
            "offers": rows,
        });
        println!("{}", serde_json::to_string_pretty(&doc).expect("json"));
    } else {
        println!("kosmo-promote  {}", args.path);
        println!(
            "  pipeline report {} | pse_candidates {} | selected {}",
            &report.report_id.to_hex()[..16],
            report.pse_candidates.len(),
            candidates.len()
        );
        println!(
            "  mode: {}",
            if args.offer {
                "OFFER (DryRun profile — engine runs in-memory)"
            } else {
                "report-only (engine untouched; use --offer to feed PSE)"
            }
        );
        let mut accepted = 0usize;
        let mut deferred = 0usize;
        let mut rejected = 0usize;
        let mut skipped = 0usize;
        for (offer, candidate) in offers.iter().zip(candidates.iter()) {
            use kosmo_pse_bridge::PromotionOutcome as O;
            let verdict = match &offer.outcome {
                O::Accepted => {
                    accepted += 1;
                    "ACCEPTED".to_string()
                }
                O::Deferred => {
                    deferred += 1;
                    "deferred".to_string()
                }
                O::Rejected { reason } => {
                    rejected += 1;
                    format!("rejected: {reason}")
                }
                O::SkippedByPolicy | O::SkippedByReportOnly => {
                    skipped += 1;
                    "skipped (policy)".to_string()
                }
            };
            println!("    {:<44} {}", candidate.label, verdict);
            if let Some(crystal) = &offer.crystal {
                println!("      └─ {}", describe_crystal(crystal, candidate));
            }
        }
        println!(
            "  outcomes: {accepted} accepted | {deferred} deferred | {rejected} rejected | {skipped} skipped"
        );
        println!("  engine commit_index: {}", state.commit_index);
        if let Some(path) = &args.state {
            println!(
                "  memory: {} crystals loaded | {} pattern hits | {} new this session{}",
                memory_loaded,
                state.pattern_hits,
                session_crystals.len(),
                if state_written {
                    format!(" | archive → {}", path.display())
                } else {
                    " | archive not written (report-only)".to_string()
                }
            );
        }
    }
}
