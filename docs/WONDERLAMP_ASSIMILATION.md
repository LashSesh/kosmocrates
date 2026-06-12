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

## Phase 4 — The norm organ: learned archetypes (landed)

The de-CRUD transformation in one sentence: Wonderlamp's norm was a
recipe for *files across fixed layers*; a Kosmocrates norm is a recipe
for *measurable targets* — it can only emit `WishFacet`s, so the whole
organ lives behind the wish door the substrate already measures,
scaffolds and validates.

| Wonderlamp | Kosmocrates | Transformation |
|---|---|---|
| `Norm{layers: NormLayers{database, model, query, service, api, frontend, …}}` | `Norm{template: Vec<NormFacetTemplate{kind: WishFacetKind, key_pattern}>}` (kosmo-hyphae/src/norm_schema.rs) | layers → facet kinds; file paths → `{name}`-placeholder key patterns; `validate()` rejects path separators, backslashes, file extensions, foreign placeholders (`{entity}`…) and frozen structural names — `Signature` `name/arity` is the one sanctioned `/`; `Resolution`/`Dependency`/`Run`/`Service` are not templatable |
| triggers active on construction | `trigger: Option<String>` — `None` until `--promote-norm --trigger <word>`; **excluded from the content hash** | arming is governance over an existing artifact; promotion re-appends the same `norm_id`, the store's loader takes the latest occurrence — an append-only audit trail |
| 31-norm built-in catalog (`catalog.rs`) | **rejected wholesale** — the catalog starts empty, pinned by test (`the_catalog_starts_empty`) | norms exist only through observation (learning) or explicit operator injection (`NormInjectionSpec`, spec bytes = evidence) |
| `learning.rs`: generated file trees → `CrossLayerPattern` (path-topology signature) → `NormCandidate` | `norm_learning.rs`: realized wish predicates → `FacetBundleObservation` (digest workspace tag, never a raw path) → boundary-aware subject abstraction (`create_user` → `create_{name}`, `username` untouched) → shape grouping | the observed unit is a *realized facet bundle*, so there is no path from observation to a file template; shapes that survive abstraction with a frozen structural key fail `validate()` and are never proposed |
| promotion at ≥N observations in ≥M domains, f64 consistency | `promotable`: ≥3 realized across ≥2 workspace tags, consistency ≥ ¾ — all `Q16::ratio` integer arithmetic | unchanged thresholds, integer math, and the output is still **unarmed**; the proposal carries a linked `NormGeneCandidate`, so the existing promotion-feedback fitness loop applies without modification |
| fitness φ ← 0.9·φ + 0.1·r (f64) | `NormFitnessTrace::smoothed_fitness(alpha: Q16)` — integer EMA on the existing trace | CROSS-007; `alpha` clamped to the unit interval |
| `genome.rs` co-activation (f64 Jaccard ≥ 0.8 single-link), `relations.rs`, `compose_norms` | `norm_genome.rs`: `cluster_genes` (Q16 Jaccard, BFS components, integer-mean cohesion), `relate` (Dependent > Conflicting > Compatible > Independent), `compose` (BFS over `requires`, conflict detection, merged sorted templates) | advisory throughout (CROSS-010): genes rank and inform, they gate nothing; composition emits templates, and making the result durable is a separate governed act |
| `~/.isls` home-dir persistence, write-once + mef_chain | `kosmo-store::NormStore` — caller-pathed directory, `norms.jsonl` + `observations.jsonl`, `allow_host_write`-gated (the same policy bit as every durable store), verify **and** validate on load and append, corruption = hard error on open | content addressing replaces the hash chain; the anti-disease gate runs at rest, not just at creation |
| chat → `intent_to_norm_ops` | `kosmo_intent::{NormCatalog, compile_wish_with_norms, is_reserved_wish_word}` | a promoted trigger expands exactly like a built-in archetype; reserved grammar words and duplicate triggers are refused at catalog construction; the empty catalog is pinned **byte-identical** to the untouched `compile_wish` |

CLI: `kosmo-run --norms <dir>` (expansion + learning on realized
`--apply` descents), `--inject-norm <spec.json>`, `--promote-norm <id>
--trigger <word>`. The e2e binary test walks the plan's lifecycle:
three realized descents of one shape in two scratch workspaces ⇒ the
third run stores the unarmed norm ⇒ its would-be trigger still expands
nothing ⇒ promotion arms it ⇒ "a loader delta" compiles to the learned
shape ⇒ and without `--norms` the same prose stays vacuous.

## Phase 5 — Spectral kit + landscape geometry (landed)

