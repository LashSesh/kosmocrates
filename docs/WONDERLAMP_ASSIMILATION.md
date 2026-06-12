# Wonderlamp Assimilation — Port Map

- **Status:** Living document — grows one section per assimilation phase.
- **Source:** `wonderlamp.zip` (ISLS — "Intelligent Semantic Ledger Substrate",
  ~56K LOC, 11 crates, same author). The abandoned predecessor of this
  system: it lacked the epistemic substrate (content addressing, Q16 gate
  paths, fail-closed policies, evidence binding, crystal memory) and kept
  collapsing into a CRUD-template forge.
- **Doctrine:** taxonomy and mathematics may cross; machinery crosses only
  where it honors the invariants (CROSS-002/006/007/010). Every organ is
  re-expressed in the Kosmocrates vocabulary — facets, not files;
  observation, not templates; Q16 at every durable seam; fail-closed,
  never degrade-gracefully. Each phase ships a **disease test** pinning
  that no port carries file-tree templates, stack names, or entity
  scaffolds.

## The rejection list (permanent)

Not ported, by decision — the disease itself:

| Rejected | Why |
|---|---|
| `order.rs` 10-layer pipeline, fixed file trees (`backend/src/models/{}.rs`) | the CRUD skeleton |
| Fixed stacks (Actix, SQLx, JWT, vanilla-JS, Postgres) | Kosmocrates is stack-free by design |
| User-entity forced injection | no entity scaffolds as load-bearing assumptions |
| `cross_file_validator.rs` CRUD-verb normalizer | symptom band-aid for weak prompting |
| 31-norm built-in catalog + hypercube domain registries + anatomy keyword tables | templates baked in stone; norms must be **learned or injected**, never shipped |
| `isls-types` 5D physics | this system has its own tripolar/Q16 vocabulary |
| `isls-agent` loop | kosmo-agent is stricter (policies, rollback, wish convergence) |
| `mef_chain` hash-chain ledger | content addressing + the Infinity Ledger are strictly stronger |
| "Graceful degradation" (emit despite failed consensus) | fail-closed systems answer honestly or not at all |
| `~/.isls` home-dir persistence | all stores are caller-pathed and policy-gated |

## Phase 1 — Deterministic foundations (landed)

**Ported, transformed:**

| Wonderlamp | Kosmocrates | Transformation |
|---|---|---|
| Konus geometric-mean aggregation (`f64`, `ln`/`exp`, ε=1e-6) | `Q16::geomean` (kosmo-core/src/fixed_point.rs) | integer n-th root of the raw product via binary search on `i128` — bit-replayable, fail-closed (empty/zero ⇒ ZERO), unit-interval clamped |
| Ophanim `CodeFeatures` extraction (Rust-only regex readers) | `xlang::symbol_sets` / `symbol_sets_auto` + `SymbolSets` (kosmo-hyphae/src/xlang.rs) | the existing 7-language classifier pass, names retained (`name/arity` function keys); fail-closed on unknown extensions |
| Ophanim scoring ψ/ρ/ω (f64, weights .40/.25/.20/.15) | `consensus::{similarity, assess_candidates, assess_features}` (kosmo-synthesizer/src/consensus.rs) | all-integer: percent-weight sum before one division (exact at boundaries); both-empty axes = vacuous agreement; outlier cutoff ψ<0.15 ⇒ D=0 kept; **neutral band** ρ=ω=HALF for structure-free patches (config/docs — agreement there is emittable, refusing would be over-closed); D_k = ψ·ρ·ω, the house tripolar form |
| Konus D_total = geomean(D_k)·Ω | `ConsensusReport::d_total` | ε floor = one raw unit: an ejected outlier drags the total decisively below Θ without hard-zeroing diagnostics |
| Monolith selection + missing-function repair menu | `ConsensusReport::{best_index, repair_targets}` | selection stays advisory; the *delivery* policy (confidence folding, Coagula rounds) lands in Phase 2 with the synthesizer |
| forge-llm `codematrix.rs` 5D fingerprint (f64, path-bound layer inference) | `CodeMatrixFingerprint` (kosmo-hyphae/src/codematrix.rs) | Q16, content-addressed (`verify_id`), language from xlang not paths; axes r/f/t/s/e documented; `resonance` = geomean of axis similarities; **strictly advisory** (CROSS-010) |

