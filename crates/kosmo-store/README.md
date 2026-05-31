# kosmo-store

Persistent JSONL CorpusCartography store for KOSMO-OPS-01 RX

`kosmo-store` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Provides `JsonlCartographyStore`: an append-only, durable implementation of the
`CorpusCartographyStore` trait (defined in `kosmo-core`) that persists each
`CartographyStoreCommit` as one JSON line (JSONL) on disk.

Opening the store replays every line from an existing file into an in-memory
manifest, enforcing gapless sequence ordering and scope consistency. The manifest
is the source of truth for sequence and scope checks; `verify_integrity()` re-reads
the durable copy from disk and detects digest mismatches or sequence gaps.

## Why it is a separate crate

`kosmo-core` is intentionally free of filesystem and process I/O so it stays
portable (parts of the substrate compile to wasm). Disk persistence is a host
capability, so it lives here — the same isolation principle as `kosmo-foundry`
(process execution) and `kosmo-pse-bridge` (the PSE crossing).

## Safety / policy contract

Writing a commit to disk is a host write. This creates an emergent safety
property stricter than the in-memory store:

- **ReportOnly is always denied.** `ImplementationMode::ReportOnly` forbids all
  mutation; no line is ever written.
- **DryRun cannot persist.** `DryRun` keeps `allow_host_write == false`. A durable
  append requires `allow_host_write`, so DryRun is denied even though it is not
  ReportOnly. Only `OperatorApproved` (or a custom profile that explicitly sets
  `allow_host_write`) may append to disk.
- **No file is created on a denied append.** The backing file is only created when
  the first successful write occurs.
- **Fail-closed on write failure.** The in-memory manifest is updated only after a
  successful disk write. If the write fails, the manifest is left untouched.
- **Reads are always permitted.** Opening the store and calling `verify_integrity()`
  never mutate the host and require no special policy mode.
- **Sequence is gapless and monotonic.** Any commit with a non-consecutive sequence
  number is rejected with `CartographyStoreError::SequenceViolation`.
- **Scope is pinned at open time.** Committing a mismatched scope yields
  `CartographyStoreError::ScopeMismatch`.

## Key API

```rust
// Open (or create) a store.
let mut store = JsonlCartographyStore::open(
    "/path/to/corpus.jsonl",
    CorpusScope::LocalHostProject,
    policy_id,
)?;

// Append a commit — requires OperatorApproved policy.
store.append(commit, &PolicyProfile::operator_approved())?;

// Verify the durable copy on disk.
let report = store.verify_integrity(evidence_bundle_id)?;
assert!(report.status.is_intact());
```

## Documentation

API reference: `cargo doc -p kosmo-store --open`

## License

MIT — see [`LICENSE`](../../LICENSE).
