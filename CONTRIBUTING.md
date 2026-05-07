# Contributing to PSE

Thanks for your interest. PSE is a single-author research project that
has reached a state where outside eyes are useful. Contributions are
welcome — read this once before sending a patch.

---

## Ground rules

PSE is **deterministic by construction**. Any change that breaks
deterministic replay, content addressing, or the fail-closed contract
is rejected, no matter how clean the code looks. Specifically:

* Crystals are byte-identical across replays of the same input.
  Two runs of `cargo run --release -p pse-traverse-cli -- run` on the
  same `ProblemSpec` MUST produce identical `TraversalRunReport`
  bytes (after JCS canonicalisation). The test
  `tests/replay_byte_identity.rs` enforces this.
* No wall-clock time, no `HashMap` iteration, no unseeded RNG in any
  code path that contributes to a hash.
* `BTreeMap` over `HashMap` for keyed collections that are hashed.
  `HashMap` is fine for transient internals that never escape the
  function.
* Any new floating-point quantity that ends up in a content address
  goes through `CanonicalNumber::quantize_default` (scale-9 banker's
  rounding) first.

If a change relaxes these properties, the PR description must
explicitly call that out and explain why.

---

## Development loop

```bash
# Build everything (debug)
cargo build --workspace --all-targets

# Run the full test suite (~695 tests as of v0.1.0)
cargo test --workspace

# Format check (CI enforces this)
cargo fmt --all -- --check

# Lints (CI enforces default-level clippy)
cargo clippy --workspace --all-targets

# Doc build (CI enforces this with -D warnings)
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# 30-second engine smoke test
cargo run --release -p pse-demo
```

CI runs `RUSTFLAGS="-D warnings"` for build, test, and clippy. New
warnings break the build by design.

---

## Commit style

Conventional-commits prefixes are already in use throughout the
history; please follow them:

```
feat(pse-core): …
fix(pse-traverse): …
chore(deps): …
docs(README): …
test(pse-evidence): …
refactor(pse-graph): …
perf(pse-cascade): …
```

Keep the subject ≤ 72 chars. Body wraps at 72. Reference the spec PDF
or strand letter (E–P) when relevant — that's how the existing log
threads design and code together.

---

## Pull-request checklist

Before opening a PR:

- [ ] `cargo fmt --all -- --check` is clean
- [ ] `cargo clippy --workspace --all-targets` is clean
- [ ] `cargo test --workspace` passes (including doc tests)
- [ ] `cargo doc --workspace --no-deps` builds without warnings
- [ ] If you changed a content-addressed type, regenerate any frozen
      golden artifacts and explain the diff in the PR description
- [ ] If you added a new public API, it has rustdoc with at least one
      `# Examples` section
- [ ] If you touched the gate / falsifier / commit path, the
      `replay_byte_identity` test still passes

---

## Adding a new domain adapter

The trait surface is intentionally tiny:

```rust
pub trait ObservationAdapter: Send + Sync {
    fn parse(&self, raw: &[u8]) -> Result<Observation, String>;
    fn source(&self) -> &str;
}
```

To ship a new adapter:

1. Add a crate under `adapters/pse-adapter-<name>` (mirror an existing
   one — `pse-adapter-airquality` is a clean reference).
2. Embed a small offline dataset (≤ 100 KiB) so the adapter is testable
   without network access. Wire it through a feature flag if the live
   fetch path needs `tokio` / `reqwest`.
3. Add the crate to the root `Cargo.toml` workspace `members`.
4. Add an integration test that runs the adapter end-to-end through
   `pse_core::macro_step`.

---

## Reporting bugs

Open a GitHub issue with:

* The exact command that triggered the failure.
* `cargo --version`, `rustc --version`, OS, and architecture.
* The full panic message and backtrace (`RUST_BACKTRACE=1`).
* Ideally a minimal reproducer — a single failing test in a fork is
  the fastest way to get a fix.

For determinism / replay regressions: please attach the input
`ProblemSpec` (or stream snippet) and both `TraversalRunReport`
JSON bytes. Bit-level diffs in those reports are the highest-priority
class of bug in this codebase.

---

## License

All contributions are licensed under MIT, the same license as the
project. By submitting a patch you agree to that licensing.
