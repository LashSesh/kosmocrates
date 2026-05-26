# Observability conventions

How Kosmocrates emits logs, traces, and metrics — what's there today,
what's coming, and the conventions every contributor should follow
when adding instrumentation.

> See also: [`HARDENING.md`](HARDENING.md) for the operational
> posture of `pse-server` (auth, TLS, probes), and
> [`RELEASING.md`](RELEASING.md) §7 for the OTel item on the
> 1.0 roadmap.

---

## 1. Logging

### Backend

Every binary in the workspace uses the [`tracing`] / [`tracing-subscriber`]
ecosystem. `pse-server` initialises a `tracing-subscriber::fmt` layer
at startup with `EnvFilter` honouring the standard `RUST_LOG`
environment variable:

```bash
# Default: INFO across the workspace.
RUST_LOG=info pse-server

# Debug one crate, info elsewhere:
RUST_LOG=pse_server=debug,tower_http=info,axum=info pse-server

# Operator-level emergency-only:
RUST_LOG=warn pse-server
```

The default formatter writes human-readable lines to stderr. For
structured JSON ingestion (Loki / Vector / Datadog / Splunk), pipe
stderr through `vector` with the [`json` parser] or — if you build
your own binary on top of the workspace — swap to
`tracing_subscriber::fmt::layer().json()` in your init code.

### Levels

Convention across the workspace:

| Level | When to use |
|---|---|
| `error!` | The operation failed and the caller will see it. Reserve for things that wake humans. |
| `warn!`  | The operation succeeded but degraded (fallback used, retry triggered, soft limit hit). |
| `info!`  | A user-visible state transition: server start, IL store loaded, signal change, release-tag detected. One per significant lifecycle event — not per request. |
| `debug!` | Per-request / per-tick detail for troubleshooting. The default `RUST_LOG=info` filters these out. |
| `trace!` | Inner-loop detail. Off by default in every deployed mode. |

`pse-core` already emits `tracing::debug!` events on every kairos
gate decision (`crates/pse-core/src/lib.rs:702`) — read those as the
reference for field naming.

### Fields

Always use structured fields, never `format!` into the message:

```rust
// Good — fields are queryable downstream:
tracing::debug!(tick = state.commit_index, d = gate.d, "kairos rejected");

// Bad — string interpolation loses structure:
tracing::debug!("kairos rejected at tick {} (d={})", state.commit_index, gate.d);
```

Field naming:

- `tick` — monotonic commit index.
- `crystal_id` — full 64-char hex SHA-256 (already content-addressed).
- `block_hash` — IL block hash.
- `source_id` — adapter source identifier.
- `signal` — current `EpistemicSignal` (`coherent` / `drifting` / `diverging`).
- `route` — HTTP route path (set by middleware, not by handlers).
- `request_id` — opaque UUID, currently TBD (see §5 roadmap).

Avoid `name = format!("...")` — store the unformatted parts in
separate fields.

---

## 2. Spans

Spans group related events under a single context so a trace UI can
render the call as one operation.

### When to add a span

- Every public async function in `tools/pse-server/` request
  handlers — but use middleware, not per-handler `#[instrument]`.
  (Currently TBD — see §5.)
- Hot paths in `pse-core` where you would otherwise want a stack-style
  trace. `macro_step` is the canonical entry point.
- Long-running synchronous loops (`pse-traverse`, `pse-cascade`)
  where you want a wall-clock measurement per iteration.

### How

```rust
#[tracing::instrument(level = "debug", skip(state, adapter), fields(tick = state.commit_index))]
fn macro_step_inner(...) { ... }
```

Conventions:

- **Always `skip(...)`** large or non-`Display` parameters. The
  default behaviour records every argument with `Debug`, which is
  noisy and can leak payloads.
- **`fields(...)`** for whatever stays small and useful (`tick`,
  `source_id`, request size). These appear on the span itself, so
  every child event inherits them.
- **`level = "debug"`** for hot-path spans so they do not show under
  the default `RUST_LOG=info`.

---

## 3. Metrics

