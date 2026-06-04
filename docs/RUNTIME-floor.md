# Runtime — the execution floor (level 5)

- **Status:** Design specification — *not yet implemented*. This document is
  the spec; the system follows it, not the other way round.
- **Date:** 2026-06-04
- **Scope:** The next floor on top of the wish-to-system loop. Covers facet
  level **5 (runtime/system)** — the axis explicitly deferred by
  `docs/HORIZON-behavior-archetype.md` §5.3 — plus its load-bearing infra, the
  execution sandbox. Folds in **behavioural composition** as the on-ramp.
- **Author:** @LashSesh

> *Spec before system.* The Kosmocrates contract applied to Kosmocrates: the
> desired shape is written down and made measurable first; the code is then the
> descent that realizes it. Read this as the *wish* for the next floor.

---

## 0. Why this document exists

The Horizon floor closed the gap between *structure* and *unit behaviour*. A
realized wish now means named things exist **and** pinned input→output pairs are
observed green — the keystone, *acceptance over generation*, judged by
`cargo test` through `observe_workspace_validated`.

But `cargo test` proves a function in isolation. It does **not** prove the
**assembled artifact runs**. A binary can have every unit test green and still
`panic!` on startup, print nothing, exit non-zero, or never bind its port. The
current judge reads source and runs *tests*; it never runs the *thing it built*.

That is the gap this floor closes. The unit of truth grows once more:

```
level 1–2  Behavior   — a function does the right thing            (cargo test)
level 3    Composition — the components' types wire together        (derived)
level 4    Archetype  — one sentence fans out into a full-stack bundle
level 5    Runtime    — the BUILT ARTIFACT, when RUN, behaves       (cargo run + probe)  ← this floor
```

The promise — *"3D-print full-stack, nothing to rework"* — is only honest once
the printer can **run its own output and watch it work.** Level 5 makes the
deterministic judge grow from *"the test is green"* to *"the program, executed,
exhibits the behaviour."*

---

## 1. The invariant this floor must preserve

The spine does **not** change. `descend_to_wish` still does:

```
observe → assess → scaffold (deterministic) | synthesize (LLM fallback) → re-observe
```

This floor adds only:

1. new **facet kinds** in `kosmo-core::WishFacetKind` (`Run`, later `Service`),
2. a new **observation mode** in `kosmo-intent` that *executes* the artifact
   (`observe_workspace_runtime`), parallel to `observe_workspace_validated`,
3. a new crate, **`kosmo-sandbox`**, that runs untrusted built code under
   resource, time, and network limits with guaranteed teardown (§4).

Every existing contract is preserved verbatim:

- **INVARIANT-007** — every new facet is content-addressed; identical wishes
  produce byte-identical assessments. A runtime facet keys on its *probe spec*,
  not on the (possibly nondeterministic) observed output.
- **CROSS-006** — assessments stay evidence-bound. A runtime observation is
  evidence: the captured exit code / output digest is the witness.
- **CROSS-010** — distance still *ranks, never gates*. A runtime facet adds a
  predicate; it grants no capability and bypasses no policy.
- **Fail-closed** — a runtime facet is met **only** on positive observation
  (the artifact ran and the probe matched). Build failure, panic, timeout,
  crash, wrong output, or an unavailable sandbox ⇒ **unmet**. A wish is never
  satisfied by absence of evidence, and *never* by an execution that did not
  happen.

---

## 2. Vocabulary extension

Two additions to `WishFacetKind`. `Run` is the load-bearing first beam (no
network, no ports); `Service` is staged after it (§7).

| Facet | Key shape (sketch) | Observed by | Scaffolded as |
|---|---|---|---|
| `Run` | `"run(--help)=>exit:0"` · `"run(add 2 3)=>out~5"` | **executing** `cargo run -- <args>` in the sandbox and matching the expectation | a bin target (`src/main.rs` with `fn main`) if absent — **honestly empty** (`todo!()`/no-op); the *behaviour* is red until the body is correct |
| `Service` | `"GET /health=>200"` · `"GET /users=>json.len>=1"` | **starting** the bin, awaiting readiness, issuing one request, matching status/body | a bin target + a route stub — red until the handler is correct |

