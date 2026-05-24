//! PSE REST API server.
//!
//! Exposes the Post-Symbolic Engine over HTTP/JSON so any language can use
//! it without a Rust or Python toolchain.
//!
//! **Design:** mostly stateless — the client carries state between calls via
//! `memory_json` fields.  The server holds no sessions.
//! Exceptions: the IL store (activated by `PSE_IL_STORE`) and the nxalien
//! governance state (activated by `PSE_NXALIEN_STATE`) are persistent,
//! file-backed stores shared across all requests.
//!
//! **Endpoints:**
//!
//!   GET  /health                  — liveness check
//!   POST /ingest                  — process text through PSE; returns crystals + state
//!   POST /context                 — render crystal records into an LLM prompt block
//!   POST /coverage                — count domain keywords in a response
//!
//!   GET  /il/status               — IL + HDAG health (active?, block_count, mean ψ)
//!   POST /il/retrieve             — Pfauenthron++ unified retrieval D=ψ·ρ·ω
//!   GET  /il/hdag/coherence       — mean coherence potential + edge counts
//!   GET  /il/hdag/order           — topological order of HDAG nodes
//!
//!   POST /nxalien/bundle          — ingest a governance bundle from any repo;
//!                                   commits rules to IL, updates cross-repo
//!                                   attractor, returns EpistemicSignal
//!   GET  /nxalien/signal          — current cross-repo EpistemicSignal
//!   POST /nxalien/validate        — constitutional pre-check of a bundle's rules
//!   GET  /nxalien/rules/current   — evolved rule set after last signal
//!
//! **Configuration (env vars):**
//!
//!   `PSE_SERVER_HOST`     bind address (default: `0.0.0.0`)
//!   `PSE_SERVER_PORT`     port         (default: `8765`)
//!   `PSE_IL_STORE`        path to IL ledger directory (optional; activates IL routes)
//!   `PSE_NXALIEN_STATE`   path to nxalien state directory (optional; activates
//!                         /nxalien/* routes and cross-repo attractor)
//!
//! **Quickstart:**
//! ```text
//!   PSE_NXALIEN_STATE=/var/pse/nxalien cargo run --release -p pse-server
//!
//!   # Push a bundle from any repo
//!   curl -s -X POST http://localhost:8765/nxalien/bundle \
//!     -H 'Content-Type: application/json' \
//!     -d @nxalien.manifest.json | jq .signal.stability
//!
//!   # Current cross-repo signal
//!   curl http://localhost:8765/nxalien/signal | jq .
//! ```

mod pse;

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use pse_adapter_il::{adapter::text_to_vector8, store::ILStore};
use pse_nxalien_evolve::{
    commit_rules_to_il,
    evolution::{propose_rule_evolution, apply_validated_proposals, EvolutionGuard},
    graph_state::GraphState,
    signal::EpistemicSignal,
};
use pse_nxalien_types::{NxAlienBundle, NxAlienPolicy, RuleAtom};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

// ── Shared server state ───────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    il: Arc<Mutex<Option<ILStore>>>,
    /// Server-side nxalien state for cross-repo attractor computation.
    /// `None` when PSE_NXALIEN_STATE is not set.
    nxalien_state: Arc<Mutex<Option<GraphState>>>,
    /// Base path for nxalien persistent state (graph_state.json, il/).
    nxalien_path: Option<PathBuf>,
}

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct IngestRequest {
    text: String,
    #[serde(default)]
    memory_json: Option<String>,
    #[serde(default)]
    records_json: Option<String>,
    #[serde(default = "default_session")]
    session: usize,
    #[serde(default)]
    question: String,
    #[serde(default)]
    source_name: Option<String>,
}

fn default_session() -> usize {
    1
}

/// IL commit summary included in `/ingest` responses when IL is active.
#[derive(Serialize)]
struct ILCommitInfo {
    crystal_id: String,
    block_hash: String,
    converged: bool,
    coherence_potential: f64,
    gate_passed: bool,
    il_stability: f64,
}

