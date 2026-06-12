# Steward — self-husbandry under an operator-named fence

Every door so far waits for an operator to speak. The steward is the door
through which the system **works on itself**: it surveys a workspace's own
wish landscape (typically this very repository), names the open chores
inside an explicit fence, and — only under `--apply` — husbands them, one
evidence-bound descent per chore. The governance is the house's, applied
to the system's own body: **the machine proposes, only the operator
disposes.**

## The fence

Nothing is fenced by default. The fence is a comma-separated list of
facet classes the steward may touch, and it exists only because the
operator spoke it:

```sh
cargo run -p kosmo-run -- --steward --fence doc,test .
```

- `--steward` without `--apply` is a **survey**: counts, the fenced plan,
  zero writes. This is the machine proposing.
- `--steward --apply` without `--fence` is **refused**. Widening the fence
  (say, to `capability`) is an explicit operator act, taken per run —
  there is no "all" shorthand, because an unbounded fence is not a fence.
- `doc,test` is the recommended fence: additive chores (a doc fiber, a
  smoke test) that the **deterministic scaffolder** can build offline —
  fenced husbandry needs no provider and no key. Classes the scaffolder
  cannot build fail honestly and appear in the report as unrealized.

## Husbandry

```sh
cargo run -p kosmo-run -- --steward --fence doc,test --apply \
    --norms .norms --steward-report steward.json .
```

Each fenced open chore becomes its own wish (evidence-bound to the
diagnosis report that proposed it) and descends through the same armament
as wish mode: deterministic scaffolds first, provider-gated LLM fallback
if one was armed, memory grounding under `--ledger`. Every descent is
recorded as a norm-learning observation — the system learns from the work
it does on itself. A failed chore is recorded and the round continues; the
exit code (4) still tells the truth at the end. `--steward-max <n>` caps
the chore list per run.

The report is content-addressed (`report_id` = SHA-256 of the body) and
**host-path-free**: the workspace appears as its identity digest, chores
as facet labels — fit for an unattended nightly artifact.

## The nightly loop (CI)

`.github/workflows/steward.yml` closes the loop on this repository:

1. **Nightly survey** (scheduled): the system surveys its own landscape
   read-only and publishes the steward report as a build artifact — the
   standing proposal.
2. **Husbandry** (manual `workflow_dispatch` with `husband: true`):
   dispatching the job **is** the operator's act. It husbands the
   `doc,test` fence and pushes the changes as a `steward/run-<n>` branch —
   never to `main`. Opening and merging the pull request remains a human
   decision: the system proposes and builds, the operator baptizes.

## What CI pins (offline, no key)

The survey's read-only contract and host-path-free report; the refusal of
fenceless husbandry and of words outside the facet vocabulary; offline
fenced husbandry (doc + test chores actually landing, observed by the
norm organ, one observation per chore); the cap; the door's exclusivity.
