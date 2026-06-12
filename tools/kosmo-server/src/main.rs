//! Kosmocrates HTTP server.
//!
//! Serves the pipeline over a REST API and an embedded single-page browser UI.
//!
//! ```text
//! USAGE:
//!     kosmo-server [--port <n>] [--host <addr>] [--open]
//!
//! Endpoints:
//!     GET  /                  Browser UI
//!     GET  /api/health        Version ping
//!     GET  /api/doors         The server's docking surface, spoken by the
//!                             server itself (content-addressed catalog,
//!                             pinned against the router)
//!     POST /api/analyse       Run pipeline, return structured report
//!     POST /api/promote       Substrate→core promotion (offer/batch/
//!                             calibration/ledger — mirrors kosmo-promote)
//!     POST /api/landscape     The wish landscape: findings projected into
//!                             ranked wish proposals with measured standing
//!                             (+ spectral geometry on request: coherent
//!                             clusters and singular proposals)
//!     POST /api/norms         The norm catalog of a caller-pathed store
//!                             (read-only: learned/injected archetypes,
//!                             armed triggers, observation count)
//!     POST /api/recall        Pfauenthron++ retrieval over an Infinity
//!                             Ledger (read-only; the ledger must exist)
//! ```

use std::net::SocketAddr;
use std::process;

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use kosmo_core::{
    Digest, Door, DoorCatalog, DoorGovernance, DoorInput, DoorNeed, DoorSurface, GateResult,
    PolicyProfile, Q16,
};
use kosmo_pipeline::{run_workspace_pipeline, ActionItemKind, IntegrationRunOptions};

// ─── Request / Response types ────────────────────────────────────────────────

#[derive(Deserialize)]
struct AnalyseRequest {
    path: String,
    #[serde(default)]
    flags: AnalyseFlags,
}

#[derive(Deserialize, Default)]
struct AnalyseFlags {
    #[serde(default)]
    all: bool,
    #[serde(default)]
    metatron: bool,
    #[serde(default)]
    lpcm: bool,
    #[serde(default)]
    systemcube: bool,
    #[serde(default)]
    surgery: bool,
    #[serde(default)]
    crystals: bool,
    #[serde(default)]
    norms: bool,
    #[serde(default)]
    motifs: bool,
    #[serde(default)]
    pse: bool,
    #[serde(default)]
    operator: bool,
    #[serde(default = "default_capacity")]
    capacity: u32,
}

fn default_capacity() -> u32 {
    100
}

#[derive(Serialize)]
struct ActionItemRow {
    rank: usize,
    action_id: String,
    kind: &'static str,
    kind_group: &'static str,
    target_id: String,
    score: String,
    description: String,
}

#[derive(Serialize)]
struct AnalyseResponse {
    path: String,
    report_id: String,
    gate: &'static str,
    gate_pass: bool,
    gate_warn: bool,
    policy: &'static str,
    source_count: usize,
    void_count: usize,
    total_severity: String,
    deficiency_entries: usize,
    certified_crystals: usize,
    resonite_pairs: usize,
    persisted_crystals: u32,
    action_items: Vec<ActionItemRow>,
    void_ranking: Vec<String>,
    metatron_diagnostics: usize,
    surgery_options: usize,
    pse_candidates: usize,
    norm_candidates: usize,
    motif_candidates: usize,
}

// ─── Formatting helpers ───────────────────────────────────────────────────────

fn q16_str(q: Q16) -> String {
    let raw = q.raw();
    let i = raw / 65536;
    let f = (raw.unsigned_abs() % 65536) * 10000 / 65536;
    format!("{}.{:04}", i, f)
}

fn hex16(d: &Digest) -> String {
    d.to_hex()[..16].to_string()
}

fn gate_label(r: &GateResult) -> &'static str {
    match r {
        GateResult::Pass => "Pass",
        GateResult::Warn { .. } => "Warn",
        GateResult::Reject { .. } => "Reject",
        GateResult::Downgrade { .. } => "Downgrade",
    }
}

fn kind_label(k: &ActionItemKind) -> &'static str {
    match k {
        ActionItemKind::FillVoid { .. } => "FillVoid",
        ActionItemKind::RepairTopology { .. } => "RepairTopology",
        ActionItemKind::PromoteToPse { .. } => "PromoteToPse",
        ActionItemKind::ReviewCrystal { .. } => "ReviewCrystal",
        ActionItemKind::ApplyNorm { .. } => "ApplyNorm",
        ActionItemKind::RealizeWishFacet { .. } => "RealizeWishFacet",
    }
}

fn kind_group(k: &ActionItemKind) -> &'static str {
    match k {
        ActionItemKind::FillVoid { .. } => "void",
        ActionItemKind::RepairTopology { .. } => "topology",
        ActionItemKind::PromoteToPse { .. } => "pse",
        ActionItemKind::ReviewCrystal { .. } => "crystal",
        ActionItemKind::ApplyNorm { .. } => "norm",
        ActionItemKind::RealizeWishFacet { .. } => "wish",
    }
}

