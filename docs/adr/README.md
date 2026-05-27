# Architecture Decision Records

This directory captures the major architectural decisions that shape
Kosmocrates. Each ADR records **what** was decided, **why**, and **what
we explicitly chose not to do** — so contributors picking up the
codebase years later understand the constraints, not just the result.

We follow the
[Michael Nygard ADR format](https://github.com/joelparkerhenderson/architecture-decision-record/blob/main/locales/en/templates/decision-record-template-by-michael-nygard/index.md),
lightly adapted.

## Index

| # | Title | Status |
|---|---|---|
| [0001](0001-fail-closed-crystallization.md) | Fail-closed crystallization as the default contract | Accepted |
| [0002](0002-deterministic-replay.md) | Deterministic replay as a non-negotiable invariant | Accepted |
| [0003](0003-wasm-over-napi-for-node.md) | WASM (not native N-API) as the supported Node binding | Accepted |

## Adding a new ADR

1. Copy [`0000-template.md`](0000-template.md) to the next number.
2. Replace the placeholders.
3. Start in `Status: Proposed`. Open a PR; flip to `Accepted` on merge
   (or `Rejected` if the proposal is killed in review).
4. Add a row to the table above.
5. Never edit an `Accepted` ADR in place — if the decision changes,
   write a new ADR that **supersedes** the old one and update the
   old one's status to `Superseded by NNNN`.

## When does a change deserve an ADR?

Write one when the answer to "why did we do it this way?" would not
be obvious from reading the code six months later. Concretely:

- A decision that constrains future contributors (a contract, a
  default, an "out of scope" boundary).
- A trade-off where the rejected alternative is a reasonable choice
  someone might later propose without context.
- A cross-crate or workspace-wide convention.

Skip an ADR for:

- Implementation details that one PR description already covers.
- Bug fixes — the commit message is enough.
- Internal refactors that change no public surface.

## References

- [Michael Nygard's original article (2011)](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
- [Joel Parker Henderson's ADR collection](https://github.com/joelparkerhenderson/architecture-decision-record)
