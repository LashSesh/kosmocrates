# nxalien rules — 
# replay sha256:f65348987203621cd009709d8b4113188fdf6b8b37521956f6e0c4f842e9aaab

[Required] rust-fmt: Run `cargo fmt --all` before any commit. Code must be rustfmt-clean.
[Required] rust-clippy: Run `cargo clippy --workspace --all-targets --locked -- -D warnings`. No warnings permitted.
[Blocking] rust-test: All workspace tests must pass before claiming completion. Run `cargo test --workspace --locked`.
[Blocking] no-direct-crystal: nxalien crates MUST NOT directly construct SemanticCrystal. Emit NxAlienHandoffCandidate only. PSE-Bridge is the sole commit path.
[Required] minimal-reversible: Prefer minimal reversible changes. Do not delete without explicit instruction. Do not introduce abstractions beyond what the task requires.
