# nxalien rules — 
# replay sha256:92d6515ce9879238d069ad296181f7a596de408c249f0b796bb2f031bc001913

[Required] rust-fmt: Run `cargo fmt --all` before any commit. Code must be rustfmt-clean.
[Required] rust-clippy: Run `cargo clippy --workspace --all-targets --locked -- -D warnings`. No warnings permitted.
[Blocking] rust-test: All workspace tests must pass before claiming completion. Run `cargo test --workspace --locked`.
[Blocking] no-direct-crystal: nxalien crates MUST NOT directly construct SemanticCrystal. Emit NxAlienHandoffCandidate only. PSE-Bridge is the sole commit path.
[Required] minimal-reversible: Prefer minimal reversible changes. Do not delete without explicit instruction. Do not introduce abstractions beyond what the task requires.
