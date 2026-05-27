# Support

How to get help with Kosmocrates.

## I have a question

Open a [GitHub Discussion](https://github.com/lashsesh/pse/discussions)
(if enabled) or a [GitHub issue](https://github.com/lashsesh/pse/issues)
tagged `question`. Include:

- What you are trying to accomplish.
- What you have tried.
- The relevant crate / CLI / binding.
- Versions: `cargo --version`, `rustc --version`, OS.

## I found a bug

Open a [bug-report issue](https://github.com/lashsesh/pse/issues/new?template=bug_report.md).
The template will prompt you for the information needed to reproduce
the bug.

## I want a feature

Open a [feature-request issue](https://github.com/lashsesh/pse/issues/new?template=feature_request.md).
Before opening, check `ROADMAP.md` — your idea may already be
scheduled or explicitly out of scope.

## I found a security vulnerability

Do **not** open a public issue. Follow `SECURITY.md` — open a private
report via GitHub's "Report a vulnerability" surface, or email the
maintainer at the address listed under `authors` in `Cargo.toml`.

## Response times

Kosmocrates is a single-maintainer project (see `GOVERNANCE.md`).
Best-effort response targets:

| Class | Target |
|---|---|
| Security report | 14 days to acknowledge |
| Bug with reproducer | 30 days to triage |
| Feature request | 30 days to respond, no implementation guarantee |
| Question | best-effort, no guarantee |

If you need stronger guarantees than that for a downstream use case,
the MIT license allows you to fork and maintain your own copy. For
commercial support arrangements, contact the maintainer directly.

## What is supported

| Surface | Support status |
|---|---|
| `crates/pse-core`, `pse-types`, `pse-graph`, `pse-evidence`, `pse-replay` | Core public API — best supported |
| `pse-server`, `pse` CLI, `nxalien` CLI, `pse-demo`, `pse-llm-demo` | End-user binaries — supported |
| `crates/pse-wasm` + npm packages | Supported |
| `bindings/python` (`pse-core` on PyPI) | Supported |
| Other `tools/*` binaries (research / dev) | Best-effort |
| `vendors/*` | Vendored; report upstream first |

## What is not supported

- Any version below the latest tagged release. Security fixes land
  on `main` and roll into the next tag (see `SECURITY.md`).
- Custom forks. The maintainer cannot triage issues from a fork
  unless they reproduce on `main`.
- Use cases that intentionally break the determinism / fail-closed
  contracts documented in `CONTRIBUTING.md`.