#[derive(Serialize)]
struct IngestResponse {
    new_crystals: Vec<pse::CrystalInfo>,
    memory_json: String,
    records_json: String,
    pattern_hits: u64,
    commit_index: u64,
    new_crystal_count: usize,
    /// Present when `PSE_IL_STORE` is active — one entry per new crystal.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    il_commits: Vec<ILCommitInfo>,
}

#[derive(Deserialize)]
struct ContextRequest {
    records_json: String,
    #[serde(default = "default_top_k")]
    top_k: usize,
}

fn default_top_k() -> usize {
    5
}

#[derive(Serialize)]
struct ContextResponse {
    context: String,
    record_count: usize,
}

#[derive(Deserialize)]
struct CoverageRequest {
    text: String,
    keywords: Vec<String>,
}

#[derive(Serialize)]
struct CoverageResponse {
    hits: usize,
    total: usize,
    coverage: f64,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

// IL-specific request/response types ─────────────────────────────────────────

#[derive(Serialize)]
struct ILEdgeCounts {
    sequential_commit: usize,
    resonance_proximity: usize,
    refinement: usize,
    metatron_isomorphic: usize,
}

#[derive(Serialize)]
struct ILStatusResponse {
    active: bool,
    block_count: usize,
    mean_coherence_potential: f64,
    edge_counts: Option<ILEdgeCounts>,
}

#[derive(Deserialize)]
struct ILRetrieveRequest {
    question: String,
    /// Accepted for backward compatibility; not used in the current retrieval path.
    #[serde(default)]
    #[allow(dead_code)]
    records_json: Option<String>,
    #[serde(default = "default_top_k")]
    top_k: usize,
}

#[derive(Serialize)]
struct ILRetrieveResult {
    crystal_id: String,
    score_d: f64,
    rank: usize,
}

#[derive(Serialize)]
struct ILRetrieveResponse {
    active: bool,
    results: Vec<ILRetrieveResult>,
}

#[derive(Serialize)]
struct ILHdagCoherenceResponse {
    mean_coherence_potential: f64,
    node_count: usize,
    edge_counts: Option<ILEdgeCounts>,
}

#[derive(Serialize)]
struct ILHdagOrderResponse {
    order: Vec<String>,
    node_count: usize,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn ingest(
    State(state): State<AppState>,
    Json(req): Json<IngestRequest>,
) -> impl IntoResponse {
    let source = req.source_name.as_deref().unwrap_or("pse-server");
    let result = match pse::ingest(
        &req.text,
        req.memory_json.as_deref(),
        req.records_json.as_deref(),
        req.session,
        &req.question,
        source,
    ) {
        Err(e) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response()
        }
        Ok(r) => r,
    };

    let memory_json = match serde_json::to_string(&result.all_crystals) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
    let records_json = match serde_json::to_string(&result.all_records) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    // IL commit: when the store is active, commit every new crystal.
    let mut il_commits: Vec<ILCommitInfo> = Vec::new();
    if let Ok(mut guard) = state.il.lock() {
        if let Some(ref mut store) = *guard {
            for rec in &result.new_records {
                let crystal_id: String = rec
                    .crystal
                    .crystal_id
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                match store.commit_with_feedback(
                    &rec.crystal,
                    &rec.source_chunks,
                    rec.session,
                    &rec.question,
                ) {
                    Ok(fb) => il_commits.push(ILCommitInfo {
                        crystal_id,
                        block_hash: fb.block_hash,
                        converged: fb.converged,
                        coherence_potential: fb.coherence_potential,
                        gate_passed: fb.gate_passed,
                        il_stability: fb.il_stability,
                    }),
                    Err(e) => eprintln!("[IL] commit error for {crystal_id}: {e}"),
                }
            }
        }
    }

    let new_crystal_count = result.new_records.len();
    let new_crystals: Vec<pse::CrystalInfo> = result
        .new_records
        .iter()
        .map(pse::CrystalInfo::from_record)
        .collect();