### 2.1 The probe spec (what a runtime facet points at)

A runtime facet *carries its probe* — the same honest seam as Behavior (you
cannot validate what you did not specify; HORIZON §5.2). The probe is
content-addressed so the key is stable and replayable:

```rust
// Sketch — not final.
struct RunProbe {
    args: Vec<String>,        // ["add", "2", "3"]
    expect: RunExpect,        // Exit(0) | StdoutContains(String) | both
    stdin: Option<String>,    // optional fed input
}
// WishFacet::run(probe) keys on the probe's normalized text → "run(add 2 3)=>out~5"
```

The expectation grammar is deliberately small and total:

```
expect := "exit:" N
        | "out~" SUBSTR          // stdout contains SUBSTR
        | "exit:" N "," "out~" SUBSTR
```

`out~` (substring) over regex on purpose: substring match is deterministic,
locale-stable, and has no catastrophic-backtracking surface. Richer matchers are
a later, opt-in extension — start total and honest.

---

## 3. The keystone, extended — from *test-green* to *run-correct*

The Horizon keystone said: the loop declares `Realized` only when behaviour is
**observed green** by `cargo test`. This floor extends the *same* principle one
rung up the reality ladder:

> The judge grows from **running the test** to **running the program**. The
> guarantee stays on **acceptance**, never on generation. The loop reports a
> runtime facet `Realized` only when the built artifact, executed under the
> sandbox, is *observed* to exhibit the probed behaviour.

Nothing about the honesty changes; only the *fidelity of the observation* rises:

- **structure** (does it exist) — guaranteed by the deterministic scaffolder.
- **unit behaviour** (does the function compute) — judged by `cargo test`.
- **runtime behaviour** (does the artifact, run, work) — judged by `cargo run` +
  probe in the sandbox.

A wish whose runtime behaviour the implementation cannot achieve **does not
falsely converge**. It stalls at `Approaching` with the red `Run` facet still
unmet — visible in the `WishSession` trajectory, exactly as a red `Behavior`
does today. The capstone's two-direction proof (correct → realized, wrong →
rejected) is re-run at this higher fidelity: a binary that prints the wrong
answer, panics, or times out is **rejected**, not accepted.

---

## 4. The execution sandbox (`kosmo-sandbox`) — the load-bearing infra

HORIZON §5.3 flagged this as the larger lift. It is the heart of this floor:
**we are executing code the loop itself may have generated.** That is a real
trust boundary, and the sandbox is where the project's safety posture lives.

A `Run`/`Service` observation must spawn a child process and *guarantee*:

1. **Timeout + kill** — a hard wall-clock budget; on expiry the process tree is
   killed (kill the group, not just the leader). Timeout ⇒ facet **unmet**,
   never a hang.
2. **Output capture** — stdout/stderr captured to bounded buffers (truncate past
   a cap; a runaway printer must not exhaust memory).
3. **Network deny by default** — the spawned artifact gets no outbound network
   unless the *environment policy* (see the web-execution docs) explicitly
   grants it. Probes that need a loopback socket (`Service`) bind localhost only.
4. **Filesystem jail** — execution happens in the throwaway workspace; no writes
   escape it. Teardown removes it regardless of outcome (the Prüfstand harness
   already models throwaway workspaces — reuse that discipline).
5. **Resource caps** — best-effort memory/CPU limits where the platform offers
   them; documented as best-effort where it does not.
6. **Determinism aids** — a fixed `KOSMO_SEED`, frozen clock hooks where
   feasible, and readiness-by-**polling** (never `sleep`) for `Service`.

The sandbox is a *capability*, not a *gate*: it does not decide what is allowed
(policy does that, CROSS-010); it provides the safe room in which an observation
can happen and be trusted. Its output is **evidence** — a `RuntimeWitness`
{ exit, stdout-digest, duration, verdict } that binds the facet (CROSS-006).

---

## 5. Worked example

Prose: *"a CLI `calc` that adds two numbers: `calc add 2 3` prints 5."*

The wish compiler (rule + LLM) produces a bundle spanning the floors built so
far plus the new runtime leaf:

