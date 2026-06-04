# Implementation Status

## Current Phase
**Vision chain COMPLETE** — Topology ✅ → Energy ✅ → Blueprint ✅ → Roundtrip ✅ → Feedback ✅ → CodeStructure ✅ → ResoniteCAD ✅ → CrystalBoost ✅ → CrystalPersist ✅ → CrystalAutoLoop ✅ → WorkspaceEntry ✅ → ActionItems ✅ → SubstrateCLI ✅ → TUI ✅ → WebUI ✅ → Synthesizer ✅ → Agent ✅ → LlmBackends ✅ → AgentRunner ✅ → Materialize ✅ → LoopClosed ✅ → GitCommit ✅ → PromotionFeedbackLoop ✅ → WishSpec ✅ → WishAttractor ✅ → WishObserver ✅ → WishGovernance ✅ → WishGeneration ✅ → WishGranularity ✅ → WishFrontDoor ✅ → WishLLM ✅ → WishScaffold ✅ → FacetSemantics ✅ → GreenTests ✅ → WishCLI ✅ → WishDescent ✅ → DescentLLM ✅ → DepScaffold ✅ → SessionPersist ✅ → ContractFacet ✅ → BehaviorFacet ✅ → ArchetypeFanout ✅ → CrateTargeting ✅ → Composition ✅ → Capstone ✅ → Pruefstand ✅ → FlowComposition ✅ → Sandbox ✅  
"Echte Topologie rein, tripolare Energie darauf, Blueprint raus, Realitätstest drüber, Wissen zurück ins Substrat — CAD-Bibliothek treibt aktiv das Ranking, überlebt Sessions, wird vollautomatisch befüllt, läuft mit einem einzigen Aufruf auf echten Workspaces, produziert priorisierte Arbeitsanweisungen — navigierbar als TUI und erreichbar über den Browser, synthetisiert mit Claude oder Cerebras — schreibt validierte Patches policy-gegated zurück in den Workspace (cargo-geprüft, mit Rollback), committet jeden akzeptierten Patch als eigenen Git-Commit, und speist Execution-Feedback als PromotionFeedback zurück in die Pipeline. Der Kompressor läuft. Und der nächste Bogen hat begonnen: ein Wunsch ist jetzt ein content-adressiertes, messbares Ziel (`Wish` + `assess_wish`) — der Gradient, an dem derselbe Kompressor künftig zur Wunsch-zu-System-Maschine entlanglaufen kann."

- **1163 substrate tests** (kosmo-core 382, kosmo-hyphae 204, kosmo-pse-bridge 35, kosmo-kcube 46, kosmo-systemcube 54, kosmo-parseback 17, kosmo-operator 8, kosmo-workbench 20, kosmo-store 14, kosmo-pipeline 120, kosmo-synthesizer 33, kosmo-synthesizer-llm 14, kosmo-agent 24, kosmo-materialize 11, kosmo-intent 62, kosmo-llm 14, kosmo-intent-llm 9, kosmo-sandbox 9, kosmo-run 17) — 0 failures; plus 2 capstone integration tests (`cargo test --workspace`: 2981)
- **147/147 eval scenarios** pass (kosmo-eval KOSMO-OPS-01 full benchmark)
- **Every Q16-score substrate type in kosmo-hyphae has `energy_assessment`** ✅
- **`BlueprintUnit::energy_assessment` wired in kosmo-systemcube (Step 5e)** ✅
- **`ContradictionEnergyReport::from_units` — real pairwise contradiction detection** ✅
- **`CompatibilityProfileReport::from_units` — real gap detection (TaintedUnit, MissingSourceRef)** ✅
- **SystemCube diagnostics surfaced in pipeline: accessors + gate contribution + summary** ✅
- **`AssimilationDecision.taint` propagated from `StructuralYield`; pipeline uses real taint in `BlueprintUnit`** ✅
- **`MicroTopologyIndex` assembled in pipeline** ✅
- **`SurgeryWorkbenchTask` conversion wired as pipeline Step 3e** ✅
- **`NormFitnessTrace` from prior feedback wired as pipeline Step 5c** ✅ — "Wissen zurück ins Substrat" loop closed
- **`TopologyAmbiguityProfile` + `ComplementVoidHypothesis` energy-ranked in pipeline Step 3f** ✅
- **`StructuralCrystalCandidate` certification work queue wired as pipeline Step 5d** ✅
- **`DeficiencyVector` always-on pipeline Step 1c** ✅
- **`PseBridgeCandidate` conversion from pipeline observations wired as Step 6b** ✅

### kosmo-sandbox — the execution sandbox (Runtime floor, beam 2) (2026-06-04)

The load-bearing infra of the Runtime floor (`docs/RUNTIME-floor.md` §4): the safe room in which a built artifact — possibly code the loop generated — is run and *trusted as observed*. A capability, not a gate.

- **Enforced (Unix):** the child runs in its own process group (`CommandExt::process_group(0)`), so a timeout fells the whole tree via `killpg(SIGKILL)` — a hung grandchild (the binary `cargo run` spawns) cannot outlive the budget. stdout/stderr drain on their own threads into capped buffers (runaway printer → truncated, never OOM or pipe-deadlock); the child is always reaped.
- Every run returns a content-addressed `RuntimeWitness { verdict, exit_code, stdout, stdout_digest, duration, truncated }`; the digest witnesses the full output even when the text is truncated.
- **Honest best-effort:** network isolation is declared (`NetworkPolicy::Deny` clears proxy env) but not yet hard-enforced — the crate refuses to claim isolation it cannot deliver (spec §8.1); filesystem containment is by the caller's throwaway `cwd`. Non-Unix reports a timeout but cannot force-kill.
- The only `unsafe` is the single `killpg` call; grouping uses the safe std `process_group`. +9 tests, warning-free.

### Behavioural composition — validated data-flow (Runtime floor, beam 1) (2026-06-04)

First beam of the Runtime floor (`docs/RUNTIME-floor.md`), and its sandbox-free on-ramp: the level-2 keystone applied to a level-3 wire — proving data actually *flows* through a composition, not merely that the types align.

- A piped behaviour spec `f(x)>>g>>h=>expected` desugars to the nested call `h(g(f(x)))`, validated green by the **existing** `cargo test` judge. `kosmo-synthesizer::parse_behavior_key` routes any `>>`-bearing key to `parse_pipeline_behavior_key`, which folds the pipeline (splitting on `=>`/`>>` as plain strings — `>>` overloads `>`, which the bracket-depth path misreads).
- `kosmo-intent` adds a `flow`/`pipeline` trigger compiling to that Behavior facet. **Observation is untouched**: the `// kosmo:behavior:` marker carries the piped key verbatim, so it round-trips through the existing green-test machinery.
- Verified live both ways: `a flow parse("2+3")>>eval=>5` over a correct pipeline → `0/1 → 1/1 REALIZED` (exit 0); over an `eval` returning 6 → stays `0/1`, `✗ Behavior …`, exit 1 (rejected). +5 tests.

### Prüfstand — empirical fidelity harness (2026-06-04)

The capstone generalized into a *corpus*. `kosmo-run --pruefstand` (module `tools/kosmo-run/src/pruefstand.rs`) descends nine reference systems — one per facet axis the floor builds (symbol/contract/module/capability/test/composition/archetype) plus the behavioural keystone in **both** directions — each in a throwaway workspace through the real descent, and compares the reached verdict to the expected one (`Match` / `Mismatch` / `Skipped`).

- Structural scenarios converge offline; behavioural ones run `cargo test` (gated by `--validated`). An unobservable scenario is *skipped*, never counted as a failure — the harness reports honestly. `is_faithful()` ⇔ zero mismatches; the CLI exits 3 otherwise.
- The corpus is the natural home for new reference systems as the substrate grows — the empirical complement to the unit suite.

Verified live: `--pruefstand --validated` → **fidelity: 9/9 matched (0 mismatched, 0 skipped)**, exit 0, with `behavior-wrong` (`a+b+1`) correctly rejected. +2 unit tests (kosmo-run 14→16).

### Capstone — the whole floor in one runnable artifact (2026-06-04)

`tools/kosmo-run/tests/capstone.rs` drives the **real `kosmo-run` binary** end-to-end, proving *acceptance over generation* in both directions through the actual CLI:

- **correct** impl + a typed `Contract` and a `Behavior` spec → `1/2 APPROACHING → 2/2 REALIZED`, exit 0, spec-test marker written;
- **incorrect** impl (`a+b+1`) + the same behaviour → stays `0/1 UNSTARTED`, `✗ Behavior add(2,3)=>5`, exit 1 — the system does not lie.

The positive case runs `cargo test` nested (skips, not fails, where the sandbox forbids it); the negative case is robust regardless. The first reference scenario for the empirical Prüfstand. +2 integration tests (verified live: both directions confirmed at the CLI).

### Composition — typed data-flow wiring (Horizon floor, level 3) (2026-06-03)

The behavioural cousin of `Dependency`: a `Composition` facet `"from>>via>>to"` says `from` returns type `via` and `to` consumes `via` — the components *wire together* (`to(from(x))` typechecks), not merely coexist.

- **`kosmo-core`**: `WishFacetKind::Composition`; `WishFacet::composition(from, via, to)`.
- **`kosmo-intent`**: `derive_compositions` reads the observed (bare) contracts and emits `f>>T>>g` for every ordered pair where `ret(f)==param0(g)` and `T != ()`; wired into `facets_from_rust_dir`, so compositions are observed *for free* from the Contract layer. `parse_contract_facet_key` parses a contract key. `composition`/`compose` trigger added.
- **`kosmo-synthesizer`**: `scaffold_composition` appends two type-compatible stubs — `pub fn from() -> via` + `pub fn to(_a0: via)` — in a single change via `append_items_to_lib` (returning two changes for one path would overwrite); the observer derives the composition back, so it round-trips.

Verified live: `--wish "a composition parse>>String>>eval" --apply` → `0/1 → 1/1 REALIZED`, two compatible stubs written, `cargo build` clean. +8 tests.

### Crate-targeting — scaffold into the named member crate (2026-06-03)

The precondition for real multi-crate full-stack wishes: a facet key may carry an `@<crate>` suffix to scaffold the item *into that workspace member* instead of the root crate.

- **`kosmo-synthesizer`**: `scaffold` splits an `@<crate>` suffix (for in-crate kinds — Symbol/Signature/Contract/Module/Capability/Test); `scaffold_into_crate` resolves the crate by package name, runs the per-kind scaffolder against the crate dir, and re-bases change paths to workspace-root-relative. Unknown crate → honest no-op. `scaffold_kind` factored out as the shared dispatch.
- **`kosmo-intent`**: `facets_from_rust_dir` emits each source item twice — bare and crate-qualified `<key>@<crate>` (`crate_of` walks to the nearest `[package]` manifest; `manifest_package_name` reads it) — so a crate-targeted wish round-trips against the crate the scaffolder wrote into.
- The `@crate` form survives the prose compiler (`clean_name` keeps the internal `@`), so `a function handle@api` compiles to `Symbol handle@api`.

Verified live: `--wish "a function handle@api and a function compute@core" --apply` → `handle` in `crates/api`, `compute` in `crates/core`, root untouched, `0/2 → 2/2 REALIZED`. +6 tests.

### Archetype expansion — full-stack fan-out (Horizon floor, beam 3) (2026-06-03)

The breadth axis of `docs/HORIZON-behavior-archetype.md`: one prose word expands into a *bundle* of facets.

- **`kosmo-intent`**: a compiler stage above the leaf triggers. `archetype_trigger` recognizes `crud`/`endpoint`/`component` (reserved, disjoint from leaf triggers); `expand_archetype(arch, name)` fans each into existing leaves; `name_after` shared by both paths. `crud user` → Module `user` + Contract `create_user(String)->String` + Contract `get_user(String)->String` + Capability `crud:user`. `endpoint <name>` → typed handler + marker; `component <name>` → module + marker.

