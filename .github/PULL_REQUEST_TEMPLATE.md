<!--
Thanks for sending a pull request. Please read CONTRIBUTING.md once
if you have not — it covers the determinism / fail-closed contracts
that every change is reviewed against.
-->

## Summary

<!-- 1-3 sentences: what does this PR change and why. -->

## Type of change

<!-- Tick all that apply. -->

- [ ] Bug fix (no public API change)
- [ ] New feature (no public API break)
- [ ] Breaking change (public API or content-addressed type)
- [ ] Documentation / examples only
- [ ] CI / tooling / build only
- [ ] Performance (no behaviour change)
- [ ] Refactor (no behaviour change)

## Determinism / replay impact

<!--
Required if you touched anything that contributes to a content hash:
crystal construction, gate/falsifier path, capsule sealing, evidence
chains, JCS canonicalisation. Otherwise write "n/a".
-->

- [ ] `tests/replay_byte_identity.rs` still passes locally
- [ ] If a content-addressed type changed, golden artifacts regenerated
      and the diff is explained below

## Checklist

- [ ] `cargo fmt --all -- --check` is clean
- [ ] `cargo clippy --workspace --all-targets --locked` is clean
- [ ] `cargo test --workspace --locked` passes (including doc tests)
- [ ] `cargo doc --workspace --no-deps` builds without warnings
- [ ] `CHANGELOG.md` updated if user-visible behaviour changed
- [ ] New public API has rustdoc with at least one `# Examples` section

## Related issues / specs

<!-- e.g. "Closes #123", "Implements specs/QTIC.pdf strand H" -->