```
Crate     calc
Contract  add(i32,i32)->i32
Behavior  add(2,3)=>5                 (level 2 — unit)
Run       run(add 2 3)=>out~5         (level 5 — the artifact, executed)  ← new
```

The descent (unchanged loop) runs:

```
iter 0:  nothing present                              → UNSTARTED   0/4
   scaffold (deterministic, GUARANTEED):
     crate, typed Contract stub (todo!()), red spec test,
     a bin target whose main parses argv and calls add (stub)
iter 1:  structure present; Behavior + Run UNMET      → APPROACHING  2/4
     (spec test red — todo!() panics; `cargo run` exits non-zero)
   deterministic scaffold exhausted → LLM fallback gets TWO narrow,
     checkable tasks: "make add_spec green" and "make run(add 2 3) print 5"
iter 2:  re-observe:
     observe_workspace_validated → add_spec green      → Behavior MET
     observe_workspace_runtime   → `cargo run -- add 2 3` stdout ~ "5"
                                                       → Run MET
                                                      → REALIZED ✓   4/4
```

`iter 0→1` is deterministic and guaranteed. `iter 1→2` depends on the
implementation (human or LLM), but **acceptance is deterministic**: the binary
must actually print `5` when run. The keystone (§3) sits exactly here.

Note the layering pays off: `Behavior add(2,3)=>5` and `Run run(add 2 3)=>out~5`
are *different observations of the same truth* — one at the function boundary,
one at the process boundary. A system can pass the first and fail the second
(e.g. the argv wiring is wrong); the floor catches precisely that class of bug,
which is invisible to every floor below it.

---

## 6. The on-ramp — behavioural composition (level 3½)

Between today (level 3 composition is *type-level*: `parse>>String>>eval` checks
`ret==param0`) and full runtime lies one cheap, sandbox-free rung worth taking
first, because it needs no new infra:

- **`Composition` with a behavioural expectation** — `"parse>>eval(\"2+3\")=>5"`:
  scaffold a spec test over the *composed* call `assert_eq!(eval(parse("2+3")), 5)`
  and observe it green via the **existing** `cargo test` judge.

This validates that data actually *flows* through the wire, not merely that the
types align — the level-2 keystone applied to the level-3 wire. It is the
natural first beam of the build order because it reuses everything and de-risks
the jump to process execution.

---

## 7. Build order (the structural sequence)

Bottom-up — otherwise nothing bears load:

1. **Behavioural composition** (§6) — validated data-flow over the existing
   `cargo test` judge. No new infra. De-risks the rest.
   *Acceptance:* a `parse>>eval(x)=>y` wish scaffolds a composed spec test and is
   met only when it is observed green.
2. **`kosmo-sandbox`** (§4) — spawn / timeout-kill / capture / jail / teardown,
   network-deny by default, emitting a `RuntimeWitness`. Pure infra; unit-tested
   against trivial programs (`true`, `exit 7`, an infinite loop that must be
   killed, a runaway printer that must be truncated).
   *Acceptance:* each guarantee in §4 has a test; a hang is killed within budget.
3. **`Run` facet** (§2) — observed by `observe_workspace_runtime` (= build, then
   `cargo run -- <args>` in the sandbox, match the probe).
   *Acceptance:* a prose wish for a CLI that prints a value descends to a running
   binary; `observe` reports `Run` met iff the executed output matches; a wrong
   binary is **rejected** (stalls at `Approaching`).
4. **Runtime capstone + Prüfstand extension** — the §5 example end-to-end
   through the real CLI; extend the reference corpus (`tools/kosmo-run/src/
   pruefstand.rs`) with run-scenarios in both directions (correct → realized,
   wrong/panicking → rejected), gated behind sandbox availability like the
   behavioural scenarios are gated behind cargo.
5. **`Service` facet** (§2) — start the artifact, await readiness by polling,
   probe a loopback endpoint. Builds on the sandbox; adds process lifecycle and
   port management. The largest lift; deliberately last.

Each step is a self-contained run in the rhythm of the Horizon beams: new facet
kind in `kosmo-core`, observation in `kosmo-intent`, deterministic scaffold in
`kosmo-synthesizer`, wired through `descend_to_wish`, with tests and a live
verification, committed and pushed green before the next.

