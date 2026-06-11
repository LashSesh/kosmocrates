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
//!     --offer              Feed the engine (DryRun profile). Default is
//!                          report-only: the engine is never touched.
//!     --all-kinds          Offer all pipeline candidate kinds, not only
//!                          CertifiedCrystal.
//!     --batch              ONE ensemble step instead of one step per
//!                          candidate — co-observation forms the edges the
//!                          connectivity metric needs; attribution stays
//!                          per-candidate via crystal.region.
//!     --ticks <n>          Re-observe the ensemble n times (temporal
//!                          stability).
//!     --calibration <m>    default | planning | adaptive | substrate —
//!                          explicit operator choice (changes what may
//!                          become memory).
//!     --state <path>       Engine crystal archive across sessions (JSON).
//!     --store <path>       CAD library (JSONL) as additional source.
//!     --feedback <path>    Engine verdicts → next run's prior_feedback.
//!     --ledger <path>      Anchor accepted crystals in the Infinity Ledger
//!                          (block + IL-HDAG node + path invariance) —
//!                          lifts QTIC from Q3 to Q5.
//!     --recall <query>     Query the anchored memory instead of promoting:
//!                          Pfauenthron++ (D = ψ·ρ·ω) over the ledger, with
//!                          the top hit's causal lineage. Read-only.
//!     --top <k>            Number of recall results (default 5).
//!     --json               Machine-readable output.
//! ```
//!
//! Fail-closed defaults: without `--offer` the engine is never touched and
//! nothing is written; every path flag is the operator's explicit write
//! authorization; the conservative default calibration commits nothing.
//!
//! The full loop: with `--batch --calibration substrate` a workspace's
//! certified structure crystallizes; `--state` warm-starts `PatternMemory`
//! next session; `--feedback` folds verdicts into the pipeline's norm
//! fitness; `--ledger` lifts the crystals to full QTIC (Q5); `--recall`
//! makes the anchored memory queryable.

use std::path::PathBuf;
use std::process;

use kosmo_core::{Digest, ImplementationMode, PolicyProfile, PromotionFeedback};
use kosmo_pipeline::{crystal_to_pse_candidate, run_workspace_pipeline, IntegrationRunOptions};
use kosmo_pse_bridge::{
    build_promotion_feedback, PromotionRequestRecord, PseBridgeCandidate, PseBridgeCandidateKind,
    PseBridgePolicy, PseBridgeRateLimit,
};
use kosmo_store::CrystalRecordStore;
use pse_adapter_kosmo::{describe_crystal, offer_batch, offer_candidates, KosmoBridgeAdapter};
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
    ledger: Option<PathBuf>,
    calibration: Calibration,
    batch: bool,
    ticks: u32,
    recall: Option<String>,
    top: usize,
}

/// Engine threshold calibration mode — an explicit operator choice because it
/// changes what may become memory (never a silent default).
#[derive(Clone, Copy, PartialEq)]
enum Calibration {
    /// Static sensor-tuned thresholds (`Config::default()`); deterministic.
    Default,
    /// Static thresholds tuned for structured, low-entropy planning artifacts
    /// (`Config::preset_planning()`); deterministic. The documented fit for
    /// substrate candidates.
    Planning,
    /// Rolling-quantile thresholds (`Config::preset_streaming()`): the gate
    /// fires on the top 5% of recent ticks. Trades cross-session determinism
    /// for out-of-the-box deployability on uncalibrated streams.
    Adaptive,
    /// Planning thresholds + the kairos gate as the sole discriminant —
    /// `consensus_threshold`/`mirror_consistency_eta` at 0, following the
    /// `preset_anomaly_detection` rationale verbatim: the cascade PI operator
    /// zeros `primal_score` after DK+SW mutate the carrier phase, so for
    /// workloads the carrier physics wasn't pre-tuned for, the 8-fold
    /// conjunctive kairos gate is the real fail-closed discriminant.
    /// Deterministic.
    Substrate,
}

