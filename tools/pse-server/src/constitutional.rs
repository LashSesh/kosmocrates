//! Constitutional middleware and `/constitutional/check` handler for pse-server.
//!
//! ## Tower middleware (`ConstitutionalLayer`)
//!
//! Wraps any axum `Router`.  When a request carries `x-nxalien-verb`,
//! `x-nxalien-target`, or `x-nxalien-description` headers the middleware
//! runs the `ConstitutionalEvaluator` before passing the request downstream.
//!
//! - **Block** → 403 JSON `{ "constitutional_block": true, … }`
//! - **Warn**  → request passes through; `x-nxalien-warn: <rule_id>` added
//! - **Allow** → transparent pass-through (zero overhead for clean requests)
//!
//! Requests without any `x-nxalien-*` headers are never evaluated.
//!
//! ## REST endpoint (`POST /constitutional/check`)
//!
//! Agents call this endpoint *before* taking an action to get an explicit
//! Allow/Block/Warn decision from the server's current rule set.
//!
//! Request:
//! ```json
//! {
//!   "action": { "verb": "run_command", "target": "rm -rf /", "description": "…" },
//!   "strict_mode": false
//! }
//! ```
//!
//! Response: serialised `EvaluationReport`.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::{Json, State},
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use pse_constitutional_interceptor::{ActionContext, ConstitutionalEvaluator, EvaluationReport};
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};

// ── AppState accessor ─────────────────────────────────────────────────────────

/// Shared handle to the live evaluator held in AppState.
pub type ConstitutionalHandle = Arc<Mutex<ConstitutionalEvaluator>>;

// ── Tower middleware ──────────────────────────────────────────────────────────

/// Tower `Layer` that wraps any service with constitutional enforcement.
///
/// ```rust,no_run
/// let router = Router::new()
///     .route("/nxalien/bundle", post(nxalien_bundle))
///     .layer(ConstitutionalLayer::new(state.constitutional.clone()));
/// ```
#[derive(Clone)]
pub struct ConstitutionalLayer {
    handle: ConstitutionalHandle,
}

impl ConstitutionalLayer {
    pub fn new(handle: ConstitutionalHandle) -> Self {
        Self { handle }
    }
}

impl<S> Layer<S> for ConstitutionalLayer {
    type Service = ConstitutionalService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        ConstitutionalService {
            inner,
            handle: self.handle.clone(),
        }
    }
}

/// Tower `Service` produced by `ConstitutionalLayer`.
#[derive(Clone)]
pub struct ConstitutionalService<S> {
    inner: S,
    handle: ConstitutionalHandle,
}

impl<S, ReqBody> Service<Request<ReqBody>> for ConstitutionalService<S>
where
    S: Service<Request<ReqBody>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let verb = header_str(req.headers(), "x-nxalien-verb");
        let target = header_str(req.headers(), "x-nxalien-target");
        let description = header_str(req.headers(), "x-nxalien-description");

        // No constitutional headers → transparent pass-through.
        if verb.is_empty() && target.is_empty() && description.is_empty() {
            let mut inner = self.inner.clone();
            return Box::pin(async move { inner.call(req).await });
        }

        let action = ActionContext::new(verb, target, description);
        let report = {
            let ev = self.handle.lock().unwrap_or_else(|p| p.into_inner());
            ev.evaluate(&action)
        };

        if report.decision.is_block() {
            let body = serde_json::to_string(&serde_json::json!({
                "constitutional_block": true,
                "decision": report.decision,
                "triggered_rules": report.triggered_rules,
                "checked_rule_count": report.checked_rule_count,
            }))
            .unwrap_or_default();
            return Box::pin(async move {
                Ok(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap())
            });
        }

        // Warn or Allow — pass through, add warn header if needed.
        let warn_rule = report.decision.rule_id().map(|s| s.to_string());
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let mut resp = inner.call(req).await?;
            if let Some(rule_id) = warn_rule {
                if let Ok(v) = rule_id.parse() {
                    resp.headers_mut().insert("x-nxalien-warn", v);
                }
            }
            Ok(resp)
        })
    }
}

fn header_str(headers: &axum::http::HeaderMap, name: &str) -> String {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

// ── REST handler ──────────────────────────────────────────────────────────────

/// Request body for `POST /constitutional/check`.
#[derive(Deserialize)]
pub struct ConstitutionalCheckRequest {
    pub action: ActionContext,
    /// Override strict mode for this check.  If omitted, uses the server's
    /// current mode (derived from EpistemicSignal stability).
    #[serde(default)]
    pub strict_mode: Option<bool>,
}

/// Response body for `POST /constitutional/check`.
#[derive(Serialize)]
pub struct ConstitutionalCheckResponse {
    pub report: EvaluationReport,
    /// Number of rules currently loaded in the server's evaluator.
    pub active_rule_count: usize,
    /// Whether the server's evaluator is in strict mode.
    pub server_strict_mode: bool,
}

/// `POST /constitutional/check` — evaluate an action against the live rule set.
///
/// Agents call this before taking an action.  The server's rule set is updated
/// every time `POST /nxalien/bundle` is called, so this always reflects the
/// current evolved governance state of all connected repos.
pub async fn constitutional_check(
    State(handle): State<ConstitutionalHandle>,
    Json(req): Json<ConstitutionalCheckRequest>,
) -> impl IntoResponse {
    let (report, active_rule_count, server_strict_mode) = {
        let ev = handle.lock().unwrap_or_else(|p| p.into_inner());
        let server_strict = ev.is_strict();
        let active = ev.rule_count();

        // The caller can override strict_mode for a single check.
        let ev_for_check = if let Some(strict) = req.strict_mode {
            ev.clone().with_strict_mode(strict)
        } else {
            ev.clone()
        };

        let report = ev_for_check.evaluate(&req.action);
        (report, active, server_strict)
    };

    let status = if report.decision.is_block() {
        StatusCode::FORBIDDEN
    } else {
        StatusCode::OK
    };

    (
        status,
        Json(ConstitutionalCheckResponse {
            report,
            active_rule_count,
            server_strict_mode,
        }),
    )
}
