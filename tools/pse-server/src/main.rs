//! PSE REST API server.
//!
//! Exposes the Post-Symbolic Engine over HTTP/JSON so any language can use
//! it without a Rust or Python toolchain.
//!
//! **Design:** fully stateless — the client carries state between calls via
//! `memory_json` and `records_json` fields.  The server holds no sessions.
//! Exception: the IL store (activated by `PSE_IL_STORE`) is a persistent,
//! file-backed ledger shared across all requests.
//!
//! **Endpoints:**
//!
//!   GET  /health               — liveness check
//!   POST /ingest               — process text through PSE; returns crystals + state
//!   POST /context              — render crystal records into an LLM prompt block
//!   POST /coverage             — count domain keywords in a response
//!
//!   GET  /il/status            — IL + HDAG health (active?, block_count, mean ψ)
//!   POST /il/retrieve          — Pfauenthron++ unified retrieval D=ψ·ρ·ω
//!   GET  /il/hdag/coherence    — mean coherence potential + edge counts
//!   GET  /il/hdag/order        — topological order of HDAG nodes
//!
//! **Configuration (env vars):**
//!
//!   `PSE_SERVER_HOST`   bind address (default: `0.0.0.0`)
//!   `PSE_SERVER_PORT`   port         (default: `8765`)
//!   `PSE_IL_STORE`      path to IL ledger directory (optional; activates IL routes)
//!
//! **Quickstart:**
//! ```text
//!   cargo run --release -p pse-server
//!
//!   # Health check
//!   curl http://localhost:8765/health
//!
//!   # IL status (when PSE_IL_STORE is set)
//!   curl http://localhost:8765/il/status | jq .
//!
//!   # Unified retrieval
//!   curl -s -X POST http://localhost:8765/il/retrieve \
//!     -H 'Content-Type: application/json' \
//!     -d '{"question":"What is entropy?","records_json":"...","top_k":5}' | jq .
//! ```

mod pse;

use std::{
    net::SocketAddr,
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
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

// ── Shared server state ───────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    il: Arc<Mutex<Option<ILStore>>>,
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

fn default_session() -> usize { 1 }

/// IL commit summary included in `/ingest` responses when IL is active.
#[derive(Serialize)]
struct ILCommitInfo {
    crystal_id:          String,
    block_hash:          String,
    converged:           bool,
    coherence_potential: f64,
    gate_passed:         bool,
    il_stability:        f64,
}

#[derive(Serialize)]
struct IngestResponse {
    new_crystals:      Vec<pse::CrystalInfo>,
    memory_json:       String,
    records_json:      String,
    pattern_hits:      u64,
    commit_index:      u64,
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

fn default_top_k() -> usize { 5 }

#[derive(Serialize)]
struct ContextResponse {
    context:      String,
    record_count: usize,
}

#[derive(Deserialize)]
struct CoverageRequest {
    text:     String,
    keywords: Vec<String>,
}

#[derive(Serialize)]
struct CoverageResponse {
    hits:     usize,
    total:    usize,
    coverage: f64,
}

#[derive(Serialize)]
struct HealthResponse {
    status:  &'static str,
    version: &'static str,
}

// IL-specific request/response types ─────────────────────────────────────────

#[derive(Serialize)]
struct ILEdgeCounts {
    sequential_commit:   usize,
    resonance_proximity: usize,
    refinement:          usize,
}

#[derive(Serialize)]
struct ILStatusResponse {
    active:                   bool,
    block_count:              usize,
    mean_coherence_potential: f64,
    edge_counts:              Option<ILEdgeCounts>,
}

#[derive(Deserialize)]
struct ILRetrieveRequest {
    question:    String,
    records_json: String,
    #[serde(default = "default_top_k")]
    top_k:       usize,
}

#[derive(Serialize)]
struct ILRetrieveResult {
    crystal_id: String,
    score_d:    f64,
    rank:       usize,
}

#[derive(Serialize)]
struct ILRetrieveResponse {
    active:  bool,
    results: Vec<ILRetrieveResult>,
}

#[derive(Serialize)]
struct ILHdagCoherenceResponse {
    mean_coherence_potential: f64,
    node_count:               usize,
    edge_counts:              Option<ILEdgeCounts>,
}

#[derive(Serialize)]
struct ILHdagOrderResponse {
    order:      Vec<String>,
    node_count: usize,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(HealthResponse { status: "ok", version: env!("CARGO_PKG_VERSION") })
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
        Err(e) => return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": e })),
        ).into_response(),
        Ok(r) => r,
    };

    let memory_json = match serde_json::to_string(&result.all_crystals) {
        Ok(j) => j,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    };
    let records_json = match serde_json::to_string(&result.all_records) {
        Ok(j) => j,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    };

    // IL commit: when the store is active, commit every new crystal.
    let mut il_commits: Vec<ILCommitInfo> = Vec::new();
    if let Ok(mut guard) = state.il.lock() {
        if let Some(ref mut store) = *guard {
            for rec in &result.new_records {
                let crystal_id: String =
                    rec.crystal.crystal_id.iter().map(|b| format!("{b:02x}")).collect();
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
    let new_crystals: Vec<pse::CrystalInfo> =
        result.new_records.iter().map(pse::CrystalInfo::from_record).collect();

    Json(IngestResponse {
        new_crystals,
        memory_json,
        records_json,
        pattern_hits: result.pattern_hits,
        commit_index: result.commit_index,
        new_crystal_count,
        il_commits,
    }).into_response()
}

async fn context(Json(req): Json<ContextRequest>) -> impl IntoResponse {
    let records: Vec<pse::CrystalRecord> = match serde_json::from_str(&req.records_json) {
        Ok(r) => r,
        Err(e) => return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": format!("records_json: {e}") })),
        ).into_response(),
    };
    let record_count = records.len();
    let context = pse::render_context(&records, req.top_k);
    Json(ContextResponse { context, record_count }).into_response()
}

async fn coverage(Json(req): Json<CoverageRequest>) -> impl IntoResponse {
    let (hits, total) = pse::score_coverage(&req.text, &req.keywords);
    let coverage = if total == 0 { 0.0 } else { hits as f64 / total as f64 };
    Json(CoverageResponse { hits, total, coverage }).into_response()
}

// ── IL handlers ───────────────────────────────────────────────────────────────

async fn il_status(State(state): State<AppState>) -> impl IntoResponse {
    let guard = match state.il.lock() {
        Ok(g) => g,
        Err(_) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "IL store lock poisoned" })),
        ).into_response(),
    };

    match &*guard {
        None => Json(ILStatusResponse {
            active: false,
            block_count: 0,
            mean_coherence_potential: 0.0,
            edge_counts: None,
        }).into_response(),
        Some(store) => {
            let edge_counts = Some(ILEdgeCounts {
                sequential_commit:   store.hdag_edge_count_by_cause("sequential_commit"),
                resonance_proximity: store.hdag_edge_count_by_cause("resonance_proximity"),
                refinement:          store.hdag_edge_count_by_cause("refinement"),
            });
            Json(ILStatusResponse {
                active: true,
                block_count: store.len(),
                mean_coherence_potential: store.mean_coherence_potential(),
                edge_counts,
            }).into_response()
        }
    }
}

