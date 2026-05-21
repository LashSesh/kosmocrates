//! PSE REST API server.
//!
//! Exposes the Post-Symbolic Engine over HTTP/JSON so any language can use
//! it without a Rust or Python toolchain.
//!
//! **Design:** fully stateless — the client carries state between calls via
//! `memory_json` and `records_json` fields.  The server holds no sessions.
//!
//! **Endpoints:**
//!
//!   GET  /health   — liveness check
//!   POST /ingest   — process text through PSE; returns crystals + state
//!   POST /context  — render crystal records into an LLM prompt block
//!   POST /coverage — count domain keywords in a response
//!
//! **Configuration (env vars):**
//!
//!   `PSE_SERVER_HOST`   bind address (default: `0.0.0.0`)
//!   `PSE_SERVER_PORT`   port         (default: `8765`)
//!
//! **Quickstart:**
//! ```text
//!   cargo run --release -p pse-server
//!
//!   # Health check
//!   curl http://localhost:8765/health
//!
//!   # Session 1: cold start
//!   curl -s -X POST http://localhost:8765/ingest \
//!     -H 'Content-Type: application/json' \
//!     -d '{"text":"Entropy measures disorder in a thermodynamic system. \
//!           In information theory entropy quantifies uncertainty. \
//!           Both formulations share a logarithmic structure. \
//!           Maxwell demon exploits information to violate the second law.",
//!          "session":1,"question":"Explain entropy"}' | jq .
//!
//!   # Session 2: warm start (pass memory_json from session 1 response)
//!   curl -s -X POST http://localhost:8765/ingest \
//!     -H 'Content-Type: application/json' \
//!     -d '{"text":"...same text...","memory_json":"...","records_json":"...","session":2}' \
//!     | jq .pattern_hits
//! ```

mod pse;

use std::net::SocketAddr;

use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct IngestRequest {
    /// The text to ingest (LLM response, document chunk, etc.).
    text: String,
    /// JSON from a previous `/ingest` response — omit for cold start.
    #[serde(default)]
    memory_json: Option<String>,
    /// JSON from a previous `/ingest` response — omit for cold start.
    #[serde(default)]
    records_json: Option<String>,
    /// 1-indexed session number shown in context output (default: 1).
    #[serde(default = "default_session")]
    session: usize,
    /// The question/prompt that produced this text (default: "").
    #[serde(default)]
    question: String,
    /// Label for this observation source (default: "pse-server").
    #[serde(default)]
    source_name: Option<String>,
}

fn default_session() -> usize { 1 }

#[derive(Serialize)]
struct IngestResponse {
    /// Crystals formed from this text (may be empty).
    new_crystals:      Vec<pse::CrystalInfo>,
    /// Pass as `memory_json` to the next `/ingest` call.
    memory_json:       String,
    /// Pass as `records_json` to the next `/ingest` call.
    records_json:      String,
    /// Pattern-memory hits (> 0 means prior topology recognised).
    pattern_hits:      u64,
    /// Monotone tick counter.
    commit_index:      u64,
    /// Number of new crystals formed.
    new_crystal_count: usize,
}

#[derive(Deserialize)]
struct ContextRequest {
    /// JSON from a previous `/ingest` response `records_json` field.
    records_json: String,
    /// Max crystals to include (default: 5).
    #[serde(default = "default_top_k")]
    top_k: usize,
}

fn default_top_k() -> usize { 5 }

#[derive(Serialize)]
struct ContextResponse {
    /// LLM-injectable context block — prepend to your system prompt.
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
    /// hits / total as a float in [0, 1].
    coverage: f64,
}

#[derive(Serialize)]
struct HealthResponse {
    status:  &'static str,
    version: &'static str,
}


// ── Handlers ──────────────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(HealthResponse { status: "ok", version: env!("CARGO_PKG_VERSION") })
}

async fn ingest(Json(req): Json<IngestRequest>) -> impl IntoResponse {
    let source = req.source_name.as_deref().unwrap_or("pse-server");
    match pse::ingest(
        &req.text,
        req.memory_json.as_deref(),
        req.records_json.as_deref(),
        req.session,
        &req.question,
        source,
    ) {
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
        Ok(result) => {
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
            })
            .into_response()
        }
    }
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
    Json(ContextResponse { context, record_count }).into_response()
}

async fn coverage(Json(req): Json<CoverageRequest>) -> impl IntoResponse {
    let (hits, total) = pse::score_coverage(&req.text, &req.keywords);
    let coverage = if total == 0 { 0.0 } else { hits as f64 / total as f64 };
    Json(CoverageResponse { hits, total, coverage }).into_response()
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

    let app = Router::new()
        .route("/health",   get(health))
        .route("/ingest",   post(ingest))
        .route("/context",  post(context))
        .route("/coverage", post(coverage))
        .layer(CorsLayer::permissive());

    println!();
    println!("PSE Server  v{}", env!("CARGO_PKG_VERSION"));
    println!("═══════════════════════════════════════════════");
    println!("  Listening : http://{addr}");
    println!();
    println!("  GET  /health");
    println!("  POST /ingest    — text → crystals + state");
    println!("  POST /context   — records → LLM prompt block");
    println!("  POST /coverage  — text + keywords → hit count");
    println!();
    println!("  curl http://{addr}/health");
    println!();

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}
