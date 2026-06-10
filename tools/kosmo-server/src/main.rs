//! Kosmocrates HTTP server.
//!
//! Serves the pipeline over a REST API and an embedded single-page browser UI.
//!
//! USAGE:
//!     kosmo-server [--port <n>] [--host <addr>] [--open]
//!
//! Endpoints:
//!     GET  /                  Browser UI
//!     GET  /api/health        Version ping
//!     POST /api/analyse       Run pipeline, return structured report

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

use kosmo_core::{Digest, GateResult, PolicyProfile, Q16};
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
        .route("/api/analyse", post(analyse));

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