fn target_id(k: &ActionItemKind) -> String {
    match k {
        ActionItemKind::FillVoid { void_id } => hex16(void_id),
        ActionItemKind::RepairTopology { surgery_option_id } => hex16(surgery_option_id),
        ActionItemKind::PromoteToPse { candidate_id } => hex16(candidate_id),
        ActionItemKind::ReviewCrystal { candidate_id } => hex16(candidate_id),
        ActionItemKind::ApplyNorm {
            norm_candidate_id, ..
        } => hex16(norm_candidate_id),
        ActionItemKind::RealizeWishFacet { facet } => facet.key.clone(),
    }
}

fn build_options(flags: &AnalyseFlags) -> IntegrationRunOptions {
    if flags.all {
        return IntegrationRunOptions::all_layers(flags.capacity);
    }
    IntegrationRunOptions {
        enable_metatron: flags.metatron,
        enable_lpcm: flags.lpcm,
        enable_systemcube: flags.systemcube,
        systemcube_capacity: flags.capacity,
        lpcm_seam_threshold: Q16::ZERO,
        enable_surgery: flags.surgery,
        enable_motif_candidates: flags.motifs,
        enable_norm_candidates: flags.norms,
        enable_crystal_candidates: flags.crystals,
        enable_pse_candidates: flags.pse,
        prior_feedback: vec![],
        prior_motifs: vec![],
        prior_motif_min_support: Q16::HALF,
        prior_crystals: vec![],
        crystal_store_path: None,
    }
}

// ─── Error type ───────────────────────────────────────────────────────────────

struct ApiError(String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": self.0 })),
        )
            .into_response()
    }
}

// ─── Route handlers ──────────────────────────────────────────────────────────

static INDEX_HTML: &str = include_str!("../static/index.html");

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// ─── Doors (the server's self-description) ──────────────────────────────────

/// The server's complete docking surface, spoken by the server itself —
/// every route as a [`Door`] with its request fields, write power and
/// needs. A test pins this catalog against the router registrations in
/// `main`, so the description cannot drift from the dispatch.
fn doors_catalog() -> DoorCatalog {
    let http = |method: &str| DoorSurface::Http {
        binary: "kosmo-server".into(),
        method: method.into(),
    };
    DoorCatalog::new(vec![
        Door::new(
            http("GET"),
            "/",
            vec![],
            "the embedded browser UI (single page, no external assets)",
            vec![],
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            http("GET"),
            "/api/health",
            vec![],
            "version ping",
            vec![],
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            http("GET"),
            "/api/doors",
            vec![],
            "this catalog: the server's complete docking surface, spoken by \
             the server itself — content-addressed, pinned against the router",
            vec![],
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            http("POST"),
            "/api/analyse",
            vec![],
            "run the dry pipeline over a workspace and return the structured \
             report (the operator flag changes gate policy, never writes)",
            vec![
                DoorInput::valued("path", "<workspace>").required(),
                DoorInput::switch("flags.all"),
                DoorInput::switch("flags.metatron"),
                DoorInput::switch("flags.lpcm"),
                DoorInput::switch("flags.systemcube"),
                DoorInput::switch("flags.surgery"),
                DoorInput::switch("flags.crystals"),
                DoorInput::switch("flags.norms"),
                DoorInput::switch("flags.motifs"),
                DoorInput::switch("flags.pse"),
                DoorInput::switch("flags.operator"),
                DoorInput::valued("flags.capacity", "<n>"),
            ],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Workspace, DoorNeed::Cargo],
        ),
        Door::new(
            http("POST"),
            "/api/promote",
            vec![],
            "substrate→core promotion (mirrors kosmo-promote): offer gated \
             candidates to the engine in memory; with offer+ledger, anchor \
             accepted crystals in the Infinity Ledger (the one host write)",
            vec![
                DoorInput::valued("path", "<workspace>").required(),
                DoorInput::switch("offer"),
                DoorInput::switch("batch"),
                DoorInput::switch("all_kinds"),
                DoorInput::valued("calibration", "default|planning|adaptive|substrate"),
                DoorInput::valued("ledger", "<dir>"),
            ],
            DoorGovernance::AppendsStore,
            vec![DoorNeed::Workspace, DoorNeed::Cargo],
        ),
        Door::new(
            http("POST"),
            "/api/landscape",
            vec![],
            "the wish landscape: findings projected into ranked proposals \
             with measured standing; geometry adds clusters and singulars",
            vec![
                DoorInput::valued("path", "<workspace>").required(),
                DoorInput::switch("all"),
                DoorInput::switch("geometry"),
            ],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Workspace, DoorNeed::Cargo],
        ),
        Door::new(
            http("POST"),
            "/api/norms",
            vec![],
            "the norm catalog of a caller-pathed store (read-only: the \
             server never mutates a norm store)",
            vec![DoorInput::valued("dir", "<dir>").required()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Store],
        ),
        Door::new(
            http("POST"),
            "/api/recall",
            vec![],
            "Pfauenthron++ retrieval over an Infinity Ledger (read-only; a \
             missing ledger is an error, never a creation)",
            vec![
                DoorInput::valued("ledger", "<dir>").required(),
                DoorInput::valued("query", "<text>").required(),
                DoorInput::valued("top", "<n>"),
            ],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Store],
        ),
    ])
}

