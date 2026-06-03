# Horizon — the Behavior + Archetype floor

- **Status:** Design specification — *not yet implemented*. This document is
  the spec; the system follows it, not the other way round.
- **Date:** 2026-06-03
- **Scope:** The next structural layer on top of the wish-to-system loop.
  Covers facet-vocabulary levels **1–2 (Behavior)** and **4 (Archetype)**.
- **Author:** @LashSesh

> *Spec before system.* This is the Kosmocrates contract applied to
> Kosmocrates itself: a desired shape is written down and made measurable
> first; the code is then the descent that realizes it. Read this as the
> *wish* for the next floor.

---

## 0. Why this document exists

The wish loop converges today — but only for **structural** facets. A
realized wish means *named things exist*, not that they *do the right
thing*. `FacetScaffolder::scaffold_signature` emits `pub fn name(_a0: (), …)`;
`scaffold_test` emits `#[test] fn name() { /* scaffolded */ }`. Both satisfy
the facet (presence) and contain no behavior (meaning).

The gap between what the system is — a deterministic **skeleton printer** —
and the goal — a **software printer** ("3D-print full-stack, nothing to
rework") — is exactly the gap between *structure* and *behavior*. This floor
closes it along two complementary axes that form a pincer:

- **Archetype (level 4)** — *breadth*, top-down: one sentence fans out into a
  bundle of facets across a recognizable full-stack shape.
- **Behavior (levels 1–2)** — *depth*, bottom-up: each leaf of that bundle is
  realized only when its behavior is *observed correct*, not merely present.

---

## 1. The invariant this floor must preserve

The spine does **not** change. The descent loop in
`tools/kosmo-run/src/main.rs::descend_to_wish` already does:

```
observe → assess → scaffold (deterministic) | synthesize (LLM fallback) → re-observe
```

This floor adds only three things and touches nothing else load-bearing:

1. new **facet kinds** in `kosmo-core::WishFacetKind`,
2. an **expansion stage** in the wish compiler (`kosmo-intent`),
3. behavior facets that are **observed by running**, reusing the green-test
   observation already built in
   `kosmo-intent::observe_workspace_validated` (Run 14).

Every existing contract is preserved verbatim:

- **INVARIANT-007** — every new facet is content-addressed; identical wishes
  produce byte-identical assessments.
- **CROSS-006** — assessments stay evidence-bound.
- **CROSS-010** — distance still *ranks, never gates*. A behavior facet adds a
  predicate; it grants no capability and bypasses no policy.
- **Fail-closed** — a behavior facet is met *only* on positive observation
  (a green spec test). Unknown ⇒ unmet. A wish is never satisfied by absence
  of evidence.

---

## 2. Vocabulary extension

Three additions to `WishFacetKind`. The first two are *behavior* leaves; the
third is a *compiler construct* that expands into leaves.

| Facet | Key shape (sketch) | Observed by | Scaffolded as |
|---|---|---|---|
| `Contract` | `"handle(Request)->Response"` | real type capture from the signature | typed stub `pub fn handle(_: Request) -> Response { todo!() }` — compiles iff the types exist; panics at runtime. **Honestly empty.** |
| `Behavior` | `"handle#0"` → a content-addressed spec case | **running the spec test** (green?) | `#[test] fn handle_spec_0() { assert_eq!(handle(req_a), resp_a) }` — **red until the body is correct.** |
| `Archetype` | `"rest_endpoint(GET,/users,Vec<User>)"` | — (never a leaf; expanded pre-assessment) | expands into a facet bundle (see §4). |

`Contract` is the typed promotion of today's `Signature` (`"name/arity"`).
`Behavior` is the *enabled* promotion of today's `Test` (from "a test exists /
runs" to "a test that pins a concrete input→output pair").

### 2.1 The spec case (what a `Behavior` facet points at)

A behavior wish must *carry its examples* — see the honest seam in §5.2. The
spec case is content-addressed so the facet key is stable and replayable:

```rust
// Sketch — not final.
struct SpecCase {
    target: String,        // "handle"
    inputs: Vec<Value>,    // serialized argument literals
    expects: Expectation,  // Equals(Value) | Panics | Matches(pattern) | …
}
// WishFacet::behavior(case) keys on Digest::of(&case) → "handle#<short-digest>"
```

The generated spec test is a pure function of the `SpecCase`, so the same case
always produces the same test bytes (deterministic scaffold).

---

## 3. The keystone — acceptance over generation

This is the load-bearing decision of the whole floor. Name it; an ADR will
record it when we commit to building.

We **cannot** guarantee that an LLM *writes* correct code. We **can**
guarantee **when the loop is allowed to declare a wish realized.**

The `Behavior` facet turns the synthesizer's job from *"write something
sensible"* into *"make exactly this one red test green."* The `re-observe`
step (`cargo test`, via `observe_workspace_validated`) is the **deterministic
judge**. The descent does not terminate until the pinned behavior is
**observed green**.

> The guarantee moves from **generation** to **acceptance**. "Rework" is
> replaced by "do not terminate until validated." The system never lies — it
> reports `Realized` only when behavior is observed-correct.

This is the honest, defensible form of the "nothing to rework" promise. Not
*"the AI gets it right"* — rather *"the machine only accepts what is right."*

Consequences:

- The deterministic path (structure) stays **guaranteed**.
- The LLM path (content) becomes **narrow and checkable**: one failing
  assertion at a time, judged by a test, not by vibes.
- A wish whose behavior the LLM cannot achieve **does not falsely converge**;
  it stalls at `Approaching` with the red spec facet still unmet — fail-closed,
  visible in the `WishSession` trajectory.

---

## 4. Archetype expansion (level 4)

An archetype is a **named, parameterized template** that fans out into a bundle
of lower facets spanning the structural and behavioral levels. The archetype
library is the "all-inclusive" surface: `entity`, `repository`,
`rest_endpoint`, `cli_command`, `ui_component`, `migration`, …

```rust
// Sketch — not final. An archetype is a pure expansion function.
trait Archetype {
    fn expand(&self, params: &ArchetypeParams) -> BTreeSet<WishFacet>;
}
```

The wish compiler grows one stage **above** the existing rule/LLM compiler,
which still handles leaf structural facets:

```
prose → archetype recognition → params → expand() → facet bundle → Wish
```

### 4.1 Worked example

Prose: *"ein Endpunkt GET /users, der die Nutzer liefert."*

`rest_endpoint(GET, /users, Vec<User>)` expands to:

```
Crate        api
Module       api::routes::users
Contract     list_users() -> Vec<User>
Symbol       User                       (the entity type)
Dependency   api -> domain
Behavior     list_users#0: [u1,u2] => len == 2
( System     GET /users => 200          ← level 5, later )
```

The descent (unchanged loop) then runs:

```
iter 0:  nothing present                       → UNSTARTED   0/6
   scaffold (deterministic, GUARANTEED):
     crate, module, type stub, typed Contract stub with todo!(),
     spec test (red), dependency edge
iter 1:  structure present, Behavior UNMET     → APPROACHING  5/6
     (the spec test is red — todo!() panics)
   deterministic scaffold exhausted → LLM fallback gets ONE narrow task:
     "make list_users_spec_0 green"
iter 2:  re-observe WITH validated tests → test green?
     yes → Behavior MET                        → REALIZED ✓   6/6
```

`iter 0→1` is deterministic and guaranteed. `iter 1→2` depends on the LLM —
but the **acceptance** is deterministic. The keystone (§3) sits exactly here.

---

## 5. Honest seams (open questions, not hidden)

### 5.1 Type-aware observation needs more than lexical parsing
Source extraction in `kosmo-intent` reads lines today. `Contract` pushes toward
real type capture. **Mitigation:** start with *shallow* types (single token:
`User`, `Request`, `Vec<User>`); defer deep generics. A `Contract` over a type
the parser can't yet capture degrades to today's `Signature` (arity only) — no
regression, just less depth.

### 5.2 You cannot validate behavior you have not specified
Prose *"a function that adds"* pins no example. The wish must **carry** its
I/O cases. Two supply paths, both fail-closed:
- **human-supplied** examples (the operator writes the cases), or
- **LLM-proposed, human-ratified** examples (the synthesizer suggests cases;
  the human accepts — the acceptance is what binds them to the wish).

This is fundamental, not a flaw: a behavior facet *is* a specification, and a
specification someone has to author. The system's honesty is that it refuses to
claim correctness for behavior nobody pinned.

### 5.3 Level 5 (runtime/system facets) needs an execution sandbox
"The server starts; the endpoint returns 200" is observed by *running* the
built artifact, not by reading source. That is a larger lift (a sandboxed
exec/observe layer) and is **explicitly out of scope for this floor** — staged
after levels 1–2 and 4 land. Listed here so the horizon is complete.

---

## 6. Build order (the structural sequence)

Bottom-up — otherwise nothing bears load:

1. **`Contract` facet** — typed signatures.
   *Acceptance:* a prose wish for a typed function descends to a compiling
   typed stub; `observe` reports the `Contract` present iff the real signature
   matches. Deterministic, offline. Extends `Signature`.
2. **`Behavior` facet** — the keystone (§3).
   *Acceptance:* a wish carrying a spec case scaffolds a **red** spec test;
   the facet is met only after `observe_workspace_validated` sees it green.
   With no LLM the wish honestly stalls at `Approaching`; with a provider the
   loop drives the body until the test passes — or fails closed.
3. **`Archetype` expansion** — once leaves *mean* behavior, archetypes are
   cheap templates over them.
   *Acceptance:* the §4.1 example converges end-to-end from a single sentence.

Each step is a self-contained run: new facet kind in `kosmo-core`, observation
in `kosmo-intent`, deterministic scaffold in `kosmo-synthesizer`, wired through
`descend_to_wish`, with tests — exactly the rhythm of Runs 10–19.

---

## 7. What this floor does *not* change

- The descent loop, the `WishSession` trajectory, the `--wish-session`
  persistence — untouched.
- `assess_wish` — untouched (it already measures any facet set).
- The deterministic-first / LLM-fallback split in `apply_synthesis` —
  untouched; behavior facets simply give the fallback a *checkable* task.
- Content-addressing, fail-closed, ranks-not-gates — preserved by
  construction (§1).

The new floor rides the existing spine. That is the point: *same machine,
larger vocabulary, from "exists" to "does the right thing, in concert."*

---

## References

- Spine: `tools/kosmo-run/src/main.rs::descend_to_wish`, `::apply_synthesis`.
- Ruler & target: `crates/kosmo-core/src/wish.rs` (`WishFacetKind`,
  `assess_wish`, `WishAssessment`).
- Validated observation (green tests, Run 14):
  `crates/kosmo-intent/src/lib.rs::observe_workspace_validated`.
- Deterministic scaffolds: `crates/kosmo-synthesizer/src/lib.rs`
  (`FacetScaffolder`, `scaffold_signature`, `scaffold_test`).
- Compiler: `crates/kosmo-intent` (rule) + `crates/kosmo-intent-llm` (LLM).
- Contracts: INVARIANT-007 (content-addressing), CROSS-006 (evidence-bound),
  CROSS-010 (ranks-not-gates) — see `crates/kosmo-core/src/wish.rs` header.