| Wonderlamp | Kosmocrates | Transformation |
|---|---|---|
| `isls-hypercube` spectral kit (`laplacian` / `fiedler_bisect` / `kuramoto_groups` / `singularities` over `Dimension` graphs) | **new crate `kosmo-spectral`** — `CouplingGraph` (Q16 weights, fail-closed simple graph), `fiedler_bisect`, `spectral_clusters`, `kuramoto_groups`, `singularities` | the graph is *abstract* (nodes are whatever the caller couples); the **float quarantine**: floats live only inside the crate, the public API is Q16/discrete-only (source-scan pin); deps = kosmo-core only, no nalgebra (manifest pin) |
| eigen-decomposition with library floats, platform-dependent transcendentals | power iteration + constant-vector deflation, seeded deterministic init, fixed iteration counts; Kuramoto integrates with an **in-crate range-reduced Taylor sine** (no libm) | bit-reproducible across IEEE-754 platforms — determinism without an asterisk |
| recursive bisection to a requested k | **conductance-bounded splitting**: a cut is kept only while `cut/min(vol) ≤ ½` — tight cliques refuse to shatter | the cluster count *emerges* (a K4 barbell yields its two cliques even when 4 are allowed); `max_clusters` is an upper bound, not a command |
| singularities = spectral-gap probes over dimension registries | articulation nodes (iterative integer Tarjan) ranked by Q16 coupling mass | pure integer; "the most consequential decisions first" without re-running the spectrum per node |
| AppSpec parser, domain registry (3 CRUD domains), rigid dimension categories | **rejected** — the consumer is `kosmo_pipeline::landscape_geometry` and its coupling features come from `WishProposal` fields ONLY: subject affinity 45, facet-kind affinity 30, severity proximity 25 (integer percent, exact at boundaries; **proximity alone never couples**) | CRUD-relapse guard #5 pinned by a disease test on the source; the population coupled is the caller's choice (kosmo-run couples the *open* proposals) |
| (no adoption story) | `--landscape --geometry` (strictly opt-in — flag-off output byte-identical, pinned) + `--adopt-cluster <i>`: ONE coherent cluster becomes ONE severity-weighted, evidence-bound wish through the existing descent | adoption graduates from blind top-k to coherent work units; `--adopt`/`--adopt-cluster` mutually exclusive, out-of-range indices name the real count |

Everything advisory (CROSS-010): geometry ranks and groups the
already-measured landscape; it gates nothing and changes nothing it
does not name.

## Phase 6 — Chat front door (landed) — assimilation complete

| Wonderlamp | Kosmocrates | Transformation |
|---|---|---|
| `isls-chat` `IntentType{CreateApplication, AddField, ModifyEntity, …}` with `affected_files` lists, forced User entity, REST assumptions | `kosmo_intent::chat::ChatIntent{MakeWish, DescendWish, ShowLandscape{geometry}, AdoptLandscape{top}, AdoptCluster{index}, ShowStatus, InjectNorm}` | **type-systemically path/file/entity-free** (source-scan pin on the enum): every variant maps onto an organ the substrate already has; a chat intent *cannot* describe structure directly — structure enters the world only through measured facets |
| dual extraction (LLM ↔ keyword fallback) | `IntentExtractor` trait, **total by signature** (`extract` cannot fail): `KeywordIntentExtractor` (ordered rules, MakeWish fallback) and `LlmIntentExtractor` (model-first, keyword fallback on ANY failure — transport, malformed JSON, unknown intent; proven without network) | the house dualism (compile_wish ↔ LlmWishCompiler) applied to routing; the fallback target is the **measurable wish door**, where an unparseable wish is honestly vacuous — never a template generator, never a swallowed utterance |
| `intent_to_norm_ops` mutating a spec | routing is **transient**: no content addressing, no evidence binding — the routed-to organs own all durable artifacts | exactly why graceful LLM degradation is legitimate here and nowhere else: a routing decision is not a replayable artifact |
| chat as the generator's steering wheel | `kosmo-run --chat "<utterance>"` — one-shot (no REPL), echoes `chat[router] → intent` for audit, delegates to existing modes; `--apply`/`--provider`/`--ledger`/`--norms` compose orthogonally; a "build …" utterance without `--apply` measures and says so | chat never escalates privileges and never bypasses a policy: it is a *front door*, all walls intact; InjectNorm maps to instructions for the explicit governance flags (chat carries no spec files, by type) |

**The assimilation is complete.** All six planned phases landed, each
with the full gate ritual: (1) deterministic consensus foundations,
(2) the fail-closed SwarmSynthesizer, (3) descent context + patch
gates, (4) the norm organ, (5) the spectral kit + landscape geometry,
(6) this front door. What Wonderlamp wanted to be — a system that takes
an utterance and grows software toward it — exists now, but on the
epistemic substrate it lacked: every step measured, every artifact
content-addressed and evidence-bound, every gate fail-closed, every
learned pattern governed. The rejection list above is permanent; the
disease tests that enforce it run in every CI pass.