    Json(IngestResponse {
        new_crystals,
        memory_json,
        records_json,
        pattern_hits: result.pattern_hits,
        commit_index: result.commit_index,
        new_crystal_count,
        il_commits,
    })
    .into_response()
}

async fn context(Json(req): Json<ContextRequest>) -> impl IntoResponse {
    let records: Vec<pse::CrystalRecord> = match serde_json::from_str(&req.records_json) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "error": format!("records_json: {e}") })),
            )
                .into_response()
        }
    };
    let record_count = records.len();
    let context = pse::render_context(&records, req.top_k);
    Json(ContextResponse {
        context,
        record_count,
    })
    .into_response()
}

async fn coverage(Json(req): Json<CoverageRequest>) -> impl IntoResponse {
    let (hits, total) = pse::score_coverage(&req.text, &req.keywords);
    let coverage = if total == 0 {
        0.0
    } else {
        hits as f64 / total as f64
    };
    Json(CoverageResponse {
        hits,
        total,
        coverage,
    })
    .into_response()
}

// ── IL handlers ───────────────────────────────────────────────────────────────

async fn il_status(State(state): State<AppState>) -> impl IntoResponse {
    let guard = match state.il.lock() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "IL store lock poisoned" })),
            )
                .into_response()
        }
    };

    match &*guard {
        None => Json(ILStatusResponse {
            active: false,
            block_count: 0,
            mean_coherence_potential: 0.0,
            edge_counts: None,
        })
        .into_response(),
        Some(store) => {
            let edge_counts = Some(ILEdgeCounts {
                sequential_commit: store.hdag_edge_count_by_cause("sequential_commit"),
                resonance_proximity: store.hdag_edge_count_by_cause("resonance_proximity"),
                refinement: store.hdag_edge_count_by_cause("refinement"),
                metatron_isomorphic: store.hdag_edge_count_by_cause("metatron_isomorphic"),
            });
            Json(ILStatusResponse {
                active: true,
                block_count: store.len(),
                mean_coherence_potential: store.mean_coherence_potential(),
                edge_counts,
            })
            .into_response()
        }
    }
}

async fn il_retrieve(
    State(state): State<AppState>,
    Json(req): Json<ILRetrieveRequest>,
) -> impl IntoResponse {
    let guard = match state.il.lock() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "IL store lock poisoned" })),
            )
                .into_response()
        }
    };

    let Some(store) = &*guard else {
        return Json(ILRetrieveResponse {
            active: false,
            results: vec![],
        })
        .into_response();
    };

    let q_vec = text_to_vector8(&req.question);
    let mut hits = store.score_tripolar(&q_vec);
    hits.truncate(req.top_k);

    let results: Vec<ILRetrieveResult> = hits
        .into_iter()
        .enumerate()
        .map(|(i, hit)| ILRetrieveResult {
            crystal_id: hit.crystal_id_hex,
            score_d: hit.score,
            rank: i + 1,
        })
        .collect();

    Json(ILRetrieveResponse {
        active: true,
        results,
    })
    .into_response()
}

async fn il_hdag_coherence(State(state): State<AppState>) -> impl IntoResponse {
    let guard = match state.il.lock() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "IL store lock poisoned" })),
            )
                .into_response()
        }
    };

    match &*guard {
        None => Json(ILHdagCoherenceResponse {
            mean_coherence_potential: 0.0,
            node_count: 0,
            edge_counts: None,
        })
        .into_response(),
        Some(store) => Json(ILHdagCoherenceResponse {
            mean_coherence_potential: store.mean_coherence_potential(),
            node_count: store.len(),
            edge_counts: Some(ILEdgeCounts {
                sequential_commit: store.hdag_edge_count_by_cause("sequential_commit"),
                resonance_proximity: store.hdag_edge_count_by_cause("resonance_proximity"),
                refinement: store.hdag_edge_count_by_cause("refinement"),
                metatron_isomorphic: store.hdag_edge_count_by_cause("metatron_isomorphic"),
            }),
        })
        .into_response(),
    }
}