impl Calibration {
    fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Planning => "planning",
            Self::Adaptive => "adaptive",
            Self::Substrate => "substrate",
        }
    }

    fn engine_config(self) -> Config {
        match self {
            Self::Default => Config::default(),
            Self::Planning => Config::preset_planning(),
            Self::Adaptive => Config::preset_streaming(),
            Self::Substrate => {
                let mut c = Config::preset_planning();
                // Kairos as the discriminant (the preset_anomaly_detection
                // rationale): consensus 0 lets any non-negative cascade score
                // through; the 8-fold conjunctive gate remains fully armed.
                c.consensus.consensus_threshold = 0.0;
                c.consensus.mirror_consistency_eta = 0.0;
                c
            }
        }
    }
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
    --batch         Offer all candidates as ONE ensemble step instead of one\n\
                    step each. Co-observation forms the graph edges the\n\
                    connectivity metric (j) needs — required for substrate\n\
                    output to resonate. Attribution stays per-candidate via\n\
                    crystal.region membership.\n\
    --ticks <n>     Re-observe the (batch) ensemble n times (default 1).\n\
                    Crystallization is temporal: a pattern commits when it\n\
                    holds across ticks, so a stable workspace observed over\n\
                    n ticks is exactly the stability the engine certifies.\n\
    --ledger <path> Anchor accepted crystals in the Infinity Ledger at\n\
                    <path> (ledger block + IL-HDAG node + path invariance).\n\
                    Lifts QTIC certificates beyond Q3 toward Q4 (auditable)\n\
                    and Q5 (path-invariant). Host write — the flag is the\n\
                    operator's authorization; only acts in --offer mode.\n\
    --recall <q>    Query the anchored memory instead of promoting:\n\
                    Pfauenthron++ retrieval (D = ψ·ρ·ω) over the ledger's\n\
                    crystals, plus the causal lineage of the top hit.\n\
                    Requires --ledger; read-only (the ledger must exist).\n\
    --top <k>       Number of recall results (default 5).\n\
    --calibration <mode>\n\
                    Engine threshold calibration — an explicit operator\n\
                    choice because it changes what may become memory:\n\
                      default   static sensor-tuned thresholds (deterministic)\n\
                      planning  static thresholds tuned for structured,\n\
                                low-entropy planning artifacts (deterministic;\n\
                                the documented fit for substrate candidates)\n\
                      adaptive  rolling-quantile thresholds (top 5% of recent\n\
                                ticks fire; trades cross-session determinism\n\
                                for out-of-the-box deployability)\n\
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
        ledger: None,
        calibration: Calibration::Default,
        batch: false,
        ticks: 1,
        recall: None,
        top: 5,
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
            "--batch" => args.batch = true,
            "--ticks" => {
                i += 1;
                args.ticks = raw
                    .get(i)
                    .ok_or("--ticks requires a number")?
                    .parse()
                    .map_err(|_| "--ticks must be a positive number".to_string())?;
                if args.ticks == 0 {
                    return Err("--ticks must be at least 1".into());
                }
            }
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
            "--ledger" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--ledger requires a directory path".into());
                }
                args.ledger = Some(PathBuf::from(&raw[i]));
            }
            "--recall" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--recall requires a query string".into());
                }
                args.recall = Some(raw[i].clone());
            }
            "--top" => {
                i += 1;
                args.top = raw
                    .get(i)
                    .ok_or("--top requires a number")?
                    .parse()
                    .map_err(|_| "--top must be a positive number".to_string())?;
                if args.top == 0 {
                    return Err("--top must be at least 1".into());
                }
            }
            "--calibration" => {
                i += 1;
                args.calibration = match raw.get(i).map(String::as_str) {
                    Some("default") => Calibration::Default,
                    Some("planning") => Calibration::Planning,
                    Some("adaptive") => Calibration::Adaptive,
                    Some("substrate") => Calibration::Substrate,
                    Some(other) => {
                        return Err(format!(
                            "unknown calibration '{other}' (default|planning|adaptive|substrate)"
                        ));
                    }
                    None => return Err("--calibration requires a mode".into()),
                };
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

/// Query the anchored memory: Pfauenthron++ retrieval (`D = ψ·ρ·ω`,
/// `ILStore::build_context_entries`) over the ledger's crystals, plus the
/// causal lineage of the top hit. Read-only — recall never creates a ledger,
/// so a missing path is a hard error, not a silent empty store.
fn run_recall(query: &str, args: &Args) -> ! {
    let Some(path) = &args.ledger else {
        eprintln!("error: --recall requires --ledger <path>");
        process::exit(2);
    };
    if !path.exists() {
        eprintln!("error: no ledger at {} — nothing to recall", path.display());
        process::exit(1);
    }
    let il = match pse_adapter_il::ILStore::open(path, "kosmo-promote") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot open ledger {}: {e}", path.display());
            process::exit(1);
        }
    };
    let query_vec = pse_adapter_il::text_to_vector8(query);
    let mut entries = il.build_context_entries(&query_vec);
    entries.truncate(args.top);

    if args.json {
        let rows: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "crystal_id": e.crystal_id,
                    "tripolar_score": e.tripolar_score,
                    "qtic_class": e.qtic_class,
                    "stability": e.stability,
                    "commit_index": e.commit_index,
                    "scale_tag": e.scale_tag,
                    "question": e.question,
                })
            })
            .collect();
        let doc = serde_json::json!({
            "query": query,
            "ledger": path.display().to_string(),
            "anchored_crystals": il.len(),
            "results": rows,
        });
        println!("{}", serde_json::to_string_pretty(&doc).expect("json"));
    } else {
        println!("kosmo-promote --recall  {:?}", query);
        println!(
            "  ledger {} | {} anchored crystal(s) | top {}",
            path.display(),
            il.len(),
            args.top
        );
        if entries.is_empty() {
            println!("  (no crystal resonates with this query — D = ψ·ρ·ω ≤ 0 for all)");
        }
        for (rank, e) in entries.iter().enumerate() {
            println!(
                "    {:>2}. D={:.4} | Q{} | stability {:.2} | t={} | {} | {}",
                rank + 1,
                e.tripolar_score,
                e.qtic_class
                    .map(|q| q.to_string())
                    .unwrap_or_else(|| "-".into()),
                e.stability,
                e.commit_index,
                e.crystal_id,
                e.question
            );
        }
        // The lineage of the best hit — retrieval returns not just a fact
        // but where it came from.
        if let Some(top) = entries.first() {
            println!("  lineage of top hit:");
            for line in il.causal_explanation(&top.crystal_id).lines() {
                println!("    {line}");
            }
        }
    }
    process::exit(0)
}

