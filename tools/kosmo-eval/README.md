# kosmo-eval

KOSMO-OPS-01 empirical validation benchmark

`kosmo-eval` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this tool validates.

## What it does

Runs 52 deterministic invariant scenarios that exercise the full KOSMO-OPS-01
data model and the RX real executors end-to-end, then prints a pass/fail table
and exits with code 0 (all pass) or 1 (any failure).

Each scenario is self-contained, runs in-process (no external state between
scenarios), and captures its own elapsed time. An optional Cerebras API
round-trip test can be appended to the run if an API key is provided.

### Scenario phases

| Phase | Scenarios | What is verified |
|---|---|---|
| `R1:Foundry` | 5 | Sandbox defaults, command policy, ReportOnly skip, plan id determinism, env policy |
| `R2:ParseBack` | 5 | Outcome semantics, severity ordering, delta id determinism, report id determinism, critical delta detection |
| `R3:ValidationClosure` | 4 | Both-fail, foundry-only fail, ReportOnly→Inconclusive, closure report id determinism |
| `R4:Cartography` | 4 | ReportOnly denial, gapless sequence enforcement, empty-store integrity, append + integrity |
| `R5:Materialization` | 2 | `allow_host_write` hardwired false (MATBOUND-002), spec id determinism |
| `R6:KCube` | 3 | ReportOnly export policy, roundtrip digest equality, roundtrip id determinism |
| `R7:PSEBridge` | 4 | ReportOnly blocks all submissions, deny-all policy, zero evidence bundle rejection, candidate id determinism |
| `R8:Acquisition` | 4 | Taint starts Unverified, Cleared requires both checks, deny-all capability, source id determinism |
| `R9:Evaluation` | 6 | INVARIANT-007 determinism, ReportOnly skip, always-fail harness, worst-wins suite aggregation, all-pass suite, policy id content-addressing |
| `RX:FoundryExec` | 3 | ReportOnly spawns nothing, denied command not executed, kind-mapping read-only |
| `RX:PersistStore` | 2 | DryRun cannot persist (emergent host-write invariant), OperatorApproved persists + reloads |
| `RX:ParseBackExec` | 6 | ReportOnly skips scan, baseline mismatch→Inconclusive, NodeAdded→Warning, NodeRemoved→Critical, snapshot determinism, unchanged workspace passes |
| `RX:Operator` | 4 | ReportOnly→Inconclusive, report content-addressing (INVARIANT-007), full DryRun cycle, OperatorApproved persists closure |

## Safety / policy contract

`kosmo-eval` runs under a mixture of `PolicyProfile::dry_run()` and
`PolicyProfile::default_report_only()` for the data-model scenarios, and uses
`PolicyProfile::operator_approved()` only in the two scenarios that explicitly
test persistence (the JSONL store and the operator pipeline). All temp files
created by those scenarios are removed immediately after the assertion.

No network access is performed except in the optional Cerebras scenario, which
requires an explicit API key.

## Usage

```sh
# Run all 52 core scenarios (no API key required).
cargo run -p kosmo-eval

# Include the Cerebras API round-trip (prompts if the env var is absent).
CEREBRAS_API_KEY=<key> cargo run -p kosmo-eval

# Use a specific Cerebras model (default: gpt-oss-120b).
CEREBRAS_MODEL=llama3.1-8b CEREBRAS_API_KEY=<key> cargo run -p kosmo-eval
```

Exit code `0` means all scenarios passed. Exit code `1` means one or more
invariants were violated; failed scenario names and diagnostics are printed in
the table.

## Documentation

API reference for the crates under test: `cargo doc --workspace --open`

## License

MIT — see [`LICENSE`](../../LICENSE).
