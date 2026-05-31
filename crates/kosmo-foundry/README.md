# kosmo-foundry

Real Foundry executor for KOSMO-OPS-01 RX

`kosmo-foundry` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Turns a `FoundryExecutionPlan` (the pure, content-addressed data model defined in
`kosmo-core`) into an actually-executed `FoundryExecutionReport` by running
allowlisted cargo commands in a policy-governed sandbox.

Where `kosmo-pipeline::simulate_foundry_check` only simulates an outcome, this
crate runs the real process: spawning `cargo check`, `cargo test`, and
`cargo clippy` against a workspace root, capturing output, enforcing per-check
timeouts, and returning a content-addressed report.

## Why it is a separate crate

`kosmo-core` is the portable, process-free substrate — it compiles to wasm and
must stay free of host I/O. Spawning processes is a host capability, so it lives
here, exactly as `kosmo-pse-bridge` isolates the PSE crossing.

## Safety / policy contract

The executor is fail-closed by construction:

- **ReportOnly is inert.** `PolicyProfile.mode == ReportOnly` returns
  `FoundryExecutionReport::skipped_by_report_only` immediately; no process is
  ever spawned.
- **Allowlist is checked before spawn.** Every check kind is mapped to a
  `(program, subcommand)` pair via `map_kind_to_subcommand` and validated against
  the plan's `FoundryCommandPolicy`. A denied command yields
  `FoundryExecutionOutcome::CommandDeniedByPolicy` and is never executed.
- **Only read-only subcommands are reachable.** The kind → subcommand table maps
  `Build`/`TypeCheck` → `cargo check`, `Test` → `cargo test`,
  `Lint` → `cargo clippy`. No mutating flag is ever passed; cargo writes only to
  `target/` inside the sandbox root.
- **Environment is stripped.** Unless `FoundryEnvironmentPolicy` permits
  passthrough, the child runs with a minimal environment. Secret-looking variables
  (`API_KEY`, `TOKEN`, `PASSWORD`, etc.) are never forwarded.
- **Timeouts are enforced.** Each check is bounded by
  `FoundryTimeoutPolicy.per_check_timeout_ms`; an overrun kills the child and
  yields `FoundryExecutionOutcome::TimedOut`.
- **Worst-wins aggregation.** The plan outcome is the most severe per-check
  outcome. An empty plan fails closed as `Inconclusive`.
- **Content-addressed audit.** Every per-check evidence id and the final report
  id are SHA-256 content digests (INVARIANT-007); identical captured output yields
  identical ids.

## Key API

```rust
// Build a standard read-only cargo plan (check + test).
let plan = kosmo_foundry::standard_cargo_plan(
    policy_id, workspace_index_id, task_id, root_digest,
    /* per_check_timeout_ms */ 30_000,
);

// Execute the plan.
let executor = FoundryExecutor::new("/path/to/workspace");
let report = executor.execute(&plan, &PolicyProfile::dry_run(), evidence_bundle_id);
assert!(report.verify_id()); // content-addressed

// Map a check kind to its cargo subcommand.
assert_eq!(map_kind_to_subcommand(&FoundryCheckKind::Lint), Some("clippy"));
```

## Documentation

API reference: `cargo doc -p kosmo-foundry --open`

## License

MIT — see [`LICENSE`](../../LICENSE).