async fn il_hdag_order(State(state): State<AppState>) -> impl IntoResponse {
    let guard = match state.il.lock() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "IL store lock poisoned" })),
            )
                .into_response()
        }
    };

    match &*guard {
        None => Json(ILHdagOrderResponse {
            order: vec![],
            node_count: 0,
        })
        .into_response(),
        Some(store) => {
            let order = store.topological_order();
            let node_count = order.len();
            Json(ILHdagOrderResponse { order, node_count }).into_response()
        }
    }
}

// ── nxalien request / response types ─────────────────────────────────────────

/// POST /nxalien/bundle — ingest a governance bundle from any repo.
///
/// The server updates the cross-repo PersistentGraph, commits each rule as a
/// SemanticCrystal into the IL store, extracts the EpistemicSignal, and runs
/// one evolution cycle.  Returns the signal and any new evolution proposals.
#[derive(Deserialize)]
struct NxAlienBundleRequest {
    /// The NxAlienBundle produced by `nxalien compile`.
    bundle: NxAlienBundle,
    /// Wall-clock timestamp of the compile run (Unix seconds, f64).
    /// If omitted the server uses its own clock.
    #[serde(default)]
    timestamp: Option<f64>,
}

#[derive(Serialize)]
struct NxAlienBundleResponse {
    /// Whether this is the first bundle from this repo (run_count == 1).
    first_run: bool,
    /// EpistemicSignal after ingestion (includes IL health overlay).
    signal: EpistemicSignal,
    /// Number of rules committed to IL this run.
    il_committed: usize,
    /// Number of evolution proposals generated.
    proposal_count: usize,
    /// Proposals that were accepted by the EvolutionGuard (rule_id → new severity).
    accepted_proposals: Vec<NxAlienProposalInfo>,
    /// Unknown slots produced from rejected proposals.
    new_unknowns: usize,
}

#[derive(Serialize)]
struct NxAlienProposalInfo {
    rule_id: String,
    from: String,
    to: String,
    attractor_alignment: f64,
}

/// GET /nxalien/signal — current cross-repo EpistemicSignal.
#[derive(Serialize)]
struct NxAlienSignalResponse {
    active: bool,
    run_count: u64,
    signal: Option<EpistemicSignal>,
}

/// POST /nxalien/validate — constitutional pre-check of a bundle's rules.
///
/// Returns per-rule QTIC class and coherence potential without persisting
/// anything.  Safe to call before `compile` to preview governance quality.
#[derive(Deserialize)]
struct NxAlienValidateRequest {
    rules: Vec<RuleAtom>,
    #[serde(default)]
    run_count: u64,
}

#[derive(Serialize)]
struct NxAlienValidateResponse {
    rules_checked: usize,
    mean_qtic_class: f64,
    mean_coherence_potential: f64,
    gate_passed_all: bool,
    entries: Vec<pse_nxalien_evolve::il_bridge::ILCommitEntry>,
}

/// GET /nxalien/rules/current — evolved rule set from last signal.
#[derive(Serialize)]
struct NxAlienRulesResponse {
    active: bool,
    run_count: u64,
    rules: Vec<RuleAtom>,
}

// ── nxalien handlers ──────────────────────────────────────────────────────────