async fn doors() -> Json<serde_json::Value> {
    Json(serde_json::to_value(doors_catalog()).unwrap_or_default())
}

async fn analyse(Json(req): Json<AnalyseRequest>) -> Result<Json<AnalyseResponse>, ApiError> {
    let policy = if req.flags.operator {
        PolicyProfile::operator_approved()
    } else {
        PolicyProfile::default_report_only()
    };
    let options = build_options(&req.flags);
    let path_str = req.path.clone();

    let report =
        tokio::task::spawn_blocking(move || run_workspace_pipeline(&path_str, &options, &policy))
            .await
            .map_err(|e| ApiError(e.to_string()))?
            .map_err(|e| ApiError(e.to_string()))?;

    let items = report.action_items();
    let action_items = items
        .iter()
        .enumerate()
        .map(|(i, item)| ActionItemRow {
            rank: i + 1,
            action_id: hex16(&item.action_id),
            kind: kind_label(&item.kind),
            kind_group: kind_group(&item.kind),
            target_id: target_id(&item.kind),
            score: q16_str(item.priority_score),
            description: item.description.clone(),
        })
        .collect();

    let source_count =
        report.hyphae_result.host_cube.void_map.voids.len() + report.source_cubes.len();

    let void_ranking = report
        .void_priority_ranking
        .iter()
        .take(20)
        .map(hex16)
        .collect();

    let gate_warn = matches!(report.final_result, GateResult::Warn { .. });

    Ok(Json(AnalyseResponse {
        path: req.path,
        report_id: report.report_id.to_hex()[..24].to_string(),
        gate: gate_label(&report.final_result),
        gate_pass: matches!(report.final_result, GateResult::Pass),
        gate_warn,
        policy: if report.policy_id == PolicyProfile::operator_approved().id {
            "OperatorApproved"
        } else {
            "ReportOnly"
        },
        source_count,
        void_count: report.void_priority_ranking.len(),
        total_severity: q16_str(report.deficiency_vector.total_severity),
        deficiency_entries: report.deficiency_vector.entries.len(),
        certified_crystals: report.certified_crystals.len(),
        resonite_pairs: report.resonite_map.len(),
        persisted_crystals: report.persisted_crystal_count,
        action_items,
        void_ranking,
        metatron_diagnostics: report.metatron_diagnostics.len(),
        surgery_options: report.surgery_options.len(),
        pse_candidates: report.pse_candidates.len(),
        norm_candidates: report.norm_candidates.len(),
        motif_candidates: report.motif_candidates.len(),
    }))
}

// ─── Promotion + recall (the unification surfaced) ──────────────────────────

#[derive(Deserialize)]
struct PromoteRequest {
    path: String,
    #[serde(default)]
    offer: bool,
    #[serde(default)]
    batch: bool,
    #[serde(default)]
    all_kinds: bool,
    /// `default | planning | adaptive | substrate` — an explicit operator
    /// choice because it changes what may become memory. The canonical
    /// definitions live in `kosmo-promote`'s `Calibration` enum.
    #[serde(default = "default_calibration")]
    calibration: String,
    /// Anchor accepted crystals in the Infinity Ledger at this path
    /// (host write; only acts together with `offer`).
    #[serde(default)]
    ledger: Option<String>,
}

fn default_calibration() -> String {
    "default".into()
}

#[derive(Deserialize)]
struct LandscapeRequest {
    path: String,
    /// Enable all optional pipeline layers (heavier scan).
    #[serde(default)]
    all: bool,
    /// Also compute the spectral geometry of the OPEN landscape (coherent
    /// clusters + singular proposals). Strictly additive: without it the
    /// response is unchanged.
    #[serde(default)]
    geometry: bool,
}

#[derive(Deserialize)]
struct NormsRequest {
    /// The norm store directory (caller-pathed, like every store).
    dir: String,
}

#[derive(Deserialize)]
struct RecallRequest {
    /// Infinity-Ledger directory. Must exist — recall is read-only and
    /// never creates a ledger.
    ledger: String,
    query: String,
    #[serde(default = "default_top")]
    top: usize,
}

fn default_top() -> usize {
    5
}

/// Engine config for a calibration mode (see `kosmo-promote::Calibration`).
fn calibration_config(mode: &str) -> Result<pse_types::Config, String> {
    use pse_types::Config;
    match mode {
        "default" => Ok(Config::default()),
        "planning" => Ok(Config::preset_planning()),
        "adaptive" => Ok(Config::preset_streaming()),
        "substrate" => {
            let mut c = Config::preset_planning();
            // Kairos as the discriminant (the preset_anomaly_detection
            // rationale); the 8-fold conjunctive gate remains fully armed.
            c.consensus.consensus_threshold = 0.0;
            c.consensus.mirror_consistency_eta = 0.0;
            Ok(c)
        }
        other => Err(format!(
            "unknown calibration '{other}' (default|planning|adaptive|substrate)"
        )),
    }
}

