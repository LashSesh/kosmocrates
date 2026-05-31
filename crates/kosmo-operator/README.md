# kosmo-operator

R1→R2→R3 operator pipeline orchestrator for KOSMO-OPS-01 RX

`kosmo-operator` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Wires `kosmo-foundry`, `kosmo-parseback`, and `kosmo-store` into a complete
validation-closure pipeline. A single `OperatorExecutor::execute` call runs all
five stages and returns one content-addressed `OperationReport`:

1. **ParseBack pre-snapshot** — `cargo metadata` captures current workspace topology.
2. **Foundry execution** — allowlisted `cargo check`/`test`/`clippy` checks.
3. **ParseBack post-snapshot** — re-scan the workspace; diff against pre.
4. **ValidationClosure** — combine sub-outcomes into a single closure decision.
5. **Persistence** (`OperatorApproved` only) — append the closure payload digest
   to the JSONL cartography store.

`OperationReport.report_id` is the SHA-256 content digest of all sub-report ids
and metadata (INVARIANT-007): identical inputs always produce identical report ids.

## Why it is a separate crate

Each sub-executor (`kosmo-foundry`, `kosmo-parseback`, `kosmo-store`) is already
isolated from `kosmo-core`. The operator is the composition layer that depends on
all three; separating it keeps the dependency graph acyclic and lets each executor
be used independently.

## Safety / policy contract

Policy governs the entire pipeline uniformly:

- **ReportOnly → zero I/O.** All sub-reports are `SkippedByReportOnly`;
  the closure is `Inconclusive`. No process is spawned and no file is touched.
- **DryRun → full read-only, no persist.** Foundry checks and ParseBack scans run
  against the live workspace; no host files are written. The closure is computed
  but never appended to disk (`persisted == false`).
- **OperatorApproved → full execution + optional persist.** All stages run. If a
  `store_path` is configured and the closure passes, the closure payload digest is
  appended to the JSONL store via `kosmo-store` (which enforces `allow_host_write`
  before writing).
- **Content-addressed throughout.** `OperationPlan.plan_id` and
  `OperationReport.report_id` are SHA-256 content digests (INVARIANT-007).
- **Fail-closed pre-snapshot.** If the pre-materialization ParseBack snapshot fails,
  an `Inconclusive` report is returned immediately rather than proceeding with
  incomplete evidence.

## Key API

```rust
// Build a standard plan (cargo check + test, full workspace ParseBack).
let plan = standard_plan(
    &workspace_root,
    ParseBackScanScope::AffectedCratesOnly,
    policy_id,
    /* per_check_timeout_ms */ 30_000,
);

// Run the full pipeline.
let executor = OperatorExecutor::new(&workspace_root)
    .with_store("/path/to/corpus.jsonl"); // optional; required for persist
let report = executor.execute(&plan, &PolicyProfile::operator_approved(), evidence_bundle_id);
assert!(report.verify_id());
assert!(report.persisted); // true only under OperatorApproved + Passed
```

## Documentation

API reference: `cargo doc -p kosmo-operator --open`

## License

MIT — see [`LICENSE`](../../LICENSE).