async fn il_retrieve(
    State(state): State<AppState>,
    Json(req): Json<ILRetrieveRequest>,
) -> impl IntoResponse {
    let records: Vec<pse::CrystalRecord> = match serde_json::from_str(&req.records_json) {
        Ok(r) => r,
        Err(e) => return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": format!("records_json: {e}") })),
        ).into_response(),
    };

    let guard = match state.il.lock() {
        Ok(g) => g,
        Err(_) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "IL store lock poisoned" })),
        ).into_response(),
    };

    let Some(store) = &*guard else {
        return Json(ILRetrieveResponse { active: false, results: vec![] }).into_response();
    };

    let q_vec = text_to_vector8(&req.question);
    let mut hits = store.score_tripolar(&q_vec);
    hits.truncate(req.top_k);

    let results: Vec<ILRetrieveResult> = hits
        .into_iter()
        .enumerate()
        .filter_map(|(i, hit)| {
            // Verify this crystal exists in the supplied records
            let known = records.iter().any(|r| {
                let hex: String = r.crystal.crystal_id.iter().map(|b| format!("{b:02x}")).collect();
                hex == hit.crystal_id_hex
            });
            if known {
                Some(ILRetrieveResult {
                    crystal_id: hit.crystal_id_hex,
                    score_d: hit.score,
                    rank: i + 1,
                })
            } else {
                // Crystal is in IL but not in the supplied records — include anyway
                Some(ILRetrieveResult {
                    crystal_id: hit.crystal_id_hex,
                    score_d: hit.score,
                    rank: i + 1,
                })
            }
        })
        .collect();

    Json(ILRetrieveResponse { active: true, results }).into_response()
}

async fn il_hdag_coherence(State(state): State<AppState>) -> impl IntoResponse {
    let guard = match state.il.lock() {
        Ok(g) => g,
        Err(_) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "IL store lock poisoned" })),
        ).into_response(),
    };

    match &*guard {
        None => Json(ILHdagCoherenceResponse {
            mean_coherence_potential: 0.0,
            node_count: 0,
            edge_counts: None,
        }).into_response(),
        Some(store) => Json(ILHdagCoherenceResponse {
            mean_coherence_potential: store.mean_coherence_potential(),
            node_count: store.len(),
            edge_counts: Some(ILEdgeCounts {
                sequential_commit:   store.hdag_edge_count_by_cause("sequential_commit"),
                resonance_proximity: store.hdag_edge_count_by_cause("resonance_proximity"),
                refinement:          store.hdag_edge_count_by_cause("refinement"),
            }),
        }).into_response(),
    }
}

async fn il_hdag_order(State(state): State<AppState>) -> impl IntoResponse {
    let guard = match state.il.lock() {
        Ok(g) => g,
        Err(_) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "IL store lock poisoned" })),
        ).into_response(),
    };

    match &*guard {
        None => Json(ILHdagOrderResponse { order: vec![], node_count: 0 }).into_response(),
        Some(store) => {
            let order = store.topological_order();
            let node_count = order.len();
            Json(ILHdagOrderResponse { order, node_count }).into_response()
        }
    }
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
    let state = AppState { il: Arc::new(Mutex::new(il_store)) };

    let app = Router::new()
        .route("/health",            get(health))
        .route("/ingest",            post(ingest))
        .route("/context",           post(context))
        .route("/coverage",          post(coverage))
        .route("/il/status",         get(il_status))
        .route("/il/retrieve",       post(il_retrieve))
        .route("/il/hdag/coherence", get(il_hdag_coherence))
        .route("/il/hdag/order",     get(il_hdag_order))
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
    println!();
    println!("  GET  /health");
    println!("  POST /ingest              — text → crystals + state");
    println!("  POST /context             — records → LLM prompt block");
    println!("  POST /coverage            — text + keywords → hit count");
    println!("  GET  /il/status           — IL + HDAG health");
    println!("  POST /il/retrieve         — Pfauenthron++ D=ψ·ρ·ω retrieval");
    println!("  GET  /il/hdag/coherence   — mean ψ + edge counts");
    println!("  GET  /il/hdag/order       — topological order");
    println!();
    println!("  curl http://{addr}/health");
    println!();

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}