/// Synchronous promotion core — the HTTP mirror of `kosmo-promote`
/// (offer/batch/calibration/ledger; no state/feedback/store surfaces, which
/// stay CLI-only). Fail-closed: without `offer` the engine is never touched;
/// the ledger is only written in offer mode.
fn do_promote(req: &PromoteRequest) -> Result<serde_json::Value, String> {
    use kosmo_core::ImplementationMode;
    use kosmo_pse_bridge::{
        PromotionOutcome, PseBridgeCandidateKind, PseBridgePolicy, PseBridgeRateLimit,
    };
    use pse_adapter_kosmo::{offer_batch, offer_candidates, KosmoBridgeAdapter};

    let pipeline_policy = PolicyProfile::default_report_only();
    let options = IntegrationRunOptions::all_layers(8);
    let report = run_workspace_pipeline(&req.path, &options, &pipeline_policy)
        .map_err(|e| format!("pipeline failed: {e}"))?;

    let kinds = if req.all_kinds {
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

    let profile = if req.offer {
        PolicyProfile {
            mode: ImplementationMode::DryRun,
            ..PolicyProfile::default()
        }
    } else {
        PolicyProfile::default()
    };
    let bridge_policy =
        PseBridgePolicy::allow(pipeline_policy.id, kinds, PseBridgeRateLimit::strict());
    let adapter = KosmoBridgeAdapter::new(&req.path)
        .with_allowed_kinds(bridge_policy.allowed_candidate_kinds.clone());

    let config = calibration_config(&req.calibration)?;
    let mut state = pse_core::GlobalState::new(&config);
    let mut offers = if req.batch {
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

    // Optional Infinity-Ledger anchor (host write — offer mode only).
    let mut block_hashes: Vec<Option<String>> = vec![None; offers.len()];
    let mut ledger_commits = 0usize;
    if let (Some(ledger), true) = (&req.ledger, req.offer) {
        let mut il = pse_adapter_il::ILStore::open(ledger, "kosmo-server")
            .map_err(|e| format!("cannot open ledger {ledger}: {e}"))?;
        let question = format!("kosmo-server:{}", req.path);
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
            match il.commit_with_feedback(crystal, &chunks, state.commit_index as usize, &question)
            {
                Ok(fb) => {
                    anchored.insert(crystal.crystal_id, (fb.block_hash, fb.qtic_certificate));
                }
                Err(e) => eprintln!("[promote] ledger commit failed: {e}"),
            }
        }
        ledger_commits = anchored.len();
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
    }

    let mut accepted = 0usize;
    let mut deferred = 0usize;
    let mut rejected = 0usize;
    let mut skipped = 0usize;
    let rows: Vec<serde_json::Value> = offers
        .iter()
        .zip(candidates.iter())
        .zip(block_hashes.iter())
        .map(|((o, c), bh)| {
            match &o.outcome {
                PromotionOutcome::Accepted => accepted += 1,
                PromotionOutcome::Deferred => deferred += 1,
                PromotionOutcome::Rejected { .. } => rejected += 1,
                PromotionOutcome::SkippedByPolicy | PromotionOutcome::SkippedByReportOnly => {
                    skipped += 1
                }
            }
            serde_json::json!({
                "label": c.label,
                "kind": format!("{:?}", c.kind),
                "outcome": o.outcome,
                "crystal_committed": o.crystal.is_some(),
                "qtic_class": o.qtic.as_ref().map(|q| q.class_u8()),
                "block_hash": bh,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "path": req.path,
        "mode": if req.offer { "offer" } else { "report-only" },
        "calibration": req.calibration,
        "batch": req.batch,
        "offered": candidates.len(),
        "accepted": accepted,
        "deferred": deferred,
        "rejected": rejected,
        "skipped": skipped,
        "engine_commit_index": state.commit_index,
        "engine_state": format!("{:?}", state.engine_state),
        "ledger_commits": ledger_commits,
        "offers": rows,
    }))
}

/// Synchronous recall core — Pfauenthron++ (`D = ψ·ρ·ω`) over the ledger's
/// anchored crystals, plus the causal lineage of the top hit. Read-only by
/// contract: a missing ledger is an error, never a silently created store.
fn do_recall(req: &RecallRequest) -> Result<serde_json::Value, String> {
    let path = std::path::Path::new(&req.ledger);
    if !path.exists() {
        return Err(format!("no ledger at {} — nothing to recall", req.ledger));
    }
    let il = pse_adapter_il::ILStore::open(path, "kosmo-server")
        .map_err(|e| format!("cannot open ledger {}: {e}", req.ledger))?;
    // The store's embedder seam — never a raw projection (hash8 by default,
    // tagged in the index; a mismatched store refuses upstream).
    let query_vec = il.embed_query(&req.query);
    let mut entries = il.build_context_entries(&query_vec);
    entries.truncate(req.top.max(1));
    let lineage = entries
        .first()
        .map(|top| il.causal_explanation(&top.crystal_id));
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
                "claims": e.claims,
            })
        })
        .collect();
    Ok(serde_json::json!({
        "query": req.query,
        "ledger": req.ledger,
        "anchored_crystals": il.len(),
        "results": rows,
        "lineage": lineage,
    }))
}