Currently: **no built-in metrics endpoint.** The supported pattern
is to scrape coarse metrics from the access log (route, status,
latency) at the reverse proxy in front of `pse-server`.

Roadmap (see `ROADMAP.md` Near-term):

- Add `/metrics` Prometheus endpoint to `pse-server` behind a
  `metrics` feature flag (off by default).
- Expose the following baseline counters / histograms:

| Metric | Type | Labels | Source |
|---|---|---|---|
| `pse_http_requests_total` | counter | `route`, `method`, `status` | tower middleware |
| `pse_http_request_duration_seconds` | histogram | `route`, `method` | tower middleware |
| `pse_crystals_total` | counter | `source_id` | `pse-core` commit path |
| `pse_pattern_hits_total` | counter | `source_id` | `pse-core` recognition path |
| `pse_il_block_count` | gauge | `store_path` | `pse-adapter-il` |
| `pse_il_mean_coherence_potential` | gauge | `store_path` | `pse-adapter-il` |
| `pse_nxalien_signal_stability` | gauge | `repo` | `pse-nxalien-evolve` |

These names follow the [Prometheus naming convention]:
`<subsystem>_<unit>_total|count|seconds`, snake_case, lowercase.

Until that lands, derive what you can from `/health`, `/ready`,
`/il/status`, and `/nxalien/signal` — all return JSON that can be
scraped by `blackbox_exporter` or a tiny scrape script.

---

## 4. Distributed tracing

Currently: **no built-in OpenTelemetry export.** Spans stay in-process.

Roadmap:

- Add `tracing-opentelemetry` + `opentelemetry-otlp` behind an
  `otel` feature flag in `tools/pse-server`.
- Honour the standard OTLP env vars:
  - `OTEL_EXPORTER_OTLP_ENDPOINT` (e.g. `http://otel-collector:4318`)
  - `OTEL_EXPORTER_OTLP_PROTOCOL` (`grpc` | `http/protobuf`)
  - `OTEL_SERVICE_NAME` (defaults to `pse-server`)
- Add `request_id` middleware so every span carries a trace-correlatable
  ID, and so the same ID surfaces in the access log.

The trade-off being weighed: `tracing-opentelemetry` pulls in a
non-trivial dependency graph (`tonic`, `prost`, `hyper`, `h2`),
which would touch the `cargo-deny` license / bans posture in
`deny.toml`. The feature stays off-by-default precisely so consumers
who do not need it pay nothing.

---

## 5. Roadmap & gaps

In rough priority order:

1. **`request_id` middleware** for `pse-server` — every request gets
   a UUID, propagated in log fields and a response header
   (`x-request-id`). Trivial code change, large operational payoff.
2. **`/metrics` Prometheus endpoint** behind `metrics` feature flag.
3. **OTel export** behind `otel` feature flag.
4. **Per-route latency span** via `tower_http::trace::TraceLayer`.
   Cheap; emits one `info!` per request with `method`/`route`/`status`/`latency`.
5. **Structured JSON formatter** as a `json-logs` feature flag,
   so operators can flip log format at boot without rebuilding.
6. **Debug-snapshot endpoint** (`/debug/snapshot`) — returns a
   redacted dump of `GlobalState` for offline reproduction of
   determinism issues. Behind `debug-snapshot` feature flag; never
   on in production.

---

## 6. Contributing instrumentation

When you add `tracing::*!` calls or `#[instrument]` annotations:

- Match the level convention in §1.
- Use structured fields per §1, not `format!`.
- For hot paths, default to `level = "debug"` so the operator
  default (`RUST_LOG=info`) does not get spammed.
- Never log a full payload — log size + content hash instead.
  (`crystal_id`, `block_hash`, `digest` are content-addressed; logging
  them is safe by construction.)
- For new metrics, follow the naming table in §3 and add the row to
  the table in the same PR.

[`tracing`]: https://docs.rs/tracing
[`tracing-subscriber`]: https://docs.rs/tracing-subscriber
[`json` parser]: https://vector.dev/docs/reference/configuration/transforms/remap/
[Prometheus naming convention]: https://prometheus.io/docs/practices/naming/
