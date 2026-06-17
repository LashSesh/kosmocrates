# Horizon — Stufe 2: the architecture spec (the city plan)

The map promised before we climb the long stair. Grounded in the code as it
stands after Run 44 — not a wish-list of features, a survey of what is already
load-bearing and what is genuinely missing.

## The telos (restated honestly)

Not a *forced unique output* — a fully-resolved plan *is* the software. The
target is **deterministic acceptance + guaranteed convergence**: state intent at
a chosen abstraction level; the machine holds an honest, complete-for-that-level
criterion and drives any capable engine to satisfy it or says precisely what it
could not reach and why. The operator's supermarket: not one path, but the
guarantee you come home with everything.

Stufe 1 (`--insist`, Run 43) closed the *acceptance* half at facet granularity.
Stufe 2 lifts the *spec* from a flat facet list to a **structured architecture** —
the whole city planned before a brick is laid.

## The finding: the architecture primitives already exist (latent)

Probing `kosmo-core` first, as the discipline demands, overturned the assumption
that this is greenfield:

- **Hierarchy / containment** is already an edge. `precedence::depends_on` reads
  path containment: `foo` ◁ `foo::bar` ◁ `foo::bar::baz`. A symbol *stands on*
  its module *stands on* its crate — the membership graph is in the keys.
- **Inter-component edges** are already facet kinds (`wish::WishFacetKind`):
  - `Dependency` — `"from->to"`: A *can see* B (a directed crate edge).
  - `Composition` — `"from>>via>>to"`: A's output *fits* B's input (typed flow).
  - `Contract` — `"name(T..)->R"`: a typed interface.
  `depends_on` already tokenizes these and recognises their endpoints.
- **Foundations-first ordering** already exists: `precedence::PrecedenceOrder::focus`
  ranks a facet set by leverage (how many others depend on it) over that implicit
  graph — the crate before its modules, both endpoints before an edge.
- **Per-component assessment** exists: `assess_wish` / `assess_wish_layered`.
- **Multi-component specs** have a seed: `--wishlist` (a flat file of wishes
  assessed against one observation).

So the architecture was never absent — it was **implicit in the facet keys and
the precedence lens, never materialised as a graph** you can see, write, or check
as a whole.

## The gap (what Stufe 2 must add)

1. **An explicit architecture object** — facets organised as a *graph* (nodes +
   typed edges), not a flat `BTreeSet` whose structure is only implicit in key
   strings. *(Run 44 took the first step: `blueprint_nodes` materialises it for a
   single wish and renders it foundations-first — see `--blueprint`.)*
2. **A spec format to *write* a city plan** — components, their internal facets,
   the edges between them. The structured successor to `--wishlist`: not a flat
   list, but components with declared dependencies/compositions.
3. **Holistic architecture assessment** — the plan "stands" only when every
   component is realised *and every declared edge holds* (the edge facets are
   already observable). Plus Run 29-style coverage: does the plan even *specify*
   its load-bearing connections?
4. **Cross-component invariants** — consistency, data invariants, "this property
   holds across modules". Genuinely absent. This is the bridge into **Stufe 3**
   (system-level verification).

## The safe additive path

The discipline (Runs 24–28, 33): probe for a safe door before touching core
grammar. There is one. Just as `--wishlist` is a flat-multi-wish door, the
architecture spec can be a **structured-multi-wish door**, assessed by:

- composing the existing per-component `assess_wish`, and
- checking the declared edges (`Dependency`/`Composition`) against the observed
  topology — they are *already observable* — and
- ordering with the existing `PrecedenceOrder`.

**No core grammar change**: it reuses the facet vocabulary (which already carries
edges) and the precedence lens. Additive, deterministic, offline-testable, in
`kosmo-run` — the same shape as `--alchemy`, `--behaviour`, `--insist`.

## The brick sequence (each its own run, honest scope)

| Brick | What | Risk | Provable offline |
|---|---|---|---|
| **44 — blueprint view** ✅ | materialise & render a wish's latent architecture graph, foundations-first (`--blueprint`) | additive | yes (done) |
| **45 — the spec format** ✅ | the whole DoD file read as ONE architecture (`--wishlist --blueprint`): components + edges, the city plan you *write* | additive | yes (done) |
| **46 — edge-as-realization** ✅ | the plan stands only when every declared edge *holds* — **absorbed into 45**: `assess_wish` already checks `Dependency`/`Composition` against the observed topology, so a `from->to` line is met only when the edge is observed | additive | yes (done) |
| **47 — coverage & honesty** ✅ | does the plan specify its connections (≥2 crates, 0 edges = a heap)? grade the *architecture* genuine/suspect when it stands (`blueprint_assessment_lines`, render-only) | additive | yes (done) |
| **48 — the pse-traverse bridge** ✅ | `traverse_bridge.rs` (`--plan`): architecture → `ProblemSpec` → `FieldCube`/`DoFGraph`/`CollapsePlan` + `PathExcision`. "The plan stands or not" → "the deterministic, foundations-first plan to *make* it stand (`scan`→`wire`→`resolve`→`verify`→`commit`), and what is operationally unreachable (excised)". Consumed via `default-features=false` — the lean core, no fork. | adapter | yes (done) |
| **49+ — invariants → Stufe 3** ◀ next | cross-component properties (consistency, data, resource) — system-level verification. `ConstraintSpec` already spans multiple dimensions; the harder, partly-research climb. | core / research | partly |

## The pre-kosmocrates substrate (surveyed Run 46)

Before brick 48, a survey of the original PSE/traverse theory found that `pse-traverse`
is, at its core, **already a conformance machine for abstract problem spaces** —
deterministic, fail-closed, content-addressed:

- **`pse-traverse` core** — `FieldCube` (dimensions + constraints + couplings) → `DoFGraph`
  (degree-of-freedom graph) → `CollapsePlan` (deterministic collapse to zero DoF) →
  `GateEngine` (fail-closed) + `PathExcision` (formal-but-unreachable). Symbolic-discrete,
  196 tests. **Structurally isomorphic** to our architecture-spec: component→Dimension,
  edge→Coupling, invariant→Constraint, realize→CollapsePlan. **This is brick 48's target.**
- **TPC / TPT / MTL / 720° Horizon** (`pse-traverse` `cognition/`,`topology/`,`horizon/`) and
  **Hivemind** (`phase-matrix`) — mature, deterministic, but **geometric-continuous**: they
  need a *meaningful embedding* of software into a 5D phase space / resonance field, which is
  the hard, unproven part (and TPT would recompute, obscurely, the cycles/components our
  dependency graph already gives). **Concepts to harvest later** (counterfactual horizon →
  richer honesty; Hivemind → multi-candidate convergence once a provider is armed), **code to
  park** for now.

The unifying fundamental concept across all of them — *collapse under constraints from many
degrees of freedom to a determined, gated, evidence-bound state, the whole landscape (incl.
the rejected) in view* — **is** the conformance machine, one altitude up. Brick 48 specialises
the discrete core to the software-architecture problem space.

Bricks 44–47 are the safe, additive, offline-provable climb that turns the flat
facet list into a checkable city plan. Brick 48+ (invariants) is where Stufe 2
meets Stufe 3 and the genuinely hard, partly-research work begins — designed when
we reach it, not bulldozed now.

## What this does *not* claim

The engine's reach still bounds the result (a spec the LLM cannot fill converges
to *honestly unmet*, not magically solved). Coverage is the ceiling: an
architecture the plan never specifies, the machine cannot insist on. Stufe 2
makes the *plan* whole and checkable; making the *generator* fill a whole-system
plan reliably is the convergence work that rides on top (and on the engine).