/// Synchronous landscape core — the substrate's findings projected into the
/// wish vocabulary, measured against the observed topology. Read-only; the
/// server never adopts or descends (that is the operator CLI's job).
fn do_landscape(req: &LandscapeRequest) -> Result<serde_json::Value, String> {
    let policy = kosmo_core::PolicyProfile::default_report_only();
    let options = if req.all {
        kosmo_pipeline::IntegrationRunOptions::all_layers(100)
    } else {
        kosmo_pipeline::IntegrationRunOptions::report_only()
    };
    let report = kosmo_pipeline::run_workspace_pipeline(&req.path, &options, &policy)
        .map_err(|e| format!("pipeline failed on {}: {e}", req.path))?;
    let landscape = kosmo_pipeline::propose_wishes(&report.hyphae_result.host_cube.void_map.voids);
    let observed = kosmo_intent::observe_workspace_deep(&req.path).ok();
    let standing = kosmo_pipeline::measure_landscape(&landscape, observed.as_ref());

    let count = |s: kosmo_pipeline::LandscapeStanding| standing.iter().filter(|x| **x == s).count();
    let rows: Vec<serde_json::Value> = landscape
        .proposals
        .iter()
        .zip(&standing)
        .map(|(p, s)| {
            serde_json::json!({
                "facet_kind": format!("{:?}", p.facet.kind),
                "facet_key": p.facet.key,
                "severity": format!("{:.2}", p.severity.to_f64()),
                "subject": p.subject,
                "rationale": p.rationale,
                "standing": s.label(),
            })
        })
        .collect();
    let mut doc = serde_json::json!({
        "path": req.path,
        "report_id": report.report_id.to_hex(),
        "proposals": rows,
        "met": count(kosmo_pipeline::LandscapeStanding::Met),
        "open": count(kosmo_pipeline::LandscapeStanding::Open),
        "beyond_observation": count(kosmo_pipeline::LandscapeStanding::BeyondObservation),
        "beyond_vocabulary": landscape.unmapped.len(),
        "unmapped": landscape
            .unmapped
            .iter()
            .map(|u| serde_json::json!({"kind": u.kind_label, "location": u.location}))
            .collect::<Vec<_>>(),
    });
    // Spectral geometry on request — one standing definition with the CLI:
    // couple the OPEN proposals, cluster, surface the singularities.
    if req.geometry {
        let open_proposals: Vec<kosmo_pipeline::WishProposal> = landscape
            .proposals
            .iter()
            .zip(&standing)
            .filter(|(_, s)| **s == kosmo_pipeline::LandscapeStanding::Open)
            .map(|(p, _)| p.clone())
            .collect();
        let geo = kosmo_pipeline::landscape_geometry(&open_proposals, 6);
        let facet_label = |i: usize| {
            format!(
                "{:?} {}",
                open_proposals[i].facet.kind, open_proposals[i].facet.key
            )
        };
        doc["geometry"] = serde_json::json!({
            "clusters": geo.clusters.iter().map(|cl| serde_json::json!({
                "facets": cl.members.iter().map(|&i| facet_label(i)).collect::<Vec<_>>(),
                "subjects": cl.subjects,
                "severity_mass": format!("{:.2}", cl.severity_mass.to_f64()),
            })).collect::<Vec<_>>(),
            "singular": geo.singular.iter().map(|s| serde_json::json!({
                "facet": facet_label(s.index),
                "coupling_mass": format!("{:.2}", s.coupling_mass.to_f64()),
            })).collect::<Vec<_>>(),
        });
    }
    Ok(doc)
}

/// Synchronous norm-catalog core — a read-only window onto a caller-pathed
/// norm store: every learned/injected archetype, its arming state, and the
/// observation count. The server never appends, promotes or injects (those
/// are explicit operator acts on the CLI).
fn do_norms(req: &NormsRequest) -> Result<serde_json::Value, String> {
    let store = kosmo_store::NormStore::open(&req.dir).map_err(|e| e.to_string())?;
    let norms: Vec<serde_json::Value> = store
        .norms()
        .iter()
        .map(|n| {
            serde_json::json!({
                "norm_id": n.norm_id.to_hex(),
                "name": n.name,
                "description": n.description,
                "level": format!("{:?}", n.level),
                "semantic_class": n.semantic_class,
                "trigger": n.trigger,
                "templates": n.template.len(),
                "origin": match &n.origin {
                    kosmo_hyphae::NormOrigin::Learned { observation_ids } =>
                        format!("learned ({} observation(s))", observation_ids.len()),
                    kosmo_hyphae::NormOrigin::OperatorInjected { source } =>
                        format!("injected ({source})"),
                },
            })
        })
        .collect();
    let armed = store.norms().iter().filter(|n| n.trigger.is_some()).count();
    Ok(serde_json::json!({
        "dir": req.dir,
        "norms": norms,
        "known": store.norms().len(),
        "armed": armed,
        "observations": store.observations().len(),
    }))
}