async fn nxalien_bundle(
    State(state): State<AppState>,
    Json(req): Json<NxAlienBundleRequest>,
) -> impl IntoResponse {
    let nxa_path = match &state.nxalien_path {
        Some(p) => p.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "nxalien routes inactive — set PSE_NXALIEN_STATE"
                })),
            )
                .into_response()
        }
    };

    let ts = req.timestamp.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    });

    let mut gs_guard = match state.nxalien_state.lock() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "nxalien state lock poisoned" })),
            )
                .into_response()
        }
    };

    let graph_state = gs_guard.get_or_insert_with(GraphState::default);

    // Ingest bundle into cross-repo PSE graph.
    let live_graph = graph_state.ingest_and_update(&req.bundle, ts);
    let first_run = graph_state.run_count == 1;

    // Commit rules to IL (server-side IL store at nxa_path/il/).
    let il_path = nxa_path.join("il");
    let il_summary = commit_rules_to_il(&req.bundle.rules, &il_path, graph_state.run_count);

    // Extract cross-repo EpistemicSignal with IL overlay.
    const K: usize = 8;
    let signal = EpistemicSignal::extract_with_il(&live_graph, graph_state, K, &il_path);

    // Evolution cycle.
    let policy = NxAlienPolicy::default();
    let proposals = propose_rule_evolution(&signal, &req.bundle.rules, &policy);
    let proposal_count = proposals.len();
    let mut evolved = req.bundle.rules.clone();
    let guard = EvolutionGuard::default();
    let new_unknowns = apply_validated_proposals(&mut evolved, &proposals, &guard);

    let accepted: Vec<NxAlienProposalInfo> = proposals
        .iter()
        .filter(|p| evolved.iter().any(|r| r.id == p.rule_id && r.severity == p.proposed_severity))
        .map(|p| NxAlienProposalInfo {
            rule_id: p.rule_id.clone(),
            from: format!("{:?}", p.current_severity),
            to: format!("{:?}", p.proposed_severity),
            attractor_alignment: p.attractor_alignment,
        })
        .collect();

    // Persist state.
    let _ = graph_state.save(&nxa_path);

    // Persist evolved rules for /nxalien/rules/current.
    let rules_path = nxa_path.join("current_rules.json");
    if let Ok(j) = serde_json::to_string_pretty(&evolved) {
        let _ = std::fs::write(&rules_path, j);
    }

    Json(NxAlienBundleResponse {
        first_run,
        signal,
        il_committed: il_summary.committed,
        proposal_count,
        accepted_proposals: accepted,
        new_unknowns: new_unknowns.len(),
    })
    .into_response()
}

async fn nxalien_signal(State(state): State<AppState>) -> impl IntoResponse {
    if state.nxalien_path.is_none() {
        return Json(NxAlienSignalResponse {
            active: false,
            run_count: 0,
            signal: None,
        })
        .into_response();
    }

    let nxa_path = state.nxalien_path.as_ref().unwrap();
    let il_path = nxa_path.join("il");

    let gs_guard = match state.nxalien_state.lock() {
        Ok(g) => g,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "nxalien state lock poisoned" })),
            )
                .into_response()
        }
    };

    match &*gs_guard {
        None => Json(NxAlienSignalResponse {
            active: true,
            run_count: 0,
            signal: None,
        })
        .into_response(),
        Some(gs) => {
            // Reconstruct a minimal live graph from stored point cloud.
            let mut graph = pse_graph::PersistentGraph::new();
            for (vid, emb) in &gs.point_cloud {
                let state: pse_types::FiveDState = emb.into();
                graph.upsert_vertex(*vid, 0.0);
                graph.embedding.insert(*vid, state);
            }
            let signal = EpistemicSignal::extract_with_il(&graph, gs, 8, &il_path);
            Json(NxAlienSignalResponse {
                active: true,
                run_count: gs.run_count,
                signal: Some(signal),
            })
            .into_response()
        }
    }
}

async fn nxalien_validate(
    State(state): State<AppState>,
    Json(req): Json<NxAlienValidateRequest>,
) -> impl IntoResponse {
    if state.nxalien_path.is_none() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "nxalien routes inactive — set PSE_NXALIEN_STATE"
            })),
        )
            .into_response();
    }

    // Dry-run: use a temp directory so nothing is persisted.
    let tmp: tempfile::TempDir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("tempdir: {e}") })),
            )
                .into_response()
        }
    };

    let summary = commit_rules_to_il(&req.rules, tmp.path(), req.run_count);

    Json(NxAlienValidateResponse {
        rules_checked: req.rules.len(),
        mean_qtic_class: summary.mean_qtic_class,
        mean_coherence_potential: summary.mean_coherence_potential,
        gate_passed_all: summary.gate_passed_all,
        entries: summary.entries,
    })
    .into_response()
}

