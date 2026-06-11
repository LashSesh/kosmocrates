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

use kosmo_core::{Digest, ImplementationMode, PolicyProfile, PromotionFeedback};
use kosmo_pipeline::{crystal_to_pse_candidate, run_workspace_pipeline, IntegrationRunOptions};
use kosmo_pse_bridge::{
    build_promotion_feedback, PromotionRequestRecord, PseBridgeCandidate, PseBridgeCandidateKind,
    PseBridgePolicy, PseBridgeRateLimit,
};
use kosmo_store::CrystalRecordStore;
use pse_adapter_kosmo::{describe_crystal, offer_candidates, KosmoBridgeAdapter};
use pse_core::{load_memory_from_crystals, GlobalState};
use pse_types::{Config, SemanticCrystal};

struct Args {
    path: String,
    offer: bool,
    all_kinds: bool,
    json: bool,
    state: Option<PathBuf>,
    store: Option<PathBuf>,
    feedback: Option<PathBuf>,
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
    --store <path>  Also offer the certified crystals in this CAD-library\n\
                    store (kosmo-substrate --store JSONL). Integrity-checked;\n\
                    read-only.\n\
    --state <path>  Persist the crystal archive across sessions (JSON).\n\
                    Loaded on start when present; written back only in\n\
                    --offer mode. The flag is the write authorization.\n\
    --feedback <p>  Close the memory→action loop: engine verdicts are\n\
                    written as PromotionFeedback (JSON) and fed into the\n\
                    NEXT run's pipeline (prior_feedback → norm fitness).\n\
                    Loaded on start; written only in --offer mode.\n\
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
        store: None,
        feedback: None,
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
            "--store" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--store requires a file path".into());
                }
                args.store = Some(PathBuf::from(&raw[i]));
            }
            "--feedback" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--feedback requires a file path".into());
                }
                args.feedback = Some(PathBuf::from(&raw[i]));
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

/// Load persisted promotion feedback, or empty when the file does not exist
/// yet. A present-but-unreadable file is a hard error (fail-closed).
fn load_feedback(path: &PathBuf) -> Result<Vec<PromotionFeedback>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("corrupt feedback {}: {e}", path.display()))
}

/// Persist the merged feedback pool. Only called in `--offer` mode with an
/// explicit `--feedback` path — the flag is the operator's write authorization.
fn save_feedback(path: &PathBuf, feedback: &[PromotionFeedback]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
    }
    let json = serde_json::to_vec(feedback).map_err(|e| format!("serialize feedback: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("cannot write {}: {e}", path.display()))
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

    // 0. Memory→action: prior engine verdicts feed THIS run's pipeline.
    //    (PromotionFeedback → prior_feedback → norm fitness, pipeline Step 5c.)
    let prior_feedback: Vec<PromotionFeedback> = match &args.feedback {
        Some(path) => match load_feedback(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(1);
            }
        },
        None => Vec::new(),
    };
    let feedback_loaded = prior_feedback.len();

    // 1. Substrate pipeline — always a passive, report-only scan.
    let pipeline_policy = PolicyProfile::default_report_only();
    let mut options = IntegrationRunOptions::all_layers(8);
    options.prior_feedback = prior_feedback.clone();
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
    let mut candidates: Vec<PseBridgeCandidate> = report
        .pse_candidates
        .iter()
        .filter(|c| kinds.contains(&c.kind))
        .cloned()
        .collect();

    // 2b. CAD-library store as a candidate source (read-only). Every record is
    //     directly evidence-bound, so it wraps without resolving its candidate.
    //     The store is integrity-checked first — a record whose content does not
    //     match its record_id is a hard error, never offered.
    let store_loaded = match &args.store {
        Some(path) => {
            let store = match CrystalRecordStore::open(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: cannot open store {}: {e}", path.display());
                    process::exit(1);
                }
            };
            if let Err(e) = store.verify_integrity() {
                eprintln!("error: store integrity {}: {e}", path.display());
                process::exit(1);
            }
            let n = store.len();
            candidates.extend(
                store
                    .records()
                    .iter()
                    .map(|r| crystal_to_pse_candidate(r, report.report_id, pipeline_policy.id)),
            );
            n
        }
        None => 0,
    };

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

    // 6b. Action→memory: persist this run's engine verdicts as
    //     PromotionFeedback for the NEXT run's pipeline. Norm-derived
    //     candidates (label `norm:…`) key their feedback to the
    //     NormGeneCandidate id they observe (= observation_digest), so the
    //     fitness loop closes; all other kinds carry ZERO (no norm
    //     association, per the PromotionFeedback contract). Merged by id —
    //     re-offering the same candidate with the same verdict is idempotent.
    let feedback_written = match (&args.feedback, args.offer) {
        (Some(path), true) => {
            let mut merged = prior_feedback.clone();
            for (offer, candidate) in offers.iter().zip(candidates.iter()) {
                let record = PromotionRequestRecord::new(
                    candidate.id,
                    offer.outcome.clone(),
                    candidate.evidence_bundle_id,
                    0,
                );
                let norm_id = if candidate.label.starts_with("norm:") {
                    candidate.observation_digest
                } else {
                    Digest::ZERO
                };
                let fb = build_promotion_feedback(&record, candidate, norm_id, pipeline_policy.id);
                if !merged.iter().any(|f| f.id == fb.id) {
                    merged.push(fb);
                }
            }
            match save_feedback(path, &merged) {
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
                    "qtic_class": o.qtic.as_ref().map(|q| q.class_u8()),
                })
            })
            .collect();
        let doc = serde_json::json!({
            "path": args.path,
            "report_id": report.report_id.to_hex(),
            "offered": candidates.len(),
            "engine_commit_index": state.commit_index,
            "mode": if args.offer { "offer" } else { "report-only" },
            "store_loaded": store_loaded,
            "feedback_loaded": feedback_loaded,
            "feedback_written": feedback_written,
            "fitness_traces": report.norm_fitness_traces.len(),
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
            "  pipeline report {} | pse_candidates {} | store {} | selected {}",
            &report.report_id.to_hex()[..16],
            report.pse_candidates.len(),
            store_loaded,
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
                if let Some(qtic) = &offer.qtic {
                    println!(
                        "      └─ QTIC Q{} — {}",
                        qtic.class_u8(),
                        qtic.class_description
                    );
                }
            }
        }
        println!(
            "  outcomes: {accepted} accepted | {deferred} deferred | {rejected} rejected | {skipped} skipped"
        );
        println!("  engine commit_index: {}", state.commit_index);
        if let Some(path) = &args.feedback {
            println!(
                "  feedback: {} loaded → {} fitness traces this run{}",
                feedback_loaded,
                report.norm_fitness_traces.len(),
                if feedback_written {
                    format!(" | verdicts → {}", path.display())
                } else {
                    " | not written (report-only)".to_string()
                }
            );
        }
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
