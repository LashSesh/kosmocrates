//! Bearer-token authentication middleware for pse-server.
//!
//! The middleware is always compiled in but **disabled by default**
//! (no `PSE_SERVER_TOKEN` env var present at startup → every request
//! is forwarded). Set `PSE_SERVER_TOKEN` to a non-empty value to
//! require an `Authorization: Bearer <token>` header on every route
//! except the probe endpoints (`/health`, `/ready`) and the keyless
//! self-description (`/doors`).
//!
//! This is the minimum auth surface — it is intentionally *not* a
//! replacement for putting the server behind a reverse proxy with
//! mTLS, OIDC, or SPIFFE in front of it. See `HARDENING.md` for the
//! recommended deployment posture.

use std::sync::OnceLock;

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

/// Cached token value, read once at the first request and never again.
/// `Some(None)` = no token configured (auth disabled);
/// `Some(Some(token))` = auth enabled with that token.
static TOKEN: OnceLock<Option<String>> = OnceLock::new();

/// Routes that must always succeed without auth, so liveness /
/// readiness probes can verify the process from outside the trust
/// boundary — and the self-description, which is keyless on every
/// surface (`kosmo-run --doors` needs no key either).
const PROBE_PATHS: &[&str] = &["/health", "/ready", "/doors"];

pub async fn bearer_auth(req: Request, next: Next) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    if PROBE_PATHS.contains(&path) {
        return Ok(next.run(req).await);
    }

    let configured = TOKEN.get_or_init(|| {
        std::env::var("PSE_SERVER_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
    });
    let Some(token) = configured else {
        // No token configured — middleware is a no-op.
        return Ok(next.run(req).await);
    };

    let presented = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");

    if constant_time_eq(presented.as_bytes(), token.as_bytes()) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// `true` if a `PSE_SERVER_TOKEN` was set at startup. Used by the
/// startup banner to print whether the server runs open or guarded.
pub fn is_enabled() -> bool {
    TOKEN
        .get_or_init(|| {
            std::env::var("PSE_SERVER_TOKEN")
                .ok()
                .filter(|s| !s.is_empty())
        })
        .is_some()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_matches() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(!constant_time_eq(b"", b"x"));
        assert!(constant_time_eq(b"", b""));
    }
}
