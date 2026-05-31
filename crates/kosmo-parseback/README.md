# kosmo-parseback

Real ParseBack executor for KOSMO-OPS-01 RX

`kosmo-parseback` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Snapshots workspace crate topology via `cargo metadata --no-deps`, diffs pre- and
post-materialization snapshots, and produces a content-addressed `ParseBackReport`.

`TopologySnapshot` captures every crate's name, sorted source file paths, and
sorted dependency names into a SHA-256 `snapshot_id` (INVARIANT-007). Identical
inputs always produce identical ids. `diff_snapshots` classifies structural
changes by severity:

| Change | Severity |
|---|---|
| `NodeRemoved` / `EdgeRemoved` | Critical |
| `NodeAdded` / `EdgeAdded` | Warning |
| `NodeModified` (files or deps changed) | Info |

Any `Critical` delta causes the report outcome to be `Failed`.

## Why it is a separate crate

`kosmo-core` is the portable, process-free substrate (filesystem-free and
process-free). Snapshotting a workspace requires both filesystem reads and a
`cargo metadata` process spawn — host capabilities that live here, following the
same isolation principle as `kosmo-foundry` and `kosmo-store`.

## Safety / policy contract

- **ReportOnly is inert.** `PolicyProfile.mode == ReportOnly` returns
  `ParseBackReport::skipped_by_report_only` immediately; no scan is performed and
  no process is spawned.
- **All other modes are read-only.** The executor reads source files and runs
  `cargo metadata`; it never writes to the host filesystem or accesses the network.
- **Baseline integrity is enforced.** The `ParseBackPlan.baseline_topology_id` must
  match the pre-snapshot's `snapshot_id`. A mismatch yields `Inconclusive` rather
  than a false pass or false fail.
- **Identical pre/post topology short-circuits.** When `pre.snapshot_id ==
  post.snapshot_id` the executor returns `TopologyUnchanged` without diffing.
- **Content-addressed throughout.** `CrateFingerprint.crate_id`,
  `TopologySnapshot.snapshot_id`, `ParseBackTopologyDelta.id`, and the final
  `ParseBackReport.id` are all SHA-256 content digests (INVARIANT-007).

## Key API

```rust
let executor = ParseBackExecutor::new(PathBuf::from("/path/to/workspace"));

// Capture a snapshot (read-only).
let pre = executor.snapshot(&ParseBackScanScope::FullWorkspace)?;

// Execute a parse-back rescan and diff.
let report = executor.execute(&plan, &pre, &PolicyProfile::dry_run(), evidence_bundle_id);
assert!(report.verify_id());

// Diff two snapshots directly.
let deltas = diff_snapshots(&pre, &post);
```

## Documentation

API reference: `cargo doc -p kosmo-parseback --open`

## License

MIT — see [`LICENSE`](../../LICENSE).
