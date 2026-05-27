---
name: Bug report
about: A reproducible defect in shipped behaviour
title: 'bug: '
labels: bug
---

<!--
Before opening: please verify the bug reproduces on the latest tagged
release or on `main`. We do not patch older tags (see SECURITY.md
and SUPPORT.md).
-->

## What happened

<!-- 1-3 sentences. Observed behaviour, not interpretation. -->

## What I expected

<!-- 1-3 sentences. -->

## Reproducer

<!--
The most important section. Smallest possible script / command /
ProblemSpec that triggers the bug. If the bug only reproduces on
specific inputs, attach a minimal input file.
-->

```bash
# commands that trigger the bug
```

## Output

<!--
Full output of the failing command. Include the panic message and a
backtrace (`RUST_BACKTRACE=1`). For replay / determinism bugs, attach
both `TraversalRunReport` JSON blobs and the bit-level diff.
-->

```
<paste output here>
```

## Environment

- Kosmocrates version (tag or `main` SHA):
- `cargo --version`:
- `rustc --version`:
- OS + version:
- Architecture (`uname -m`):
- Relevant feature flags / env vars (`PSE_*`, `RUST_LOG`, …):

## Determinism / replay impact

<!--
Tick if relevant. Replay / content-addressing bugs are the highest-
priority class of defect.
-->

- [ ] This bug breaks deterministic replay
- [ ] This bug changes a content hash
- [ ] This bug bypasses a fail-closed gate
- [ ] None of the above