async fn landscape(Json(req): Json<LandscapeRequest>) -> Result<Json<serde_json::Value>, ApiError> {
    tokio::task::spawn_blocking(move || do_landscape(&req))
        .await
        .map_err(|e| ApiError(e.to_string()))?
        .map(Json)
        .map_err(ApiError)
}

async fn norms(Json(req): Json<NormsRequest>) -> Result<Json<serde_json::Value>, ApiError> {
    tokio::task::spawn_blocking(move || do_norms(&req))
        .await
        .map_err(|e| ApiError(e.to_string()))?
        .map(Json)
        .map_err(ApiError)
}

async fn promote(Json(req): Json<PromoteRequest>) -> Result<Json<serde_json::Value>, ApiError> {
    tokio::task::spawn_blocking(move || do_promote(&req))
        .await
        .map_err(|e| ApiError(e.to_string()))?
        .map(Json)
        .map_err(ApiError)
}

async fn recall(Json(req): Json<RecallRequest>) -> Result<Json<serde_json::Value>, ApiError> {
    tokio::task::spawn_blocking(move || do_recall(&req))
        .await
        .map_err(|e| ApiError(e.to_string()))?
        .map(Json)
        .map_err(ApiError)
}

// ─── Server args ─────────────────────────────────────────────────────────────

struct ServerArgs {
    port: u16,
    host: String,
    open: bool,
}

fn parse_server_args() -> Result<ServerArgs, String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut args = ServerArgs {
        port: 7777,
        host: "127.0.0.1".into(),
        open: false,
    };
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "-h" | "--help" => {
                println!(concat!(
                    "Kosmocrates Server — REST API + embedded browser UI\n",
                    "\n",
                    "USAGE:\n",
                    "    kosmo-server [--port <n>] [--host <addr>] [--open]\n",
                    "\n",
                    "OPTIONS:\n",
                    "    --port <n>     Listen port (default: 7777)\n",
                    "    --host <addr>  Bind address (default: 127.0.0.1)\n",
                    "    --open         Open browser after starting\n",
                    "\n",
                    "ENDPOINTS:\n",
                    "    GET  /              Browser UI\n",
                    "    GET  /api/health    Version ping\n",
                    "    GET  /api/doors     The server's docking surface (self-described)\n",
                    "    POST /api/analyse   Run pipeline, return JSON report\n",
                    "\n",
                    "NOTE: intended for local development use only.\n",
                ));
                process::exit(0);
            }
            "--port" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--port requires a number".into());
                }
                args.port = raw[i]
                    .parse()
                    .map_err(|_| format!("invalid port: {}", raw[i]))?;
            }
            "--host" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--host requires an address".into());
                }
                args.host = raw[i].clone();
            }
            "--open" => args.open = true,
            flag if flag.starts_with('-') => {
                return Err(format!("unknown flag '{flag}'; run --help for usage"));
            }
            _ => {}
        }
        i += 1;
    }
    Ok(args)
}

