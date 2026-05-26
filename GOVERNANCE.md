# Project Governance

Kosmocrates is currently a **single-maintainer** open-source project.
This document describes how decisions are made today and the path to
a broader governance model.

## Roles

### Maintainer

Sebastian Klemm (`@LashSesh`) is the sole maintainer. They have final
say on:

- Public API changes and version cuts.
- Merging pull requests.
- Architectural decisions documented in `specs/` or `docs/`.
- Security advisories.

The maintainer's responsibilities:

- Triage incoming issues and PRs within a reasonable time window
  (best effort — see `SUPPORT.md`).
- Maintain the determinism, replay, and fail-closed contracts
  documented in `CONTRIBUTING.md`.
- Coordinate security disclosures per `SECURITY.md`.
- Cut releases following `RELEASING.md`.

### Contributors

Anyone who opens an issue, sends a pull request, or participates in
discussions. Contributors are subject to the `CODE_OF_CONDUCT.md`.
PRs from contributors are reviewed against the same criteria as the
maintainer's own changes.

## Decision-making

For the single-maintainer phase:

- **Bug fixes, documentation, refactors:** merged at maintainer
  discretion after review.
- **New features, public API changes:** require a `feat(...)` commit
  with a clear rationale; large additions should open a tracking
  issue first to validate scope.
- **Breaking changes:** allowed pre-1.0 (see `RELEASING.md` §1) but
  must be called out in `CHANGELOG.md` under a `### Breaking changes`
  section.
- **Architectural decisions:** documented either in `specs/` (formal
  specifications) or in the relevant crate's rustdoc / module-level
  comments. Significant cross-crate decisions get a short ADR in
  `docs/adr/` (this directory is created on first use).

## Path to multi-maintainer governance

The project transitions to a multi-maintainer model when **either**
of the following holds:

1. The maintainer formally invites another contributor to co-maintain
   and that person accepts in writing in a GitHub issue.
2. There is sustained activity from 3+ external contributors over a
   90-day window and at least one of them volunteers for sustained
   review duty.

At that point this document is revised to add:

- A defined review quorum for breaking changes.
- A formal RFC process (likely as `docs/rfc/`).
- A defined deprecation policy for public API.

Until then, treat this project as benevolent-dictator-for-now. If you
need governance guarantees stronger than that for a downstream use
case, fork the project — the MIT license allows it.

## Conflict resolution

If you disagree with a maintainer decision:

1. Comment on the relevant PR or issue with a concrete counter-proposal.
2. If still unresolved, open a new issue tagged `governance` summarising
   the disagreement and the requested decision.
3. The maintainer commits to responding within 14 days; if no response
   in that window, the dispute may be escalated by emailing the
   address listed in `Cargo.toml`.

## Funding

If the project becomes sustaining-fundable in the future, a transparent
funding policy will be added here (allocation, reporting cadence,
disclosure of conflicts of interest). For now: no funding is solicited
or accepted in a way that would create governance leverage.