---

## 8. Honest seams (open questions, not hidden)

### 8.1 Executing generated code is a trust boundary
This is the defining risk of the floor, not a footnote. The sandbox (§4) is the
mitigation, and it is **mandatory infra, sequenced before** the `Run` facet that
needs it. Network-deny-by-default and guaranteed teardown are non-negotiable.
Where the host platform cannot enforce a limit, the sandbox documents it as
best-effort rather than pretending — and the Prüfstand *skips* (never falsely
passes) a runtime scenario it cannot safely observe.

### 8.2 Execution is nondeterministic; the facet must not be
Wall-clock, scheduling, ports, randomness. The *key* is the probe spec (stable);
the *observation* is evidence (may vary). Mitigations: substring (not exact)
output matching, a fixed `KOSMO_SEED`, readiness by polling not sleeping, and —
where a probe is inherently flaky — an N-of-N consistency requirement before the
facet is declared met. Fail-closed on any inconsistency.

### 8.3 You still cannot validate behaviour you did not specify
A `Run` facet *is* a specification (HORIZON §5.2 holds at this level too). The
probe args and expectation are authored — human-supplied or LLM-proposed and
human-ratified. The system's honesty is that it refuses to claim a runtime
property nobody pinned.

### 8.4 `cargo run` cost
Building then running is heavier than `cargo test`. Mitigation: runtime
observation is **opt-in** (only when the wish carries a `Run`/`Service` facet),
exactly as `--validated` gates the test run today; reuse the build artifacts
from the validated pass where possible.

---

## 9. Adjacent floor, explicitly out of scope — System Invariants

A different axis is worth naming so the horizon stays complete, and so it is not
confused with this floor: **invariants** — a facet that is a *predicate over the
whole observed topology* rather than a single item ("every `Contract` has a
`Behavior`"; "every public `Symbol` has a `Contract`"; "no `panic!` in a lib
crate"). That is a **governance/integrity** floor — building *laws the system
upholds*, not *behaviour the artifact exhibits*. It is orthogonal to runtime and
**out of scope here**; listed so the map is honest.

---

## 10. What this floor does *not* change

- The descent loop, the `WishSession` trajectory, `--wish-session` persistence —
  untouched.
- `assess_wish` — untouched (it already measures any facet set).
- The deterministic-first / LLM-fallback split in `apply_synthesis` —
  untouched; runtime facets simply give the fallback a *checkable* task at a
  higher fidelity.
- The structural and unit-behaviour judges — untouched; runtime observation is a
  new, opt-in mode beside them, not a replacement.
- Content-addressing, fail-closed, ranks-not-gates — preserved by construction
  (§1).

The new floor rides the existing spine. That is the point: *same machine, one
rung higher on the reality ladder — from "the test is green" to "the program,
run, does the right thing."*

---

## References

- Spine: `tools/kosmo-run/src/main.rs::descend_to_wish`, `::apply_synthesis`.
- Ruler & target: `crates/kosmo-core/src/wish.rs` (`WishFacetKind`,
  `assess_wish`, `WishAssessment`).
- Validated observation (green tests): `crates/kosmo-intent/src/lib.rs::
  observe_workspace_validated` — the model `observe_workspace_runtime` follows.
- Deterministic scaffolds: `crates/kosmo-synthesizer/src/lib.rs`
  (`FacetScaffolder`); the bin-target scaffold extends these.
- Throwaway-workspace discipline to reuse for the sandbox & corpus:
  `tools/kosmo-run/src/pruefstand.rs`, `tools/kosmo-run/tests/capstone.rs`.
- Prior floor & deferred axis: `docs/HORIZON-behavior-archetype.md` (§5.3 names
  this floor; §4.1 sketches `System GET /users => 200`).
- Web-execution environment & network policy (sandbox posture):
  https://code.claude.com/docs/en/claude-code-on-the-web
- Contracts: INVARIANT-007 (content-addressing), CROSS-006 (evidence-bound),
  CROSS-010 (ranks-not-gates) — see `crates/kosmo-core/src/wish.rs` header.