**Invariants pinned by tests:** determinism (identical inputs ⇒ identical
report, bit for bit), soft unanimity, fail-closed empties, disease test
(no Actix/SQLx/JWT/path-templates in the assimilated sources — judged on
the shipped code, not the test's own forbidden list).

## Phase 2 — SwarmSynthesizer, fail-closed (landed)

| Wonderlamp | Kosmocrates | Transformation |
|---|---|---|
| `SwarmOracle` wrapping `Box<dyn Oracle>` | `SwarmSynthesizer` wrapping `Arc<dyn ChatOracle>` (kosmo-synthesizer-llm/src/swarm.rs); `LlmSynthesizer` implements `ChatOracle` by delegating to its existing backoff transport; `ScriptedOracle` for hermetic tests | the wrapper joins the house family (`Contextual(Grounded(Swarm(...)))` composition order) and reuses `build_user_prompt` / `parse_synthesis_response` — n JSON patches, content-addressed on arrival |
| Chameleon lenses (Thronengel framing) | `LENSES` — four quality dimensions (correctness / completeness / robustness / consistency), cycling via `lens_prompt(k, n)` | wording pinned **verbatim** by test and pinned stack-free (no actix/jwt/crud/rest words) — lens drift toward architecture prescriptions is structurally caught |
| Monolith "graceful degradation" (emit best despite failed consensus) | **`confidence = min(best.confidence, d_total)`** | the one transformation that matters: a divergent ensemble lands below the agent's existing `min_confidence` gate and is skipped by *policy*, not emitted by charity. No new gate, no float (CROSS-007/010 untouched). All-unparseable ⇒ `SynthesisError::permanent` |
| Swarm-Coagula repair (≤2 rounds, then emit anyway) | bounded self-completion: repair prompt = own answer + the `repair_targets` quorum menu, re-assessed each round; a non-answer ends repair honestly | repair is framed as self-completion (never imports a peer's body); rounds bounded by `ConsensusConfig::max_repair_rounds`; telemetry (`coagula_rounds=…`) travels in the rationale |
| n× cost accounting | `tokens_used` sums every served call | unchanged concept |

CLI: `kosmo-run --swarm <n>` (clamped 2–6) wraps any real provider;
`--swarm` with the mock provider is refused as *consensus theater* — the
mock answers identically n times, which would manufacture agreement.

## Phase 3 — Descent context + Mikro/Meso patch gates (landed)

| Wonderlamp | Kosmocrates | Transformation |
|---|---|---|
| `type_context.rs` growing TypeContext (Rust-only regex, entity-file paths, byte-budget capping) | `kosmo_synthesizer::context::TypeContext` — `absorb_patch` via the xlang classifier over a patch's file changes; `render(budget_lines)` with the three-stage degradation (full `fn route/1 @ src/router.rs` → name+kind → most-recent-N behind an honest elision marker); deletes retract an origin's symbols; crate roots (`lib`/`main`/`mod`) are not modules | language-agnostic, path-template-free; advisory like memory grounding: `SynthesisRequest.descent_context` is `#[serde(default)]` and **not** part of `request_id` |
| prompt injection of all produced types | `build_user_prompt`: `# Symbols already created in this descent` section ("Reference these exactly; do not re-create or rename them") | same anti-hallucination trick, kosmo wire contract |
| `gates.rs` Mikro/Meso (informational; imports resolved against the HDAG layer *plan*) | `patch_gates::gate_patch` over `kosmo_core::GateResult`, worst-wins | **fail-closed and plan-free**: judgments come from filesystem truth (Create-over-existing / Modify-of-missing ⇒ Reject) and descent knowledge (use-after-delete of a descent-created module ⇒ Reject; duplicate definitions across a patch ⇒ Reject; origin shift ⇒ Warn; structureless source ⇒ Warn; non-code skipped — fail-closed ≠ over-closed) |
| (no delivery discipline) | `ContextualSynthesizer` — wraps any `ActionSynthesizer`; injects the rendered context, gates the result, absorbs only non-rejected patches | gate `Reject` ⇒ `confidence = ZERO` + `gate-reject:` rationale: auditable, unmaterializable through the existing `min_confidence` policy filter; rejected symbols never enter the context (they never existed) |

Wiring: one descent context per `AgentSession::run` and per wish-descent
LLM fallback (`wish_fallback` wraps `Contextual(Grounded(Swarm|Llm))`).
The deterministic `FacetScaffolder` path stays exact and ungated — it
builds precisely what it observes back.