async fn nxalien_rules_current(State(state): State<AppState>) -> impl IntoResponse {
    let nxa_path = match &state.nxalien_path {
        Some(p) => p.clone(),
        None => {
            return Json(NxAlienRulesResponse {
                active: false,
                run_count: 0,
                rules: vec![],
            })
            .into_response()
        }
    };

    let rules_path = nxa_path.join("current_rules.json");
    let rules: Vec<RuleAtom> = std::fs::read_to_string(&rules_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let run_count = state
        .nxalien_state
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.run_count))
        .unwrap_or(0);

    Json(NxAlienRulesResponse {
        active: true,
        run_count,
        rules,
    })
    .into_response()
}

// ── Server startup ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let host = std::env::var("PSE_SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PSE_SERVER_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8765);

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("invalid PSE_SERVER_HOST / PSE_SERVER_PORT");

    // IL store: optional, activated by PSE_IL_STORE env var.
    let il_store: Option<ILStore> = std::env::var("PSE_IL_STORE").ok().and_then(|path| {
        match ILStore::open(&path, "pse-server") {
            Ok(s) => {
                println!("  IL store  : {path} ({} block(s))", s.len());
                Some(s)
            }
            Err(e) => {
                eprintln!("  [IL] Warning: cannot open store at {path}: {e}");
                None
            }
        }
    });
    let il_active = il_store.is_some();

    // nxalien state: optional, activated by PSE_NXALIEN_STATE env var.
    let nxalien_path: Option<PathBuf> = std::env::var("PSE_NXALIEN_STATE").ok().map(PathBuf::from);
    let nxalien_graph_state: Option<GraphState> = nxalien_path.as_ref().map(|p| {
        std::fs::create_dir_all(p).ok();
        let gs = GraphState::load(p);
        println!("  nxalien   : {p:?} (run_count={})", gs.run_count);
        gs
    });
    let nxa_active = nxalien_path.is_some();

    let state = AppState {
        il: Arc::new(Mutex::new(il_store)),
        nxalien_state: Arc::new(Mutex::new(nxalien_graph_state)),
        nxalien_path: nxalien_path.clone(),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/ingest", post(ingest))
        .route("/context", post(context))
        .route("/coverage", post(coverage))
        .route("/il/status", get(il_status))
        .route("/il/retrieve", post(il_retrieve))
        .route("/il/hdag/coherence", get(il_hdag_coherence))
        .route("/il/hdag/order", get(il_hdag_order))
        .route("/nxalien/bundle", post(nxalien_bundle))
        .route("/nxalien/signal", get(nxalien_signal))
        .route("/nxalien/validate", post(nxalien_validate))
        .route("/nxalien/rules/current", get(nxalien_rules_current))
        .layer(CorsLayer::permissive())
        .with_state(state);

    println!();
    println!("PSE Server  v{}", env!("CARGO_PKG_VERSION"));
    println!("═══════════════════════════════════════════════");
    println!("  Listening : http://{addr}");
    if il_active {
        println!("  IL store  : active (PSE_IL_STORE)");
    } else {
        println!("  IL store  : inactive (set PSE_IL_STORE to activate)");
    }
    if nxa_active {
        println!("  nxalien   : active (PSE_NXALIEN_STATE)");
    } else {
        println!("  nxalien   : inactive (set PSE_NXALIEN_STATE to activate)");
    }
    println!();
    println!("  GET  /health");
    println!("  POST /ingest              — text → crystals + state");
    println!("  POST /context             — records → LLM prompt block");
    println!("  POST /coverage            — text + keywords → hit count");
    println!("  GET  /il/status           — IL + HDAG health");
    println!("  POST /il/retrieve         — Pfauenthron++ D=ψ·ρ·ω retrieval");
    println!("  GET  /il/hdag/coherence   — mean ψ + edge counts");
    println!("  GET  /il/hdag/order       — topological order");
    println!("  POST /nxalien/bundle      — ingest bundle → cross-repo signal");
    println!("  GET  /nxalien/signal      — current EpistemicSignal");
    println!("  POST /nxalien/validate    — dry-run constitutional check");
    println!("  GET  /nxalien/rules/current — evolved rule set");
    println!();
    println!("  curl http://{addr}/health");
    println!();

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}