// ─── main ────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let args = match parse_server_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(2);
        }
    };

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/health", get(health))
        .route("/api/doors", get(doors))
        .route("/api/analyse", post(analyse))
        .route("/api/promote", post(promote))
        .route("/api/recall", post(recall))
        .route("/api/landscape", post(landscape))
        .route("/api/norms", post(norms));

    let addr_str = format!("{}:{}", args.host, args.port);
    let addr: SocketAddr = addr_str.parse().unwrap_or_else(|_| {
        eprintln!("error: invalid address {addr_str}");
        process::exit(2);
    });

    let listener = TcpListener::bind(addr).await.unwrap_or_else(|e| {
        eprintln!("error: cannot bind to {addr}: {e}");
        process::exit(1);
    });

    eprintln!("kosmo-server  http://{addr}");
    eprintln!("  Ctrl+C to stop  |  POST /api/analyse  |  GET /api/health");

    if args.open {
        let url = format!("http://{addr}");
        let _ = std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .or_else(|_| std::process::Command::new("open").arg(&url).spawn());
    }

    axum::serve(listener, app).await.unwrap_or_else(|e| {
        eprintln!("server error: {e}");
        process::exit(1);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh polyglot scratch workspace (mirrors the kosmo-promote tests).
    fn scratch(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join("kosmo-server-tests").join(tag);
        std::fs::remove_dir_all(&root).ok();
        let ws = root.join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        let files: [(&str, &str); 4] = [
            (
                "fib.py",
                "import os\n\ndef a(x):\n    return x\n\ndef b(y):\n    return y\n",
            ),
            (
                "fib.rs",
                "use std::io;\npub fn a(x: u32) -> u32 { x }\npub fn b(y: u32) -> u32 { y }\n",
            ),
            (
                "fib.go",
                "package main\nimport \"fmt\"\nfunc a(n int) int { return n }\nfunc b(n int) int { return n }\n",
            ),
            (
                "fib.js",
                "function a(x) {\n  return x;\n}\nfunction b(y) {\n  return y;\n}\n",
            ),
        ];
        for (name, content) in files {
            std::fs::write(ws.join(name), content).unwrap();
        }
        root
    }

    #[test]
    fn recall_on_missing_ledger_is_an_error() {
        let req = RecallRequest {
            ledger: "/tmp/kosmo-server-tests/definitely-missing-ledger".into(),
            query: "anything".into(),
            top: 5,
        };
        let err = do_recall(&req).unwrap_err();
        assert!(err.contains("no ledger"), "must name the failure: {err}");
        assert!(
            !std::path::Path::new(&req.ledger).exists(),
            "recall is read-only — it must never create a ledger"
        );
    }

    #[test]
    fn bad_calibration_is_an_error() {
        assert!(calibration_config("yolo").is_err());
        assert!(calibration_config("substrate").is_ok());
    }

    #[test]
    fn promote_report_only_is_inert() {
        let root = scratch("promote-report-only");
        let req = PromoteRequest {
            path: root.join("ws").to_string_lossy().into_owned(),
            offer: false,
            batch: true,
            all_kinds: true,
            calibration: "substrate".into(),
            ledger: None,
        };
        let doc = do_promote(&req).expect("report-only must succeed");
        assert_eq!(doc["mode"], "report-only");
        assert_eq!(doc["engine_commit_index"], 0, "engine untouched");
        assert_eq!(doc["accepted"], 0);
        assert_eq!(
            doc["skipped"], doc["offered"],
            "every candidate skipped by policy"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn promote_offer_batch_substrate_crystallizes_and_recall_finds_it() {
        let root = scratch("promote-full");
        let ledger = root.join("il");
        let req = PromoteRequest {
            path: root.join("ws").to_string_lossy().into_owned(),
            offer: true,
            batch: true,
            all_kinds: true,
            calibration: "substrate".into(),
            ledger: Some(ledger.to_string_lossy().into_owned()),
        };
        let doc = do_promote(&req).expect("offer must succeed");
        assert!(
            doc["accepted"].as_u64().unwrap() > 0,
            "the ensemble must crystallize: {doc}"
        );
        assert!(doc["ledger_commits"].as_u64().unwrap() >= 1);
        let offers = doc["offers"].as_array().unwrap();
        let committed: Vec<_> = offers
            .iter()
            .filter(|o| o["crystal_committed"] == true)
            .collect();
        assert!(
            committed.iter().all(|o| o["qtic_class"] == 5),
            "anchored → Q5"
        );

        // The HTTP recall finds what the HTTP promote anchored — with content.
        let recall = do_recall(&RecallRequest {
            ledger: ledger.to_string_lossy().into_owned(),
            query: "missing test coverage for a module".into(),
            top: 3,
        })
        .expect("recall must succeed");
        let results = recall["results"].as_array().unwrap();
        assert!(!results.is_empty(), "anchored knowledge must be findable");
        assert_eq!(results[0]["qtic_class"], 5);
        assert!(recall["lineage"].as_str().is_some(), "lineage present");
        assert!(
            !results[0]["claims"].as_array().unwrap().is_empty(),
            "the tank's claims surface over HTTP too"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn landscape_endpoint_maps_and_measures() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-server-landscape-{nanos}"));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub mod router;\n").unwrap();
        std::fs::write(
            root.join("src/router.rs"),
            "pub fn dispatch(p: &str) -> u32 { p.len() as u32 }\n",
        )
        .unwrap();

        let doc = do_landscape(&LandscapeRequest {
            path: root.to_string_lossy().into_owned(),
            all: false,
            geometry: false,
        })
        .expect("landscape must succeed");
        assert!(
            doc.get("geometry").is_none(),
            "geometry is strictly opt-in — the plain response is unchanged"
        );
        let proposals = doc["proposals"].as_array().unwrap();
        assert!(!proposals.is_empty(), "findings project to proposals");
        let router_doc = proposals
            .iter()
            .find(|p| p["facet_key"] == "router" && p["facet_kind"] == "Doc")
            .expect("doc proposal for the undocumented module");
        assert_eq!(router_doc["standing"], "open");
        assert!(doc["open"].as_u64().unwrap() >= 1);
        assert!(!doc["report_id"].as_str().unwrap().is_empty());

        // With geometry: the open doc+test of `router` form ONE cluster —
        // the same standing definition as the CLI surface.
        let doc = do_landscape(&LandscapeRequest {
            path: root.to_string_lossy().into_owned(),
            all: false,
            geometry: true,
        })
        .expect("landscape must succeed");
        let clusters = doc["geometry"]["clusters"].as_array().unwrap();
        assert_eq!(clusters.len(), 1, "{clusters:?}");
        let facets: Vec<&str> = clusters[0]["facets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f.as_str().unwrap())
            .collect();
        assert!(facets.contains(&"Doc router"), "{facets:?}");
        assert!(facets.contains(&"Test router_smoke"), "{facets:?}");

        // A nonexistent path is a hard error, not an empty landscape.
        assert!(do_landscape(&LandscapeRequest {
            path: "/nonexistent/kosmo-landscape-xyz".into(),
            all: false,
            geometry: false,
        })
        .is_err());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn norms_endpoint_is_a_read_only_window() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kosmo-server-norms-{nanos}"));

        // An empty (even nonexistent) store reads as the empty catalog —
        // the structural Wonderlamp rejection, visible over HTTP.
        let doc = do_norms(&NormsRequest {
            dir: dir.to_string_lossy().into_owned(),
        })
        .expect("an empty store is readable");
        assert_eq!(doc["known"], 0);
        assert_eq!(doc["armed"], 0);
        assert_eq!(doc["observations"], 0);
        assert!(!dir.exists(), "reading must not create anything");

        // Seed one armed norm via the store's own discipline, then read it.
        let norm = kosmo_hyphae::Norm::new(
            "loader",
            "module plus loader fn",
            kosmo_hyphae::NormLevel::Molecule,
            "module+symbol",
            [
                kosmo_hyphae::NormFacetTemplate::new(kosmo_core::WishFacetKind::Module, "{name}"),
                kosmo_hyphae::NormFacetTemplate::new(
                    kosmo_core::WishFacetKind::Symbol,
                    "{name}_load",
                ),
            ],
            [],
            [],
            Digest::of_bytes(b"cand"),
            Digest::of_bytes(b"ev"),
            Digest::of_bytes(b"pol"),
            kosmo_hyphae::NormOrigin::OperatorInjected {
                source: "test".into(),
            },
        );
        let mut store = kosmo_store::NormStore::open(&dir).unwrap();
        store
            .append_norm(&norm, &PolicyProfile::operator_approved())
            .unwrap();
        store
            .promote(norm.norm_id, "loader", &PolicyProfile::operator_approved())
            .unwrap();

        let doc = do_norms(&NormsRequest {
            dir: dir.to_string_lossy().into_owned(),
        })
        .unwrap();
        assert_eq!(doc["known"], 1);
        assert_eq!(doc["armed"], 1);
        let row = &doc["norms"].as_array().unwrap()[0];
        assert_eq!(row["name"], "loader");
        assert_eq!(row["trigger"], "loader");
        assert_eq!(row["templates"], 2);
        assert!(row["origin"].as_str().unwrap().starts_with("injected"));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// This server's own source — the doors catalog is pinned against the
    /// router registrations, so the self-description cannot drift.
    const MAIN_SRC: &str = include_str!("main.rs");

    /// Every `(method, route)` the router actually registers:
    /// `.route("<path>", get(...)|post(...))` literals in `main`.
    fn registered_routes() -> std::collections::BTreeSet<(String, String)> {
        let mut routes = std::collections::BTreeSet::new();
        for line in MAIN_SRC.lines() {
            if line.trim_start().starts_with("//") {
                continue; // prose about routes is not a route
            }
            let Some(start) = line.find(".route(\"") else {
                continue;
            };
            let after = &line[start + ".route(\"".len()..];
            let Some(end) = after.find('"') else { continue };
            let path = &after[..end];
            let tail = &after[end..];
            let method = if tail.contains("get(") {
                "GET"
            } else if tail.contains("post(") {
                "POST"
            } else {
                continue;
            };
            routes.insert((method.to_string(), path.to_string()));
        }
        routes
    }

    #[test]
    fn the_doors_catalog_is_pinned_to_the_router() {
        let registered = registered_routes();
        let cataloged: std::collections::BTreeSet<(String, String)> = doors_catalog()
            .doors
            .iter()
            .map(|d| match &d.surface {
                DoorSurface::Http { method, .. } => (method.clone(), d.name.clone()),
                other => panic!("a server door must be HTTP, got {other:?}"),
            })
            .collect();
        assert_eq!(
            registered, cataloged,
            "the catalog and the router must speak the same routes — \
             an undescribed route is a hole in the docking surface"
        );
    }

    #[test]
    fn the_servers_surface_is_workspace_safe_and_describes_itself() {
        let catalog = doors_catalog();
        // The server never mutates a workspace and never decrees: its only
        // write power is the explicit ledger anchor behind /api/promote.
        for door in &catalog.doors {
            assert_ne!(
                door.governance,
                DoorGovernance::WritesWorkspace,
                "{} must not write workspaces",
                door.name
            );
            assert_ne!(
                door.governance,
                DoorGovernance::GovernanceAct,
                "{} must not decree",
                door.name
            );
        }
        assert_eq!(
            catalog
                .doors
                .iter()
                .filter(|d| d.governance == DoorGovernance::AppendsStore)
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>(),
            vec!["/api/promote"],
            "exactly one store-appending door"
        );
        // The catalog lists itself, deterministically.
        assert!(catalog.doors.iter().any(|d| d.name == "/api/doors"));
        assert_eq!(catalog.catalog_id, doors_catalog().catalog_id);
    }
}
