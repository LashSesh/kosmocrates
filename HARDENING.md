# Hardening guide — pse-server

`pse-server` (in `tools/pse-server/`) is the supported HTTP surface
for Kosmocrates. By default it is a developer / single-tenant tool;
this document is the checklist for taking it to a multi-tenant or
internet-exposed deployment.

> See also: [`SECURITY.md`](SECURITY.md) for the threat model and
> private vulnerability reporting flow.

---

## Authentication

The server ships with a built-in bearer-token middleware
(`tools/pse-server/src/auth.rs`). It is **disabled by default**.

Enable it by setting the env var at startup:

```bash
export PSE_SERVER_TOKEN="$(openssl rand -hex 32)"
pse-server
# → "  auth      : bearer token required (PSE_SERVER_TOKEN)"
```

When enabled:

- Every route except `/health` and `/ready` requires
  `Authorization: Bearer <token>`.
- Token comparison runs in constant time
  (`tools/pse-server/src/auth.rs::constant_time_eq`).
- An unset / empty `PSE_SERVER_TOKEN` disables the middleware
  silently — the startup banner makes the posture explicit.

**The built-in middleware is the minimum, not the recommendation.**
For multi-tenant or production-grade deployments, terminate auth in a
reverse proxy and front the server with one of:

- mTLS (e.g. Envoy / NGINX with client-cert verification).
- OIDC (e.g. oauth2-proxy, Authentik).
- SPIFFE / SPIRE identities in a service mesh.

The built-in middleware then acts as a defence-in-depth layer.

---

## TLS

`pse-server` speaks plain HTTP on the bound port (default `8765`).
There is no built-in TLS termination. The supported pattern is:

- Local development: `localhost:8765`, plain HTTP.
- Container deployment: front with a reverse proxy
  (NGINX / Caddy / Envoy / Traefik) that owns the TLS certificate
  lifecycle.
- Kubernetes deployment: use an Ingress controller with cert-manager.

Built-in TLS is intentionally out of scope (see `ROADMAP.md`) — the
project ships one HTTP listener and lets the surrounding stack handle
crypto rotation, OCSP stapling, and SNI.

---

## CORS

The default `tools/pse-server/src/main.rs` uses
`CorsLayer::permissive()` so the browser-based `web/` demo can call
the API from any origin. **This is unsafe for any non-development
deployment.**

For a hardened deployment, replace the permissive layer with an
allowlist:

```rust
use tower_http::cors::{CorsLayer, Any};

let cors = CorsLayer::new()
    .allow_origin([
        "https://app.example.com".parse().unwrap(),
    ])
    .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
    .allow_headers([axum::http::header::AUTHORIZATION, axum::http::header::CONTENT_TYPE]);
```

The env-driven version (`PSE_SERVER_CORS_ORIGINS=https://a.example.com,https://b.example.com`)
is on the roadmap.

---

## Liveness vs readiness

Two distinct probes, both unauthenticated:

| Endpoint | Purpose | Suggested probe |
|---|---|---|
| `GET /health` | Liveness — the process is up. Cheap; no downstream check. | Kubernetes `livenessProbe`. |
| `GET /ready` | Readiness — the process will serve traffic. Mirrors `/health` today; reserved for future async warm-up gates. | Kubernetes `readinessProbe`, load-balancer health check. |

Both return `{"status": "ok"|"ready", "version": "X.Y.Z"}` as JSON.

---

## Rate limiting

Not built-in. The recommended pattern is to terminate rate limiting
at the reverse proxy:

- NGINX: `limit_req_zone` / `limit_req`.
- Envoy: `envoy.filters.http.local_ratelimit`.
- Cloudflare / Fastly: WAF rule.

If you need in-process rate limiting (e.g. no proxy in front),
`tower-governor` is a drop-in `tower::Layer` that integrates with
the axum router used here.

---

## Logging & tracing

The server initialises `tracing-subscriber` at startup
(`tools/pse-server/src/main.rs`). The standard `RUST_LOG` env var
controls verbosity:

```bash
# Default: INFO across the workspace.
RUST_LOG=info pse-server

# Debug the request lifecycle:
RUST_LOG=pse_server=debug,tower_http=info,axum=info pse-server
```

Output goes to stderr in a human-readable format. For structured
JSON logs (Loki / Vector / Datadog), wrap stderr with `vector` or
swap the formatter — see [`OBSERVABILITY.md`](OBSERVABILITY.md) once
that document lands.

OpenTelemetry export (OTLP/HTTP) is on the roadmap as an opt-in
feature flag.

---

## Container deployment

The `docker/Dockerfile` produces a non-root image (uid `10001`) on
`debian:bookworm-slim`. To run hardened:

```bash
docker run --rm -d \
  -p 127.0.0.1:8765:8765 \
  -e PSE_SERVER_TOKEN="$(openssl rand -hex 32)" \
  -e PSE_SERVER_HOST=0.0.0.0 \
  -e PSE_SERVER_PORT=8765 \
  -e PSE_IL_STORE=/var/lib/kosmocrates/il_store \
  -e PSE_NXALIEN_STATE=/var/lib/kosmocrates/nxalien_state.json \
  -e RUST_LOG=info \
  --read-only \
  --tmpfs /tmp \
  -v kosmocrates-data:/var/lib/kosmocrates \
  --cap-drop=ALL \
  --security-opt=no-new-privileges \
  ghcr.io/lashsesh/kosmocrates/pse-server:latest
```

Key additions over `docker-compose.yml`:

- `-p 127.0.0.1:8765:8765` — bind only to loopback; let the reverse
  proxy expose the public port.
- `--read-only` + `--tmpfs /tmp` — immutable root filesystem.
- `--cap-drop=ALL` — drop all Linux capabilities.
- `--security-opt=no-new-privileges` — block setuid escalation.

---

## What is explicitly out of scope (today)

These are not provided and not on the roadmap unless contributors
volunteer:

- A built-in identity provider (OIDC / SAML server).
- Built-in TLS termination.
- Per-tenant request quotas.
- Audit-log forwarding (use stderr → vector / fluent-bit).
- Encrypted-at-rest IL / NxAlien state files (use disk-level
  encryption — LUKS, EBS encryption, encrypted EFS).

---

## Reporting issues

Vulnerabilities go through the private flow in [`SECURITY.md`](SECURITY.md)
— never a public issue.

Non-vulnerability hardening suggestions (better defaults, additional
middleware, new probe surfaces) are welcome as PRs against `main`.
The `tools/pse-server/` module is owned by `@LashSesh` (see
`.github/CODEOWNERS`).