/// The thresholds the Kairos gate actually applied on the last step: the
/// adaptive calibrator's quantile-derived values when active, the static
/// config thresholds otherwise.
fn effective_thresholds(state: &GlobalState, config: &Config) -> pse_types::ThresholdConfig {
    match &state.adaptive {
        Some(c) => c.calibrate(&config.thresholds),
        None => config.thresholds.clone(),
    }
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

    // Recall mode: query the anchored memory instead of promoting.
    if let Some(query) = args.recall.clone() {
        run_recall(&query, &args);
    }

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
    let config = args.calibration.engine_config();
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

    // 5. Offer. Singles: one engine step per candidate. Batch: one ensemble
    //    step — co-observation forms the edges the connectivity metric needs;
    //    attribution stays per-candidate via crystal.region membership.
    //    With --ticks > 1 the ensemble is re-observed: crystallization is
    //    temporal, so a stable workspace held across ticks is exactly the
    //    stability the engine certifies. An acceptance from any tick stands.
    let mut offers: Vec<pse_adapter_kosmo::CrystalOffer> = Vec::new();
    for tick in 0..args.ticks {
        let tick_offers = if args.batch {
            offer_batch(
                &mut state,
                &config,
                &profile,
                &bridge_policy,
                &candidates,
                &adapter,
            )
        } else {
            offer_candidates(
                &mut state,
                &config,
                &profile,
                &bridge_policy,
                &candidates,
                &adapter,
            )
        };
        if tick == 0 {
            offers = tick_offers;
        } else {
            for (acc, new) in offers.iter_mut().zip(tick_offers) {
                if !acc.outcome.is_accepted() {
                    *acc = new;
                }
            }
        }
    }

    // 5b. Infinity-Ledger lift — anchor accepted crystals (operator-authorized
    //     host write; the flag is the authorization, only acts in --offer mode).
    //     Each unique crystal is committed once: ledger block + IL-HDAG node +
    //     path-invariance check. The returned QTIC certificate supersedes the
    //     promotion path's Q3 ceiling (toward Q4 auditable / Q5 path-invariant)
    //     and is applied to every offer sharing that crystal.
    let mut block_hashes: Vec<Option<String>> = vec![None; offers.len()];
    let ledger_commits = match (&args.ledger, args.offer) {
        (Some(path), true) => {
            let mut il = match pse_adapter_il::ILStore::open(path, "kosmo-promote") {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: cannot open ledger {}: {e}", path.display());
                    process::exit(1);
                }
            };
            let question = format!("kosmo-promote:{}", args.path);
            let mut anchored: std::collections::BTreeMap<
                [u8; 32],
                (String, Option<pse_adapter_il::qtic::QticCertificate>),
            > = std::collections::BTreeMap::new();
            for (i, offer) in offers.iter().enumerate() {
                let Some(crystal) = &offer.crystal else {
                    continue;
                };
                if anchored.contains_key(&crystal.crystal_id) {
                    continue;
                }
                let chunks = vec![candidates[i].label.clone()];
                match il.commit_with_feedback(
                    crystal,
                    &chunks,
                    state.commit_index as usize,
                    &question,
                ) {
                    Ok(fb) => {
                        anchored.insert(crystal.crystal_id, (fb.block_hash, fb.qtic_certificate));
                    }
                    Err(e) => eprintln!(
                        "warning: ledger commit failed for {}: {e}",
                        candidates[i].label
                    ),
                }
            }
            let commits = anchored.len();
            for (i, offer) in offers.iter_mut().enumerate() {
                let Some(crystal) = &offer.crystal else {
                    continue;
                };
                if let Some((hash, cert)) = anchored.get(&crystal.crystal_id) {
                    block_hashes[i] = Some(hash.clone());
                    if let Some(cert) = cert {
                        offer.qtic = Some(cert.clone());
                    }
                }
            }
            commits
        }
        _ => 0,
    };

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
            .zip(block_hashes.iter())
            .map(|((o, c), bh)| {
                serde_json::json!({
                    "candidate_id": c.id.to_hex(),
                    "kind": format!("{:?}", c.kind),
                    "label": c.label,
                    "confidence_raw": c.confidence.raw(),
                    "outcome": o.outcome,
                    "crystal_committed": o.crystal.is_some(),
                    "qtic_class": o.qtic.as_ref().map(|q| q.class_u8()),
                    "block_hash": bh,
                })
            })
            .collect();
        let doc = serde_json::json!({
            "path": args.path,
            "report_id": report.report_id.to_hex(),
            "offered": candidates.len(),
            "engine_commit_index": state.commit_index,
            "engine_state": format!("{:?}", state.engine_state),
            "last_gate": state.last_gate.as_ref().map(|g| {
                let th = effective_thresholds(&state, &config);
                serde_json::json!({
                    "kairos": g.kairos,
                    "metrics": {
                        "d": { "value": g.d, "threshold": th.d, "pass": g.d >= th.d },
                        "q": { "value": g.q, "threshold": th.q, "pass": g.q >= th.q },
                        "r": { "value": g.r, "threshold": th.r, "pass": g.r >= th.r },
                        "g": { "value": g.g, "threshold": th.g, "pass": g.g >= th.g },
                        "j": { "value": g.j, "threshold": th.j, "pass": g.j >= th.j },
                        "p": { "value": g.p, "threshold": th.p, "pass": g.p >= th.p },
                        "n": { "value": g.n, "threshold": th.n, "pass": g.n >= th.n },
                        "k": { "value": g.k, "threshold": th.k, "pass": g.k >= th.k },
                    },
                })
            }),
            "mode": if args.offer { "offer" } else { "report-only" },
            "calibration": args.calibration.as_str(),
            "store_loaded": store_loaded,
            "ledger_commits": ledger_commits,
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
            "  mode: {} | calibration: {}",
            if args.offer {
                "OFFER (DryRun profile — engine runs in-memory)"
            } else {
                "report-only (engine untouched; use --offer to feed PSE)"
            },
            args.calibration.as_str()
        );
        let mut accepted = 0usize;
        let mut deferred = 0usize;
        let mut rejected = 0usize;
        let mut skipped = 0usize;
        for ((offer, candidate), block_hash) in offers
            .iter()
            .zip(candidates.iter())
            .zip(block_hashes.iter())
        {
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
                if let Some(hash) = block_hash {
                    println!("      └─ IL: block {}", &hash[..hash.len().min(16)]);
                }
            }
        }
        println!(
            "  outcomes: {accepted} accepted | {deferred} deferred | {rejected} rejected | {skipped} skipped"
        );
        println!("  engine commit_index: {}", state.commit_index);
        // Why deferred? Show the last gate snapshot against the effective
        // thresholds — the conjunctive Kairos gate needs every metric to pass.
        if args.offer {
            if let Some(g) = &state.last_gate {
                let th = effective_thresholds(&state, &config);
                let m = |name: &str, v: f64, t: f64| {
                    format!("{name} {v:.3}/{t:.2}{}", if v >= t { "✓" } else { "✗" })
                };
                println!(
                    "  gate (last step): {} {} {} {} {} {} {} {} → kairos: {} | engine: {:?}",
                    m("d", g.d, th.d),
                    m("q", g.q, th.q),
                    m("r", g.r, th.r),
                    m("g", g.g, th.g),
                    m("j", g.j, th.j),
                    m("p", g.p, th.p),
                    m("n", g.n, th.n),
                    m("k", g.k, th.k),
                    g.kairos,
                    state.engine_state
                );
            }
        }
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