Elegant by design: **no new facet kind, no scaffolder change** — archetypes are pure templates over leaves the substrate already builds and observes (validating the spec's claim "once leaves mean behaviour, archetypes are cheap templates"). The structural bundle converges offline; `String→String` handlers compile without extra type defs.

Verified live: `--wish "a crud user" --apply` → `0/4 → 4/4 REALIZED ✓` in one descent step, writing the module file, both handlers, and the marker. +5 tests (kosmo-intent 51→55, kosmo-run 11→12).

With beams 1+2+3 the loop spans the full floor: **types** (Contract), **validated behaviour** (Behavior, the keystone), and **breadth** (Archetype). A single sentence now fans into a structural+behavioural bundle, and the descent realizes it — accepting only what is observed correct.

### Behavior facets — the keystone, acceptance over generation (Horizon floor, beam 2) (2026-06-03)

The load-bearing beam of `docs/HORIZON-behavior-archetype.md`. A `Behavior` facet `"name(args)=>expected"` is satisfied **only when a scaffolded spec-test pinning that input→output pair passes** — observed by running the suite, fail-closed.

- **`kosmo-core`**: `WishFacetKind::Behavior`; `WishFacet::behavior(spec)`.
- **`kosmo-synthesizer`**: `scaffold_behavior` appends a `// kosmo:behavior: <spec>`-marked `#[test] fn kosmo_spec_<hash>() { assert_eq!(name(args), expected); }` — **red** until the body is correct; deterministic test name from the key digest; idempotent via the marker. `parse_behavior_key` / `split_on_fat_arrow` parse the spec.
- **`kosmo-intent`**: `behavior_specs_from_source` pairs each marker with its test fn name; `behavior_facets(specs, passing)` (pure, **fail-closed**) emits a facet only for specs whose test is green; wired into `observe_workspace_validated`. Lexical `observe_workspace_deep` never emits Behavior (it can't know a test passes). `behavior`/`spec` trigger added.
- **`kosmo-run`**: `wish_needs_validation` auto-enables validated observation for any wish carrying a behaviour — the keystone demands the suite run.

**Acceptance over generation**: we cannot guarantee the LLM *writes* correct code; we guarantee *when the loop may declare a wish realized*. The descent terminates only when behaviour is observed green. Verified live: correct `add` → `0/1 → 1/1 REALIZED`; wrong `add` (`a+b+1`) → honest stall at `0/1 UNSTARTED`, exit 1 — the system does not lie. +12 tests.

With beams 1+2 the skeleton carries **types** and **validated behaviour**. Beam 3 (`Archetype`) fans a single high-level wish into bundles of these leaves — full-stack breadth over behavioural depth.

### Contract facets — typed signatures (Horizon floor, beam 1) (2026-06-03)

First beam of the behavior axis specified in `docs/HORIZON-behavior-archetype.md`. A `Contract` facet `"name(T0,T1)->R"` is the typed promotion of `Signature` (`name/arity`): parameter + return *types*, not just the count.

- **`kosmo-core`**: `WishFacetKind::Contract`; `WishFacet::contract(name, &[types], ret)` + `contract_key(key)` constructors (canonical key format).
- **`kosmo-intent`** (observe): `parse_fn_types` reads a function's opening line into `(param_types, ret)` — depth-aware comma split, `->` treated as an atom, receivers (`self`/`&self`/`&mut self`) skipped, whitespace normalized, return defaults to `()`. `item_facets` now emits `Contract` alongside `Symbol` + `Signature` (additive — no regression). `contract` trigger added.
- **`kosmo-synthesizer`** (scaffold): `scaffold_contract` builds the dual — `pub fn name(_a0: T0, …) -> R { todo!("…") }`, **honestly empty** at runtime, idempotent via the `fn name` marker. `parse_contract_key` mirrors the observer so **scaffold → observe round-trips**.
- **`kosmo-intent-llm`**: `contract` mapping added.

Verified live: `kosmo-run --wish "a contract add(i32,i32)->i32" --apply` → `iter 0: 0/1 UNSTARTED → iter 1: 1/1 REALIZED ✓`, writing `pub fn add(_a0: i32, _a1: i32) -> i32 { todo!("kosmo: implement add") }`; `cargo check` is clean and the body panics until filled. +14 tests, 0 failures.

The structural skeleton now carries *types*. Beam 2 (the keystone, `Behavior`) makes those bodies get filled and **validated** — acceptance over generation.

### Session persistence — every descent auditable and replayable (2026-06-03)

`kosmo-run --wish … --wish-session <path>`:
- **save**: after a descent (`--apply`) or a single observation (no `--apply`), the full `WishSession` is serialized as pretty-printed JSON and written to `<path>` — the complete trajectory of `WishAssessment[]` including distances, met counts, unmet facets, and content-addressed ids;
- **resume**: if `<path>` already exists and its embedded wish id matches the current prose-derived wish, the prior session is loaded and descent continues from it — iterations accumulate across invocations;
- **identity guard**: `load_prior_session` discards a session whose wish id doesn't match rather than silently merging foreign trajectories;
- +2 tests: JSON round-trip (in-memory serialize/deserialize preserves iterations, wish id, and final status); file round-trip (save → load matches; different-wish load returns `None`).

Kosmo-run: 6 → 8 tests.

### Dependency deterministically scaffoldable — last offline gap closed (2026-06-03)

`FacetScaffolder::scaffold_dependency(ws, "from->to")`:
- locates `from` and `to` by package name (`find_crate_manifests` walks the workspace skipping `target`/dotted dirs; `package_name` parses `[package] name`);
- computes the relative path (`relative_path`) and adds `to = { path = "<rel>" }` to `from`'s `[dependencies]` (`add_path_dependency`, creating the section if absent); idempotent via `dep_already_present`;
- honest no-op if either crate is missing.

Every **structural** facet now converges deterministically (no LLM, no keys): Crate · Module · Symbol · Signature · Capability · Test · Dependency. Only `Resolution` ("the bad thing is gone") has no structural scaffold, by nature.

Verified live: `kosmo-run --wish "dependency alpha->beta" --apply` → `iter 0: 0/1 UNSTARTED → iter 1: 1/1 REALIZED ✓`, writing `beta = { path = "../beta" }`; ParseBack's `cargo metadata` then reports the edge. +2 tests (kosmo-synthesizer 16 → 17, kosmo-run 5 → 6).

### Descent unifies both Wish→Patch backends (2026-06-03)

The descent loop now spans both ends of the `Wish → Patch` contract. `apply_synthesis(root, unmet, fallback)`:
- tries the deterministic `FacetScaffolder` first (exact for structural facets);
- for facets it can't build (today: `Dependency`, `Resolution`), consults an optional LLM `ActionSynthesizer` fallback — wired in `--wish --apply` only when `--provider` (mock/claude/cerebras) is given;
- deterministic always wins when it produces a patch.

+1 test: a `Dependency` facet writes nothing without a fallback, and is routed to the synthesizer when one is present (kosmo-run 4 → 5). With `--provider mock` the fallback proposes nothing (the mock is empty); a real provider attempts it — convergence then depends on the LLM, while the deterministic path stays guaranteed.

### kosmo-run --wish --apply — the descent, executed (2026-06-03)

The attractor descent (Runs 1–2's `WishSession`) is now actually driven to convergence, deterministically and offline.

- [x] `descend_to_wish(path, wish, …)` — observe → assess → scaffold unmet → write → re-observe, looping until realized / no-progress / max-iters; returns the `WishSession` trajectory
- [x] `apply_scaffold` — the only filesystem write in wish mode, gated by `--apply`
- [x] `descent_report` — one line per iteration + the verdict
- [x] +1 test: a symbol+crate wish converges to REALIZED on a temp workspace (kosmo-run 3 → 4)

Verified live: `--wish "a function alpha and a function beta and a crate widget" --apply` → `iter 0: met 0/3 UNSTARTED → iter 1: met 3/3 REALIZED ✓`, writing the two functions, the new crate, and the workspace-member edit. The loop closes: prose in, realized system out.

### kosmo-run --wish — the CLI front door (2026-06-03)

The whole wish-to-system loop is now **one command**. `kosmo-run` (the agent runner) gains a deterministic, offline **wish mode**:

- [x] `--wish "<prose>"` — compile (rule compiler) → observe (`observe_workspace_deep`) → `assess_wish` → report met/missing facets; exits 0 only when realized
- [x] `--validated` — observe green tests too (`observe_workspace_validated`)
- [x] `--scaffold` — print the `FacetScaffolder`'s proposed file changes (dry run)
- [x] `--json` — the assessment as JSON
- [x] testable `wish_report` / `scaffold_report` (return `String`); +3 tests; depends on kosmo-intent (added)

Verified live against this repo: `--wish "a crate kosmo-core and a crate ghost_xyz and a function compile_wish"` → APPROACHING, met 2/3, missing `Crate ghost_xyz`. No LLM or key required — the deterministic front door a human can actually type.

### Green tests — Test facets bound to validated behaviour (2026-06-02)

A `Test` facet now means a test that *passes*, not just one that exists.

**`kosmo-intent`**:
- [x] `parse_test_results(output)` — pure libtest-output parser (`test NAME ... ok|FAILED`; ignored/summary skipped)
- [x] `passing_test_facets(results)` — `Test` facets for green tests only, keyed by bare name
- [x] `run_workspace_tests(root)` — runs `cargo test`, parses verdicts (a non-zero exit from failing tests is fine)
- [x] `observe_workspace_validated(root)` — deep observation with lexical `Test` facets swapped for the green set (`ObservedTopology::retain` added in kosmo-core); opt-in/heavy, falls back to lexical if the run can't start
- [x] 3 tests incl. a live green-vs-red `cargo test` run (kosmo-intent 36 → 39)

The strongest binding so far of a wish to *validated behaviour*: "I want a green test `X`" converges only when `X` actually passes.

### Richer facet semantics — Test (2026-06-02)

`WishFacetKind::Test` (keyed by test fn name): `facets_from_source` is now stateful — `#[test]`/`#[tokio::test]` + the next `fn NAME` → `Test(name)`. `FacetScaffolder::scaffold_test` emits `#[test] fn name() {}`; rule compiler `test` + LLM mapping. v1 observes test *presence*; tying "green" to the cargo validator (a test that passes) is the next refinement. +4 tests (kosmo-intent 33 → 36, kosmo-synthesizer 15 → 16).

### Richer facet semantics — Capability (2026-06-02)

Makes the existing `Capability` kind observable via source markers: `facets_from_source` reads `// kosmo:capability: <name>` (and `//!`). `FacetScaffolder::scaffold_capability` writes the marker; rule compiler `capability`/`feature` + LLM mapping. A wish can target a named behaviour — the facet closest to human intent. +3 tests (kosmo-intent 31 → 33, kosmo-synthesizer 14 → 15).

### Richer facet semantics — Signature (2026-06-02)

`WishFacetKind::Signature` (`"name/arity"`): the extractor (`item_facets`, now multi-facet) emits a `Signature` per `pub fn` alongside `Symbol`; `fn_arity` counts args from the opening line (generics/arrays don't inflate it). `FacetScaffolder::scaffold_signature` realizes it (`pub fn name(_a0: (), …)`, via a shared `append_to_lib`); rule compiler `signature`/`sig` + LLM mapping. A wish can target a function's arity, not just its name. +4 tests (kosmo-intent 28 → 31, kosmo-synthesizer 13 → 14).

### Richer facet semantics — Dependency (2026-06-02)

`WishFacetKind::Dependency` (`"from->to"`): `facets_from_snapshot` emits one per ParseBack `dep_edge`; `WishFacet::dependency(from, to)`; rule compiler `dependency`/`depends` + LLM mapping recognize it; scaffolder leaves it unscaffolded (no inferable path). A wish can now target the dependency structure, not just presence. +2 tests (kosmo-intent 26 → 28).

### FacetScaffolder — deterministic build-toward-intent (2026-06-02)

The synthesis end made real *without* a model: a deterministic synthesizer that realizes wish facets offline, so the loop builds and converges with no network.

**`kosmo-synthesizer`**:
- [x] `FacetScaffolder` (impl `ActionSynthesizer`) acts on `RealizeWishFacet`: `Symbol` → append `pub fn <name>() {}` to `src/lib.rs`/`main.rs`; `Module` → create `src/<name>.rs` + `pub mod <name>;`; `Crate` → create `<name>/Cargo.toml`+`src/lib.rs` + best-effort `[workspace] members` registration; `Capability`/`Resolution` → empty
- [x] Reads the workspace to stay idempotent (already-realized facet → empty patch); writes only the structural skeleton (behaviour is the LLM synthesizer's job)
- [x] 5 tests (kosmo-synthesizer 9 → 13, kosmo-agent 23 → 24) incl. end-to-end `agent_wish_builds_symbol_and_converges`: apply mode, Symbol absent (`ONE`) → scaffolded → realized (`ZERO`), offline

**Both synthesis backends now exist:** the deterministic `FacetScaffolder` (offline, byte-reproducible) and the facet-aware LLM synthesizer (`kosmo-synthesizer-llm`). Same shape for the front door: `RuleWishCompiler` (deterministic) and `LlmWishCompiler` (LLM).

### LLM ends, real — shared transport + prose→Wish (2026-06-02)

The NL front door gets a real LLM backend, behind the same deterministic contract as the rule compiler.

**`kosmo-llm`** (new, shared transport):
- [x] `LlmConfig` / `LlmProvider` (Anthropic Messages API + OpenAI-compatible), `complete(system, user)` with 429/529/5xx retry+backoff, `config_from_env`, string-aware brace-balanced `extract_json_object`, `truncate`
- [x] The substrate's only non-deterministic step now lives in one crate (CROSS-007: the temperature float never escapes the request body)
- [x] 14 tests (config/endpoint/body/extract shapes, JSON extraction edge cases, empty-key fail-fast)

**`kosmo-intent-llm`** (new):
- [x] `LlmWishCompiler` implements `kosmo-intent::WishCompiler`: prose → JSON facet list → content-addressed `Wish`; `from_env` / `claude` / `cerebras`; drops into the agent loop where the rule compiler does
- [x] Pure prompt + parse (`system_prompt`, `build_prompt`, `parse_wish_response`) — facets with unknown kinds / empty keys dropped; prose is the wish label; `Wish` id deterministic
- [x] 9 tests (parse incl. fences, unknown-kind/empty-key dropping, empty→vacuous, fail-fast on empty key); live calls gated by credentials

**Note:** `kosmo-synthesizer-llm` still carries its own transport; migrating it onto `kosmo-llm` (re-exporting `LlmConfig`) is a clean follow-up — no behaviour change.

**Both non-deterministic ends are now real:** prose→`Wish` (`kosmo-intent-llm`) and the facet-aware patch synthesizer (`kosmo-synthesizer-llm`, with the `RealizeWishFacet` prompt directive). Each has a deterministic reference backend (rule compiler / scaffolder) for offline, byte-reproducible runs.

### The human front door — natural-language → Wish (2026-06-02)

A person states intent in prose; out comes a structured, content-addressed `Wish` the loop can descend toward.

**`kosmo-intent`**:
- [x] `compile_wish(prose, policy_id, evidence)` — deterministic, dependency-free: scans prose for structural triggers (`crate`/`package`, `module`/`mod`, `function`/`fn`/`method`, `type`/`struct`/`enum`/`trait`/`symbol`) and turns each `keyword NAME` phrase into a required facet; the prose is the wish label; handles backticks/quotes + fillers
- [x] `WishCompiler` trait + `RuleWishCompiler` — the extension point for an LLM-backed compiler (counterpart to `kosmo-synthesizer-llm`: the model is the only non-deterministic part, the emitted `Wish` stays content-addressed)
- [x] 10 new tests (kosmo-intent 17 → 26, kosmo-agent 22 → 23) incl. end-to-end `agent_wish_from_prose_realized`: prose → Wish → the loop realizes it. Convention: name after the keyword; free word order is the LLM backend's job

**The full chain, end to end:** prose → `compile_wish` → `Wish` → agent attaches it → observes (crate/module/symbol) → measures distance (Lyapunov `V`) → generates facet-directed actions → synthesizes/applies → re-observes → converges to the attractor, fail-closed on divergence. The Wunsch-zu-System loop is now closed *from a sentence to a converged workspace*, at crate/module/symbol granularity, deterministically — with the two non-deterministic ends (NL→Wish and patch synthesis) cleanly quarantined behind traits, each with a deterministic reference backend today and an LLM backend as a drop-in later.

### Finer granularity — Module/Symbol facets (2026-06-02)

A wish can now target modules and public symbols, not just whole crates.

**`kosmo-intent`** — name-preserving lexical extractor:
- [x] `facets_from_source(&str)` — deterministic, dependency-free Rust lexer → `Module` facets (`mod`) + `Symbol` facets (public `fn`/`struct`/`enum`/`trait`/`type`/`union`/`const`/`static`), keyed by bare name; handles `pub(...)`, `async`/`const`/`unsafe`, generics, comments/attrs
- [x] `facets_from_rust_dir(dir)` (walks `.rs`, skips `target`/`.git`) + `observe_workspace_deep(root)` = crate (cargo metadata) ∪ module/symbol facets
- [x] Built our own extractor rather than reuse `code_hdag` (its `CodeHDAG` keeps node *labels*, not names; touching it would alter the content-addressed `hdag_id`)

**`kosmo-agent`**:
- [x] `observe_wish` now uses `observe_workspace_deep`, so the loop measures and builds toward module/symbol wishes; end-to-end `agent_wish_realized_on_symbol`
- [x] 10 new tests (kosmo-intent 8 → 17, kosmo-agent 21 → 22). Known limits: bare-name symbol keys (no crate/module qualification yet), `extern` and macro-generated items not captured

**Next (the human front door):** natural-language → `Wish` compilation, so a person can state intent in prose and get a structured, content-addressed `Wish` the loop can descend toward.

### The wish builds toward itself — facet-directed synthesis (2026-06-02)

The generation half (the fifth rung): the agent stops merely *measuring* the gap to the wish and starts *closing* it. The repair loop becomes a build-toward-intent loop.

**`kosmo-pipeline`** — intent-directed action:
- [x] `ActionItemKind::RealizeWishFacet { facet: kosmo_core::WishFacet }` — first-class directed work carrying the unmet facet (counterpart to `FillVoid`, on the intent axis); the pipeline scan never emits it (the agent does)

**`kosmo-agent`** — generation wiring:
- [x] Each `run()` with a wish observes at the start, turns each unmet facet into a top-priority `RealizeWishFacet` action, prepends it to the queue (wish-directed work first, then voids)
- [x] `AgentRunReport::wish_directed_count()` — how many facet-directed steps the run took
- [x] End-to-end: `agent_wish_builds_toward_and_converges` runs the loop in apply mode with a scaffolding synthesizer — run 1: wished crate absent (distance `ONE`) → writes `Cargo.toml`; run 2: realized (distance `ZERO`). The loop builds toward the wish and converges
- [x] 4 new tests (kosmo-agent 17 → 21); fixed a pre-existing flake in `agent_run_id_is_deterministic` (isolated workspace instead of the shared system temp dir)

**`kosmo-synthesizer-llm` + tools** — the synthesizer and renderers see the new kind:
- [x] LLM prompt gains a `RealizeWishFacet` directive (the model is told exactly what to build); `kosmo-substrate` / `kosmo-tui` / `kosmo-server` / `kosmo-run` render the new kind

**Where the arc stands:** all six pieces are real — target (`Wish`) · ruler (`assess_wish`) · contract (attractor) · observation (`kosmo-intent`) · governance (the loop enforces convergence) · generation (the loop acts toward the wish). The minimal wish-to-system loop is closed at crate granularity, demonstrated end-to-end with a scaffolding synthesizer. Frontier from here: finer facet granularity (Module/Symbol via a name-preserving extractor) and natural-language → `Wish` compilation (the human-facing front door).

### `kosmo-agent` — the wish governs the loop (2026-06-02)

The fourth rung: the wish drives the execution loop. Attach a wish and each `run()` measures the workspace against it and tracks convergence toward the attractor across runs — fail-closed on divergence. One `run()` = one step of the dynamics.

**`kosmo-agent`** (dep added: `kosmo-intent`):
- [x] `AgentSession::with_wish(wish, evidence_bundle_id)` — attaches a `WishSession`; each `run()` observes the workspace (read-only `cargo metadata` via `kosmo-intent`) and folds the distance into the trajectory. Fail-soft: a non-cargo workspace leaves the run intact, no wish outcome
- [x] `AgentRunReport.wish: Option<WishRunOutcome>` — `WishAssessment` + cross-run `AttractorStatus` + `diverged` (this run raised the distance) + `agenda()` (unmet facets = prioritized remaining work)
- [x] Contraction enforced live: `wish_diverging()` / `WishRunOutcome::diverged` surface a regression away from the attractor fail-closed (a driver loop can halt / roll back)
- [x] `wish_trace()` / `wish_assessment()` accessors
- [x] 5 tests (kosmo-agent 12 → 17) incl. end-to-end divergence detection across two real `cargo metadata` scans (rename the wished crate away → distance rises ZERO → ONE → Diverging)

**Next rung (the generation half):** turn the agenda into action — feed `WishRunOutcome::agenda()` into the synthesizer as facet-directed work (e.g. a new `ActionItemKind::RealizeWishFacet`), so the loop doesn't just *measure* the gap to the wish but *builds toward* closing it, and rejects any synthesized patch that makes the session diverge. That is the step where the repair loop becomes a build-toward-intent loop.

### `kosmo-intent` — connect the wish ruler to the real workspace (2026-06-02)

The third rung of the wish-to-system arc. Runs 1–2 measured a wish against a hand-supplied observation; this crate reads a *real* workspace and turns it into one, then ties target + ruler + convergence contract into a stateful session.

**`kosmo-intent`** (new crate; deps: `kosmo-core` + `kosmo-parseback`):
- [x] `facets_from_snapshot` / `observe_snapshot` / `observe_workspace(root)` — read-only adapter (one `cargo metadata` via `kosmo-parseback`) → `ObservedTopology` of `Crate` facets; crate names are scope-independent, so it uses `AffectedFilesOnly` for speed
- [x] `WishSession` — stateful descent: `observe()` assesses (Run 1) + appends distance; `trace()` exposes the `WishConvergenceTrace` (Run 2); `is_contractive()` / `at_attractor()` / `is_converged()`; a rising distance is `Diverging` (fail-closed). Serde-round-trippable (persist & resume)
- [x] Known boundary: `Module` / `Symbol` facets need a name-preserving source extractor (`CodeHDAG` keeps only node *labels*, not symbol names) — a later run; the facet-set API merges new sources without an interface change
- [x] 8 tests incl. a live `cargo metadata` scan of the real workspace (graceful skip if cargo unavailable); ranks-never-gates

**Next rung:** wire `WishSession` into the agent loop — feed `WishAssessment::unmet_facets` into the synthesizer's action queue so the loop builds *toward the wish*, and reject any patch that makes the session `Diverging` (enforce the contraction invariant live).

### `kosmo-core::attractor` — the wish as a fixed-point attractor (2026-06-02)

Turns "the compressor converges" from a claim into a checkable contract. The wish is the attractor `x*`; the Run-1 distance `V` is a Lyapunov function (`V ≥ 0`, `V = ZERO` only at `x*`); a trajectory converges iff `V` is monotone non-increasing and reaches `ZERO`.

**`kosmo-core::attractor`** — pure types over the Run-1 distance:
- [x] `WishConvergenceTrace` — content-addressed, evidence-bound (CROSS-006) distance trajectory (`Vec<Q16>`, oldest first); derives `AttractorStatus` + `first_divergence`
- [x] `AttractorStatus` = Converged / Converging / Stalled / Diverging / Indeterminate; `ConvergenceStep` = Contracting / Stalled / Diverging per transition
- [x] Contraction invariant = intent-axis analogue of LPCM `monotone_contractive_filter`: a step increasing `V` is a regression → `is_contractive()` false, offending index recorded (fail-closed, the loop rejects the patch)
- [x] `at_attractor()` + fixed-point stability (`f(x*) = x*`): extending a converged trace with `ZERO` stays converged
- [x] `from_assessments(&[WishAssessment])` builds a trajectory from Run-1 output; mixed-wish → `Indeterminate` empty trace
- [x] `MAX_STRICT_CONTRACTION_STEPS = 65537`: `Q16` discreteness bounds a strictly-contracting trajectory's length — finite convergence as a counting argument
- [x] 18 tests (kosmo-core 360 → 378); ranks-never-gates; zero new deps

**Next rung:** the pipeline-scan adapter — populate `ObservedTopology` from the live scan (parse-back snapshot + resolved voids), assess against a user `Wish`, feed `unmet_facets` into the synthesizer's action queue, and accumulate a `WishConvergenceTrace` across runs so the loop can refuse any step that diverges from the attractor.

### `kosmo-core::wish` — Wunsch-zu-System seed: intent as a measurable target (2026-06-02)

The first rung of the wish-to-system arc. Until now the substrate measured voids against *implicit* structural completeness; `Wish` makes the target *explicit*, so the same convergence loop can later descend toward a stated intent instead of merely "be whole". This run ships the target type and the distance function only — wiring into the pipeline/agent loop is a later run.

**`kosmo-core::wish`** — pure, dependency-free types + one pure function:
- [x] `Wish` — content-addressed (`id = SHA-256(JCS(content))`), evidence-bound (CROSS-006) desired topology; predicates sorted by facet + de-duplicated, so predicate order never affects `id`
- [x] `WishFacet` / `WishFacetKind` (`Crate`/`Module`/`Symbol`/`Capability`/`Resolution`) — positive-only targets; "the bad thing is gone" → a `Resolution` facet (a wish is never satisfied by absence of evidence)
- [x] `WishPredicate` (`WishFacet` + `Q16` weight; `require` = unit weight, `weighted` clamps negatives to `ZERO`)
- [x] `ObservedTopology` — caller-supplied set of present facets (live-pipeline adapter is a later run)
- [x] `assess_wish(wish, observed, evidence) -> WishAssessment` — pure/deterministic; `distance: Q16` = weighted unmet fraction (`ZERO` ⇒ realized, `ONE` ⇒ nothing met); `unmet_facets` = remaining gradient; `WishClosureStatus` = Realized/Approaching/Unstarted/Vacuous
- [x] Doctrine: distance *ranks*, never *gates* (CROSS-010 on the intent axis); fail-closed (a facet not positively observed is unmet)
- [x] 21 tests (kosmo-core 339 → 360); zero new dependencies

**Next rung:** an adapter that populates `ObservedTopology` from the live pipeline scan (parse-back snapshot + resolved-void set), then `assess_wish` against a user-supplied `Wish` so `unmet_facets` feeds the synthesizer's action queue — turning the repair loop into a build-toward-intent loop.

### Git-commit-per-patch + PromotionFeedback loop — Compressor live (2026-06-02)

**Git-commit layer** (`kosmo-materialize`, `kosmo-agent`, `kosmo-run`):
- [x] `MaterializeOptions::git_commit: bool` — `AppliedToHost` path runs `git add -A && git commit -m "kosmo-agent: apply patch {short} ({n} file(s))\n\npatch-id: {full}"` in the workspace root; fail-open (error → `diagnostics`, patch stays on disk)
- [x] `MaterializeReport::commit_sha: Option<String>` → `MaterializationAttempt::commit_sha` → `AgentRunReport` step; shown in `kosmo-run` text output
- [x] `AgentOptions::commit_to_git: bool` (default `false`) threads the flag into `MaterializeOptions`
- [x] `kosmo-run --commit` (requires `--apply`) — each accepted patch lands as its own revertable git commit

**PromotionFeedback loop** (`kosmo-agent`, `kosmo-pipeline`):
- [x] `AgentSession` accumulates `PromotionFeedback` records per synthesized step: `Accepted` if validation passed, `Rejected` if not; keyed on `ActionItemKind::PromoteToPse { candidate_id }` or `ApplyNorm { norm_candidate_id }` (others use `Digest::ZERO`)
- [x] `WorkspacePipelineSession::extend_prior_feedback()` — new method that appends records to `options.prior_feedback`
- [x] At the start of every `run()` the pending records are drained into the pipeline session, updating `NormFitnessTrace` scoring before the next scan — "Wissen zurück ins Substrat" closed at the agent layer
- [x] `AgentSession::pipeline_feedback_pending()` exposes queue depth
- [x] 2 new tests: `pipeline_feedback_queued_after_synthesized_steps`, `pipeline_feedback_drained_into_next_run` (12 agent tests total)

**Compressor invocation:** `kosmo-run --provider cerebras --apply --commit --max-steps N .` iterates scan → synthesize → validate → commit → re-scan → feedback re-ranks → convergence.

### `kosmo-materialize` — Write/Validate Layer & Closed Loop (2026-06-02)

The agent's `dry_run = false` path is now armed: patches reach disk under policy control, get compiled/tested, and are kept or rolled back. This closes the production loop end to end.

**`kosmo-materialize`** — policy-gated patch application:
- [x] `Materializer::materialize(patch, policy, validator, options)` → content-addressed `MaterializeReport`
- [x] Fail-closed strategy: `ReportOnly` → `SkippedByPolicy` (no I/O); `DryRun` → sandbox (copy workspace minus `target`/`.git` to temp, apply, validate, host untouched); `OperatorApproved`/`AutonomousBounded` + `allow_host_write` → in-place (backup touched files → apply → validate → keep or **rollback**); host-write mode without `allow_host_write` → blocked
- [x] `MaterializeOutcome`: SkippedByPolicy / SandboxValidated / SandboxRejected / AppliedToHost / RolledBack; `applied_to_host` true only on net persistence
- [x] `PatchValidator` trait; `CargoFoundryValidator` runs `cargo check` (+ optional `cargo test`) via `kosmo-foundry`'s hardened sandbox/timeout/env-scrub; `AlwaysPass`/`AlwaysFail` stubs for hermetic tests
- [x] CROSS-006: `evidence_bundle_id` never ZERO across all outcomes; INVARIANT-007: deterministic `report_id`; CROSS-002/013: no net host mutation unless validation passes
- [x] Backup/restore round-trips create/modify/delete; 11 tests (skip, sandbox pass/reject, apply-on-success, rollback of modified/created/deleted files, blocked-without-host-write, determinism, evidence-non-zero, apply/restore round-trip)
- [x] deps: `kosmo-foundry`; registered as workspace member

**`kosmo-agent` wired**:
- [x] `AgentSession::with_validator(Arc<dyn PatchValidator>)` — non-dry-run branch applies + validates via `kosmo-materialize`; `MaterializationAttempt::from_materialize_report` maps the result; failed validation rolls back and records negative feedback; filesystem errors fail closed (skip + continue)
- [x] `AlwaysPass`/`AlwaysFail`/`CargoFoundryValidator` re-exported from `kosmo-agent`
- [x] 2 new tests: real materialization applies via passing validator; rolls back via failing validator (10 agent tests total)

**`kosmo-run --apply`**:
- [x] Escalates to `OperatorApproved` + `CargoFoundryValidator`; writes validated patches to the workspace and rolls back any that fail cargo; default stays dry-run (report-only, no writes)

### `kosmo-synthesizer-llm` + `kosmo-run` — Real LLM Backends & Agent Runner (2026-06-02)

The synthesizer trait gets two production backends and the agent loop gets a CLI driver — the "loslegen ohne forschen" entry point for the execution layer.

**`kosmo-synthesizer-llm`** — Claude + OpenAI-compatible backends:
- [x] `LlmSynthesizer` implements `ActionSynthesizer` over `LlmProvider::Anthropic` (Messages API) and `LlmProvider::OpenAiCompatible` (`/chat/completions`)
- [x] `LlmConfig::claude()` / `::cerebras()` / `::openai_compatible()` + `with_model` / `with_max_tokens` / `with_temperature_milli` / `with_timeout_secs`
- [x] `LlmSynthesizer::from_env()` — provider auto-detect (`ANTHROPIC_API_KEY`→Claude, `CEREBRAS_API_KEY`→Cerebras); `KOSMO_LLM_PROVIDER` / `_API_KEY` / `_MODEL` / `_BASE_URL` overrides
- [x] Pure offline-testable core: `system_prompt()`, `build_user_prompt()`, `extract_json_object()` (brace-balanced, string-literal-aware, fence/prose tolerant), `parse_synthesis_response()`
- [x] Wire schema: one JSON object `{ rationale, confidence_pct (int 0-100), test_hint, files:[{path,op,content}] }`; `confidence_pct` → `Q16::ratio` so no float crosses the boundary (CROSS-007 respected; temperature float lives only inside the outbound request body)
- [x] Per-provider request body + headers + content extraction; 429/5xx retry with exponential backoff (4s/8s/16s); transient vs permanent `SynthesisError` classification
- [x] Non-determinism is contained at the LLM call; returned `Patch`/`SynthesisResult` are content-addressed (INVARIANT-007)
- [x] 14 tests (endpoints, per-provider bodies/extraction, prompt content, JSON extraction incl. braces-in-strings, confidence clamping/default, delete-op mapping, determinism, empty-key fast-fail) + 1 `#[ignore]` live Cerebras smoke test
- [x] deps: `reqwest` (blocking, json); registered as workspace member

**`tools/kosmo-run`** — the agent runner CLI:
- [x] `kosmo-run [OPTIONS] [PATH]` — `--provider claude|cerebras|mock|env`, `--model`, `--max-steps`, `--min-confidence <pct>`, `--all`, `--capacity`, `--json`, `--no-color`
- [x] Builds the synthesizer (env keys or flags), runs `AgentSession` dry-run, renders the ranked queue: per-step kind, confidence %, file/line/token counts, rationale, verify hint, per-file change kind, materialization status
- [x] `--json` emits the full content-addressed `AgentRunReport`; exit code 2 when the pipeline gate rejects
- [x] `mock` provider runs fully offline (no key) — instant "try the loop" path
- [x] Dry-run only: report-only policy, no host writes; real materialization still deferred to a future `kosmo-materialize`

### `kosmo-synthesizer` + `kosmo-agent` — Closed-Loop Execution Layer (2026-06-02)

The agent/synthesis stack closes the loop from ranked `ActionItem` → patch proposal → dry-run materialization → feedback record.

**`kosmo-synthesizer`** — pluggable synthesis abstraction:
- [x] `ActionSynthesizer` trait: `synthesize(&SynthesisRequest) -> Result<SynthesisResult, SynthesisError>`; `name() -> &str`; `token_budget() -> u32` (default 4096)
- [x] `SynthesisRequest` content-addressed from `(action_id, workspace_path_hash, policy_id)` — same action on same workspace always yields the same `request_id`
- [x] `FileChange { path, kind: Create/Modify/Delete, content }` with `line_count() -> u32`
- [x] `Patch::new` — sorts file changes by path for canonical ordering (INVARIANT-007); content-addressed from `(request_id, changes_hash)`
- [x] `SynthesisResult` content-addressed from `(patch_id, confidence_raw)`; carries `rationale`, `confidence: Q16`, `test_hint`, `tokens_used`
- [x] `SynthesisError { message, recoverable }` — `permanent()` / `transient()` constructors
- [x] `MockSynthesizer::confident()` (Q16 0.90) / `::uncertain()` (Q16 0.30) / `.with_change(FileChange)`
- [x] 9 tests (patch determinism, confidence levels, line counts, canonical ordering, workspace-path sensitivity)

**`kosmo-agent`** — stateful closed-loop runner:
- [x] `AgentOptions { max_steps, min_confidence, dry_run, pipeline_options }` — `Default` uses `report_only()` + dry_run=true + min_confidence=HALF + max_steps=5
- [x] `ValidationResult::dry_run()` → `GateResult::Warn { "dry-run: patch recorded but not validated" }`
- [x] `MaterializationAttempt` content-addressed from `(patch_id, applied)`; carries validation, blocking_reason, lines_added
- [x] `ExecutionFeedback` content-addressed from `(action_id, materialization_id, is_positive)`; `is_positive` true when validation gate is acceptable
- [x] `AgentStep { step_number, action, synthesis, materialization, feedback }`
- [x] `AgentRunReport` content-addressed from `(workspace_hash, step feedback IDs)`; tracks synthesized/skipped/materialized/lines-proposed counters
- [x] `AgentSession::run(workspace)` flow: pipeline → rank actions → for-each (synthesize → confidence-filter → dry-run attempt → feedback) → report
- [x] `AgentSession::feedback_history()` — accumulated across repeated `run()` calls
- [x] Non-dry-run hook present; real materialization deferred to future `kosmo-materialize` crate
- [x] 8 tests (dry-run report, max_steps, confidence filtering, deterministic run_id, lines-proposed sum, applied=false, feedback positivity, cross-run accumulation)

### `kosmo-server` — HTTP Server + Browser UI (2026-06-02)

REST API server with embedded single-page app — the "jeder der das sieht" entry point.

- [x] `GET /` — embedded browser UI (GitHub-dark theme, ~300 lines HTML/CSS/JS, no build step)
- [x] `GET /api/health` — version ping
- [x] `POST /api/analyse` — JSON request `{ path, flags }` → structured `AnalyseResponse` DTO
- [x] `AnalyseResponse`: gate, stats, action queue rows (all), void ranking, optional layer counts, crystal counts
- [x] Browser UI: path input + Enter key, per-layer checkboxes, spinner during request, elapsed time
- [x] Gate badge coloured by result (Pass=green, Warn=yellow, Reject=red)
- [x] Action queue table with kind badges colour-coded by group
- [x] Crystal CAD library section + optional layers section + void ranking chips (conditional)
- [x] `--port <n>` / `--host <addr>` / `--open`; pipeline on `spawn_blocking` thread
- [x] `tools/kosmo-server` registered as workspace member; deps: `axum`, `tokio`, `serde`, `serde_json`

### `kosmo-tui` — Interactive Terminal Dashboard (2026-06-02)

Full-screen ratatui TUI for navigating the action queue interactively.

- [x] Three-pane layout: action list │ detail │ workspace stats, with header + status bar
- [x] Scrollable action queue with per-kind colouring and scroll % indicator
- [x] Detail pane: kind, score, target ID, action ID, word-wrapped description
- [x] Header: path, policy, gate (coloured), run counter
- [x] Keybindings: q/Esc/Ctrl+c, r=rerun, ↑↓/jk, PgUp/PgDn, g/G
- [x] `r` rerun: "Analysing…" transition frame → pipeline re-run → selection reset
- [x] Same flag surface as `kosmo-substrate` (all layers, store, operator, capacity)
- [x] deps: `ratatui 0.29`, `crossterm 0.28`; no additional workspace dep changes

### `kosmo-substrate` CLI Binary — Workspace Topology Analysis (2026-06-02)

The full pipeline is now accessible as a standalone CLI binary — no Rust knowledge required to run it.

- [x] `kosmo-substrate [OPTIONS] [PATH]` — zero external deps (manual arg parsing, raw ANSI, serde_json)
- [x] `--output text` — rich terminal UI: box-drawing header, action queue, crystal CAD library section, void priority ranking, optional layers section; top-20 action items shown inline
- [x] `--output json` — full `IntegrationRunReport` serialized as pretty-printed JSON
- [x] `--output summary` — single-line CI-friendly output: `GATE | workspace=... | voids=N | actions=M | ...`
- [x] `--store <path>` — persistent cross-session CAD library with tilde expansion; implies `--crystals`
- [x] `--operator` — OperatorApproved policy (enables crystal persist); default is ReportOnly
- [x] `--all` / `--metatron` / `--lpcm` / `--systemcube` / `--surgery` / `--crystals` / `--norms` / `--motifs` / `--pse`
- [x] `--fail-on-reject` / `--fail-on-warn` — exit code 1 for CI gate integration
- [x] `--capacity <n>` — SystemCube D-density denominator (default 100)
- [x] `vlen()` — ANSI-aware visual-width measurement for correct box alignment with colored values
- [x] `gate_str()`, `kind_label()`, `kind_color()` — colored terminal formatting
- [x] `WorkspacePipelineSession` used internally; session run counter shown in header
- [x] `tools/kosmo-substrate` registered as workspace member

### `ActionItem` — CAM Layer: Report → Ranked Actionable Directives (2026-06-01)

The pipeline now distills its rich diagnostic output into a single unified, priority-ranked work queue — completing the "CAD/CAM" metaphor.

- [x] `ActionItemKind` enum: `FillVoid`, `RepairTopology`, `PromoteToPse`, `ReviewCrystal`, `ApplyNorm`
- [x] `ActionItem` struct: content-addressed `action_id`, `priority_score: Q16`, `kind`, `description`, `policy_id`
- [x] `rank_score(pos, total)` — position-based Q16 priority (`ONE` for top, proportionally decreasing)
- [x] `IntegrationRunReport::action_items()` — aggregates all five categories, sorts merged list descending by `priority_score`
- [x] `ReviewCrystal` items only for `EvidenceOnly` candidates (not `Pending`/`Certified` — no operator action needed)
- [x] 7 new pipeline tests (120 total); 2 new eval scenarios (147 total, 927 substrate tests)

### `run_workspace_pipeline` + `WorkspacePipelineSession` — Filesystem Entry Point (2026-06-01)

The pipeline is now callable with a single filesystem path — no manual `WorkspaceIndex` construction needed.

- [x] `run_workspace_pipeline(root, options, policy)` — scans with content (HDAG extraction), delegates to `run_dry_pipeline`; returns `Result<IntegrationRunReport, WorkspaceError>`
- [x] `WorkspacePipelineSession::new(options, policy)` — stateful session across multiple runs
- [x] `WorkspacePipelineSession::run(root)` — each call scans, runs pipeline, auto-persists crystals
- [x] `WorkspacePipelineSession::run_count()` — monotonic run counter
- [x] `WorkspaceError` re-exported from `kosmo-pipeline` for ergonomic error handling
- [x] 6 new pipeline tests (113 total); 2 new eval scenarios (145 total, 909 substrate tests)

### Pipeline Step 5f — Crystal Auto-Persistence (2026-06-01)

The full session-to-session CAD library loop is now automatic at the pipeline level.

- [x] `IntegrationRunOptions::crystal_store_path: Option<PathBuf>` — opt-in store path (`#[serde(skip)]`)
- [x] `IntegrationRunOptions::with_crystal_store_path(path)` — builder method
- [x] **Step 5f pre-run**: on entry, if path exists, open store → merge records into effective `prior_crystals` (dedup by `record_id`)
- [x] **Step 5f post-cert**: after Step 5d-cert, append certified crystals to store (policy-gated; ReportOnly/DryRun denied)
- [x] `IntegrationRunReport::persisted_crystal_count: u32` — observational field, NOT in `report_id` hash
- [x] `summary()` updated: `(certified: N, resonites: M, persisted: P)`
- [x] 5 new pipeline tests (107 total); 2 new eval scenarios (143 total, 896 substrate tests)

### `CrystalRecordStore` — Durable CAD Library Persistence (2026-06-01)

Crystal records now survive across integration runs: the CAD library is a JSONL-backed
append-only store with the same host-write policy invariant as `JsonlCartographyStore`.

- [x] `StructuralCrystalRecord::verify_id()` — recomputes and verifies content-addressed `record_id`
- [x] `CrystalRecordStore::open(path)` — replay JSONL, verify every record_id on open
- [x] `CrystalRecordStore::append(record, policy)` — policy-gated (ReportOnly/DryRun denied), dedup by `record_id`, fsync
- [x] `CrystalRecordStore::records()` — `&[StructuralCrystalRecord]` for `IntegrationRunOptions::prior_crystals`
- [x] `CrystalRecordStore::verify_integrity()` — re-verify all records after reload
- [x] `CrystalStoreError` enum — manual Display/Error impl (no thiserror dep)
- [x] `kosmo-hyphae` dependency added to `kosmo-store`
- [x] 7 new store tests (14 total); 1 new eval scenario (141 total, 886 substrate tests)

### Crystal-Boosted SourceCube Scoring — `crystal_resonance` Dimension (2026-06-01)

The CAD library now actively influences energy ranking of current-run SourceCubes.

- [x] Pipeline Step 2b: `crystal_resonance` dimension added to `CubeDimensionProfile` when `prior_crystals` is non-empty AND source content (HDAG) is available
- [x] Best structural proximity to any prior crystal (same rho/omega formula as `Resonite::from_records`) used as the dimension value
- [x] Only set when `> ZERO` — no false-zero baseline; runs without prior crystals are unchanged
- [x] `crystal_resonance` contributes to ρ (coherence) in tripolar energy → pattern-matched voids rank higher
- [x] 2 new pipeline tests (102 total); 1 new eval scenario (140 total, 872 substrate tests)

### Crystal Structural Fingerprint + Resonite Pipeline Wiring (2026-06-01)

CAD library elements now carry code-structure provenance; cross-run pattern proximity is measured via Resonite.

- [x] `StructuralCrystalCandidate`: `source_void_id`, `rho_coherence`, `omega_phase` — all in `candidate_id` hash
- [x] `from_decision_with_signals(decision, void_id, rho, omega)` — HDAG signal injection at candidate creation
- [x] `StructuralCrystalRecord`: `source_void_id`, `rho_coherence`, `omega_phase` — all in `record_id` hash
- [x] `from_certificate(cert, candidate)` — structural provenance propagated from candidate to record
- [x] `Resonite::from_records(a, b, policy_id)` — structural proximity: `((ONE-|ρ_diff|) + (ONE-|ω_diff|)) / 2`; symmetric, Q16 (CROSS-007)
- [x] Pipeline Step 5d: `from_decision_with_signals` with HDAG signals from `hdag_by_void_id`
- [x] Pipeline Step 5e-resonite: pairwise `Resonite` between current and prior crystals; `resonite_count` in `report_id`
- [x] `IntegrationRunReport.resonite_map: Vec<Resonite>` — covered by `verify_policy_consistency`
- [x] 6 new `crystal.rs` tests (197 total); 6 new pipeline tests (100 total); 3 new eval scenarios (139 total, 855 substrate tests)

### CodeHDAG Pipeline Integration — Code-Structure-Aware Topology (2026-06-01)

Deepens void detection and energy assessment from file-presence to code-structure. Controlled
by `WorkspaceIndex::scan_path_with_content` — passive by default, opt-in via entry `content`.

- [x] `WorkspaceEntry.content: Option<String>` (`#[serde(skip)]`) — source text for HDAG extraction; excluded from `index_id` (digest already content-addresses bytes)
- [x] `WorkspaceIndex::scan_path_with_content(root, policy_id)` — populates `.content` for `.rs` source/test files (valid UTF-8 only)
- [x] `HostCube.hdag_by_void_id: BTreeMap<Digest, CodeHDAG>` — HDAG keyed by void_id; `hdag_count` in `HostCubeContent` participates in `cube_id`
- [x] `MissingTestFiber` severity scales with HDAG definition count: `HALF + HALF × min(N, 8) / 8`
- [x] Pipeline Step 2b: `SourceCube.dimension_profile` enriched with `rho_coherence` + `omega_phase` from `CodeHDAG` when content available
- [x] `IntegrationRunReport.source_cubes: Vec<SourceCube>` — SourceCubes exposed in report
- [x] `CubeDimensionProfile::from_raw_map(BTreeMap<String, Q16>)` — new constructor for raw-key dimension maps
- [x] 4 new `host.rs` HDAG tests; 1 `cube.rs` test; 3 pipeline SourceCube tests; 2 new eval scenarios `rx-hyphae-hdag-*` (136 total, 825 substrate tests)

### Crystal Certification Pipeline — `StructuralCrystalRecord` (2026-06-01)

Closes the gap between `StructuralCrystalCandidate` (pending, `support_score = Q16::ZERO`) and
`StructuralCrystalRecord` (certified, durable CAD library element). Adds cross-run accumulation
via `prior_crystals` seeding.

- `ConstraintProgram::from_candidate(candidate, replay_status)` — evaluates standard 5-constraint
  program from candidate ID fields alone (no `EvidenceBundle` object required at pipeline step 5d-cert)
- `StructuralCrystalCandidate::certify(replay_status) -> Option<(AssimilationCertificate, StructuralCrystalRecord)>` — single-call certification: program → proof → certificate → record
- `CorpusEntityKind::CrystalRecord` variant — certified crystal records are first-class corpus
  entities, enabling cross-run accumulation in the persistent cartography
- Pipeline Step 5d-cert: every `Pending` candidate from the current run is certified;
  `certified_crystals: Vec<StructuralCrystalRecord>` in `IntegrationRunReport`
- `certified_crystal_count: u32` in `ReportContent` — participates in `report_id` content-addressing
- `IntegrationRunOptions.prior_crystals: Vec<StructuralCrystalRecord>` — seed corpus with crystal
  records from prior runs (CAD library accumulation loop closed)
- `verify_policy_consistency()` covers `certified_crystals[i].policy_id`
- `summary()` reports `crystal_candidates: N (certified: M)`
- 4 new `crystal.rs` tests (186 total); 4 new pipeline tests (91 total); 2 new `RX:Crystal` /
  `RX:Pipeline` eval scenarios (134 total, 815 substrate tests)

### `AssimilationLedger` — Content-Addressed Decision Audit Log (2026-06-01)
- [x] `AssimilationLedger { ledger_id, run_id, events, policy_id }` — sequenced, content-addressed log of all `AssimilationDecision`s in a run (INVARIANT-007)
- [x] Two-pass construction: placeholder `run_id` → get `ledger_id` → real `run_id` seals over `ledger_id`; makes `run_id` sensitive to decision outcomes
- [x] `HyphaeRunResult.ledger: AssimilationLedger` — every passive run carries its full decision log
- [x] `RunContent.ledger_id` participates in `run_id` content-addressing
- [x] `ReportContent.hyphae_ledger_id` propagates ledger commitment into pipeline `report_id`
- [x] 4 new hyphae tests (182 total); 1 new eval scenario `RX:Hyphae` (132 total, 807 substrate tests)

### Motif Feedback Loop + `SuggestPattern` Yield Kind (2026-06-01)
- [x] `yield_for_intent` now selects yield kind from intent kind: `SuggestPattern` → `StructuralYieldKind::MotifProposal`; `ReduceDeficiency` → `DeficiencyFill`; others → `DeficiencyFill`
- [x] `SourceFrontierGraph::augmented_with_prior_motifs` — appends `SuggestPattern` intents for motifs meeting `min_support`; re-seals `graph_id`
- [x] `passive_run_augmented(index, policy, additional_intents)` — augments frontier with extra intents before gate processing; `passive_run` delegates to it with empty slice (backward-compatible)
- [x] `IntegrationRunOptions.prior_motifs: Vec<MotifCandidate>` + `prior_motif_min_support: Q16` — pipeline injects `SuggestPattern` intents from prior-run motifs, closing the cross-run feedback loop
- [x] `MotifCandidate` → `PseBridgeCandidate::StructuralObservation` in Step 6b — motif support signal reaches PSE evaluation
- [x] 2 new hyphae tests (178 total); 2 new pipeline tests (87 total); 3 new eval scenarios `RX:Hyphae`/`RX:Pipeline` (131 total, 803 substrate tests)

### Pipeline Step 5a: `MotifCandidate` from Void Kind Frequency (2026-06-01)
- [x] `enable_motif_candidates: bool` in `IntegrationRunOptions` (default false); included in `all_layers()`
- [x] Step 5a: one `MotifCandidate` per `HostVoidKind` variant observed; `support_score = kind_count / total_voids` (Q16 ratio); evidence = `hyphae.run_id` (CROSS-006: non-ZERO)
- [x] `motif_candidate_count` in `ReportContent` — participates in `report_id` content-addressing (INVARIANT-007)
- [x] `verify_policy_consistency()` covers `motif_candidates[i].policy_id`
- [x] `summary()` reports `motif_candidates: N`
- [x] 4 new pipeline tests (85 total); 2 new `RX:Pipeline` eval scenarios (128 total, 789 substrate tests)

### `ReduceDeficiency` Intents in Frontier + Spec §2.2 Yield Compliance (2026-06-01)
- [x] `SourceFrontierGraph::from_void_map` now generates both `FillVoid` intents (one per void) and `ReduceDeficiency` intents (one per deficiency kind) from the derived `DeficiencyVector`
- [x] `SourceFrontierGraph::from_void_map_and_deficiencies` — explicit constructor accepting a pre-computed `DeficiencyVector`
- [x] `yield_for_intent` propagates `deficiency_kind_ref` from `ReduceDeficiency` intents, satisfying spec §2.2 (a yield must reference a void OR a deficiency)
- [x] `ReduceDeficiency` yields are fully processed by the gate cascade → `AssimilationDecision` with non-ZERO id
- [x] 4 new hyphae tests (176 total — 3 frontier, 1 run); 3 new `RX:Hyphae` eval scenarios (126 total, 780 substrate tests)

### Hyphae `yield_for_intent` Taint/Authority Propagation (2026-06-01)
- [x] `yield_for_intent` now propagates `intent.taint.clone()` and `intent.authority.clone()` — no hardcoded overrides
- [x] A `TaintLabel::Clean` + `AuthorityLabel::Foundry` intent produces a yield that passes all gates → `Accepted` decision (fully open clean path)
- [x] `TaintLabel::Unverified` + `AuthorityLabel::Agent` intent (the `from_void_map` default) still produces `EvidenceOnly` — backward-compatible
- [x] 2 new hyphae tests (172 total); 1 new `RX:Hyphae` eval scenario (124 total, 777 substrate tests)

### Decision Taint Propagation to BlueprintUnit (2026-06-01)
- [x] `AssimilationDecision.taint: TaintLabel` — propagated from `StructuralYield.taint` via `from_trace()`; participates in `decision_id` content-address
- [x] `DecisionContent` updated to include `taint` field — different taints produce different `decision_id` (INVARIANT-007)
- [x] Pipeline Step 5e updated: `BlueprintUnit` uses `decision.taint.clone()` instead of hardcoded `TaintLabel::Synthetic`
- [x] Architecture is now open for non-Synthetic runs: a future `OperatorAssisted` run with Clean yields will produce `Accepted` (not `AcceptedWithTaint`) units and `compatibility_score = Q16::ONE`
- [x] 1 new hyphae test (170 total); 1 new pipeline test (81 total); 1 new `RX:Pipeline` eval scenario (123 total, 775 substrate tests)

### SystemCube Diagnostics Surfaced in Pipeline (2026-06-01)
- [x] `IntegrationRunReport::systemcube_contradiction_energy() -> Option<Q16>` — direct accessor, no drilling
- [x] `IntegrationRunReport::systemcube_compatibility_score() -> Option<Q16>` — direct accessor
- [x] `summary()` now includes `compat=<score>` and `contradiction_energy=<total>` in the systemcube field
- [x] SystemCube gate contribution: `Warn` when `compatibility.gaps` is non-empty (structural advisory, not energy-based); `Pass` when fully compatible
- [x] 3 new pipeline tests (80 total); 2 new `RX:Pipeline` eval scenarios (122 total, 772 substrate tests)

### CompatibilityProfileReport Real Gap Detection (2026-06-01)
- [x] `CompatibilityProfileReport::from_units(manifest_id, host_snapshot_id, policy, units)` — replaces `perfect()` stub in `export_dry_run`
  - Accepted units only; `AcceptedWithTaint` → `TaintedUnit` gap, severity `Q16::HALF`; `source_ref == Digest::ZERO` → `MissingSourceRef` gap, severity `Q16::ONE`
  - `compatibility_score = Q16::ONE − avg_gap_severity`, clamped to `[0, ONE]`
  - All-clean manifest → score = `Q16::ONE`; empty accepted set → `EmptyManifest` status
  - Gaps sorted by `unit_id`; deterministic regardless of input order (INVARIANT-007)
- [x] `SystemCube::export_dry_run` now calls `from_units` — every `KcubeExportReport` carries real compatibility diagnostics
- [x] 5 new `kosmo-systemcube` compatibility tests (54 total); 2 new `RX:Compatibility` eval scenarios (120 total, 769 substrate tests)

### ContradictionEnergyReport Real Detection (2026-06-01)
- [x] `ContradictionEnergyReport::from_units(manifest_id, policy, units)` — replaces `zero_energy` stub
  - Filters to accepted units only; iterates unit_id-ordered pairs (INVARIANT-007)
  - Same `source_ref`, same `kind` → `Duplicate`, weight `Q16::HALF`
  - Same `source_ref`, different `kind` → `RoleConflict`, weight `Q16::ONE`
- [x] `SystemCube::export_dry_run` now calls `from_units` — manifests with role conflicts surface non-zero total_energy
- [x] 5 new `kosmo-systemcube` energy tests (49 total); 2 new `RX:ContradictionEnergy` eval scenarios (118 total, 764 substrate tests)

### BlueprintUnit Energy Assessment — Step 5e (2026-06-01)
- [x] `BlueprintUnit::energy_assessment(gate)` added to `kosmo-systemcube`
  - ψ = `Q16::ONE` for Accepted/AcceptedWithTaint; `Q16::ZERO` for RejectedOpaque
  - Taint factor via `EnergyFactors::taint_factor(&self.taint)` — Synthetic reduces energy, Quarantined zeroes it
  - `evidence_bundle_id = self.unit_id` (self-referential, CROSS-006: always non-ZERO)
- [x] Pipeline Step 5e: `BlueprintUnit`s are energy-ranked before passing to `SystemCube::new` (accepted/clean first, tainted below)
- [x] 3 new systemcube tests (44 total); 2 new `RX:BlueprintEnergy` eval scenarios (116 total, 759 substrate tests)

### PseBridgeCandidate Pipeline Integration — Step 6b (2026-06-01)
- [x] `enable_pse_candidates: bool` in `IntegrationRunOptions` (default false)
- [x] Pipeline Step 6b: collects `PseBridgeCandidate` from `norm_candidates` (kind=`StructuralObservation`, ψ=`fitness_score`, evidence=`evidence_bundle_id`) and from `ambiguity_profiles` + `complement_void_hypotheses` (kind=`TopologyObservation`, ψ=`confidence_score`); sorted by confidence desc, then by id for determinism
- [x] `IntegrationRunReport.pse_candidates: Vec<PseBridgeCandidate>` — PSE submission work queue
- [x] `ReportContent.pse_candidate_count` participates in `report_id`
- [x] `verify_policy_consistency()` covers `pse_candidates[i].policy_id`
- [x] `summary()` reports `pse_candidates: N`
- [x] `kosmo-pse-bridge` added as dep to `kosmo-pipeline`
- [x] 3 new pipeline tests (77 total); 2 new `RX:Pipeline` eval scenarios (114 total, 756 substrate tests)

### DeficiencyVector Pipeline Integration — Step 1c (2026-06-01)
- [x] Pipeline Step 1c: `DeficiencyVector::from_void_map(&hyphae.host_cube.void_map)` — always present (not gated on any option)
- [x] `IntegrationRunReport.deficiency_vector: DeficiencyVector` — entries sorted by kind; `total_severity` is Q16 average
- [x] `ReportContent.deficiency_vector_id` participates in `report_id`
- [x] `verify_policy_consistency()` covers `deficiency_vector.policy_id`
- [x] `summary()` reports `deficiency: N entries (severity=...)`
- [x] 3 new pipeline tests (74 total); 2 new `RX:Pipeline` eval scenarios (112 total)

### StructuralCrystalCandidate Pipeline Integration — Step 5d (2026-06-01)
- [x] `enable_crystal_candidates: bool` in `IntegrationRunOptions` (default false)
- [x] Pipeline Step 5d: one `StructuralCrystalCandidate::from_decision(&decision)` per accepted decision; all start `support_score = Q16::ZERO` (Pending certification)
- [x] `IntegrationRunReport.crystal_candidates: Vec<StructuralCrystalCandidate>` — explicit certification work queue
- [x] `ReportContent.crystal_candidate_count` participates in `report_id`
- [x] `verify_policy_consistency()` covers `crystal_candidates[i].policy_id`
- [x] `summary()` reports `crystal_candidates: N`
- [x] 3 new pipeline tests (71 total); 2 new `RX:Pipeline` eval scenarios (110 total)

### TopologyAmbiguityProfile + ComplementVoidHypothesis Pipeline Integration — Step 3f (2026-06-01)
- [x] Pipeline Step 3f: flatten `ambiguities` and `void_hypotheses` from all `metatron_diagnostics`; energy-rank each by `confidence_score` (most confident first)
- [x] `IntegrationRunReport.ambiguity_profiles: Vec<TopologyAmbiguityProfile>` — empty when Metatron disabled
- [x] `IntegrationRunReport.complement_void_hypotheses: Vec<ComplementVoidHypothesis>` — empty when Metatron disabled
- [x] `ReportContent.ambiguity_profile_count` and `void_hypothesis_count` participate in `report_id`
- [x] `verify_policy_consistency()` covers both collections
- [x] `summary()` reports `ambiguities: N | void_hyp: M`
- [x] 3 new pipeline tests (68 total); 2 new `RX:Pipeline` eval scenarios (108 total)

### NormFitnessTrace Pipeline Integration — Step 5c (2026-06-01)
- [x] `prior_feedback: Vec<PromotionFeedback>` added to `IntegrationRunOptions` (default empty)
- [x] Pipeline Step 5c: for each `NormGeneCandidate`, fold matching feedback via `NormFitnessTrace::observe_from_feedback`; only traces with ≥1 observation included
- [x] `IntegrationRunReport.norm_fitness_traces: Vec<NormFitnessTrace>` — one per candidate with matched feedback
- [x] `ReportContent.norm_fitness_trace_count` participates in `report_id`
- [x] `verify_policy_consistency()` covers `norm_fitness_traces[i].policy_id`
- [x] `summary()` reports `norm_candidates: N (traces: M)`
- [x] 3 new pipeline tests (65 total); 2 new `RX:Pipeline` eval scenarios (106 total)

### SurgeryWorkbenchTask Pipeline Integration — Step 3e (2026-06-01)
- [x] Pipeline Step 3e: `surgery_workbench_tasks: Vec<SurgeryWorkbenchTask>` — 1:1 from `surgery_options` via `SurgeryWorkbenchTask::from_option()`; same energy-ranked order
- [x] `IntegrationRunReport.surgery_workbench_tasks` (empty when `surgery_options` is empty)
- [x] `ReportContent.surgery_workbench_task_count` participates in `report_id`
- [x] `verify_policy_consistency()` covers `surgery_workbench_tasks[i].policy_id`
- [x] `summary()` reports `surgery: N (tasks: M)`
- [x] 3 new pipeline tests (62 total); 2 new `RX:Pipeline` eval scenarios (104 total)

### MicroTopologyIndex Pipeline Integration — Step 3d (2026-06-01)
- [x] Pipeline Step 3d: fold `(micrograph, fingerprint, diagnostic)` triples into `MicroTopologyIndex` after metatron loop
- [x] `IntegrationRunReport.metatron_index: MicroTopologyIndex` (empty-state when Metatron disabled)
- [x] `ReportContent.metatron_index_id` participates in `report_id`
- [x] `verify_policy_consistency()` covers `metatron_index.policy_id`
- [x] `summary()` reports `metatron_index.index_id` prefix
- [x] 4 new pipeline tests (59 total); 2 new `RX:Pipeline` eval scenarios (102 total)

### TopologyAmbiguityProfile + ComplementVoidHypothesis energy_assessment (2026-06-01)
- [x] `TopologyAmbiguityProfile::energy_assessment(gate)` — ψ=`confidence_score`; `evidence_bundle_id=micrograph_id` (CROSS-006)
- [x] `ComplementVoidHypothesis::energy_assessment(gate)` — ψ=`confidence_score`; evidence = first non-ZERO `evidence_ids` entry, else `micrograph_id` (CROSS-006)
- [x] 4 new metatron.rs tests (169 hyphae tests total)

### SemanticLossRecord + MicrographLiftReport energy_assessment + Pipeline Step 3c (2026-06-01)
- [x] `SemanticLossRecord::energy_assessment(gate)` — ψ=`loss_ratio`; `evidence_bundle_id=region_id` (CROSS-006)
- [x] `MicrographLiftReport::energy_assessment(gate)` — ψ=`loss_ratio`; `evidence_bundle_id=micrograph_id` (CROSS-006)
- [x] Pipeline Step 3c: `lift_reports: Vec<MicrographLiftReport>` in `IntegrationRunReport` — one per void when Metatron enabled, energy-ranked by `loss_ratio`
- [x] `ReportContent.lift_report_count` participates in `report_id`
- [x] `summary()` reports lift_reports count alongside metatron diagnostics
- [x] 4 new metatron.rs tests + 3 new pipeline tests; 2 new `RX:Pipeline` eval scenarios (100 total)

### Resonite + CubeMandorla + CompositeSupportCube energy_assessment (2026-06-01)
- [x] `Resonite::energy_assessment(gate)` — ψ=`resonance_score`; symmetric (r(a,b)≡r(b,a)); `evidence_bundle_id=resonite_id`
- [x] `CubeMandorla::energy_assessment(gate)` — ψ=`overlap_score`; `evidence_bundle_id=mandorla_id`
- [x] `CompositeSupportCube::energy_assessment(gate)` — ψ=`aggregate_support`; `evidence_bundle_id=composite_id`
- [x] All three use self-referential content addresses as evidence (CROSS-006: no new fields needed)
- [x] 3 crystal.rs tests + 4 swarm.rs tests (6 total new tests)

### NormGeneCandidate Pipeline Integration — Step 5b (2026-06-01)
- [x] `enable_norm_candidates: bool` in `IntegrationRunOptions` (default false)
- [x] Pipeline Step 5b: one `NormGeneCandidate` per accepted decision; initial `fitness_score = Q16::ONE`; `evidence_bundle_id = decision.evidence_bundle_id` (CROSS-006)
- [x] `IntegrationRunReport.norm_candidates: Vec<NormGeneCandidate>` (energy-ranked)
- [x] `ReportContent.norm_candidate_count` participates in `report_id`
- [x] `verify_policy_consistency()` covers `norm_candidates[i].policy_id`
- [x] `summary()` reports `norm_candidates` count
- [x] 3 new pipeline unit tests (52 total); 2 new `RX:Pipeline` eval scenarios (98 total)

### Void Priority Ranking — Pipeline Step 1b (2026-06-01)
- [x] `HostVoid::energy_assessment(gate, policy_id)` — ψ=`severity`; `evidence_bundle_id=void_id` (CROSS-006 self-referential content address)
- [x] `TopologicalVoidMap::priority_ranking(gate)` — severity-ordered void repair queue via `rank_by_energy`; ties broken by `void_id`
- [x] Pipeline Step 1b: `void_priority_ranking: Vec<Digest>` in every `IntegrationRunReport`
- [x] `ReportContent.void_priority_count` participates in `report_id`
- [x] `summary()` reports `voids: N (priority ranked)`
- [x] 5 new void_map.rs unit tests; 2 new `RX:Pipeline` eval scenarios (96 total, 709 substrate tests)

### Surgery Energy Assessment + Pipeline Step 3b (2026-06-01)
- [x] `TopologicalSurgeryOption::energy_assessment(gate)` — ψ=`confidence_score`; `evidence_bundle_id=diagnostic_id` (CROSS-006)
- [x] Pipeline Step 3b: energy-ranked surgery options from Metatron diagnostics
- [x] `IntegrationRunOptions.enable_surgery: bool` (default false; requires `enable_metatron`)
- [x] `IntegrationRunReport.surgery_options: Vec<TopologicalSurgeryOption>` (energy-ranked)
- [x] `verify_policy_consistency()` covers surgery options
- [x] 4 new surgery.rs unit tests; 3 new `RX:Pipeline` eval scenarios (94 total)

### from_host_and_composite removed + MorphogenicCorpusUpdate as Step 4d (2026-06-01)
- [x] `HostTargetDelta::from_host_and_composite` removed (raw `max_by_key` violation); tests migrated to `from_source_cubes`
- [x] Pipeline Step 4d: `MorphogenicCorpusUpdate::skeleton(cartography_update_id, collapse_plan_id, policy_id)`
- [x] `IntegrationRunReport.morphogenic_update` + `verify_policy_consistency()` + `summary()` + `ReportContent`
- [x] 2 new `RX:Pipeline` eval scenarios (91 total)

### JsonlCartographyStore Persistence in Pipeline (2026-06-01)
- [x] `CartographyEntryKind::CartographyUpdate` added to kosmo-core
- [x] `kosmo-store` dep added to kosmo-pipeline
- [x] `persist_cartography_update(update, path, scope, policy)` in new `persistence` module
- [x] Policy-gated: fails with `PolicyDenied` if `allow_host_write == false`
- [x] CROSS-006: `evidence_bundle_id = update.update_id` (non-ZERO content ref)
- [x] 3 unit tests in persistence.rs; 2 new `RX:Pipeline` eval scenarios (89 total)

### StructuralCrystalCandidate Energy Assessment (2026-06-01)
- [x] `energy_assessment(gate)` added to `StructuralCrystalCandidate`
- [x] taint = Q16::ONE (quarantined yields rejected at gate cascade, never candidates)
- [x] 3 new crystal.rs unit tests; 2 new `RX:EnergyRanking` scenarios (87 total)

### Phase 4c HostTargetCollapsePlan Pipeline Weld (2026-06-01)
- [x] `run_dry_pipeline` Step 4c: `HostTargetCollapsePlan::from_delta(&void_fill_delta, policy.id)` — zero-cost, no host writes
- [x] `IntegrationRunReport` carries `collapse_plan: HostTargetCollapsePlan`
- [x] `ReportContent` gains `collapse_plan_id` — collapse plan changes the report_id
- [x] `verify_policy_consistency()` covers `collapse_plan.policy_id`
- [x] `summary()` reports step count and `CollapsePlanStatus`
- [x] 3 new `RX:Pipeline` eval scenarios (85 total): PlanningOnly status, policy traceability, content-addressing

### MotifCandidate Policy Alignment + SeamGraph → Energy Wiring (2026-06-01)
- [x] `MotifCandidate` gains `policy_id` field, updated content addressing, `energy_assessment(gate)`
- [x] `SourceCube::energy_assessment` gains `seam_coherence: Q16` parameter
- [x] `HostTargetDelta::from_source_cubes` gains `seam_map: &BTreeMap<Digest, Q16>`
- [x] Pipeline Step 4b moved after LPCM; seam_map built from lpcm_reports
- [x] 2 new `RX:EnergyRanking` scenarios (82/82 total); 682 substrate tests, 0 failures

### Phase 4 CubeSwarm + HostTargetDelta Pipeline Integration (2026-06-01)
- [x] `run_dry_pipeline` Step 2b: accepted decisions → SourceCubes → CubeSwarm → CompositeSupportCube
- [x] `HostTargetDelta::from_source_cubes` called (energy-ranked, not raw Q16)
- [x] `IntegrationRunReport` carries `swarm_composite` + `void_fill_delta`
- [x] `verify_policy_consistency()` covers swarm + delta policy_ids
- [x] 4 `RX:Pipeline` scenarios all pass (80/80 total)

### Energy Kernel Adoption in Selection Paths (2026-06-01)
- [x] `SourceCube::energy_assessment(gate, license, foundry)` — ψ=support_score, ρ=avg profile coverage
- [x] `NormGeneCandidate::energy_assessment(gate)` — ψ=fitness_score, fail-closed gate
- [x] `HostTargetDelta::from_source_cubes` replaces raw `.max_by_key` with `rank_by_energy`
- [x] 4 `RX:EnergyRanking` scenarios all pass (76/76 total)
- [x] Proven: quarantined taint beats higher raw support_score in ranking

### PSE Feedback Loop — "Wissen zurück ins Substrat" (2026-06-01)
- [x] `FeedbackOutcome` + `PromotionFeedback` in `kosmo-core` (circular dep avoidance)
- [x] `fitness_signal` mapping: Accepted→energy, Deferred→¼, Rejected/Skipped→0
- [x] `CartographyEntryKind::PromotionFeedback` variant
- [x] `build_promotion_feedback` in `kosmo-pse-bridge`
- [x] `NormFitnessTrace::observe_from_feedback` in `kosmo-hyphae`
- [x] 4 `RX:FeedbackLoop` eval scenarios all pass (72/72 total)

## Completed Steps

### Phase 0 — Orientation (2026-05-30)
- [x] Read `specs/kosmocrates_spec_corpus_implementation_handoff.md`
- [x] Inspected full repository layout (crates, adapters, tools, vendors, bindings)
- [x] Identified existing PSE primitives and their alignment with spec
- [x] Created control files: IMPLEMENTATION_STATUS, SPEC_TRACEABILITY, IMPLEMENTATION_DECISIONS, SAFETY_POLICY, PHASE_CHECKLIST
- [x] Confirmed branch: `claude/amazing-allen-Gvldy`

## Repository Map

### Language / Toolchain
- Rust, workspace resolver v2, edition 2021, MSRV 1.82, stable toolchain
- `serde_jcs` in workspace deps → deterministic JSON canonicalization already available
- `sha2 = "0.11"` in workspace deps → SHA-256 hashing available

### Existing Crates (PSE substrate)
| Crate | Role |
|---|---|
| `pse-types` | Shared data model: `Hash256`, `content_address`, `RunDescriptor`, `EvidenceChain`, `GateSnapshot`, `CommitProof` |
| `pse-core` | PSE analytical engine, filter, explore, metatron attach, topology ops |
| `pse-evidence` | Crystal archival, evidence chain verification, `Archive` |
| `pse-metatron` | Graph-theoretic Metatron scan (spectral, platonic, scaffold) |
| `pse-cascade` | Cascade operators, phase ladder, mandorla, dual consensus |
| `pse-graph` | Persistent graph, observation ingestion |
| `pse-memory` | Pattern memory |
| `pse-replay` | Replay verification |
| `pse-topology` | Topology operations |
| `pse-traverse` | Traversal cognition |
| `pse-gateway` | Gateway / routing |

### Existing Types Relevant to Spec
| Existing | Spec Target | Status |
|---|---|---|
| `Hash256 = [u8; 32]` | `Digest` | Alias needed; type exists |
| `content_address<T>()` (JCS+SHA-256) | canonical serialization profile | Exists |
| `RunDescriptor` (PSE domain) | `RunDescriptor` (HYPHAE) | Different; new type needed |
| `EvidenceChain`, `EvidenceEntry` | `EvidenceBundle` / `EvidenceRef` | Partial; spec types differ |
| `GateSnapshot` | `GateResult` | Related; new type needed |
| `CommitProof` | — | PSE-specific |
| — | `AuthorityLabel` | Missing |
| — | `TaintLabel` | Missing |
| — | `CapabilityLock` | Missing |
| — | `PolicyProfile` / `ImplementationMode` | Missing |
| — | `LedgerEvent` | Missing |
| — | `FoundryCheckResult` | Missing |
| — | `Q16` fixed-point | Missing |

### Missing Crates (to be created)
- `crates/kosmo-core` — new substrate types (Phase 1)
- `crates/kosmo-workbench` — Workbench/Foundry (Phase 2)
- `crates/kosmo-hyphae` — HYPHAE v0.3/v0.4+ (Phase 3+)
- `crates/kosmo-systemcube` — SystemCube v0.4.3 (Phase 9)

### Phase 1 — Core Substrate Types (2026-05-30)
- [x] Created `crates/kosmo-core` crate with 6 modules
- [x] `Digest` newtype (SHA-256 + JCS), hex serde, `ZERO` sentinel
- [x] `canonical_bytes` (JCS RFC 8785)
- [x] `Q16` fixed-point (`i64`-backed, 2^16 scale), no-float arithmetic
- [x] `EvidenceRef`, `EvidenceBundle` (content-addressed, policy-scoped, replay-status)
- [x] `AuthorityLabel`, `TaintLabel`, `LicenseStatus`, `CapabilityLock`, `Capability`
- [x] `ImplementationMode`, `PolicyProfile`, `PolicyViolation` (fail-closed defaults)
- [x] `GateResult` (with merge semantics), `LedgerEvent`, `LedgerEventKind`
- [x] `FoundryCheckResult`, `FoundryOutcome`, `FoundryCheckKind`
- [x] `RunDescriptor` (HYPHAE — distinct from pse-types::RunDescriptor)
- [x] 43 tests pass (0 failures)
- [x] Fixed pre-existing duplicate `readme` key in 68 workspace crate Cargo.toml files
- [x] Added `crates/kosmo-core` to workspace members

### Phase 2 — Workbench MVP Skeleton (2026-05-30)
- [x] Created `crates/kosmo-workbench` crate with 5 modules
- [x] `workspace.rs`: `WorkspaceIndex` (scan_path + from_entries, content-addressed, sorted)
- [x] `task_spec.rs`: `TaskSpec` / `TaskKind` (content-addressed, label-keyed)
- [x] `context_pack.rs`: `ContextPack` with CROSS-005 enforcement (ExternalContentDenied), taint propagation
- [x] `foundry.rs`: `FoundryRunner` (ReportOnly → Skipped, DryRun → executes), `FoundryRunOutput` with EvidenceBundle
- [x] `report.rs`: `RunReport` (content-addressed, human-readable `to_text()`)
- [x] 20 tests pass (0 failures), 2 `#[ignore]` integration tests
- [x] CROSS-005 and CROSS-013 verified by named tests

### Phase 3 — HYPHAE v0.3 Passive Run (2026-05-30)
- [x] Created `crates/kosmo-hyphae` crate with 10 modules
- [x] `void_map.rs`: `HostVoid`, `HostVoidKind`, `TopologicalVoidMap` (content-addressed, sorted)
- [x] `deficiency.rs`: `DeficiencyEntry`, `DeficiencyKind`, `DeficiencyVector` (from_void_map, integer Q16 severity)
- [x] `frontier.rs`: `SourceIntent`, `SourceIntentKind`, `SourceEvidence`, `SourceFrontierGraph`
- [x] `code_hdag.rs`: `CodeObservation`, `HDAGNode`, `HDAGEdge`, `CodeHDAG` (skeleton, source backref)
- [x] `motif.rs`: `MotifCandidate` (Q16 support score, CROSS-010 test)
- [x] `structural_yield.rs`: `StructuralYield` (workbench-usability gate, void-ref requirement)
- [x] `gates.rs`: `GateKind`, `GateCascade`, `GateTrace` (TaintGate, EvidenceGate, VoidRefGate, AuthorityGate, PolicyGate)
- [x] `assimilation.rs`: `AssimilationDecision`, `AssimilationOutcome`, `NegativeEvidenceRecord` (CROSS-012)
- [x] `host.rs`: `HostBinding`, `HostCube` (from_workspace_index, structural void analysis)
- [x] `run.rs`: `HyphaeRunResult`, `passive_run()` (full pipeline, no host writes)
- [x] 36 tests pass (0 failures)
- [x] CROSS-005, CROSS-010, CROSS-012 verified by named tests
- [x] Added `crates/kosmo-hyphae` to workspace members

## Open Blockers
- None.

### Phase 4 — CubeSwarm MVP (2026-05-30)
- [x] `cube.rs`: `CubeDimensionProfile`, `RepositoryCube`, `SourceCube` (content-addressed, BTreeMap dimensions)
- [x] `swarm.rs`: `SourceCubeWorker`, `CubeMandorla`, `CompositeSupportCube`, `CubeSwarm`
- [x] `delta.rs`: `DeltaAction`, `VoidFillDelta`, `DeltaStatus`, `HostTargetDelta`
- [x] CubeSwarm sorts cubes by cube_id at construction (deterministic replay)
- [x] CubeMandorla detection: cubes targeting the same void form a mandorla
- [x] CompositeSupportCube: integer-averaged Q16 aggregate support (no floats)
- [x] HostTargetDelta: planning-only, report-only artifact (no host mutation)
- [x] 53 tests pass (0 failures)

### Phase 5 — HYPHAE v0.4 Persistent Layer (2026-05-30)
- [x] `corpus.rs`: `CorpusEntity`, `CorpusRelation`, `CorpusCartography` (append-only, entity+relation dedup)
- [x] `SourceCubeIndex`, `MotifIndex`, `NegativeEvidenceIndex` (filtered views)
- [x] `CartographyPrecheck`, `CorpusCartographyUpdate`, `ReplayManifest`
- [x] `crystal.rs`: `StructuralCrystalCandidate`, `ConstraintProgram`, `AssimilationCertificate`
- [x] `ReplayProof`, `StructuralCrystalRecord`, `Resonite` (symmetric)
- [x] `DualFabricGateCascade` (merges two GateTraces)
- [x] `norm.rs`: `NormGeneCandidate` (not trusted — governance path required), `NormFitnessTrace`
- [x] `collapse.rs`: `CollapseStep`, `HostTargetCollapsePlan` (PlanningOnly status), `MorphogenicCorpusUpdate` skeleton
- [x] `update_from_run` is idempotent (relation dedup fixed)
- [x] 79 tests pass (0 failures)

### Phase 6 — Metatron v0.4.1 M1/M2 (2026-05-30)
- [x] `metatron.rs`: `TopologyRegionRef`, `RegionExtractionProfile`, `ProjectionProfile`
- [x] `SemanticLossRecord` (Q16 loss_ratio, integer arithmetic — CROSS-007)
- [x] `MetatronMicrograph` (source_evidence_id backref — CROSS-006)
- [x] `MicrographLiftReport`, `MetatronRegionFingerprint` (structural hash, not semantic)
- [x] `AnomalyRecord`, `TopologyAmbiguityProfile`, `ComplementVoidHypothesis`
- [x] `MicroTopologyDiagnostic` (sorted anomalies/ambiguities/hypotheses)
- [x] `MicroTopologyIndex` (idempotent add)
- [x] `lift_region()` M1 pipeline, `diagnose_micrograph()` M2 pipeline
- [x] Fingerprint equality ≠ semantic equivalence documented as invariant
- [x] 92 tests pass (0 failures)

### Phase 7 — Metatron Planning-only Surgery (2026-05-30)
- [x] `surgery.rs`: `TopologicalSurgeryKind`, `SurgeryPrecondition`, `SurgeryEffect`, `SurgeryRisk`
- [x] `TopologicalSurgeryOption` (from_diagnostic, sorted by option_id, source_id disambiguator)
- [x] `SurgeryBackedCollapseStep` (CollapseStep + surgery option provenance)
- [x] `SurgeryTaskStatus` / `SurgeryWorkbenchTask` (PlanningOnly status, from_option)
- [x] 103 tests pass (0 failures)

### Phase 8 — LPCM v0.4.2 Passive Report (2026-05-30)
- [x] `lpcm.rs`: `Fragment`, `FragmentKind`, `FragmentField` (sorted by fragment_id, HDAG node backrefs)
- [x] `SupportMassVector` (Q16-scaled integer masses, `local_majority_candidate()`, no floats)
- [x] `CandidateDirection` / `CandidateDirectionReason` (LocalMajority = candidate only, not truth)
- [x] `LocalCondensationCandidate` (derived from CandidateDirection, gate-pending)
- [x] `SeamGraph` / `SeamEdge` (Q16 compatibility scores, threshold-filtered)
- [x] `monotone_contractive_filter()` — `MonotoneFilterOutcome` (Contractive / SpuriousExpansion / Insufficient)
- [x] `DoFContractionReport` (advisory only, content-addressed, `summary()`)
- [x] `LpcmPassiveReport::build()` — full passive pipeline, no host writes
- [x] CROSS-010: 51% majority → CandidateDirection only, never gate bypass
- [x] CROSS-013: LpcmPassiveReport has no host-mutation interface; `allow_host_write = false`
- [x] `allow_synthetic_sourcecube = false` enforced in default policy
- [x] 127 tests pass (0 failures, 0 warnings); +24 LPCM tests

### Phase 9 — SystemCube v0.4.3 Passive Export (2026-05-30)
- [x] New crate `crates/kosmo-systemcube` added to workspace
- [x] `blueprint_unit.rs`: `BlueprintUnit` / `BlueprintUnitKind` / `BlueprintUnitStatus`
      (evidence-bound; opaque units → `RejectedOpaque`; tainted units → `AcceptedWithTaint`)
- [x] `manifest.rs`: `SystemCubeManifest` (accepted-only, sorted IDs, JSON round-trip stable)
- [x] `energy.rs`: `ContradictionEnergyReport` / `ContradictionRecord` / `EnergyStatus`
      (Q16 weight sum, sorted by (unit_a_id, unit_b_id), advisory only)
- [x] `compatibility.rs`: `CompatibilityProfileReport` / `CompatibilityGap` / `CompatibilityStatus`
      (Q16 score, gaps sorted by unit_id, no-host-snapshot stub)
- [x] `lib.rs`: `DDensityReport` (Q16::ratio, Available/Unavailable), `SystemCube`,
      `KcubeExportReport` / `KcubeExportMode` (DryRun / BlockedByPolicy)
- [x] `SystemCube::export_dry_run()` — full passive pipeline, no disk I/O
- [x] CROSS-010: D-density=1.0 does NOT authorise materialization; mode=BlockedByPolicy
- [x] CROSS-013: no host-write interface; `allow_host_write = false`
- [x] `allow_systemcube_materialization = false` in default PolicyProfile → BlockedByPolicy
- [x] 36 tests pass (0 failures, 0 warnings)

### Phase 10 — Integration Hardening (2026-05-30)
- [x] New crate `crates/kosmo-pipeline` added to workspace
- [x] `aggregator.rs`: `GateTraceAggregator`, `AggregatedGateResult`, `LayerGateSummary`
      — fail-closed worst-wins merge (Reject > Warn > Pass), sorted by trace_id, content-addressed
- [x] `IntegrationRunOptions` — flags for optional layers (Metatron, LPCM, SystemCube)
      with `report_only()` and `all_layers()` constructors
- [x] `IntegrationRunReport` — unified content-addressed report with `verify_policy_consistency()`
      proving single PolicyProfile governs every layer
- [x] `run_dry_pipeline()` — single entry point wiring all layers:
      1. HYPHAE v0.3 passive run → gate contribution
      2. CorpusCartography::empty + update_from_run (append-only)
      3. Optional Metatron: lift_region + diagnose_micrograph per void
      4. Optional LPCM: FragmentField + SupportMassVector + SeamGraph + build() per void
      5. Optional SystemCube: BlueprintUnit per accepted decision + export_dry_run()
      6. GateTraceAggregator → AggregatedGateResult → final_result
- [x] CROSS-002: allow_host_write = false in default policy (structural + tested)
- [x] CROSS-013: no host-write interface in IntegrationRunReport (structural + tested)
- [x] Traceability: verify_policy_consistency() checks policy_id in every sub-report
- [x] Determinism: all-layers run with same inputs → identical report_id
- [x] Fail-closed: single gate Reject propagates to final_result (tested)
- [x] 24 tests pass (0 failures, 0 warnings); +6 aggregator tests, +18 pipeline tests

## Completed Phase Summary
| Phase | Crate | Tests |
|---|---|---|
| 0 | Control files | — |
| 1 | kosmo-core | 43 |
| 2 | kosmo-workbench | 20 |
| 3 | kosmo-hyphae (v0.3) | — |
| 4 | kosmo-hyphae (CubeSwarm) | — |
| 5 | kosmo-hyphae (v0.4) | — |
| 6 | kosmo-hyphae (Metatron) | — |
| 7 | kosmo-hyphae (Surgery) | 127 total |
| 8 | kosmo-hyphae (LPCM) | 127 total |
| 9 | kosmo-systemcube | 36 |
| 10 | kosmo-pipeline | 24 |

**Total: 230 tests across 4 new crates, 0 failures, 0 warnings.**

### Phase 11 — Operator-Approved Materialization (2026-05-30)
- [x] `PolicyProfile::operator_approved()` added to `kosmo-core`
      (allow_host_write=true, all require_* guards retained, no network/memory-promotion/synthetic)
- [x] `materialization.rs` in `kosmo-pipeline`:
  - `OperatorApprovalToken` (covers specific plan_id, Human/Operator authority required, content-addressed)
  - `ParseBackExpectation` (topology before/after declaration per step, content-addressed)
  - `WorkbenchMaterializationTask` (step + token + foundry_checks + parse_back, content-addressed)
  - `MaterializationOutcome` (Blocked / FoundryRequired)
  - `MaterializationPlan::evaluate()` — full governance chain:
    - Blocked: no token / wrong plan / agent authority / wrong mode / allow_host_write=false
    - FoundryRequired: valid token + OperatorApproved + allow_host_write=true
  - `simulate_foundry_check()` — Passed in OperatorApproved, Skipped in ReportOnly
- [x] Blocked invariants tested: 4 distinct block cases
- [x] FoundryRequired invariants: parse-back declared per step, ≥1 Foundry check per task
- [x] Token authority: Operator/Human sufficient; Agent insufficient
- [x] OperatorApproved policy: allow_host_write=true, require_foundry=true, require_parseback=true
- [x] 46 tests pass in kosmo-pipeline (0 failures, 0 warnings); +22 materialization tests

## Final Status — All Phases Complete

| Phase | Crate | Key Artifact |
|---|---|---|
| 0 | Control files | Repo survey + control docs |
| 1 | kosmo-core | Digest, Q16, PolicyProfile, EvidenceBundle |
| 2 | kosmo-workbench | WorkspaceIndex, TaskSpec, FoundryRunner |
| 3-7 | kosmo-hyphae | HYPHAE v0.3+v0.4, Metatron, Surgery (127 tests) |
| 8 | kosmo-hyphae | LPCM v0.4.2 (127 tests incl.) |
| 9 | kosmo-systemcube | SystemCube, BlueprintUnit, KcubeExportReport (36 tests) |
| 10 | kosmo-pipeline | run_dry_pipeline, GateTraceAggregator (46 tests) |
| 11 | kosmo-pipeline | OperatorApprovalToken, MaterializationPlan (46 tests incl.) |

**Total: ~254 tests across 5 new production crates. 0 failures. 0 warnings.**

## Open Blockers
None. Entire spec corpus (Phases 0–11) is implemented.

---

# KOSMO-OPS-01 — Operationalization Staircase

Spec: `KOSMO_OPS_01_STATUS.md`. Branch: `claude/kosmo-ops-01-operationalization`.

## Phases R0–R9 (Data Model)

### Phase R0 — Foundation Types (2026-05-31)
- [x] `Digest` newtype (32-byte SHA-256), `Q16` fixed-point (`i64 × 2^16`)
- [x] `PolicyProfile` — `ReportOnly` / `DryRun` / `OperatorApproved`
- [x] `EvidenceBundle` — content-addressed evidence record
- [x] `ParseBackTopologyDelta` / `ParseBackSeverity` — topology diff types
- [x] `FoundryCheckKind` / `FoundryCheckSpec` / `FoundryCommandPolicy` / `FoundryTimeoutPolicy`
- [x] Hosted in `crates/kosmo-core`

### Phase R1 — Foundry Execution Data Types (2026-05-31)
- [x] `FoundryExecutionPlan` (content-addressed over check specs + policy refs)
- [x] `FoundryCheckOutcome` (`Passed` / `Failed` / `SkippedByReportOnly` / `CommandDeniedByPolicy`)
- [x] `FoundryExecutionReport` (content-addressed; `verify_id()`)
- [x] `worst_outcome()` worst-wins aggregation
- [x] 8 unit tests

### Phase R2 — CorpusCartography Data Types (2026-05-31)
- [x] `CartographyEntryKind` / `CartographyStoreCommit` (content-addressed)
- [x] `CorpusCartographyStore` trait (`append`, `read_manifest`, `verify_integrity`)
- [x] `CorpusDiagnosticReport` (content-addressed)
- [x] `CartographyStorageManifest` (monotone head sequence)
- [x] 12 unit tests

### Phase R3 — ParseBack Data Types (2026-05-31)
- [x] `ParseBackPlan` / `ParseBackReport` (content-addressed; `verify_id()`)
- [x] `ParseBackOutcome` (`Passed` / `Failed` / `Inconclusive` / `TopologyUnchanged` / `SkippedByReportOnly`)
- [x] `ParseBackSeverity` worst-wins severity ladder
- [x] `ParseBackTopologyDelta` (`NodeAdded` / `NodeRemoved` / `EdgeAdded` / `EdgeRemoved` / `NodeModified`)
- [x] 9 unit tests

### Phase R4 — Operator Orchestration Data Types (2026-05-31)
- [x] `ValidationClosureReport` (content-addressed over all sub-report IDs)
- [x] `OperatorGateOutcome` (`Passed` / `Failed` / `Inconclusive`)
- [x] Gate synthesis logic (`worst_outcome_of_closure()`)
- [x] 7 unit tests

### Phases R5–R9 — Eval Benchmark Scenarios (2026-05-31)
- [x] `tools/kosmo-eval` binary (`kosmo-eval`): 42 scenarios covering R1–R9 invariants
- [x] R1:Foundry — 9 scenarios (SkippedByReportOnly, CommandDeniedByPolicy, content-addressing, etc.)
- [x] R2:Cartography — 8 scenarios (append, manifest, integrity, worst-wins, fail-closed)
- [x] R3:ParseBack — 7 scenarios (topology delta, severity ladder, report content-addressing)
- [x] R4:Operator — 6 scenarios (closure synthesis, gate outcome, INVARIANT-007)
- [x] R5–R9 misc — 12 scenarios (cross-cutting invariants, policy contracts, round-trip)
- [x] All 42 pass with `EXIT 0`

## Phases RX — Real Executors

### Phase RX-1 — Real Foundry Executor (2026-05-31)
- [x] New crate `crates/kosmo-foundry` added to workspace
- [x] `FoundryExecutor::execute()` — runs allowlisted cargo check/test/clippy commands
      via `std::process::Command`; `ReportOnly` → `SkippedByReportOnly` (no spawn);
      `CommandDeniedByPolicy` checked before spawn
- [x] `standard_cargo_plan()` / `minimal_check_plan()` plan constructors
- [x] `map_kind_to_subcommand()` — deterministic command mapping
- [x] 8 unit tests (including real `cargo check -p kosmo-core` integration test)

### Phase RX-2 — Persistent CorpusCartography Store (2026-05-31)
- [x] New crate `crates/kosmo-store` added to workspace
- [x] `JsonlCartographyStore` — JSONL append-only durable backend
- [x] `verify_integrity()` — re-reads disk, detects digest mismatch and sequence gaps
- [x] Emergent invariant: `DryRun` cannot persist (`allow_host_write = false`); only `OperatorApproved`
- [x] 9 unit tests (including real filesystem round-trip)

### Phase RX-3 — Real ParseBack Executor (2026-05-31)
- [x] New crate `crates/kosmo-parseback` added to workspace
- [x] `ParseBackExecutor::snapshot()` — `cargo metadata --format-version 1 --no-deps`
      snapshots crate-level topology; `INVARIANT-007`: identical inputs → identical IDs
- [x] `CrateFingerprint` — SHA-256(name + files_id + dep_names); content-addressed
- [x] `TopologySnapshot` — content-addressed; `diff_snapshots()` produces delta list
- [x] `ParseBackExecutor::execute()` — takes pre-snapshot explicitly, takes post-snapshot
      internally, diffs, maps severity (NodeRemoved/EdgeRemoved → Critical, NodeAdded/EdgeAdded
      → Warning, NodeModified → Info)
- [x] `ReportOnly` → `SkippedByReportOnly`; baseline mismatch → `Inconclusive`
- [x] 17 unit tests (including real workspace integration tests)

### Phase RX-4 — Operator Orchestrator (2026-05-31)
- [x] New crate `crates/kosmo-operator` added to workspace
- [x] `OperatorExecutor::execute()` — R1→R2→R3 full pipeline:
      1. ParseBack pre-snapshot
      2. Foundry check execution
      3. ParseBack post-snapshot + diff
      4. `ValidationClosureReport` synthesis
      5. Optional JSONL store persistence (only if `OperatorApproved` + `allow_host_write`)
- [x] `OperationPlan` / `OperationReport` (both content-addressed, `verify_id()`)
- [x] `standard_plan()` convenience constructor
- [x] 8 unit tests (including real `cargo check -p kosmo-parseback` integration + temp store)

### Phase RX-BENCH — Eval Benchmark Extended (2026-05-31)
- [x] `tools/kosmo-eval` extended to 52 scenarios (10 new RX scenarios)
- [x] RX:ParseBackExec — 6 scenarios (report-only skip, baseline mismatch, node-added warning,
      node-removed critical, deterministic snapshot, unchanged-workspace passes)
- [x] RX:Operator — 4 scenarios (report-only inconclusive, content-addressed report,
      full-cycle dry-run, approved-persists-closure)
- [x] All 52 scenarios pass with `EXIT 0` in < 30 s on a warm build

## Completed OPS-01 Summary

| Phase | Crate | Tests |
|---|---|---|
| R0 | kosmo-core (ext) | 43 total |
| R1 | kosmo-core | 8 |
| R2 | kosmo-core | 12 |
| R3 | kosmo-core | 9 |
| R4 | kosmo-core | 7 |
| R5–R9 | kosmo-eval (42 scenarios) | — |
| RX-1 | kosmo-foundry | 8 |
| RX-2 | kosmo-store | 9 |
| RX-3 | kosmo-parseback | 17 |
| RX-4 | kosmo-operator | 8 |
| RX-BENCH | kosmo-eval (52 scenarios) | — |

**Total: 614 tests workspace-wide. 0 failures. 0 warnings. 52/52 eval scenarios pass.**

## Open Blockers
None. KOSMO-OPS-01 staircase R0–RX is fully implemented.

---

# KOSMO-TOPO-ENERGY-01 — Real Topology In, Tripolar Energy On It (2026-05-31)

Front of the production-machine vision chain:
`real topology in → tripolar energy → (blueprint out → validation → feedback)`.
This round delivered the first two links.

### TE-1 — Unified tripolar energy kernel (`kosmo-core::energy`)
- [x] `TripolarEnergy { psi, rho, omega }` — `D = ψ·ρ·ω`, Q16 integer arithmetic, no floats (CROSS-007)
- [x] `EnergyFactors` — gate/taint/license/foundry/seam/contradiction, each `[0,1]`, derived fail-closed from `GateResult`/`TaintLabel`/`LicenseStatus`/`FoundrySurvival`
- [x] `EnergyKernel` — `D · ∏ factors`; `EnergyAssessment` content-addressed + evidence-bound
- [x] `rank_by_energy` — deterministic, tie-break on `subject_id`, never drops a candidate
- [x] **Non-bypass invariant (CROSS-010):** `Reject` → gate factor 0 → energy 0; energy ranks but never gates
- [x] 20 unit tests

### TE-2 — Real code topology extraction (`kosmo-hyphae::code_hdag`)
- [x] `CodeHDAG::extract_from_rust_source` — dependency-free lexical extractor
- [x] Nodes: modules, imports, fn defs, type defs, tests; edges: `Imports`/`Contains`/`Tests`/`Implements`
- [x] Content-addressed to `location:line:text`; deterministic (INVARIANT-007); new `Contains` edge kind; content-address now covers full edge wiring
- [x] Topology→energy bridge: `rho_coherence()`, `omega_phase()`, `energy_kernel()`, `energy_assessment()` (ψ caller-supplied; ρ, ω derived from graph)
- [x] 12 new unit tests

### TE-3 — Empirical benchmark (`tools/kosmo-eval`)
- [x] +5 `RX:Energy` scenarios (tripolar exactness, gate non-bypass, hard-state zeroing, content-addressing, deterministic ranking)
- [x] +3 `RX:Topology` scenarios (real-graph extraction, deterministic extraction, full topology→energy chain)
- [x] 60/60 scenarios pass, EXIT 0; `kosmo-eval` now depends on `kosmo-hyphae`

### Maintenance
- [x] Fixed two pre-existing `-D warnings` failures (unused imports in `cartography.rs` and `kosmo-operator`)

| Phase | Crate | Tests |
|---|---|---|
| TE-1 | kosmo-core (energy) | 20 |
| TE-2 | kosmo-hyphae (code_hdag) | 12 new (139 crate total) |
| TE-3 | kosmo-eval | 60 scenarios |

**Total: 646 substrate tests (was 614). 0 failures. 0 warnings. 60/60 eval scenarios pass.**

### Decisions
AD-015 (tripolar energy kernel + non-bypass invariant), AD-016 (lexical topology extraction + topology→energy bridge).

---

# KOSMO-KCUBE-01 — Real `.kcube` Archive Executor (2026-06-01)

Delivers "Blueprint raus + Realitätstest drüber": the host-capability bridge
that turns a `KcubeExportPolicy`-gated artifact list into a real `.kcube` file
on disk (and reads it back for import/verify workflows).

### KC-1 — `kosmo-kcube` executor crate
- [x] `KcubeArtifact { kind, path, bytes }` — typed input to the write operation
- [x] `KcubeExecutor::write` — deterministic framed binary archive, artifacts sorted by path, roundtrip SHA-256 verify
- [x] `KcubeExecutor::read` / `parse_kcube_file` — deserializes the manifest back to `KcubePackage`
- [x] `kcube_file_name(scope, sequence)` — safe slug + sequence → `{scope}-seq{n}.kcube`
- [x] Policy gates: `allow_write=false` → `DeniedByPolicy` (no disk touch); artifact kind allowlist; overwrite guard
- [x] `require_roundtrip_verification=true` (default): re-reads file, compares artifact-section SHA-256
- [x] `KcubeReadError` — typed error variants (too-short / bad-magic / bad-version / truncated sections / parse error)
- [x] CROSS-006: `evidence_bundle_id ≠ ZERO` in every report variant
- [x] CROSS-007: no floats (`written_bytes`, `elapsed_ms` are `u64`)
- [x] INVARIANT-007: `KcubeWriteReport.verify_id()` and `KcubePackage.verify_id()` pass after roundtrip read
- [x] 25 unit tests; 0 new external dependencies

### KC-2 — Empirical benchmark (`tools/kosmo-eval`)
- [x] +5 `RX:Kcube` scenarios: deny when `allow_write=false`, write+roundtrip, content-addressed package, overwrite guard, `read` parses manifest
- [x] 65/65 scenarios pass, EXIT 0

| Phase | Crate | Tests |
|---|---|---|
| TE-1 | kosmo-core (energy) | 20 |
| TE-2 | kosmo-hyphae (code_hdag) | 12 new (139 crate total) |
| TE-3 | kosmo-eval (energy+topology) | 60 scenarios |
| KC-1 | kosmo-kcube | 25 |
| KC-2 | kosmo-eval (kcube) | 65 scenarios |

**Total: 673 substrate tests (was 646). 0 failures. 0 warnings. 65/65 eval scenarios pass.**

---

# KOSMO-SCKCUBE-01 — SystemCube → .kcube Weld (2026-06-01)

Closes the "Blueprint raus" vision link: `SystemCube` can now write a real
`.kcube` archive to disk via the `KcubeExecutor`.

### SC-1 — `SystemCube::export_to_kcube` + `PolicyProfile::operator_approved_with_systemcube`
- [x] `SystemCube::export_to_kcube(executor, capacity, export_policy, op_policy, evidence_bundle_id, sequence) -> KcubeWriteReport`
- [x] `SystemCube::to_kcube_artifacts` — serializes manifest, export assessment, and accepted blueprint units into `Vec<KcubeArtifact>`
- [x] `PolicyProfile::operator_approved_with_systemcube` — new constructor enabling `allow_systemcube_materialization = true`
- [x] Policy gate: `allow_systemcube_materialization = false` → `SkippedByReportOnly` without touching filesystem
- [x] Artifact kinds: `CartographyManifest`, `ValidationClosureReport`, `StructuralCrystal` (one per accepted unit)
- [x] CROSS-006: evidence bound in every report variant; INVARIANT-007: `verify_id()` passes on all outputs
- [x] 5 new unit tests in `kosmo-systemcube`; `kosmo-systemcube` gains `kosmo-kcube` dependency

### SC-2 — Empirical benchmark (`tools/kosmo-eval`)
- [x] +3 `RX:SystemCubeKcube` scenarios: blocked by default policy, write creates archive, archive parses back
- [x] 68/68 scenarios pass, EXIT 0

| Phase | Crate | Tests |
|---|---|---|
| TE-1 | kosmo-core (energy) | 20 |
| TE-2 | kosmo-hyphae (code_hdag) | 12 new (139 crate total) |
| TE-3 | kosmo-eval (energy+topology) | 60 scenarios |
| KC-1 | kosmo-kcube | 25 |
| KC-2 | kosmo-eval (kcube) | 65 scenarios |
| SC-1 | kosmo-systemcube (weld) | 5 new (41 crate total) |
| SC-2 | kosmo-eval (systemcube-kcube) | 68 scenarios |

**Total: 678 substrate tests (was 673). 0 failures. 0 warnings. 68/68 eval scenarios pass.**

## Open Blockers (SCKCUBE)
None. Remaining bridges in priority order:
1. Adopt `EnergyKernel` for `SourceCube`/`BlueprintUnit`/`NormGene` ranking (kernel available; legacy heuristics still alongside it).
2. Close the PSE feedback loop (`kosmo-pse-bridge` is candidate-only / one-directional; no path from PSE `SemanticCrystal` back to `CorpusCartography`/`NormGene`).
3. Cross-language materialization: weld the `.kcube` reader/importer to a consumer outside the Rust substrate.
