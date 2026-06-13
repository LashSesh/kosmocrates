# Changelog

All notable changes to PSE are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Pre-1.0 caveat: while every change tries to preserve the
content-addressing and replay contracts, the public Rust API surface
is still open to breaking changes between 0.x releases. Crystal IDs
and report bytes will only break across minor versions if a release
note explicitly says so.

## [Unreleased]

### Added

* **The remaining voices — eight more surfaces self-describe; the
  inventory reaches 137 doors (Spreizung V)**

  kosmo-substrate, kosmo-promote, kosmo-tui and the five traverse CLIs
  (pse-traverse-cli, -horizon, -cognition, -topology, -lpcm) join the
  federation, each with the same discipline: a content-addressed
  catalog spoken from inside the binary (`--doors` / `doors`), pinned
  by test against its own parser or dispatch. The pins earned their
  keep immediately: kosmo-substrate's and kosmo-tui's forgotten `-V`
  version aliases surfaced as set-diffs and now stand in the catalog.

  **kosmo-tui's catalog speaks keys**: the interactive vocabulary
  (q/c/r/p/l/j/k/g/G) is cataloged as inputs and pinned against the
  event loop's `Char(…)` arms — a key the loop handles but the catalog
  doesn't name fails the build, and vice versa. kosmo-promote's recall
  mode is its own read-only door beside the store-appending promote
  door; the traverse CLIs carry the all-read-only pin (53 doors of
  horizon/cognition/topology/LPCM analysis — `--out` writes only
  operator-named files).

  Federated live across **thirteen surfaces: 137 doors under one
  catalog id** — kosmo-run 18, pse 18, pse-server 17,
  pse-traverse-topology 16, pse-traverse-cognition 14, pse-metatron 9,
  pse-traverse-horizon 9, pse-traverse-cli 8, pse-traverse-lpcm 8,
  kosmo-server 8, kosmo-promote 4, kosmo-substrate 4, kosmo-tui 4.
  +17 tests (a parser/dispatch pin and a recompute/governance pin per
  surface; the TUI key pin).

* **The PSE layer joins the inventory — pse and pse-metatron
  self-describe (Spreizung IV)**

  The cognition layer's surfaces speak for themselves. `pse doors`
  catalogs the PSE CLI's eighteen subcommand doors — including the
  **live-world door** (`observe` over eight domain adapters: crypto
  markets, energy grid, seismology, weather, air quality, IoT, syslog,
  vitals) — pinned against the dispatch match in `main` by source scan,
  **adapter vocabulary included**: an adapter the dispatch matches but
  the catalog doesn't name (or vice versa) fails the build. The write
  power is pinned honest: exactly one store-appender (`observe` grows
  the crystal store), exactly one workspace-writer (`build-wasm`), and
  the two stubs (`navigate`, `bench`) are cataloged *as* stubs — the
  inventory tells the truth about placeholders too.

  `pse-metatron doors` catalogs the Metatron closure CLI (inspect,
  project-local, isomorphism, spectral-gap, close, replay, verify) with
  the all-read-only pin: analyses observe artifacts, `--out` writes
  only the file the operator names.

  Both CLIs use the shared `kosmo_core::doors` vocabulary — kosmo-core
  is dependency-free foundation, so the PSE→kosmo-core edge adds no
  cycle and leaves CROSS-002 (kosmo never imports pse) untouched.
  Federated live: **five surfaces, 70 doors, one catalog id**
  (kosmo-run 18, pse 18, pse-server 17, pse-metatron 9, kosmo-server
  8). Honest residue, named: the eight extraction operators
  (Band/Ratio/Correlation/Granger/Spectral/Topological/Phase/
  Contraction) stay library-captive — their door needs its own design
  (graph-region inputs), a later Spreizung. +4 tests (two pins per
  surface).

* **Federation — the ecosystem inventory across three surfaces
  (Spreizung III)**

  The catalogs entangle. **pse-server learns self-description**:
  `GET /doors` speaks its seventeen routes from inside the server —
  request fields, write power, needs — pinned against the router by the
  same source-scan technique, with the governance truth pinned too
  (exactly two store-appending doors, `/ingest` and `/nxalien/bundle`;
  nothing writes a workspace, nothing decrees). `/doors` joins
  `/health` and `/ready` as a keyless probe path: self-description
  needs no token on any surface.

  Catalogs now **federate as artifacts**, not over the network:
  `kosmo-run --doors-merge <files>` unites the binary's own catalog
  with other surfaces' emitted JSON into one deterministic,
  deduplicated, content-addressed ecosystem inventory. Trust is
  mathematical and fail-closed: `Door::verify_id` and
  `DoorCatalog::verify` recompute every identity from the visible
  content, and a tampered catalog is refused by file name (pinned e2e —
  one reworded summary breaks the merge). Demonstrated live: kosmo-run
  + kosmo-server + pse-server harvest into **one 43-door inventory
  under a single catalog id**. The federated rendering shows surfaces
  and owners; the browser panel grows a *Doors* card rendering
  `GET /api/doors`. +4 tests (tamper-evidence in the vocabulary,
  federation e2e incl. refusal, the two pse-server pins).

* **Organ doors — foundry, witness, parseback, kcube, codematrix
  (Spreizung II)**

  Five substrate organs that were reachable only as side effects of
  `--apply`/`--all` become directed doors of their own — each in the
  doors catalog by build-enforced pin, one door per run:

  - `--foundry <kinds>` — the loop's own gate executor, invoked alone:
    allowlisted cargo checks (`build,test,lint,typecheck`; the
    vocabulary is closed and refusals name the word) under the same
    timeout discipline and content-addressed evidence the agent uses,
    worst-wins outcome, exit 6 on failure. `DryRun` is the least power
    that executes — `ReportOnly` stays inert by design.
  - `--witness "<argv>"` — one execution of the workspace's binary
    under the sandbox witness (cwd-confined, 60s budget, output capped
    but digest-complete): the raw, content-addressed evidence of a run;
    exit 7 unless clean.
  - `--parseback [--parseback-baseline <f>]` — the topology eye:
    a content-addressed snapshot (crates, files, dependency edges);
    with a baseline file, severity-ranked drift. The baseline is
    written once and **never silently replaced** (pinned byte-exact) —
    delete it to rebaseline.
  - `--kcube <dir>` — the blueprint exporter: full diagnosis →
    SystemCube → a real, **roundtrip-verified `.kcube` archive**. Each
    diagnosis run is its own observation event with its own cube
    identity, so each export lands a distinctly named archive; silent
    overwrite stays refused by the kcube guard. En route, the pipeline
    report now carries the built `SystemCube` itself (`#[serde(skip)]`
    — the wire format and `report_id` are byte-identical).
  - `--codematrix` — the 5D fingerprint lens (relationality, cohesion,
    topology, symmetry, entropy) per source plus the most resonant
    pairs — strictly advisory: it ranks, it never gates (CROSS-010).

  +6 e2e tests (gates fail-closed; witness evidence; baseline-once +
  drift; per-diagnosis archives; advisory lens; one-door-per-run), and
  the catalog pin grew the surface to eighteen described doors.

* **Doors — the self-describing docking surface (Spreizung I)**

  The design law lands as mechanism: *every function gets its own
  addressed door; a chat window is one door among many, never the door.*
  The operator's standing question — "what, exactly, could I operate
  right now?" — now has a machine answer spoken by the system itself.

  `kosmo_core::doors` is the vocabulary: a `Door` is one operator-facing
  entry point with its surface (CLI flag / HTTP route), name and aliases,
  one summary, its inputs (value shapes, required-ness), its **write
  power as data** (`read-only` / `writes-workspace` / `appends-store` /
  `governance-act`) and its needs (provider/cargo/network/workspace/
  store/file) — content-addressed; a `DoorCatalog` is a surface's
  deterministic, deduped, content-addressed inventory, and catalogs
  merge across surfaces.

  Surfaces describe **themselves**: `kosmo-run --doors` speaks the
  binary's thirteen doors (text and `--json`), `GET /api/doors` speaks
  the server's eight routes. And the description cannot drift: each
  surface carries a source-pin test that scans its own parser/router and
  asserts set-equality with the catalog — a new flag or route without a
  description fails the build, as does a described door without a
  mechanism. Self-description is offline, keyless, workspace-free and
  byte-deterministic (pinned e2e). The server's catalog additionally
  pins its governance truth: no door writes a workspace, no door
  decrees, exactly one appends a store (`/api/promote`'s explicit ledger
  anchor). Protocol and the road to the merged ecosystem inventory (and
  the catalog-generated GUI at its end) in `docs/DOORS.md`. +11 tests
  (4 vocabulary, 3 kosmo-run incl. the parser pin, 2 e2e, 2 server incl.
  the router pin).

* **The steward — self-husbandry under an operator-named fence (Etappe V)**

  The system now works on itself, governed. `kosmo-run --steward` surveys
  a workspace's own wish landscape (run on this repository, it sees its
  own body: 780 proposals, 645 open chores) and names the open proposals
  inside an explicit **fence** — a comma-separated list of facet classes
  the operator spoke (`--fence doc,test`). Nothing is fenced by default:
  husbandry (`--apply`) without a fence is refused, words outside the
  facet vocabulary are refused, and widening the fence is itself an
  explicit per-run operator act. Under `--apply`, each fenced open chore
  descends as its own evidence-bound wish through the same armament as
  wish mode — deterministic scaffolds first, so a `doc,test` fence
  husbands offline with no provider and no key — and every descent is
  recorded as a norm-learning observation: the system learns from the
  work it does on itself. A failed chore is recorded and the round
  continues (exit 4 still tells the truth at the end); `--steward-max`
  caps the chore list per run.

  The report (`--steward-report`) is content-addressed and
  **host-path-free** (the workspace appears as its identity digest), fit
  for an unattended nightly artifact — and `.github/workflows/steward.yml`
  makes that literal: a scheduled nightly **self-survey** publishes the
  plan as a build artifact (the machine proposes), while the husbandry
  job runs only on manual dispatch and pushes a `steward/*` branch, never
  to `main` — opening and merging the pull request stays the operator's
  baptism. Protocol in `docs/STEWARD.md`. +10 tests (4 unit: fence
  vocabulary/refusals, fenced-open selection honoring standing and cap,
  report counts/json/host-path-freedom; 6 e2e: read-only survey,
  fenceless and unknown-class refusals, door exclusivity, offline
  husbandry observed by the norm organ, the cap).

* **The reforge — external empiricism as a bench (Etappe IV)**

  The system now proves itself against truth it did not author.
  `kosmo-run --reforge` collects ground truth at runtime by probing
  **external oracles** — `expr`, `factor`, `basename`, binaries this
  repository did not write — and refuses to invent a single answer
  (missing tool, non-zero exit, empty or ambiguous output ⇒ the target
  is *skipped*, never faked). Each probed truth set becomes a wish of
  budgeted `Run` facets (`args=>exit:0,out~answer,ms<60000`), evidence
  content-addressed over `(tool, truths)`; a fresh scratch workspace
  with an empty `main` is then **forged to that wish via the real
  provider descent** — the same `descend_to_wish` loop, judged by the
  same runtime observer that executes the program and matches its
  output and exit code against the oracle's answers.

  The honesty boundary is pinned offline: re-forging implements
  behaviour, so `--reforge` without a provider refuses with a clear
  error, and `--provider mock` is rejected as *forging theater* — the
  mock cannot implement behaviour and the bench will not pretend it
  can. The outcome is a content-addressed JSON report
  (`--reforge-report <file>`; report_id = digest of the body) listing
  per target the wish id, probes, iterations and realization; exit 0
  only when no attempted target failed (exit 5 otherwise). The stranger
  protocol — one command, no trust required — is documented in
  `docs/REFORGE.md`. +6 tests (5 unit: target wellformedness, echo
  oracle round-trip, missing/ambiguous oracles yield no truth, wish
  format pinned byte-exact, report counts/json; 1 e2e: the
  provider/mock refusals).

* **Polyglot fabrication — Python joins the build loop (Etappe III)**

  The architecture's language-freedom claim becomes mechanism. A
  cargo-less Python workspace is now **observed by Python's own law**
  (file = module, `__init__.py` stands for its package) without `cargo
  metadata` and without an interpreter: Module facets per file,
  Symbol/Signature facets via the xlang extractor's verified rules,
  Test facets for recognized test functions, Doc facets from module and
  item docstrings. Inside cargo workspaces, Python files stop being
  blind spots: polyglot observation merges them into the same topology,
  so the landscape's "beyond observation" residue shrinks to what is
  genuinely unobservable (crate roots).

  Fabrication follows: in Python workspaces the `FacetScaffolder`
  builds Module/Symbol/Doc/Test by Python conventions (location-aware —
  scaffolds land in the file that already provides the module, wherever
  it lives); in **mixed** workspaces the Rust path keeps precedence and
  a polyglot fallback builds what Rust cannot name (a Python module's
  docstring lands in the `.py` file while its smoke test lands as a
  Rust test — genuinely mixed fabrication, pinned e2e on the adopted
  cluster of a mixed workspace).

  The Prüfstand grows a Python arm with the same fidelity contract,
  offline and interpreter-free: a known-good Python system is accepted
  as realized, an empty one is *fabricated* to realized (the descent
  scaffolds by Python law), and arity signatures judge in both
  directions (`greet/0` realized, `greet/2` rejected) — **17/17
  `--validated`**. Landscape/geometry/adoption pins lifted to the
  polyglot truth: the mixed fixture now shows four open proposals in
  two coherent clusters (one per language). Also landed en route: the
  norm-observation pin for adopted cluster descents (the feature
  shipped with the interop pass; its binary pin had silently failed to
  apply). +7 tests.

* **The quality axes — speed as a facet; the landscape grows to five
  finding classes (Etappe II)**

  The wish vocabulary crosses into quality: a `Run` probe's expectation
  accepts a tail-anchored wall-clock budget — `"hi=>out~hi,ms<50"`
  means *the program, executed, answers correctly AND stays under
  50ms*, measured from the sandbox witness's duration and fail-closed
  (a blown budget is a miss, never a warning; a malformed budget fails
  the whole key). Pinned end to end: a fixture that sleeps 60ms
  realizes a generous budget and measurably misses a 10ms one.

  The landscape projection grows from two to **five** finding classes:
  `MissingErrorHandling` → `Test("<stem>_handles_errors")` and
  `IncompleteFunctionBody` → `Test("<stem>_complete")` (the fix is
  *demonstrated* by a test, not declared), `MissingImplementation` →
  `Capability("impl:<slug>")` (the free-form intent as a deterministic
  capability slug, subject = the host module the standing measurement
  can actually see). `MissingTypeAnnotation` and `Custom` stay honest
  residue. +4 tests.

* **The venture — whole-system fabrication from dependent wishes
  (Etappe I of the summit staircase)**

  A `Venture` (kosmo-core/src/venture.rs) is a content-addressed web of
  wish stages with explicit dependencies — the executable counterpart of
  the substrate's collapse plans. Fail-closed at construction
  (out-of-range, self- and cyclic dependencies are construction errors:
  an invalid venture never exists as a value), deterministic execution
  order (smallest-ready-index Kahn walk), evidence-bound (CROSS-006).
  The `VentureSession` is pure, serializable orchestration state: a
  failed stage cascades `Blocked` over every transitive dependent — the
  staircase never attempts what cannot stand.

  Spec files carry stage wishes as **prose**, compiled through the same
  grammar as the front door — including promoted norm triggers — so a
  venture stage's wish is byte-identical to the wish the same prose
  would produce at `--wish` (pinned). Vacuous stages are hard errors.

  `kosmo-run --venture <spec.json>` previews the staircase read-only
  (order, standings, live measurement per stage) and erects it under
  `--apply`: stage by stage in dependency order, each under the full
  armament (gates, validation, rollback, memory, swarm), each realized
  stage recorded as a norm-learning observation, progress persisted to
  `--venture-session` after every stage. Resume is identity-checked: a
  changed spec is refused, never silently restarted.

  The Etappe-I exit criterion is pinned by binary test: a venture of
  five functionally dependent stages (modules → docs → test →
  capability) erects a multi-component system in an empty crate,
  offline, 5/5 realized — and resumes as a no-op. +14 tests.

* **The interop pass — no new technology, everything thought together**

  The UX premise made structural: the system's worth in practice is
  decided by whether its organs compose, so every surface catches up
  with every organ.

  - **Adopted descents feed norm learning.** `--adopt`,
    `--adopt-cluster` and chat-routed adoption now record
    facet-bundle observations like a spoken wish does (previously the
    system learned nothing from precisely the work it proposed
    itself). Pinned: an adopted-cluster descent leaves exactly one
    observation in the store.
  - **The server catches up:** `POST /api/landscape` accepts
    `geometry: true` (strictly additive — without it the response is
    byte-identical, pinned) and returns the coherent clusters and
    singular proposals; new `POST /api/norms` is a read-only window
    onto a caller-pathed norm store (catalog, arming state, origin,
    observation count — the server never appends or promotes). The
    browser panel renders the geometry and the adopt-cluster hint.
  - **The TUI catches up:** the `l` landscape pane renders the
    spectral shape (cluster and singularity lines) under the peaks,
    from the same standing definition as CLI and HTTP.
  - **Status becomes a cockpit:** `--chat "status"` reports the armed
    organs in one glance — norm catalog (known/armed/observations)
    and memory source — before the measured landscape.
  - **First contact is an invitation:** a keyless `kosmo-run .` now
    answers with the offline doors (`--landscape`, `--wish`, `--chat`,
    `--atelier`) instead of a bare missing-key error.

* **The wish atelier — a wish is shaped over rounds before it is realized**

  The front door stops being one-shot: `kosmo-run --atelier <draft.json>`
  opens a durable, content-addressed `WishDraft`
  (kosmo-intent/src/atelier.rs) that operator and machine refine together,
  one invocation per round (`--chat` carries the utterance). The
  governance symmetry of the norm organ, applied to dialogue: **the
  machine proposes, only the operator disposes.** What you *dictate*
  (grammar-recognized facets of your own words, including your promoted
  norm triggers) enters the wish directly; what the machine *proposes*
  stays pending until an explicit `accept <n>` — pinned by test at the
  library AND the CLI seam (an unaccepted proposal never materializes).

  Proposals come from two sources: the substrate's own companion
  heuristics (deterministic, offline — doc/test fibers for accepted
  structural targets, filtered against what the workspace already
  exhibits) and, with a real `--provider`, the model-backed
  `LlmWishRefiner` (kosmo-intent-llm/src/refine.rs) under a pinned
  suggest-and-ask-only contract ("never plan file layouts, never name
  technologies, never generate code"); unknown kinds are dropped, a dead
  or rambling model costs suggestions, never correctness (honest note
  instead). The refiner work also surfaced and fixed a real gap: the LLM
  facet-kind mapper predated the Doc facet and never accepted `"doc"`.

  The round language is total and fail-closed toward dictation: exact
  verdict forms (`accept 1,3` / `accept all` / `reject 5` / `drop 2` on
  the displayed numbering) and exact realize phrases (`realize`,
  `build it`) — "build a module parser" stays prose. Each round renders
  the dialogue, the wish-so-far **measured live** against the workspace
  (✓/✗ per facet), the numbered proposals with rationale and source, and
  the model's open questions. `realize` freezes the accepted facets into
  a wish labelled with the full dialogue and evidence-bound to it
  (CROSS-006: the prose history *is* the evidence), then descends —
  writes only under `--apply`, and a realized atelier descent records a
  norm-learning observation like any other. A tampered draft file is
  refused by its content address. +18 tests (10 core, 5 refiner, 3
  binary e2e); Prüfstand 13/13 `--validated`.

### Fixed

* **Workspace build repaired after the dependabot `rand_distr` 0.6 bump.**
  PR #207 raised `rand_distr` to 0.6 (built on rand 0.10) while the
  workspace `rand` stays 0.9 — exactly the pairing the manifest comment
  forbids ("bump both together or not at all") — breaking
  `mef-solvecoagula` and with it every strict workspace build. Both
  manifests (root + vendored infinityledger) and the lockfile are back on
  the rand-0.9-compatible 0.5 line.

* **Wonderlamp assimilation, phase 6 — the chat front door (assimilation
  complete)**

  One utterance, routed onto the organs — never past them. `ChatIntent`
  (kosmo-intent/src/chat.rs) is the predecessor's `isls-chat` with the
  disease excised at the type level: where Wonderlamp's intents carried
  `affected_files` lists, a forced User entity and REST assumptions, a
  Kosmocrates intent is *type-systemically incapable* of naming a path,
  file or entity (pinned by a source-scan test on the enum). The seven
  variants map onto existing organs only: MakeWish, DescendWish,
  ShowLandscape{geometry}, AdoptLandscape{top}, AdoptCluster{index},
  ShowStatus, InjectNorm.

  Routing is **total and transient**: `IntentExtractor::extract` cannot
  fail — the deterministic `KeywordIntentExtractor` bottoms out in
  MakeWish, so an utterance nobody understood flows to the *measurable*
  wish door (where an unparseable wish is honestly vacuous), never to a
  template generator and never to an error. And routing is not a durable
  artifact (no content addressing, no evidence binding — the routed-to
  organs own all of that), which is precisely why the LLM router
  (`LlmIntentExtractor`, kosmo-intent-llm) may degrade to the keyword
  rules on ANY failure — transport, malformed JSON, unknown intent —
  proven without network in tests. Its prompt contract pins the model to
  classification only ("never plan, never generate code, never name
  files"); fields a model invents are never read.

  CLI: `kosmo-run --chat "<utterance>"` (one-shot, no REPL) echoes the
  routing decision for audit (`chat[keyword] → adopt cluster 1`) and
  delegates to the existing modes; `--apply`/`--provider`/`--ledger`/
  `--norms` compose orthogonally. Chat never escalates: a "build …"
  utterance without `--apply` measures and says so. With a real
  `--provider` the model routes first (mock routing would be theater —
  keywords instead); `--chat` + `--wish` is refused. InjectNorm maps to
  instructions for the explicit governance flags — chat carries no spec
  files, by type. +16 tests (6 router, 5 extractor, 5 binary);
  Prüfstand 13/13 `--validated`.

  **This closes the Wonderlamp assimilation: all six planned phases are
  landed** — integer consensus core, fail-closed SwarmSynthesizer,
  descent context + patch gates, the norm organ, the spectral kit with
  landscape geometry, and the chat front door. The rejection list in
  `docs/WONDERLAMP_ASSIMILATION.md` is permanent.

* **Wonderlamp assimilation, phase 5 — the spectral kit and the landscape's
  geometry**

  The one genuinely float-bearing mathematics gets its quarantine crate:
  `kosmo-spectral` (deps: kosmo-core only — no nalgebra, pinned). Inside:
  power-iteration Fiedler bisection with deflation, recursive spectral
  clustering, Kuramoto synchronization at a fixed horizon, and iterative
  integer Tarjan articulation points. Outside: a **Q16/discrete-only
  public API** (`CouplingGraph` in, index partitions and Q16 masses out),
  pinned by a source-scan test over every public signature. Determinism
  without a determinism asterisk: fixed iteration counts, seeded inits,
  and no libm — the one needed `sin` is an in-crate range-reduced Taylor
  polynomial, so results reproduce across IEEE-754 platforms. The key
  honesty move over the predecessor: a bisection is accepted only while
  its conductance stays ≤ ½, so tight clusters refuse to shatter and the
  cluster count *emerges* instead of being imposed (a K4 barbell yields
  exactly its two cliques even when four clusters are allowed).

  On top of it, `kosmo_pipeline::landscape_geometry`: wish proposals
  couple into a graph whose edge weights come from **`WishProposal`
  fields only** (the structural guard against Wonderlamp's domain
  registries growing back, pinned by a disease test) — subject affinity
  45, facet-kind affinity 30, severity proximity 25, integer-percent
  arithmetic exact at the boundaries, and proximity alone never couples
  (two unrelated findings of similar badness are not a cluster). The
  geometry returns coherent clusters (heaviest severity mass first) and
  the articulation singularities — the proposals whose removal
  disconnects the landscape: the most consequential decisions first.

  CLI: `kosmo-run --landscape --geometry` renders clusters and singular
  proposals on both text and JSON surfaces — strictly opt-in, the
  landscape without the flag is byte-identical (pinned). And adoption
  graduates from blind top-k to coherent work: `--adopt-cluster <i>`
  takes ONE cluster as ONE severity-weighted, evidence-bound wish through
  the existing descent (mutually exclusive with `--adopt`; out-of-range
  indices name the real count). +19 tests (10 spectral, 6 geometry, 3
  CLI).

* **Wonderlamp assimilation, phase 4 — the norm organ: learned archetypes**

  The de-CRUD insight, made structural: a `Norm`
  (kosmo-hyphae/src/norm_schema.rs) is a content-addressed bundle of
  `NormFacetTemplate`s — facet kind + key pattern over the single
  `{name}` placeholder — so a norm can only ever emit measurable
  `WishFacet`s, never file trees, stacks or entity scaffolds.
  `validate()` is the anti-disease gate (path separators, backslashes,
  file extensions, foreign placeholders and frozen structural names are
  rejected; `name/arity` of `Signature` is the one sanctioned `/`), and
  it runs at every door: injection, learning, store append, store load,
  catalog construction. Wonderlamp's 31-norm built-in catalog stays on
  the rejection list: **the catalog starts empty, always** (pinned by
  test), and norms exist only through observation or operator
  injection.

  Learning (norm_learning.rs): every realized `--apply` descent records
  a content-addressed `FacetBundleObservation` (facets, digest
  workspace tag — never a raw path — languages, realized flag).
  `abstract_bundle` finds the bundle's subject (the dominant shared
  lowercase subword, boundary-aware so `user` rewrites in `create_user`
  but never inside `username`) and lifts the bundle to a shape;
  `promotable` proposes a shape once it was realized ≥3× across ≥2
  workspaces with consistency ≥ 3/4 (all Q16 integer arithmetic). A
  proposal lands in the store **unarmed** (`trigger = None`) with its
  linked `NormGeneCandidate`, so the existing promotion-feedback
  fitness loop applies unchanged; `NormFitnessTrace::smoothed_fitness`
  adds Wonderlamp's exponential φ-update as integer EMA.

  Governance: the trigger word is *outside* the norm's content hash —
  arming is governance over an existing artifact, not a new artifact.
  `kosmo-store::NormStore` (norms.jsonl + observations.jsonl,
  caller-pathed, `allow_host_write`-gated like every durable store,
  corruption = hard error on open) encodes promotion as an append-only
  audit trail: same `norm_id`, trigger set, loader takes the latest.
  `kosmo-run --inject-norm <spec.json>` (the spec file's bytes are the
  evidence; arrives unarmed) and `--promote-norm <id> --trigger <word>`
  are explicit operator acts; reserved grammar words are refused
  (`kosmo_intent::is_reserved_wish_word`).

  Activation: `NormCatalog` indexes only armed norms (re-validated at
  the door; reserved/duplicate triggers are hard errors) and
  `compile_wish_with_norms` expands a promoted trigger exactly like a
  built-in archetype — `compile_wish` itself is untouched and the empty
  catalog is pinned byte-identical to it. Genome layer
  (norm_genome.rs): co-activation Jaccard clustering into `NormGene`s
  (single-link, Q16), `relate` (Dependent > Conflicting > Compatible >
  Independent), `compose` (BFS over `requires`, conflict detection,
  merged templates) — all advisory, CROSS-010.

  End to end (binary test): three realized descents of one shape in two
  scratch workspaces ⇒ the third run prints and stores the unarmed
  norm ⇒ the would-be trigger still expands nothing ⇒
  `--promote-norm --trigger loader` arms it ⇒ "a loader delta"
  compiles to the learned shape — and without `--norms`, the same
  prose stays vacuous. +42 tests across the organ.

* **Wonderlamp assimilation, phase 3 — descent context and patch gates**

  The facet-to-facet amnesia is healed: `TypeContext`
  (kosmo-synthesizer) absorbs every accepted patch through the xlang
  classifier (7 languages, never regex, never a path template) and
  renders into each subsequent prompt as `# Symbols already created in
  this descent` — Wonderlamp's decisive anti-hallucination trick,
  language-agnostic. Advisory like memory grounding: the new
  `SynthesisRequest.descent_context` is serde-defaulted and not part of
  `request_id`. Three-stage capping degrades honestly (full → name+kind
  → most-recent-N behind an elision marker); deletions retract an
  origin's symbols.

  Wonderlamp's informational gates become real ones:
  `patch_gates::gate_patch` (worst-wins `GateResult`) judges filesystem
  truth and descent knowledge — never an upfront plan. Create over an
  existing path, modify of a missing one, duplicate definitions across
  a patch, and use-after-delete of a descent-created module are
  `Reject`s; origin shifts and structureless source files warn; non-code
  is skipped (fail-closed, not over-closed). `ContextualSynthesizer`
  delivers fail-closed: a rejected patch arrives with `confidence =
  ZERO` and a `gate-reject:` rationale — auditable, unmaterializable
  through the existing `min_confidence` filter — and its symbols are
  never absorbed (they never existed).

  Wired where multi-file generation lives: one context per
  `AgentSession::run` and per wish-descent LLM fallback
  (`Contextual(Grounded(...))`). Prüfstand stays 13/13.

* **Wonderlamp assimilation, phase 2 — the fail-closed SwarmSynthesizer**

  The one non-deterministic boundary gets the consensus discipline
  everything else already has. `SwarmSynthesizer` (kosmo-synthesizer-llm)
  fans one request into `n` perspectives — the four Chameleon lenses,
  pinned verbatim and stack-free — parses n JSON patches through the
  existing wire contract, and scores them with phase 1's integer
  consensus core.

  The transformation that matters: where Wonderlamp emitted the best
  candidate even when consensus failed, the swarm answers honestly —
  `confidence = min(best.confidence, d_total)`. A divergent ensemble
  lands below the agent's existing `min_confidence` filter and is
  skipped by policy, not emitted by charity (pinned by test: three
  disjoint 90%-self-confident answers ⇒ confidence < HALF). Bounded
  Coagula rounds let the best candidate complete itself with the
  functions its peers agree on; an ensemble where nothing parses is a
  permanent error. `ChatOracle` abstracts the transport
  (`LlmSynthesizer` implements it; `ScriptedOracle` scripts hermetic
  offline tests — failures, repairs, token accounting, provenance
  citations all covered).

  CLI: `kosmo-run --swarm <n>` (2–6) wraps any real provider;
  `--swarm` + mock is refused as consensus theater.

* **Wonderlamp assimilation, phase 1 — deterministic foundations for
  swarm consensus**

  `wonderlamp.zip` (ISLS, ~56K LOC) is the abandoned predecessor of this
  system: same vision, no epistemic substrate, and a chronic collapse
  into CRUD templates. The assimilation ports its valuable organs onto
  the substrate it lacked — taxonomy and mathematics cross, the disease
  does not (`docs/WONDERLAMP_ASSIMILATION.md` carries the full port map
  and the permanent rejection list).

  Phase 1 lands the deterministic foundations, integerized beyond the
  original:

  * `Q16::geomean` — the n-th root of the raw product via binary search
    on `i128`: bit-replayable soft unanimity (empty/zero ⇒ ZERO),
    replacing Wonderlamp's `ln`/`exp` float pipeline.
  * `kosmo_hyphae::xlang::symbol_sets(_auto)` — the existing 7-language
    classifier pass with names retained (`name/arity` function keys):
    the language-agnostic substitute for ISLS's Rust-only regex readers.
  * `kosmo_hyphae::codematrix::CodeMatrixFingerprint` — the 5D quality
    fingerprint (richness / functions / types / structure / error
    handling) as content-addressed Q16 with `verify_id`, language from
    xlang instead of path guessing; `resonance` = geomean of axis
    similarities. Strictly advisory (CROSS-010).
  * `kosmo_synthesizer::consensus` — Ophanim/Konus re-expressed:
    per-candidate `D_k = ψ·ρ·ω` (the house tripolar form; agreement via
    percent-weighted Jaccard with exact boundary arithmetic, outlier
    cutoff kept, **neutral band** ρ=ω=HALF for structure-free patches),
    ensemble `d_total = geomean(max(D_k, ε)) ⊗ Ω`, `best_index`, and the
    Coagula `repair_targets` menu. Pure, offline, deterministic — the
    fail-closed *delivery* (confidence folding, repair rounds) is
    phase 2's SwarmSynthesizer.

  Every phase ships a disease test: the assimilated sources are pinned
  free of stack names, file-tree templates, and entity scaffolds.

* **Landscape surfaces — and adopted wishes carry the full armament**

  The wish landscape reaches every operator surface: `POST /api/landscape`
  (kosmo-server; read-only by design — adopting/descending stays on the
  CLI where `--apply` is explicit), a browser panel ("Map the landscape",
  ranked table with standing badges and the adopt command line), and the
  TUI (`l` — summary line plus the top peaks under the stats). One
  standing definition for all of them:
  `kosmo_pipeline::{LandscapeStanding, measure_landscape}` (lifted out of
  kosmo-run; CLI, server and TUI now share it).

  Adoption is armed: an adopted landscape descent uses the same fallback
  chain as wish mode (`wish_fallback` — deterministic scaffolder first,
  then the provider-gated LLM, memory-grounded under `--ledger` via
  `GroundedSynthesizer`). Landscape meets tank: the system realizes its
  own proposals with its own anchored knowledge.

  The HTTP recall caught up with the tank while we were there: hits now
  carry their `claims` and the query embeds through the store's embedder
  seam (`embed_query`) instead of a raw projection; the browser recall
  table renders the claim lines.

* **The wish landscape — the findings become the wish menu**

  The operator's words made executable: *"damit ließe sich eine ganze
  Wunschlandschaft mappen."* `kosmo-run --landscape` runs the substrate
  pipeline and projects every finding the wish vocabulary can express into
  a ranked **wish-proposal landscape** (`kosmo_pipeline::propose_wishes`,
  pure and deterministic): `MissingDocFiber{m}` → `Doc(stem(m))`,
  `MissingTestFiber{m}` → `Test("<stem>_smoke")` — content-addressed
  proposals carrying the void's severity (ranking **and** the adopted
  predicate's weight), the subject module, and full provenance. Path
  targets normalise to module stems (`src/router.rs` → `router`), so the
  projection speaks the name the wish world observes.

  Every proposal is measured against the observed topology and rendered
  with an honest standing: **met**, **open** (adoptable), **beyond
  observation** (non-Rust modules, crate roots — the wish world cannot see
  the target, so it is residue, not a stalling wish), **beyond vocabulary**
  (finding kinds no facet expresses yet — listed and counted, never
  dropped). Inexpressible findings remain first-class residue
  (`UnmappedVoid`).

  `--adopt <n>` turns the top open proposals into ONE wish: predicates
  weighted by severity (`WishPredicate::weighted`), the wish
  evidence-bound to the diagnosis itself (`evidence_bundle_id` = the
  pipeline `report_id`). Read-only by default — it prints the wish and the
  contract line; under `--apply` the existing descent takes over. Live on
  a mixed workspace: 6 proposals (2 open, 4 beyond observation), top-2
  adopted, `iter 0: 0/2 UNSTARTED → iter 1: 2/2 REALIZED` — the doc stub
  above `pub mod router;`, the smoke test in the lib.

  Supporting growth in the Doc facet: module declarations are now
  docu-observable (`/// above mod x;` yields `Doc(x)` — the same place the
  Module facet is observed) and docu-scaffoldable
  (`line_declares_module`), so module-level findings round-trip.

  Diagnosis → landscape → choice → wish → descent → reality: the system
  now proposes its own goals; the operator chooses; nothing is adopted
  automatically (energy ranks, never gates — applied to intent).

* **The Doc facet — the wish language learns the language of the findings**

  Stage 2 begins where the substrate's diagnosis is loudest: its two most
  common findings are missing tests and missing docs, and while tests were
  already expressible, *"documented"* was not a wish the system could
  measure. Now it is, end to end:

  * `WishFacetKind::Doc` (keyed like `Symbol`, `name` or `name@crate`):
    a public item carrying a doc comment immediately above its definition.
  * **Observation** (`kosmo-intent::facets_from_source`): `///`, `//!` and
    `#[doc = …]` lines are tracked through attribute blocks; documented
    public items yield `Doc` facets, private or undocumented items honestly
    do not.
  * **Prose** (`compile_wish`): `docs of helper`, `docs for helper`,
    `documented helper`, `documentation for helper` (`for` joins the
    filler words, which also reads better for `a test for …`).
  * **Deterministic scaffold** (`FacetScaffolder::scaffold_doc`): finds the
    public item across the crate's sources, inserts a doc-stub line above
    its attribute block (docs precede attrs), and stays an honest no-op
    when the item is already documented or does not exist. The stub names
    itself a stub — structurally present, honestly minimal, exactly the
    contract `todo!()` scaffolds follow.
  * **Convergence**, proven live and by test: `kosmo-run --wish "docs for
    route" --apply` descends `UNSTARTED (0/1) → REALIZED (1/1)` with the
    stub landing above the item; an already-documented item realizes
    without writing; scaffold → observe round-trips (the second scaffold
    is empty).

  The loop this closes: the substrate *finds* `MissingDocFiber`, the
  memory *anchors* it as claims, and the wish language can now *target*
  it — diagnosis, memory, and intent finally speak one vocabulary.

* **The memory tank — anchored knowledge carries content, and the
  embedding becomes a versioned socket**

  Stage 1 of the road past the masterwork skeleton: recall now returns
  *knowledge*, not just metadata. Every ledger commit persists bounded
  **claim lines** (`ILStore` index `claims`, 8 × 200 chars, deterministic):
  `kosmo-promote --ledger` anchors what was actually certified — an
  ensemble crystal carries its deduplicated member findings with
  multiplicity (`void_hyp:MissingTestFiber … ×3`), a single crystal its
  label, kind and metadata, both with evidence/run provenance. The claims
  surface everywhere the memory does: `--recall` prints them under each
  hit (text + JSON), `CrystalSummary.claims` →
  `MemoryGroundingEntry.claims` → the LLM prompt's *Anchored knowledge*
  section renders them as content lines, and the budget estimator counts
  them honestly.

  Underneath, the text-embedding seam is now explicit and swappable:
  `pse_adapter_il::TextEmbedder` (deterministic, named, fixed-dimension)
  with `HashEmbedder8`/`hash8-v1` wrapping the original 4-gram projection
  bit-for-bit. The store's index is **tagged with its embedder id**
  (pre-seam ledgers normalise to `hash8-v1`); opening a populated store
  with a different embedder refuses loudly — cosine across embedding
  functions is noise, and the system prefers failing to lying.
  `ILStore::open_with_embedder` + `embed_query` make a real embedding
  model a drop-in: the dim-4 test embedder round-trips commit → recall at
  its own dimension, pinned by test. All query paths
  (`context_for_query`, `build_grounded_prompt`, `causal_retrieval`,
  `LedgerRecall`) go through the store's embedder.

  Compatibility pinned by tests: pre-claims/pre-seam indexes load
  unchanged (serde defaults; claims honestly empty), the default
  embedder's vectors are bit-identical to the legacy projection, and the
  whole content chain — promote-anchor → recall claims → grounding
  claims → prompt lines — is asserted end to end.

* **Memory-grounded synthesis — the anchored knowledge works**

  The loop that closed at recall now drives the build: hand `kosmo-run` the
  same Infinity-Ledger the promotion path anchors into (`--ledger <path>`,
  `--ground-top <n>`), and every action's synthesis request is grounded in
  the crystals the system has already learned — recalled per action with the
  same Pfauenthron retrieval (`D = ψ·ρ·ω`) that powers
  `kosmo-promote --recall`, rendered into the LLM prompt as a
  clearly-delimited *Anchored knowledge* section, and **cited** in the
  result: `SynthesisResult::grounding_crystal_ids` carries the provenance
  from every generated patch back to the certified knowledge that informed
  it (`kosmo-run` prints `memory grounded by N anchored crystal(s): …` per
  step).

  The bridge becomes a round trip without bending a single layering rule:
  `kosmo-pse-bridge` gains the reverse-direction contract
  (`MemoryRecall` trait + `MemoryGroundingEntry`, mirror of the cognition
  layer's recall summary), `pse-adapter-kosmo::LedgerRecall` implements it
  above both stacks, and `kosmo-*` consumers only ever see the trait —
  CROSS-002 intact. `AgentSession::with_recall` grounds the agent loop
  (`AgentOptions::grounding_top`), and the new
  `kosmo_synthesizer::GroundedSynthesizer` wraps any backend with memory
  for the wish-descent's LLM fallback. The discipline holds everywhere:
  grounding is advisory context — float scores from the retrieval side
  decorate prompts and reports but never gate, never enter a
  content-addressed identity (`request_id` unchanged by context, house
  rule); a missing ledger is a hard error; recall failures are loud
  (fail-closed, never silent memory-free degradation); and an action that
  resonates with nothing carries an honestly empty citation list.

  Proven live end-to-end: `kosmo-promote --offer --batch --all-kinds
  --calibration substrate --ledger …` anchored a polyglot workspace's
  knowledge (14/14 accepted, Q5, all eight Kairos gates open), then
  `kosmo-run --provider mock --ledger …` synthesized on the same workspace
  with steps citing the anchored crystal — learning → anchoring →
  remembering → **building with memory**, one unbroken chain.

### Fixed

* **Workspace builds on current stable (CI green again)** — two latent
  breakages surfaced by the floating `stable` toolchain (CI: 1.96):

  The six vendored Infinity-Ledger crates (`vendors/infinityledger/mef-*`)
  are **auto-adopted into the root workspace** — cargo pulls path
  dependencies living inside the repository into the enclosing workspace,
  nested workspace manifest or not — so their `*.workspace = true` keys
  resolve against the *root* `[workspace.dependencies]`. Two root-level
  bumps therefore reached code that was never compiled against them:
  `rand_distr` 0.6 (built on rand 0.10) met workspace `rand` 0.9 and broke
  `mef-solvecoagula` (E0277), and the split ndarray lines (literal `0.15`
  in mef-core/mef-solvecoagula vs workspace `0.17` in mef-tic/mef-spiral)
  broke `mef-core`'s pipeline with cross-version `ArrayBase` types (E0308).
  Fix: `rand_distr` pinned back to the 0.5 line with a comment documenting
  the rand-pairing constraint, and all mef crates unified on workspace
  ndarray. The lockfile shrank: `rand 0.10.1`, `rand_core 0.10.1`, and
  `ndarray 0.15.6` left the graph.

* **clippy 1.96** — `explicit_counter_loop` in the twin
  `verify_integrity` loops (`kosmo-core/src/cartography.rs`,
  `kosmo-store/src/lib.rs`) rewritten with the `(1_u64..).zip(…)` idiom;
  two `unnecessary_sort_by` descending sorts (`kosmo-pipeline`,
  `pse-adapter-il`) now use `sort_by_key(Reverse(…))`.

* **rustdoc 1.96 under `-D warnings`** — stricter intra-doc-link scoping
  surfaced 32 latent findings once the vendored crates compiled again:
  cross-module links like `` [`ILStore::commit_as`] `` in `pse-adapter-il`
  module docs now carry explicit `(crate::…)` targets, literal brackets in
  prose (`∈ [0,1]`, tensor indices `T[1] − T[4]`, mef-spiral's PoR
  formula) are code-spanned, and the unfenced USAGE blocks in
  `nxalien-cli` / `kosmo-substrate` got the same ```text fences as the
  other tools. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace` is
  clean again.

### Changed

* **Supply-chain gates are now enforceable** (`cargo deny check` clean on
  all four invariants):

  * Every internal path dependency carries `version = "0.1.0"` (87
    insertions across kosmo-*, the adapters, and mef-core). cargo-deny's
    `wildcards = "deny"` counts versionless path dependencies as wildcard
    requirements — the pse-* crates already followed the path+version
    convention; the rest of the workspace now matches.
  * `kosmo-promote` is `publish = false` like every other `tools/` binary.
  * `deny.toml`: RUSTSEC-2024-0436 (paste — unmaintained, compile-time
    proc-macro via simba ← nalgebra) documented-ignored;
    `CDLA-Permissive-2.0` allowed (webpki-root-certs CA-bundle data via
    rustls-platform-verifier ← reqwest).
  * `security.yml`: the comment now states the actual behaviour — the
    cargo-deny action checks the `--all-features` graph (its input
    default), which is deliberately broader than what ships by default.

### Added

* **Recall — the anchored memory is queryable**

  The last missing link: remembering → finding. `kosmo-promote --recall
  <query>` runs Pfauenthron++ retrieval over the Infinity Ledger's anchored
  crystals — the query embeds via `text_to_vector8`, every crystal is ranked
  by the tripolar score `D = ψ·ρ·ω` (semantic × structural × temporal, the
  same unified retrieval the cognition system uses for its own memory), and
  the top hits return with QTIC class, stability, provenance (the promotion
  scope travels as the crystal's `question`), and the **causal lineage** of
  the best hit. Read-only by contract: `--recall` requires `--ledger`, a
  missing ledger is a hard error, and recall never creates one — no silent
  empty store. First query ever over promoted substrate knowledge, live:
  `--recall "missing test coverage for python module"` → `D=0.4668 | Q5 |
  stability 0.76 | kosmo-promote:/tmp/polyglot7`, lineage: causal root.

* **Full QTIC — the Infinity-Ledger lift (Q5 for promoted substrate knowledge)**

  The meeting point `docs/CONVERGENCE.md` §5 named is live: the two HDAGs meet
  *through the crystal*. `kosmo-promote --ledger <path>` anchors every accepted
  crystal via the canonical `pse_adapter_il::ILStore::commit_with_feedback`
  path — a ledger block (the canonical trace anchor), an IL-HDAG node (the 5D
  resonance tensor the code-born structure acquires), and the path-invariance
  check — and the returned certificate supersedes the promotion path's Q3
  ceiling. Live on a seven-language workspace, accepted crystals report
  **`QTIC Q5 — Path-invariant QTIC — full QTIC (PathInv = 1)`** with their
  block hash: the same conformance class the cognition system awards its own
  best memory. Anchoring is deduplicated per unique crystal, **idempotent**
  (an identical crystal re-anchors to the same block; the ledger does not
  grow — pinned by test), and a host write, so the flag is the operator's
  authorization and nothing acts outside `--offer` mode.

* **Resonance — substrate knowledge crystallizes in the cognition engine**

  The calibration finding, fixed end to end, with each blocker made visible
  before each fix. New gate diagnostics print the eight Kairos metrics against
  the effective thresholds plus the engine state, so *why deferred* is always
  answerable. They exposed two structural blockers in sequence: (1) the engine
  forms graph edges *pairwise within a batch* and the adapter mapped every
  candidate to the **same** vertex, so the connectivity metric `j` was pinned
  at zero — fixed by per-candidate vertex identity
  (`observation_source_id`) and `offer_batch`, which co-observes the
  candidates as one ensemble (`j` 0.000 → 0.935 live) while attribution stays
  per-candidate and honest via `crystal.region` membership; (2)
  `engine: Rejected("consensus failed")` — the carrier physics is sensor-
  tuned — addressed by `--calibration substrate`, which follows the
  `preset_anomaly_detection` rationale verbatim (the fully-armed 8-fold
  conjunctive Kairos gate is the discriminant; the cascade consensus stands
  down). Result, live on a 7-language workspace: **30/30 ACCEPTED**, real
  `SemanticCrystal`s (stability 0.76, region = the full 30-vertex ensemble),
  archived and warm-starting the next session. Every relaxation is an explicit
  operator choice (`--batch`, `--ticks`, `--calibration`); the conservative
  default calibration stays fail-closed and commits nothing — pinned by test.

* **QTIC certificates, the memory→action loop, and the convergence map**

  Three pieces rounding the unification into an operable whole. (1) Every
  crystal committed through the promotion path receives a **QTIC conformance
  certificate** (`pse_adapter_kosmo::qtic_for_promoted`): classes are earned,
  never granted — unanchored promotions honestly cap at Q3 with
  `trace_ready`/`path_inv` naming exactly what an IL commit adds. (2)
  `kosmo-promote --feedback <path>` closes the loop in the other direction:
  engine verdicts persist as `PromotionFeedback` (`Accepted` → full fitness,
  `Deferred` → ¼, `Rejected`/`Skipped` → zero) and load into the next run's
  `prior_feedback`, where pipeline Step 5c folds them into
  `NormFitnessTrace`s — memory shapes the next run's ranking. (3)
  [`docs/CONVERGENCE.md`](docs/CONVERGENCE.md) maps every echo structure
  between the layers (energy, crystals, gates, Metatron², HDAG², stores) with
  per-pair verdicts and the **never-converge list** (dependency direction,
  arithmetic split, decision split, fail-closed defaults). Plus: the README
  two-layer notice now reflects the validated, connected state, and an MSRV
  violation in `pse-adapter-il` (`is_multiple_of`, 1.87+) that would have
  broken the 1.82 CI job is fixed.

* **Promotion memory — cross-session engine state and the CAD library as a source**

  The promotion path learns. `kosmo-promote --state <path>` persists the PSE
  engine's crystal archive across sessions (JSON array of `SemanticCrystal`)
  and warm-starts `PatternMemory` on the next run — the same `pse-core`
  cross-session mechanism `cross_session_proof.rs` uses — so repeated offers of
  recurring substrate output build the resonance that can eventually flip
  `Deferred` into `Accepted`. The explicit flag is the operator's write
  authorization (the `--store` precedent); report-only never writes, and a
  corrupt archive is a **hard error** with the file left untouched — never a
  silent cold start over discarded memory. And the durable CAD library itself
  became a promotion source: `StructuralCrystalRecord` now carries its
  certifying candidate's `evidence_bundle_id` as a first-class content-addressed
  field (CROSS-006 direct instead of transitive), so `kosmo-promote --store
  <cadlib.jsonl>` can wrap store-loaded crystals without resolving candidates —
  integrity-checked first, a tampered record is a hard error and is never
  offered. `crystal_to_pse_candidate` simplifies accordingly (evidence from the
  record; the pipeline's candidate-lookup disappears). +5 binary-integration
  tests (archive round-trip, report-only inertness, corrupt-archive,
  store-to-engine, tampered-store).

* **Substrate→core unification — certified crystals flow into the PSE engine**

  The integration `SUBSTRATE.md` had deferred pending empirical validation
  (147/147) is implemented **end to end**. Offer side:
  `kosmo-pipeline::crystal_to_pse_candidate` wraps every crystal certified in a
  run as a `PseBridgeCandidate` of new kind `CertifiedCrystal` (confidence
  `(ρ+ω)/2` in `Q16`, cross-language fingerprint as metadata, evidence-bound).
  Consumption side: the new `adapters/pse-adapter-kosmo` canonicalizes
  candidates into PSE `Observation`s — fail-closed at the crossing
  (unparseable, tampered, evidence-free, or disallowed-kind payloads are never
  ingested) — and `offer_candidate` feeds them through `pse_core::macro_step`
  under full policy gating; the `Q16→f64` seam is here, confidence becoming the
  semantic `phase_hint` so structurally similar crystals can resonate. PSE
  alone decides crystallization: committed crystal → `Accepted`, clean
  ingestion without crystallization → `Deferred`. The operator entry point is
  the new `kosmo-promote` CLI (report-only by default, `--offer` to feed the
  engine in-memory, `--json`), live-verified over a seven-language workspace
  (30 candidates ingested, commit_index 30). A capstone integration test
  drives the whole stack — clean-taint yield → gate cascade → certified,
  fingerprinted crystal → bridge → real engine — for crystals from three
  languages, plus the fail-closed counterpart (ReportOnly never touches the
  engine). The dependency direction holds: no `kosmo-*` crate imports `pse-*`;
  the adapter lives on the PSE side and consumes the bridge.

* **The `kosmo-*` layer passes every CI gate (production readiness)**

  The toolchain pins floating `stable`, which had moved to rustc/clippy 1.94 —
  newer lints fired as errors under the CI `RUSTFLAGS="-D warnings"` gate, and
  the layer had never been run through `cargo fmt` (the `pse-*` core was
  already clean on both). Fixed every clippy violation across all 19 `kosmo-*`
  crates and 5 tools (all behavior-preserving; a workspace `clippy.toml`
  raises `too-many-arguments-threshold` to 12 because content-addressed
  record constructors take one argument per content field), formatted the
  whole layer with `cargo fmt` (74 files, pure formatting), and repaired every
  broken intra-doc link for `cargo doc -D warnings`. The full gate set —
  fmt --check, clippy --all-targets, test --all-targets, test --doc,
  doc --no-deps, all under `-D warnings` — is green for the substrate layer.

* **Cross-language substrate — seven languages into one hypercube**

  The Rust-only link at the head of the cube chain is gone.
  `kosmo-hyphae::xlang` lifts Python, JavaScript, Go, C, Java, and C++ source
  into the **same** content-addressed `CodeHDAG` a Rust file produces (the
  language taxonomy mined from the PSE-Codex corpus and its `normalize`
  Rosetta table; its tree-sitter + `f64` spectral machinery deliberately not
  ported — CROSS-007). Keyword-anchored extraction for the corpus four;
  a deliberately conservative heuristic for the keyword-less C family that
  under-counts rather than emit a false positive. A content-addressed `Q16`
  `CrossLanguageFingerprint` (function/type/import/test densities,
  integer-only `similarity`) makes structure comparable across languages:
  `HostCube` stores one per void, `SourceCube`s gain a
  `cross_language_resonance` dimension, and certified crystals carry the
  fingerprint into the CAD library so `crystal_resonance` matches voids
  against prior crystals **across languages and across runs** (a Go crystal
  can resonate with a structurally-similar Python void). Verified end-to-end:
  `kosmo-substrate` over a 7-language workspace produces voids with
  HDAG-scaled severities for every file. Also consolidated the wish-to-system
  machine's documentation into [`docs/WISH_TO_SYSTEM.md`](docs/WISH_TO_SYSTEM.md)
  ("CAD/CAM for software") and raised `kosmo-llm`, `kosmo-intent-llm`, and
  `kosmo-pse-bridge` module docs to reference level.

* **`Service` facet — observe by serving and probing (Runtime floor, beam 5)**

  The deepest observation, and the **completion of the Runtime floor**: a wish
  realized only when the artifact, **started as a server**, answers an HTTP
  probe. `kosmo-sandbox` gains a service mode — `serve_and_probe` picks a free
  loopback port, starts the server (told its port via `KOSMO_PORT`), detects
  readiness by *actually probing* over a raw-TCP HTTP/1.1 request (retry until it
  answers or the budget elapses — a server that binds but isn't serving yet is
  not mistaken for ready), then **group-kills** the whole tree (a server never
  exits on its own), returning a content-addressed `ServiceWitness`. A `Service`
  facet keys on `"method:path=>expect"` (`GET:/health=>200`; status and/or
  `body~<substr>`). `scaffold_service` ensures a bin + `// kosmo:service:`
  marker (sharing the bin-probe mechanism with `Run`);
  `observe_workspace_service` pre-builds, then for each marker serves + probes,
  emitting the facet only if the server answers and matches (`service_matches`
  fail-closed — never-ready/spawn-failed never counts). `kosmo-run` self-selects
  service observation (`wish_needs_service`); a `service` trigger compiles the
  prose. Verified end-to-end with a std-only HTTP server: `a service
  GET:/health=>200` realizes when served, is rejected over an empty `main`.
  Live `--pruefstand --validated` now reports **13/13** — the keystone proven at
  all three boundaries (unit, process, service). +12 tests (kosmo-sandbox 9→13,
  kosmo-core 383→384, kosmo-synthesizer 35→36, kosmo-intent 67→72, kosmo-run +1).

* **Runtime capstone + Prüfstand extension (Runtime floor, beam 4)**

  The runtime floor proven end-to-end and measured empirically. A **runtime
  capstone** (`tools/kosmo-run/tests/capstone.rs`) drives the spec's §5 worked
  example through the real CLI: a typed `Contract`, a unit `Behavior`, and a
  `Run` probe of the built binary over one little calculator — the *same truth*
  validated at the function boundary and the process boundary
  (`add(2,3)=>5` **and** `run add,2,3=>out~5`), reaching `3/3 REALIZED`. The
  **Prüfstand corpus** (`pruefstand.rs`) gains the keystone at the process
  boundary in both directions — `run-correct` (a binary that prints the sum →
  realized) and `run-empty` (an empty `main` → rejected) — via a new `bin`
  scenario flag that writes `src/main.rs`. Live `--pruefstand --validated` now
  reports **11/11 matched** across every facet axis plus the keystone at both
  boundaries. +1 capstone integration test (2→3).

* **`Run` facet — observe by executing the artifact (Runtime floor, beam 3)**

  The level-5 keystone is live: a wish is realized only when the **running
  program** exhibits the probed behaviour — from *"the test is green"* to *"the
  program, run, does the right thing."* A `Run` facet keys on `"args=>expect"`
  (`add,2,3=>out~5` — comma-args passed after `cargo run --`; `exit:<n>` and/or
  `out~<substr>`). Symmetric with `Behavior`: `kosmo-synthesizer::scaffold_run`
  ensures a bin target and writes a `// kosmo:run:` marker (the stub `main` is
  honestly empty — red until it prints the right thing);
  `kosmo-intent::observe_workspace_runtime` reads the markers, **executes** each
  probe through `kosmo-sandbox`, and emits the facet only on a *clean exit that
  matches* (`run_matches` is fail-closed — a timeout/crash never counts, even if
  the substring was printed). `kosmo-run` self-selects runtime observation when
  the wish carries a `Run` facet (`wish_needs_runtime`), and a `run` trigger
  compiles the prose. Verified live both ways: `a run add,2,3=>out~5` over a
  binary that prints the sum → `0/1 → 1/1 REALIZED` (exit 0); over an empty
  `main` → `✗ Run …`, exit 1 — rejected. +9 tests (kosmo-core 382→383,
  kosmo-synthesizer 33→35, kosmo-intent 62→67, kosmo-run +1).

* **`kosmo-sandbox` — the execution sandbox (Runtime floor, beam 2)**

  The load-bearing infra of the Runtime floor (`docs/RUNTIME-floor.md` §4): the
  safe room in which a built artifact — possibly code the loop generated — is
  run and *trusted as observed*. A capability, not a gate. **Enforced (Unix):**
  the child runs in its own process group (`CommandExt::process_group`) so a
  **timeout fells the whole tree** via `killpg(SIGKILL)` — a hung grandchild
  (the binary `cargo run` spawns) cannot outlive the budget; stdout/stderr are
  drained on their own threads into **capped buffers** (a runaway printer is
  truncated, never an OOM or pipe-deadlock); the child is always reaped. Every
  run returns a content-addressed `RuntimeWitness { verdict, exit_code, stdout,
  stdout_digest, duration, truncated }` — the digest witnesses the full output
  even when the captured text is truncated. **Honest best-effort:** network
  isolation is *declared* (`NetworkPolicy::Deny` clears proxy env) but not yet
  hard-enforced — the crate does not claim isolation it cannot deliver (spec
  §8.1); filesystem containment is by the caller's throwaway `cwd`. +9 tests
  (exit codes, stdin, prompt kill of an infinite loop and a backgrounded child,
  output truncation, spawn-failure-as-verdict).

* **Behavioural composition — validated data-flow (Runtime floor, beam 1)**

  The sandbox-free on-ramp to the Runtime floor (`docs/RUNTIME-floor.md` §6): a
  piped behaviour spec `f(x)>>g>>h=>expected` desugars to the nested call
  `h(g(f(x)))` and is validated green by the *existing* `cargo test` judge — the
  level-2 keystone applied to a level-3 wire, proving data actually *flows*
  through a composition, not merely that the types align. Implementation is
  deliberately tiny and reuses everything: `kosmo-synthesizer::parse_behavior_key`
  detects `>>` and folds the pipeline (`parse_pipeline_behavior_key`; `>>`
  overloads `>`, so it splits on `=>`/`>>` as plain strings rather than the
  bracket-depth path); `kosmo-intent` adds a `flow`/`pipeline` trigger that
  compiles to that Behavior facet. Observation is **untouched** — the
  `// kosmo:behavior:` marker carries the piped key verbatim, so it round-trips.
  Verified live in both directions: `a flow parse("2+3")>>eval=>5` over a correct
  pipeline descends `0/1 → 1/1 REALIZED` (exit 0); over an `eval` that returns 6
  it stays `0/1`, `✗ Behavior …`, exit 1 — rejected. +5 tests (kosmo-synthesizer
  30→33, kosmo-intent 61→62, kosmo-run +1).

* **Prüfstand — empirical fidelity harness (`--pruefstand`)**

  The capstone generalized into a *corpus*. A new `kosmo-run --pruefstand` mode
  (module `tools/kosmo-run/src/pruefstand.rs`) descends a built-in reference
  corpus of nine known systems — one per facet axis the floor builds
  (symbol/contract/module/capability/test/composition/archetype) plus the
  behavioural keystone in **both** directions — and checks the verdict the
  substrate reaches against the verdict it *should* reach. It measures one
  thing: does the loop accept exactly the systems it should? Each scenario runs
  in a throwaway workspace through the real descent; structural ones converge
  offline, behavioural ones run `cargo test` (gated by `--validated`), and an
  unobservable scenario is *skipped*, never miscounted as a failure. Verified
  live: `--pruefstand --validated` → **fidelity: 9/9 matched (0 mismatched,
  0 skipped)**, exit 0 — including `behavior-wrong` (impl returns `a+b+1`)
  correctly **rejected**. Exit 3 if any verdict is wrong. +2 unit tests
  (kosmo-run 14→16).

* **Capstone — the whole Horizon floor in one runnable artifact**

  A binary-level integration test (`tools/kosmo-run/tests/capstone.rs`) that
  drives the **real `kosmo-run` CLI** end-to-end (`CARGO_BIN_EXE_…`), proving the
  keystone — *acceptance over generation* — in both directions through the actual
  binary, not an in-process helper:
  - **correct** impl (`add(a,b)=a+b`) + a typed `Contract` and a `Behavior` spec
    → `1/2 APPROACHING → 2/2 REALIZED`, exit 0, the spec-test marker written;
  - **incorrect** impl (`a+b+1`) + the same behaviour → stays `0/1 UNSTARTED`,
    `✗ Behavior add(2,3)=>5`, exit 1 — the system does not lie.
  The positive case validates by running `cargo test` nested; it skips (not
  fails) where the sandbox forbids nested cargo, while the negative case is
  robust regardless. This is the first reference scenario for the empirical
  Prüfstand. +2 integration tests.

* **Composition facets — typed data-flow wiring (Horizon floor, level 3)**

  The behavioural cousin of `Dependency`: a `Composition` facet `"from>>via>>to"`
  says `from` returns type `via` and `to` consumes `via` — the components *wire
  together* (`to(from(x))` typechecks), not just coexist. `kosmo-intent`
  **derives** compositions from the observed contracts (`derive_compositions`:
  for every ordered pair whose return type matches the next first-parameter type
  and isn't unit, emit `f>>T>>g`), so the facet is observed for free from the
  Contract layer. `kosmo-synthesizer::scaffold_composition` realizes one by
  appending two type-compatible stubs in a *single* change (`append_items_to_lib`
  avoids the same-path overwrite) — `pub fn from() -> via` and
  `pub fn to(_a0: via)` — which the observer then derives back, so it
  round-trips. The `composition`/`compose` trigger accepts
  `a composition parse>>String>>eval`. Verified live: that wish descends
  `0/1 → 1/1 REALIZED` and `cargo build` is clean. +8 tests (kosmo-core 381→382,
  kosmo-intent 57→61, kosmo-synthesizer 28→30, kosmo-run 13→14).

* **Crate-targeting — scaffold into the named member crate (multi-crate full-stack)**

  A facet key may carry an `@<crate>` suffix to scaffold the item *into that
  workspace member* instead of the root crate — the precondition for real
  multi-crate full-stack wishes. `kosmo-synthesizer::FacetScaffolder` resolves
  the crate by package name (`find_crate_manifests`), runs the per-kind
  scaffolder as if the crate dir were the root, and re-bases the change paths to
  workspace-root-relative (`scaffold_into_crate`); an unknown crate is an honest
  no-op. `kosmo-intent::facets_from_rust_dir` now emits each source item twice —
  bare and crate-qualified `<key>@<crate>` (via `crate_of` / the nearest
  `[package]` manifest) — so the targeted item round-trips. Applies to in-crate
  kinds (Symbol/Signature/Contract/Module/Capability/Test); Crate/Dependency/
  Behavior stay workspace-level. The `@crate` form survives the prose compiler,
  so `a function handle@api` works. Verified live:
  `--wish "a function handle@api and a function compute@core" --apply` lands
  `handle` in `crates/api` and `compute` in `crates/core`, root untouched,
  `0/2 → 2/2 REALIZED`. +6 tests (kosmo-intent 55→57, kosmo-synthesizer 25→28,
  kosmo-run 12→13).

* **Archetype expansion — full-stack fan-out (Horizon floor, beam 3)**

  The breadth axis of `docs/HORIZON-behavior-archetype.md`: one prose word
  expands into a *bundle* of facets. `kosmo-intent` grows a compiler stage above
  the leaf triggers — `compile_wish` recognizes archetype keywords
  (`crud` / `endpoint` / `component`) and `expand_archetype` fans each into the
  existing leaves. `crud <name>` → a module, `create_`/`get_` typed handlers
  (`String→String`, so they compile), and a `crud:<name>` capability marker;
  `endpoint <name>` → a typed handler + marker; `component <name>` → a module +
  marker. Elegantly, this adds **no new facet kind and no scaffolder change** —
  archetypes are pure templates over leaves the substrate already builds and
  observes, so the structural bundle converges offline. Archetype keywords are
  reserved (kept disjoint from leaf triggers and common names). Verified live:
  `--wish "a crud user" --apply` descends `0/4 → 4/4 REALIZED` in one step.
  +5 tests (kosmo-intent 51→55, kosmo-run 11→12).

* **`Behavior` facets — validated behaviour, the keystone (Horizon floor, beam 2)**

  The load-bearing beam of `docs/HORIZON-behavior-archetype.md`: a `Behavior`
  facet `"name(args)=>expected"` is satisfied **only when a scaffolded spec-test
  pinning that input→output pair actually passes**. `kosmo-synthesizer`
  scaffolds a `// kosmo:behavior:`-marked `#[test]` asserting
  `name(args) == expected` — **red** until the body is correct. `kosmo-intent`
  observes it through the suite: `behavior_specs_from_source` pairs each marker
  to its test fn, and `behavior_facets` (fail-closed) emits the facet *only* for
  specs whose test is green — so `observe_workspace_validated` is the
  deterministic judge. `kosmo-run` auto-enables validated observation for any
  wish carrying a behaviour. The `behavior` / `spec` trigger accepts the
  space-free prose form `a behavior add(2,3)=>5`.

  This is **acceptance over generation**: the guarantee moves from *writing*
  correct code to *accepting* only what is observed correct. Verified live —
  with a correct `add`, the wish descends `0/1 → 1/1 REALIZED`; with a wrong
  `add` (`a+b+1`) it honestly stalls at `0/1 UNSTARTED` (exit 1) rather than
  lying. +12 tests (kosmo-core 380→381, kosmo-intent 45→51, kosmo-synthesizer
  22→25, kosmo-run 9→11).

* **`Contract` facets — typed function signatures (Horizon floor, beam 1)**

  The first beam of the behavior axis (`docs/HORIZON-behavior-archetype.md`):
  a `Contract` facet `"name(T0,T1)->R"` is the typed promotion of `Signature`
  (`name/arity`). `kosmo-intent` now observes parameter + return *types* from a
  function's opening line (`parse_fn_types`, shallow types; receivers skipped),
  emitting a `Contract` alongside the existing `Symbol` / `Signature` (additive
  — a `Signature` wish is unchanged). `kosmo-synthesizer::scaffold_contract`
  builds the dual: a typed stub `pub fn name(_a0: T0, …) -> R { todo!(…) }` —
  structurally present, **honestly empty** at runtime. Scaffold → observe
  round-trips, so a contract wish descends `0/1 → 1/1 REALIZED`. The `contract`
  trigger (rule + LLM compilers) accepts the space-free prose form
  `a contract add(i32,i32)->i32`. Verified live: the stub compiles
  (`cargo check` clean) and `todo!()` panics until filled. +14 tests
  (kosmo-core 378→380, kosmo-intent 39→45, kosmo-synthesizer 17→22, kosmo-run
  8→9).

* **`--wish-session <path>` — every descent is now auditable and replayable**

  `kosmo-run --wish … --wish-session trajectory.json` writes the complete
  `WishSession` as pretty-printed, content-addressed JSON after every run (with
  `--apply`) or observation (without `--apply`). If the file already exists and
  carries the same wish id, it is loaded and the descent **resumes** from the
  prior trajectory — the session accumulates iterations across invocations.
  `load_prior_session` guards the identity check: a session for a different wish
  is always discarded rather than silently merged. +2 tests (JSON round-trip;
  save + load + identity rejection).

* **`Dependency` facets are now deterministically scaffoldable — the last offline gap closes**

  `FacetScaffolder::scaffold_dependency` realizes a `"from->to"` edge by locating
  both crates by package name in the workspace and adding `to = { path = "<rel>" }`
  to `from`'s `[dependencies]` (idempotent; honest no-op if a crate is missing).
  Every structural facet — Crate · Module · Symbol · Signature · Capability ·
  Test · Dependency — now converges deterministically, no LLM and no keys.
  Verified live: a `dependency alpha->beta` wish descends `0/1 → 1/1 REALIZED`,
  then `cargo metadata` confirms the edge. +2 tests (scaffold + end-to-end descent).

* **Descent unifies both `Wish → Patch` backends — deterministic-first, LLM-fallback**

  In `--wish --apply`, facets the `FacetScaffolder` can't build (e.g. a
  `Dependency` edge) now fall through to the LLM synthesizer when a `--provider`
  is given (`mock`/`claude`/`cerebras`); the deterministic scaffolder still wins
  whenever it produces a patch. The same `ActionSynthesizer` contract on both
  sides of the loop. +1 test (the fallback is consulted only when the scaffolder
  is empty).

* **`kosmo-run --wish … --apply` — the attractor descent, executed**

  Wish mode gains a convergence loop: observe → assess → scaffold every unmet
  facet → write it → re-observe, until the wish is realized (or no progress is
  possible / max 8 iterations). Driven by a `WishSession` so the full trajectory
  is recorded and printed (`iter 0: met 0/3 UNSTARTED → iter 1: met 3/3
  REALIZED ✓`). `apply_scaffold` is the only filesystem write, gated by `--apply`.
  Verified live: a prose wish for two functions and a new crate converges in a
  single descent step. +1 test.

* **`kosmo-run --wish "<prose>"` — the wish-to-system machine on the command line**

  The agent runner gains a deterministic, **offline** wish mode: compile a
  plain-prose wish (rule compiler), observe the workspace (`--validated` adds
  green tests), and report the distance — which facets are present, which are
  missing — with an exit code that is 0 only when the wish is realized.
  `--scaffold` prints the `FacetScaffolder`'s proposed file changes (dry run);
  `--json` emits the assessment. No LLM and no key required, e.g.
  `kosmo-run --wish "a crate kosmo-api and a function handle" --scaffold .`.
  Verified live against this repo. +3 tests.

* **Green tests — `Test` facets bound to validated behaviour**

  `kosmo-intent::observe_workspace_validated(root)` runs the suite and replaces
  lexical `Test` facets (mere presence) with the set of tests that actually
  *pass*: `parse_test_results` (pure libtest-output parser) + `passing_test_facets`
  + `run_workspace_tests`, with `ObservedTopology::retain` in kosmo-core doing the
  swap. A `Test` wish now means "a *green* test named X" — the strongest binding
  of a wish to validated behaviour. Opt-in (heavy: runs the suite); falls back to
  lexical presence if the run can't start. +3 tests incl. a live green-vs-red run.

* **Richer facets IV — `Test` (tests as wish targets)**

  `WishFacetKind::Test` (keyed by test fn name). `facets_from_source` (now
  stateful) detects `#[test]`/`#[tokio::test]` + the following `fn NAME` →
  `Test(name)`. `FacetScaffolder` emits `#[test] fn name() {}`; rule compiler
  (`test`) + LLM mapping. v1 observes test *presence*; tying "green" to the
  cargo validator (a test that *passes*) is the next refinement. +4 tests.

* **Richer facets III — `Capability` (behaviours as wish targets, via markers)**

  Makes the existing `Capability` kind *observable*: `facets_from_source` reads
  `// kosmo:capability: <name>` markers (also `//!`). `FacetScaffolder` writes
  the marker; rule compiler (`capability`/`feature`) + LLM mapping. A wish can
  now target a named behaviour — the facet closest to human intent. +3 tests.

* **Richer facets II — `Signature` (function arity as a wish target)**

  `WishFacetKind::Signature` (keyed `"name/arity"`). `facets_from_source` now
  emits a `Signature` per `pub fn` alongside the `Symbol` (arity counted from the
  opening line; generics/arrays don't inflate it). `FacetScaffolder` realizes it
  (`pub fn name(_a0: (), …)`, via a shared `append_to_lib`); the rule compiler
  (`signature`/`sig`) and the LLM mapping recognize it. +4 tests.

* **Richer facets I — `Dependency` (crate dependency edges as wish targets)**

  `WishFacetKind::Dependency` (keyed `"from->to"`). `facets_from_snapshot` emits
  one per ParseBack `dep_edge`, so a wish can require "crate A depends on B".
  The rule compiler (`dependency`/`depends`) and the LLM kind-mapping recognize
  it; `WishFacet::dependency(from, to)` builds one. No structural scaffold (a
  dependency edit needs a path the scaffolder can't infer). +2 tests.

* **`FacetScaffolder` — the loop builds toward a wish offline, deterministically**

  A deterministic `ActionSynthesizer` (in `kosmo-synthesizer`) that realizes
  `RealizeWishFacet` actions without an LLM:
  - `Symbol` → append `pub fn <name>() {}` to `src/lib.rs`/`main.rs` (idempotent);
  - `Module` → create `src/<name>.rs` + add `pub mod <name>;` to the crate root;
  - `Crate` → create `<name>/Cargo.toml` + `src/lib.rs`, best-effort `[workspace]
    members` registration.

  It reads the workspace to stay idempotent (an already-realized facet → empty
  patch). The deterministic counterpart to the LLM synthesizer: it writes only
  the structural skeleton, but makes the build-toward-intent loop runnable and
  verifiable with no model. 5 new tests incl. end-to-end
  `agent_wish_builds_symbol_and_converges` — in apply mode the loop finds a
  Symbol absent (distance `ONE`), scaffolds it, and the next run observes it
  realized (distance `ZERO`), offline.

* **LLM ends, real — shared `kosmo-llm` transport + `kosmo-intent-llm` (prose → Wish)**

  The natural-language front door now has a real LLM backend, behind the same
  deterministic contract as the rule compiler.

  - `kosmo-llm` — a shared LLM transport crate: `LlmConfig` / `LlmProvider`
    (Anthropic Messages API + any OpenAI-compatible endpoint), `complete(system,
    user)` with 429/529/5xx retry+backoff, `config_from_env`, and a string-aware
    brace-balanced `extract_json_object`. The one non-deterministic step in the
    substrate now lives in exactly one place. (`kosmo-synthesizer-llm` keeps its
    own transport for now; migrating it onto `kosmo-llm` is a planned cleanup.)
  - `kosmo-intent-llm::LlmWishCompiler` implements `kosmo-intent::WishCompiler`:
    prose → a JSON facet list → a content-addressed `Wish`. Drops into the agent
    loop exactly where the rule compiler does. The model emits facets; the `Wish`
    id stays deterministic over the sorted/de-duped facet set + prose label.
  - 23 new tests (config/transport shapes; JSON extraction incl. fences, nested
    braces, escaped quotes; prompt building; response parsing incl.
    unknown-kind/empty-key dropping; fail-fast on empty API key). Live calls are
    gated by credentials, never hit in tests.

* **The human front door — natural-language → `Wish`**

  A person can now state intent in prose and get a structured, content-addressed
  `Wish` the loop can descend toward.

  - `kosmo-intent::compile_wish(prose, policy_id, evidence)` — a deterministic,
    dependency-free compiler that scans prose for structural triggers
    (`crate`/`package`, `module`/`mod`, `function`/`fn`/`method`,
    `type`/`struct`/`enum`/`trait`/`symbol`) and turns each `keyword NAME` phrase
    into a required facet. The prose becomes the wish's label (part of its
    identity). Handles backticks/quotes and fillers ("a crate called `X`").
  - `WishCompiler` trait + `RuleWishCompiler` reference impl — the extension
    point for an LLM-backed compiler (the counterpart to `kosmo-synthesizer-llm`),
    where the model is the only non-deterministic part and the emitted `Wish`
    stays content-addressed.
  - 10 new tests incl. end-to-end `agent_wish_from_prose_realized`: prose →
    `Wish` → the loop measures the real workspace and realizes it. Convention:
    name the thing after the keyword ("a crate kosmo-server, a module routes").

* **Finer granularity — Module/Symbol facets from a name-preserving extractor**

  A wish can now target more than whole crates: modules and public symbols.

  - `kosmo-intent::facets_from_source(&str)` — a deterministic, dependency-free
    lexical Rust extractor that emits `Module` facets (`mod`, public or not) and
    `Symbol` facets (public `fn` / `struct` / `enum` / `trait` / `type` /
    `union` / `const` / `static`), keyed by bare name. Handles `pub(...)`, fn
    modifiers (`async`/`const`/`unsafe`), generics, and skips comments/attrs.
  - `facets_from_rust_dir(dir)` walks a tree's `.rs` files (skipping
    `target`/`.git`); `observe_workspace_deep(root)` merges crate facets (cargo
    metadata) with module/symbol facets — crate + module + symbol granularity.
  - `kosmo-agent` now observes via `observe_workspace_deep`, so the loop can
    measure and build toward module/symbol wishes, not just crate-presence ones.
  - 10 new tests incl. an end-to-end symbol-granularity wish realized through
    the agent loop. Known limits: bare-name keys (no crate qualification yet),
    `extern` and macro-generated items not captured.

* **The wish builds toward itself — facet-directed synthesis (the loop closes)**

  The generation half of the wish arc: the agent no longer just *measures* the
  gap to the wish, it *acts* to close it.

  - `ActionItemKind::RealizeWishFacet { facet }` (kosmo-pipeline) — a first-class,
    intent-directed action carrying the unmet `WishFacet` (the counterpart to
    `FillVoid`, on the intent axis). The pipeline scan never emits these; the
    agent does, from the wish agenda.
  - Each `AgentSession::run()` with a wish observes the workspace at the start,
    turns each unmet facet into a top-priority `RealizeWishFacet` action, and
    prepends it to the queue — so the loop builds *toward* the wish before it
    repairs voids. `AgentRunReport::wish_directed_count()` reports how much
    directed work a run did.
  - The synthesizer is told exactly what to build: the LLM prompt
    (`kosmo-synthesizer-llm`) gains a `RealizeWishFacet` directive; the tool
    renderers (`kosmo-substrate` / `kosmo-tui` / `kosmo-server` / `kosmo-run`)
    label the new kind.
  - End-to-end proof: `agent_wish_builds_toward_and_converges` runs the loop in
    apply mode with a scaffolding synthesizer — run 1 finds the wished crate
    absent (distance `ONE`) and writes it; run 2 observes it realized (distance
    `ZERO`). The repair loop is now a build-toward-intent loop that converges.
  - 4 new agent tests (17 → 21); fixed a pre-existing flake in
    `agent_run_id_is_deterministic` (now scans an isolated dir, not shared temp).

* **`kosmo-agent` — the wish governs the loop (observe · converge · enforce)**

  The wish now drives the execution loop, not just the type system. Attach a
  wish and each `run()` measures the workspace against it and tracks convergence
  toward the attractor across runs — fail-closed on divergence.

  - `AgentSession::with_wish(wish, evidence_bundle_id)` attaches an internal
    `WishSession`. Each `run()` observes the workspace via `kosmo-intent`
    (read-only `cargo metadata`) and folds the distance into the trajectory —
    one `run()` is one step of the dynamics `x_t → x_{t+1}`. Fail-soft: a
    non-cargo workspace leaves the run intact with no wish outcome.
  - `AgentRunReport.wish: Option<WishRunOutcome>` carries the run's
    `WishAssessment`, the cross-run `AttractorStatus`, a `diverged` flag (this
    run raised the distance), and `agenda()` — the unmet facets, i.e. the
    prioritized remaining work toward the wish.
  - Contraction enforced live: `wish_diverging()` / `WishRunOutcome::diverged`
    surface a regression away from the attractor fail-closed, so a driving loop
    can halt or roll back rather than accept a step in the wrong direction.
  - `wish_trace()` / `wish_assessment()` accessors. 5 new tests incl. an
    end-to-end divergence detection across two real `cargo metadata` scans.

* **`kosmo-intent` — connect the wish ruler to the real workspace**

  The third rung of the wish-to-system arc. Runs 1–2 measured a wish against a
  hand-supplied observation; this crate reads a **real** workspace and turns it
  into one, then ties target + ruler + convergence contract together in a
  stateful session.

  - `observe_workspace(root)` / `observe_snapshot(&TopologySnapshot)` /
    `facets_from_snapshot(...)` — read-only adapter (one `cargo metadata` via
    `kosmo-parseback`) that turns crate topology into an `ObservedTopology` of
    `Crate` facets. (`Module` / `Symbol` facets need a name-preserving source
    extractor — a later run; the facet-set API merges them in without change.)
  - `WishSession` — a descent toward a wish-attractor: each `observe(...)`
    assesses the workspace (Run 1), appends the distance, and exposes a
    `WishConvergenceTrace` (Run 2). `is_contractive()` / `at_attractor()` /
    `is_converged()` surface the contract; a step that *raises* the distance is
    flagged `Diverging`, fail-closed. Serializable, so a descent persists and
    resumes across sessions.
  - 8 tests incl. a live `cargo metadata` scan of the real workspace; depends
    only on `kosmo-core` + `kosmo-parseback`.

* **`kosmo-core::attractor` — the wish as a fixed-point attractor (convergence contract)**

  Formalizes "the compressor converges". The wish is the attractor `x*`; the
  Run-1 wish distance `V` is a Lyapunov function (`V ≥ 0`, `V = ZERO` only at
  `x*`). A trajectory converges iff `V` is monotone non-increasing and reaches
  `ZERO`.

  - `WishConvergenceTrace` — content-addressed, evidence-bound record of a
    distance trajectory (`distances: Vec<Q16>`, oldest first). Derives
    `AttractorStatus` (Converged / Converging / Stalled / Diverging /
    Indeterminate) and `first_divergence: Option<u32>`.
  - Contraction invariant (intent-axis analogue of LPCM's
    `monotone_contractive_filter`): a step that *increases* `V` is a regression —
    `is_contractive()` is false and the offending index is recorded, fail-closed,
    so the loop can reject the patch that moved away from the wish.
  - `at_attractor()` (latest distance is `ZERO`) plus fixed-point stability
    (`f(x*) = x*`): extending a converged trace with `ZERO` stays converged.
  - `from_assessments(&[WishAssessment])` builds the trajectory directly from
    Run-1 output; a mixed-wish slice yields an `Indeterminate`, empty trace.
  - `MAX_STRICT_CONTRACTION_STEPS = 65537`: because `Q16` is a *discrete* lattice,
    a strictly contracting trajectory over `[0, 1]` reaches the attractor in
    bounded time — convergence is a counting argument, not an asymptotic hope.
  - Ranks, never gates. 18 new tests; zero new dependencies.

* **`kosmo-core::wish` — the Wunsch-zu-System seed: intent as a measurable target**

  The first rung of the wish-to-system arc. The substrate has always measured
  voids against an *implicit* target — structural completeness. `Wish` makes the
  target *explicit*, so the same convergence loop can later descend toward a
  stated intent instead of merely "be structurally whole". This run ships only
  the target and the ruler; wiring it into the agent loop is a later run.

  - `Wish` — content-addressed (`id = SHA-256(JCS(content))`), evidence-bound
    (CROSS-006) declaration of a desired topology: a de-duplicated, facet-sorted
    set of `WishPredicate`s (each a `WishFacet` + `Q16` weight). Predicate order
    never affects identity (deterministic replay).
  - `WishFacet` / `WishFacetKind` — positive-only structural targets
    (`Crate`, `Module`, `Symbol`, `Capability`, `Resolution`). "The bad thing
    must be gone" is modelled as a `Resolution` facet the pipeline emits once a
    void is closed — a wish is never satisfied by *absence* of evidence.
  - `ObservedTopology` — the set of facets observed present in a workspace
    (caller-supplied now; populated from the live pipeline in a later run).
  - `assess_wish(wish, observed, evidence) -> WishAssessment` — pure,
    deterministic distance function. `distance: Q16` is the weighted unmet
    fraction (`ZERO` ⇒ realized, `ONE` ⇒ nothing met); `unmet_facets` is the
    remaining **gradient** the agent loop must close. `WishClosureStatus` =
    Realized / Approaching / Unstarted / Vacuous.
  - Doctrine: the wish distance *ranks*, it never *gates* (CROSS-010 applied to
    the intent axis) — it grants no capability and bypasses no policy.
  - 21 new tests (content addressing, predicate order-independence, dedup,
    fail-closed unmet, weighted distance, unit-interval bounds, evidence-bound).

* **Git-commit-per-patch + PromotionFeedback loop — the compressor is live**

  Two orthogonal layers that close the full convergence cycle.

  **Git-commit layer (traceability)**
  - `MaterializeOptions::git_commit: bool` — when set and the outcome is
    `AppliedToHost`, runs `git add -A && git commit` with a deterministic
    message including the full `patch-id` so every accepted patch lands as a
    standalone, revertable git commit. Fail-open: commit failure is recorded
    as a diagnostic but does not un-apply the patch.
  - `MaterializeReport::commit_sha: Option<String>` carries the new SHA back
    up through `MaterializationAttempt::commit_sha` to the run report.
  - `AgentOptions::commit_to_git: bool` threads the flag from the agent
    session into `MaterializeOptions` for each materialization attempt.
  - `kosmo-run --commit` (requires `--apply`) enables git commits;
    `--apply --commit` output shows the short SHA per accepted patch.

  **Feedback loop (learning / convergence)**
  - After each synthesized step, `AgentSession` builds a `PromotionFeedback`
    record — `FeedbackOutcome::Accepted` if validation passed, `Rejected` if
    not — keyed on the action's `norm_candidate_id`.
  - At the start of every subsequent `run()` the pending records are drained
    into `WorkspacePipelineSession` via the new `extend_prior_feedback()`
    method, updating `NormFitnessTrace` scoring so the pipeline re-ranks
    candidates before the next scan. The "Wissen zurück ins Substrat" loop
    is now closed at the agent layer too.
  - `AgentSession::pipeline_feedback_pending()` exposes the queue depth.
  - 2 new agent tests: `pipeline_feedback_queued_after_synthesized_steps`,
    `pipeline_feedback_drained_into_next_run`.

  **End-to-end compressor invocation:**
  ```
  kosmo-run --provider cerebras --apply --commit --max-steps 50 .
  ```
  Iterates: scan → synthesize → validate → commit → re-scan (void set shrinks)
  → feedback re-ranks → repeat until convergence.

* **`kosmo-materialize` — the write/validate layer; agent loop closed**

  Arms the agent's `dry_run = false` path: a `Patch` can now be applied to disk,
  compiled/tested, and kept or rolled back — all policy-gated.

  - `Materializer::materialize(patch, policy, validator, options)` with
    fail-closed policy strategy: `ReportOnly` → no I/O; `DryRun` → **sandbox**
    (copy workspace to temp, apply, validate, host untouched);
    `OperatorApproved`/`AutonomousBounded` + `allow_host_write` → **in-place**
    (back up touched files, apply, validate, **roll back on failure**)
  - `MaterializeReport` content-addressed (outcome, applied-to-host, compile/test
    results); CROSS-006 evidence never ZERO; `MaterializeOutcome` =
    SkippedByPolicy / SandboxValidated / SandboxRejected / AppliedToHost / RolledBack
  - `PatchValidator` trait with real `CargoFoundryValidator` (drives `cargo
    check`/`cargo test` via `kosmo-foundry`'s hardened sandbox) and `AlwaysPass`
    / `AlwaysFail` stubs; backup/restore round-trips create/modify/delete
  - **`kosmo-agent` wired**: `AgentSession::with_validator(...)` makes the
    non-dry-run branch really apply + validate via `kosmo-materialize`; failed
    validation rolls the host back and records negative feedback
  - **`kosmo-run --apply`**: escalates to `OperatorApproved` + `CargoFoundryValidator`
    and writes validated patches to the workspace (rolling back any that fail
    cargo); default remains dry-run with no writes
  - 11 materialize tests + 2 new agent wiring tests; 0 failures

* **`kosmo-synthesizer-llm` + `kosmo-run` — real LLM backends and the agent runner**

  Turns the agent loop from a mock into a working tool driven by a real model.

  - `kosmo-synthesizer-llm`: `LlmSynthesizer` implements `ActionSynthesizer`
    over two wire protocols — **Claude** (Anthropic Messages API: `/v1/messages`,
    `x-api-key` + `anthropic-version`, `content[0].text`) and any
    **OpenAI-compatible** endpoint (`/chat/completions`, `Bearer` auth,
    `choices[0].message.content`) covering **Cerebras** (the free-tier bridge),
    OpenAI, Groq, Together, Ollama, …
  - Pure, offline-tested core: `system_prompt`, `build_user_prompt`,
    `extract_json_object` (brace-balanced, fence/prose tolerant, string-aware),
    `parse_synthesis_response` — the model returns one JSON patch object
    (`confidence_pct` integer → `Q16::ratio`, so no float crosses our boundary)
  - `LlmConfig::claude()` / `::cerebras()` / `::openai_compatible()`;
    `LlmSynthesizer::from_env()` with provider auto-detect (`ANTHROPIC_API_KEY`
    → Claude, `CEREBRAS_API_KEY` → Cerebras) and `KOSMO_LLM_*` overrides;
    temperature defaults to 0; 429/5xx retry with exponential backoff
  - Non-determinism is contained: the LLM is the only non-deterministic step;
    the returned `Patch`/`SynthesisResult` are content-addressed again
  - `tools/kosmo-run`: the agent runner CLI — `kosmo-run [--provider
    claude|cerebras|mock|env] [--model M] [--max-steps N] [--min-confidence P]
    [--all] [--json] PATH`; renders the ranked queue, synthesized patches,
    per-step confidence/lines/tokens, rationale and verify hint; dry-run only
    (report-only policy, no host writes); `mock` provider runs offline with no key
  - 14 synthesizer-llm tests (+1 ignored live Cerebras smoke test); both new
    crates registered as workspace members

* **`kosmo-agent` + `kosmo-synthesizer` — closed-loop execution layer**

  The agent/synthesis stack turns the pipeline's ranked `ActionItem` queue into
  a dry-run patch loop with content-addressed audit trail.

  - `kosmo-synthesizer`: `ActionSynthesizer` trait (pluggable backend: LLM,
    rule-based, mock); `SynthesisRequest` / `Patch` / `SynthesisResult` all
    content-addressed (INVARIANT-007); `MockSynthesizer::confident()` (Q16 0.90)
    and `::uncertain()` (Q16 0.30); `FileChange` with `Create`/`Modify`/`Delete`
    kinds; `SynthesisError { recoverable }` for transient vs permanent errors
  - `kosmo-agent`: `AgentSession::run(workspace)` — pipeline → synthesize →
    dry_run materialize → feedback loop; `AgentOptions { max_steps,
    min_confidence, dry_run, pipeline_options }`; `ExecutionFeedback` /
    `MaterializationAttempt` / `AgentRunReport` all content-addressed;
    confidence filter skips low-confidence results before materialization;
    feedback accumulates across agent runs
  - 9 synthesizer tests + 8 agent tests; 0 failures

* **Distribution — `Dockerfile.kosmo`, `install.sh`, documentation**

  Everything needed to ship the substrate tools to end users.

  - `docker/Dockerfile.kosmo` — multi-stage (cargo-chef + debian-slim) image building
    `kosmo-substrate` and `kosmo-server`; default entrypoint is `kosmo-substrate /workspace`;
    override to `kosmo-server` for the browser UI; EXPOSE 7777; healthcheck on `/api/health`
  - `install.sh` — bash install script; `./install.sh` (local path), `./install.sh --git`
    (from upstream); checks for Rust, prints quickstart after install
  - `SUBSTRATE.md` — new "Getting started" section covering all four entry points
    (CLI, TUI, browser UI, REST API) with copy-paste examples
  - `README.md` — quickstart updated with substrate tools block; "Where to go next"
    table updated with four top-of-table substrate rows

* **`kosmo-server` — HTTP server + embedded browser UI**

  A self-contained web server that exposes the pipeline over a REST API and serves
  an embedded single-page app — no build step, no npm, no external CDN.

  - `GET /` — serves the embedded browser UI (GitHub-dark theme, vanilla JS + CSS)
  - `GET /api/health` — version ping: `{ "status": "ok", "version": "..." }`
  - `POST /api/analyse` — JSON body `{ "path": "...", "flags": { ... } }` → structured report
  - Browser UI: path input, per-layer checkboxes, Analyse button with spinner;
    renders gate badge, stats grid, action queue table (top 100), crystal CAD library
    section, optional-layers summary, void priority ranking chips
  - Action kind badges colour-coded by group (void=cyan, topology=yellow, pse/norm=green)
  - `--port <n>` (default 7777), `--host <addr>` (default 127.0.0.1), `--open`
  - Pipeline runs on a `spawn_blocking` thread; axum + tokio runtime
  - `tools/kosmo-server` registered as workspace member

* **`kosmo-tui` — interactive terminal dashboard (ratatui)**

  A full-screen TUI binary for navigating workspace topology results interactively.

  - Three-pane layout: action queue list (left) │ item detail (right) │ workspace stats (bottom)
  - Scrollable action queue with scroll-position indicator (`N%`)
  - Detail pane: kind (coloured), score, target ID, action ID, word-wrapped description,
    norm name for `ApplyNorm` items
  - Header bar: workspace path, policy, gate result (coloured), session run counter
  - Status bar: full keybinding reference
  - Keybindings: `q`/`Esc`=quit, `r`=rerun, `↑↓`/`jk`=navigate, `PgUp`/`PgDn`=page,
    `g`=top, `G`=bottom, `Ctrl+c`=force quit
  - `r` rerun: shows "Analysing…" frame, re-runs the full pipeline, resets selection to top
  - Same flags as `kosmo-substrate`: `--store`, `--operator`, `--all`, per-layer flags, `--capacity`
  - `tools/kosmo-tui` registered as workspace member; deps: `ratatui 0.29`, `crossterm 0.28`

* **`kosmo-substrate` CLI binary — workspace topology analysis + ranked action queue**

  A zero-dependency command-line binary that wraps the full pipeline and renders
  results in a human-readable terminal UI, single-line summary, or structured JSON.

  - `kosmo-substrate [OPTIONS] [PATH]` — analyses any Rust workspace or directory
    and produces a priority-ranked action queue from the HYPHAE → Metatron →
    SystemCube → Crystal → PSE pipeline.
  - `--output text|json|summary` — rich ANSI terminal output (default), JSON report
    dump, or CI-friendly single-line summary.
  - `--store <path>` — persistent CAD library across invocations (tilde-expanded);
    implies `--crystals`; parent directory created automatically.
  - `--operator` — `OperatorApproved` policy (enables crystal persistence to store).
  - `--all` / individual layer flags (`--metatron`, `--lpcm`, `--systemcube`,
    `--surgery`, `--crystals`, `--norms`, `--motifs`, `--pse`).
  - `--fail-on-reject` / `--fail-on-warn` — non-zero exit codes for CI gating.
  - `--capacity <n>` — SystemCube D-density denominator.
  - `WorkspacePipelineSession` drives repeated runs; `--session run #N` shown in header.
  - Box-drawing header with ANSI-aware alignment (visual-width padding via `vlen()`).
  - `tools/kosmo-substrate` added to workspace members.

* **`ActionItem` / `IntegrationRunReport::action_items()` — CAM layer**

  The pipeline now completes the CAD/CAM metaphor by producing a single,
  unified, priority-ranked work queue from the full diagnostic report.

  - `ActionItemKind`: `FillVoid { void_id }`, `RepairTopology { surgery_option_id }`,
    `PromoteToPse { candidate_id }`, `ReviewCrystal { candidate_id }`,
    `ApplyNorm { norm_candidate_id, name }` — covers all five actionable categories.
  - `ActionItem`: content-addressed (`action_id = Digest::of(kind_tag, target_id, policy_id)`),
    carries `priority_score: Q16` and a human-readable `description`.
  - `rank_score(pos, total)` — position-based Q16 priority derived from within-category
    energy ranking: `Q16::ONE` for the top-ranked item, proportionally decreasing.
  - `IntegrationRunReport::action_items()` — merges all five categories into a
    single `Vec<ActionItem>` sorted descending by `priority_score`. Only `EvidenceOnly`
    crystal candidates appear as `ReviewCrystal`; `Pending`/`Certified` need no review.
  - 7 new pipeline tests (120 total); 2 new eval scenarios (147 total, 927 substrate tests).

* **`run_workspace_pipeline` + `WorkspacePipelineSession` — single-call filesystem entry point**

  The pipeline is now directly usable on any filesystem path — no manual workspace
  construction required. This is the "next instance zur Software-Produktion" entry point.

  - `run_workspace_pipeline(root, options, policy)` — equivalent to
    `WorkspaceIndex::scan_path_with_content` + `run_dry_pipeline`. Source files are read
    with content so HDAG extraction is active and `crystal_resonance` dimensions appear.
  - `WorkspacePipelineSession::new(options, policy)` — wraps options and policy across
    repeated `run(root)` calls; `run_count()` tracks how many runs have completed.
  - When `crystal_store_path` is set in options, the session automatically accumulates
    crystal knowledge: every `run()` auto-loads prior crystals and auto-persists new ones
    with dedup. The CAD library grows richer with each call, zero boilerplate.
  - `WorkspaceError` re-exported from `kosmo-pipeline` for ergonomic `?` propagation.
  - 6 new pipeline tests (113 total); 2 new eval scenarios (145 total, 909 substrate tests).

* **Pipeline Step 5f — crystal auto-persistence closes the session-to-session loop**

  The pipeline now manages the CAD library feedback loop automatically. One call,
  zero boilerplate — just set `crystal_store_path` in `IntegrationRunOptions`.

  - `IntegrationRunOptions::crystal_store_path: Option<PathBuf>` — when set, the
    pipeline opens the store at entry (loading any persisted records into the
    effective `prior_crystals`) and appends newly-certified crystals after Step 5d-cert.
  - `IntegrationRunOptions::with_crystal_store_path(path)` — fluent builder.
  - `IntegrationRunReport::persisted_crystal_count: u32` — observational count of
    newly-written records; NOT included in `report_id` (host-state dependent).
  - Policy invariant preserved: `ReportOnly` and `DryRun` cannot write (store is
    read-only in those modes); only `OperatorApproved` (or `allow_host_write`) writes.
  - Dedup by `record_id` — re-running on the same workspace never grows the store
    with duplicate records.
  - 5 new pipeline tests (107 total); 2 new eval scenarios (143 total, 896 substrate tests).

* **`CrystalRecordStore` — durable JSONL-backed CAD library persistence**

  Crystal records now survive across integration runs; the CAD library can be
  pre-loaded into `IntegrationRunOptions::prior_crystals` from the previous session.

  - `StructuralCrystalRecord::verify_id()` — recomputes the `record_id` from fields
    for integrity checking; used by the store on open and in `verify_integrity`.
  - `CrystalRecordStore::open(path)` — replays the JSONL file, verifying every
    `record_id`; returns `Err(IntegrityViolation)` on any tampered record.
  - `CrystalRecordStore::append(record, policy)` — same host-write invariant as
    `JsonlCartographyStore`: `ReportOnly` and `DryRun` are denied; only
    `OperatorApproved` (or a profile with `allow_host_write`) can persist. Dedup
    by `record_id` — re-appending an already-stored record is a silent no-op.
  - `CrystalRecordStore::records()` → `&[StructuralCrystalRecord]` for direct use
    as `IntegrationRunOptions::prior_crystals` without an extra copy.
  - `CrystalRecordStore::verify_integrity()` → re-verifies every record after reload.
  - `CrystalStoreError` — simple error enum with manual `Display`/`Error` impl
    (no thiserror dependency added to `kosmo-store`).
  - `kosmo-hyphae` added as a dependency of `kosmo-store`.
  - 7 new store tests (14 total); 1 new eval scenario (141 total, 886 substrate tests).

* **Crystal-boosted SourceCube scoring — `crystal_resonance` dimension**

  Closes the CAD library feedback loop: prior certified crystal records now influence
  the energy ranking of current-run SourceCubes via structural proximity.

  - Pipeline Step 2b: when `prior_crystals` is non-empty and source content is available,
    the best structural resonance between the current void's HDAG (rho/omega signals) and
    every prior crystal record is computed and stored as `crystal_resonance` dimension in
    the `CubeDimensionProfile`.
  - Uses the same rho/omega proximity formula as `Resonite::from_records`; pure Q16
    arithmetic, no floats (CROSS-007).
  - The dimension contributes to `ρ (coherence)` in the tripolar energy assessment —
    voids that match a known certified pattern rank higher in the void-fill plan.
  - `crystal_resonance` only appears when both HDAG and prior_crystals are present
    (no false-zero baseline: runs without prior crystals are unchanged).
  - 2 new pipeline tests (102 total); 1 new eval scenario (140 total, 872 substrate tests).

* **Crystal structural fingerprint + Resonite pipeline wiring (Step 5e-resonite)**

  Closes the loop between code structure and the CAD library: certified crystal records
  now carry structural provenance, and cross-run pattern proximity is computed via Resonite.

  - `StructuralCrystalCandidate`: new fields `source_void_id: Option<Digest>`,
    `rho_coherence: Q16`, `omega_phase: Q16`; both participate in `candidate_id`
    content-addressing so HDAG-enriched candidates differ from file-presence-only ones.
  - `StructuralCrystalCandidate::from_decision_with_signals(decision, void_id, rho, omega)` —
    builds a candidate with code-structure signals. `from_decision` now delegates to it
    with defaults `(None, ONE, ONE)`.
  - `StructuralCrystalRecord`: new fields `source_void_id`, `rho_coherence`, `omega_phase`
    propagated from the candidate at certification time; all three participate in `record_id`.
  - `StructuralCrystalRecord::from_certificate(cert, candidate)` — updated signature
    (second argument carries the structural provenance).
  - `Resonite::from_records(a, b, policy_id)` — structural proximity score:
    `((ONE - |ρ_a - ρ_b|) + (ONE - |ω_a - ω_b|)) / 2`; symmetric, Q16, no floats (CROSS-007).
  - Pipeline Step 5d: candidates built with HDAG signals via `from_decision_with_signals`
    (intent's `target_void_id` + `hdag_by_void_id` lookup).
  - Pipeline Step 5e-resonite: pairwise `Resonite` between every current certified crystal
    and every prior crystal; `resonite_count` participates in `report_id`.
  - `IntegrationRunReport.resonite_map: Vec<Resonite>` — covered by `verify_policy_consistency`.
  - 6 new `crystal.rs` tests (197 total); 6 new pipeline tests (100 total); 3 new eval scenarios
    (`rx-crystal-*` ×2, `rx-pipeline-resonite-*` ×1) → 139 total, 855 substrate tests.

* **CodeHDAG pipeline integration — code-structure-aware void severity and SourceCube dimensions**

  Topology observation deepened from file-presence to code-structure. When workspace entries
  carry source content (via `scan_path_with_content`), the pipeline now extracts `CodeHDAG`
  per source file and wires structural signals into the hyphae + pipeline layers.

  - `WorkspaceEntry.content: Option<String>` (`#[serde(skip)]`) — source text for HDAG
    extraction; excluded from `index_id` content-addressing (digest already addresses bytes).
  - `WorkspaceIndex::scan_path_with_content(root, policy_id)` — scans `.rs` source/test
    files and populates `content` for HDAG extraction.
  - `HostCube.hdag_by_void_id: BTreeMap<Digest, CodeHDAG>` — HDAG keyed by void_id;
    `hdag_count` participates in `cube_id` so enriched cubes differ from file-only cubes.
  - `MissingTestFiber` severity scales with HDAG definition count:
    `HALF + HALF × min(N, 8) / 8` (more definitions → higher urgency for test coverage).
  - Pipeline Step 2b: accepted-decision `SourceCube` dimensions now include
    `rho_coherence` and `omega_phase` from the CodeHDAG when content is available.
  - `IntegrationRunReport.source_cubes: Vec<SourceCube>` — SourceCubes are now exposed
    in the report for downstream inspection and testing.
  - `CubeDimensionProfile::from_raw_map(BTreeMap<String, Q16>)` — new constructor for
    raw-key dimension maps (used by the HDAG enrichment path).
  - 4 new `host.rs` tests + 4 `cube.rs` tests; 3 new pipeline tests; 2 new eval scenarios
    (`rx-hyphae-hdag-extracted-from-source-content`, `rx-hyphae-hdag-severity-scales-with-definition-count`).

* **Crystal certification pipeline — `StructuralCrystalRecord` + cross-run CAD library accumulation**

  - `ConstraintProgram::from_candidate(candidate, replay_status)` — evaluates the standard
    5-constraint program from candidate fields alone (no `EvidenceBundle` object required).
  - `StructuralCrystalCandidate::certify(replay_status)` — single call produces
    `(AssimilationCertificate, StructuralCrystalRecord)` for every `Pending` candidate.
  - `CorpusEntityKind::CrystalRecord` — certified crystal records are first-class corpus
    entities; the corpus now accumulates proven patterns across runs.
  - Pipeline Step 5d-cert: `certified_crystals: Vec<StructuralCrystalRecord>` in
    `IntegrationRunReport`; `certified_crystal_count` in `ReportContent`.
  - `IntegrationRunOptions.prior_crystals: Vec<StructuralCrystalRecord>` — seed the corpus
    with certified records from previous runs, closing the CAD library accumulation loop.
  - 4 new `crystal.rs` tests (186 total); 4 new pipeline tests (91 total); 2 new eval
    scenarios `RX:Crystal`/`RX:Pipeline` (134 total, 815 substrate tests).

* **`AssimilationLedger` — sequenced, content-addressed audit log of all decisions per run**
  (INVARIANT-007 strengthened: `run_id` is now sensitive to decision outcomes, not just
  decision count).

  - `AssimilationLedger { ledger_id, run_id, events, policy_id }` added to
    `kosmo-hyphae/assimilation`. Built via two-pass construction: a placeholder pass
    derives `ledger_id` from the ordered event sequence, then the final `run_id` is
    sealed with `ledger_id` in its content hash.
  - `HyphaeRunResult.ledger: AssimilationLedger` — every passive run now carries its
    full decision log.
  - `RunContent.ledger_id` participates in `run_id` content-addressing.
  - `ReportContent.hyphae_ledger_id` propagates the ledger commitment into the pipeline
    `report_id`.
  - 4 new hyphae tests (182 total); 1 new `RX:Hyphae` eval scenario (132 total).

* **Motif feedback loop + `SuggestPattern` yield kind** — closes the cross-run feedback
  loop so motifs observed in one pipeline run propagate as structural proposals
  into the next run's frontier.

  - `yield_for_intent` now selects `StructuralYieldKind::MotifProposal` for
    `SuggestPattern` intents (previously always `DeficiencyFill`).
  - `SourceFrontierGraph::augmented_with_prior_motifs` appends `SuggestPattern`
    intents for motifs meeting a configurable `min_support` threshold.
  - `passive_run_augmented(index, policy, additional_intents)` — backward-compatible
    wrapper; `passive_run` delegates to it with an empty slice.
  - `IntegrationRunOptions.prior_motifs: Vec<MotifCandidate>` and
    `prior_motif_min_support: Q16` — pipeline uses them to inject intents at the
    top of each run.
  - `MotifCandidate` → `PseBridgeCandidate::StructuralObservation` in Step 6b.
  - 2 new hyphae tests (178 total); 2 new pipeline tests (87 total); 3 new eval
    scenarios (131 total, 803 substrate tests).

* **Pipeline Step 5a: `MotifCandidate` from void kind frequency** — closes the gap
  between `MotifCandidate` (fully implemented with `energy_assessment`) and the
  pipeline (which had no step to generate or expose them).

  - `enable_motif_candidates: bool` in `IntegrationRunOptions` (default false;
    included in `all_layers()`).
  - Step 5a counts `HostVoidKind` occurrences, produces one `MotifCandidate` per
    kind with `support_score = kind_count / total_voids` (Q16 ratio, no floats,
    CROSS-007). Evidence = `hyphae.run_id` (CROSS-006: always non-ZERO).
    Results are energy-ranked before inclusion in the report.
  - `motif_candidate_count` participates in `report_id` (INVARIANT-007).
  - `verify_policy_consistency()` and `summary()` updated.
  - 4 new pipeline tests (85 total); 2 new `RX:Pipeline` eval scenarios (128 total,
    789 substrate tests).

* **`ReduceDeficiency` intents in frontier + spec §2.2 yield compliance** — closes
  the gap between the `DeficiencyVector` (already computed in Step 1c) and the
  `SourceFrontierGraph` (previously void-map-only), and ensures every yield
  produced from a `ReduceDeficiency` intent satisfies the spec §2.2 reference
  invariant (a yield must reference a void OR a deficiency).

  - `SourceFrontierGraph::from_void_map` now derives the `DeficiencyVector`
    internally and appends one `ReduceDeficiency` intent per deficiency kind.
    An empty void map still produces an empty frontier.
  - `SourceFrontierGraph::from_void_map_and_deficiencies` exposed for callers
    that already hold a pre-computed vector.
  - `yield_for_intent` extracts `deficiency_kind_ref` from `ReduceDeficiency`
    intents and passes it into `StructuralYield::new`; all other intent kinds
    continue to produce `deficiency_kind_ref = None`.
  - 4 new hyphae tests (176 total); 3 new `RX:Hyphae` eval scenarios (126 total,
    780 substrate tests).

* **`yield_for_intent` taint/authority propagation** — removes the last
  hardcoded trust override in the passive HYPHAE run path, opening the clean
  intent → Accepted decision path end-to-end.

  - `yield_for_intent` now calls `intent.taint.clone()` and
    `intent.authority.clone()` instead of hardcoding `TaintLabel::Synthetic` /
    `AuthorityLabel::Agent`. The `from_void_map` default remains
    `Unverified`/`Agent`, so all existing passive-run outcomes are unchanged.
  - A `TaintLabel::Clean` + `AuthorityLabel::Foundry` intent now naturally
    produces an `Accepted` decision under operator-approved policy — no special
    casing needed anywhere in the gate stack.
  - 2 new hyphae tests (172 total); 1 new `RX:Hyphae` eval scenario (124 total,
    777 substrate tests).

* **Decision taint propagation to BlueprintUnit** — closes the data flow gap
  between `StructuralYield.taint` and `BlueprintUnit`; every trust signal now
  travels through the full pipeline chain.

  - `AssimilationDecision.taint: TaintLabel` added, propagated from the source
    `StructuralYield` in `from_trace()`. The `taint` field participates in
    `decision_id` content-addressing so different taints produce distinct IDs
    (INVARIANT-007).
  - Pipeline Step 5e uses `decision.taint.clone()` instead of the hardcoded
    `TaintLabel::Synthetic`. All current passive-scan decisions remain Synthetic
    (same runtime behaviour), but a future `OperatorAssisted` run with Clean
    yields will automatically produce fully-compatible `Accepted` blueprint units.
  - 1 new hyphae test (170 total); 1 new pipeline test (81 total); 1 new
    `RX:Pipeline` eval scenario (123 total, 775 substrate tests).

* **SystemCube diagnostics surfaced in pipeline** — compatibility and contradiction
  energy are now first-class citizens of `IntegrationRunReport`, with accessors,
  gate contribution, and summary inclusion.

  - `IntegrationRunReport::systemcube_compatibility_score() -> Option<Q16>` and
    `systemcube_contradiction_energy() -> Option<Q16>` — direct accessors that avoid
    drilling through `Option<KcubeExportReport>`.
  - SystemCube gate contribution upgraded: `Warn` when `compatibility.gaps` is
    non-empty (structural advisory signal, not energy — respects CROSS-010); `Pass`
    when all accepted units are clean.
  - `summary()` now includes `compat=<score>` and `contradiction_energy=<total>` in
    the systemcube section.
  - 3 new pipeline tests (80 total); 2 new `RX:Pipeline` eval scenarios (122 total,
    772 substrate tests).

* **CompatibilityProfileReport real gap detection** — replaces the `perfect()` stub
  in `SystemCube::export_dry_run` with unit-aware gap analysis; every
  `KcubeExportReport` now carries real compatibility diagnostics.

  - `CompatibilityProfileReport::from_units(manifest_id, host_snapshot_id, policy, units)`:
    `AcceptedWithTaint` units produce a `TaintedUnit` gap (severity `Q16::HALF`);
    `source_ref == Digest::ZERO` produces a `MissingSourceRef` gap (severity `Q16::ONE`).
    `compatibility_score = Q16::ONE − avg_gap_severity`, clamped to `[0, ONE]`.
    Gaps sorted by `unit_id`; opaque-rejected units excluded (INVARIANT-007).
  - 5 new `kosmo-systemcube` tests (54 total); 2 new `RX:Compatibility` eval
    scenarios (120 total, 769 substrate tests).

* **ContradictionEnergyReport real pairwise detection** — replaces the `zero_energy`
  stub in `SystemCube::export_dry_run` with a deterministic, unit_id-ordered pairwise
  scan of accepted units; surfaces real `RoleConflict` and `Duplicate` signals.

  - `ContradictionEnergyReport::from_units(manifest_id, policy, units)`:
    same `source_ref` + same `kind` → `Duplicate` (weight `Q16::HALF`);
    same `source_ref` + different `kind` → `RoleConflict` (weight `Q16::ONE`).
    Units iterated in `unit_id` order for determinism (INVARIANT-007).
  - `SystemCube::export_dry_run` now calls `from_units` — every `KcubeExportReport`
    carries real contradiction diagnostics rather than a constant zero.
  - 5 new `kosmo-systemcube` tests (49 total); 2 new `RX:ContradictionEnergy` eval
    scenarios (118 total, 764 substrate tests).

* **BlueprintUnit energy assessment — Step 5e** — completes energy integration for
  `kosmo-systemcube`; every artifact type in the production chain now has
  `energy_assessment`, enabling deterministic priority ordering across all layers.

  - `BlueprintUnit::energy_assessment(gate)`: ψ = `Q16::ONE` for accepted units
    (Accepted / AcceptedWithTaint); `Q16::ZERO` for opaque-rejected. The taint factor
    separately reduces energy for tainted units (Synthetic → ½, Quarantined → 0).
    `evidence_bundle_id = self.unit_id` (self-referential, CROSS-006).
  - Pipeline Step 5e: blueprint units are energy-ranked before `SystemCube::new`,
    surfacing the most trusted units at the top of every manifest.
  - 3 new `kosmo-systemcube` tests (44 total); 2 new `RX:BlueprintEnergy` eval
    scenarios (116 total, 759 substrate tests).

* **PseBridgeCandidate pipeline integration — Step 6b** — surfaces all actionable
  pipeline observations as PSE-ready candidates, completing the observation→submission
  funnel without gating any decisions on PSE acceptance (CROSS-010).

  - `enable_pse_candidates: bool` in `IntegrationRunOptions` (default false). When
    enabled, norm candidates become `StructuralObservation` candidates (ψ=`fitness_score`,
    evidence=`evidence_bundle_id`) and ambiguity profiles + void hypotheses become
    `TopologyObservation` candidates (ψ=`confidence_score`). All are sorted by confidence
    descending, with `id` as deterministic tie-break.
  - `IntegrationRunReport.pse_candidates: Vec<PseBridgeCandidate>`; count participates
    in `report_id`. `verify_policy_consistency()` covers all candidate `policy_id`
    fields. `summary()` reports `pse_candidates: N`.
  - 3 new pipeline tests (77 total); 2 new `RX:Pipeline` eval scenarios (114 total,
    756 substrate tests).

* **DeficiencyVector pipeline integration — Step 1c** — always-on diagnostic summary
  of structural deficiencies derived from the host void map (test coverage gaps,
  documentation gaps). Never requires an option flag.

  - `IntegrationRunReport.deficiency_vector: DeficiencyVector` always present.
    `deficiency_vector_id` participates in `report_id`. `verify_policy_consistency()`
    covers `deficiency_vector.policy_id`. `summary()` reports `deficiency: N entries`.
  - 3 new pipeline tests (74 total); 2 new `RX:Pipeline` eval scenarios (112 total,
    753 substrate tests).

* **StructuralCrystalCandidate pipeline integration — Step 5d** — surfaces the
  explicit certification work queue: one candidate per accepted decision, all
  starting with `support_score = Q16::ZERO` (Pending certification status).

  - `enable_crystal_candidates: bool` in `IntegrationRunOptions` (default false).
    `IntegrationRunReport.crystal_candidates`; count participates in `report_id`.
    `verify_policy_consistency()` covers candidate `policy_id` fields. `summary()`
    reports `crystal_candidates: N`.
  - 3 new pipeline tests (71 total); 2 new `RX:Pipeline` eval scenarios (110 total,
    750 substrate tests).

* **TopologyAmbiguityProfile + ComplementVoidHypothesis pipeline integration — Step 3f** —
  surfaces previously discarded metatron M2 diagnostic details as energy-ranked
  top-level collections in the report.

  - Pipeline Step 3f: flatten `.ambiguities` and `.void_hypotheses` from all
    `metatron_diagnostics`, energy-rank each by `confidence_score` (most-confident first).
    Both collections are empty when `enable_metatron` is false.
  - `IntegrationRunReport.ambiguity_profiles` + `.complement_void_hypotheses`; counts
    participate in `report_id`. `verify_policy_consistency()` covers both. `summary()`
    reports `ambiguities: N | void_hyp: M`.
  - 3 new pipeline tests (68 total); 2 new `RX:Pipeline` eval scenarios (108 total,
    747 substrate tests).

* **NormFitnessTrace pipeline integration — Step 5c** — closes the full
  "Wissen zurück ins Substrat" loop: PSE promotion outcomes feed back into the
  substrate as fitness observations, which can re-rank norm gene candidates.

  - `IntegrationRunOptions.prior_feedback: Vec<PromotionFeedback>` (default empty).
    On each run, feedback records with matching `norm_candidate_id` are folded into
    `NormFitnessTrace::observe_from_feedback`. Only traces with ≥1 observation
    are included in the report.
  - `IntegrationRunReport.norm_fitness_traces: Vec<NormFitnessTrace>`;
    `norm_fitness_trace_count` participates in `report_id`. `verify_policy_consistency()`
    covers all trace `policy_id` fields. `summary()` reports `norm_candidates: N (traces: M)`.
  - 3 new pipeline tests (65 total); 2 new `RX:Pipeline` eval scenarios (106 total,
    744 substrate tests).

* **SurgeryWorkbenchTask pipeline integration — Step 3e** — every energy-ranked
  `TopologicalSurgeryOption` now converts into a workbench-compatible task immediately
  after Step 3b, closing the surgery → workbench gap.

  - Pipeline Step 3e: `surgery_options.iter().map(SurgeryWorkbenchTask::from_option).collect()`;
    tasks are in the same energy-ranked order as the source options. Empty when
    `surgery_options` is empty (i.e., `enable_surgery` or `enable_metatron` is false).
  - `IntegrationRunReport.surgery_workbench_tasks: Vec<SurgeryWorkbenchTask>`;
    `surgery_workbench_task_count` participates in `report_id`. `verify_policy_consistency()`
    covers all task `policy_id` fields. `summary()` reports `surgery: N (tasks: M)`.
  - 3 new pipeline tests (62 total); 2 new `RX:Pipeline` eval scenarios (104 total,
    741 substrate tests).

* **MicroTopologyIndex pipeline integration — Step 3d** — closes the last metatron
  integration gap; `MicroTopologyIndex` existed in the spec but was never assembled.

  - Pipeline Step 3d: after the metatron loop, all `(MetatronMicrograph,
    MetatronRegionFingerprint, MicroTopologyDiagnostic)` triples are folded into a
    `MicroTopologyIndex` via `MicroTopologyIndex::add`. Produces an empty-state index
    when `enable_metatron` is false.
  - `IntegrationRunReport.metatron_index: MicroTopologyIndex`; `index_id` participates
    in `report_id` (content-addressed). `verify_policy_consistency()` covers
    `metatron_index.policy_id`. `summary()` reports `index_id` prefix.
  - 4 new pipeline tests (59 total); 2 new `RX:Pipeline` eval scenarios (102 total,
    738 substrate tests).

* **TopologyAmbiguityProfile + ComplementVoidHypothesis energy_assessment** —
  completes energy integration for all Q16-score types in kosmo-hyphae.
  Every substrate type that carries a Q16 score now has `energy_assessment`.

  - `TopologyAmbiguityProfile::energy_assessment(gate)`: ψ = `confidence_score`;
    `evidence_bundle_id = micrograph_id` (the source micrograph, CROSS-006).
  - `ComplementVoidHypothesis::energy_assessment(gate)`: ψ = `confidence_score`;
    `evidence_bundle_id` = first non-ZERO entry in `evidence_ids`, falling back to
    `micrograph_id` (CROSS-006: always non-ZERO). Both forms allow `rank_by_energy`
    over a diagnostic's sub-items.
  - 4 new `metatron.rs` tests (169 hyphae tests total, 734 substrate tests).

* **SemanticLossRecord + MicrographLiftReport energy integration + pipeline Step 3c** —
  closes the last energy_assessment gap in kosmo-hyphae; lift quality signal now
  surfaces in every metatron-enabled pipeline run.

  - `SemanticLossRecord::energy_assessment(gate)`: ψ = `loss_ratio` (high loss =
    high energy = most urgent to review); `evidence_bundle_id = region_id` (CROSS-006).
  - `MicrographLiftReport::energy_assessment(gate)`: ψ = `loss_ratio`;
    `evidence_bundle_id = micrograph_id` (CROSS-006).
  - Pipeline Step 3c: the M1 lift report (`MicrographLiftReport`) is no longer
    discarded. When `enable_metatron` is true, one report per void is collected,
    energy-ranked by `loss_ratio` (most lossy lifts first), and stored in
    `IntegrationRunReport.lift_reports`. `ReportContent.lift_report_count`
    participates in `report_id`.
  - `summary()` now reports `metatron: N (lift_reports: M)`.
  - 4 new `metatron.rs` tests + 3 new pipeline tests + 2 new `RX:Pipeline`
    eval scenarios (100 total, 730 substrate tests).

* **Resonite, CubeMandorla, CompositeSupportCube energy_assessment** —
  completes energy integration for all swarm and crystal structural types.

  - `Resonite::energy_assessment(gate)`: ψ = `resonance_score`; symmetric
    (r(a,b) produces the same assessment as r(b,a)); `evidence_bundle_id =
    resonite_id` (self-referential, CROSS-006).
  - `CubeMandorla::energy_assessment(gate)`: ψ = `overlap_score`;
    `evidence_bundle_id = mandorla_id` (self-referential, CROSS-006).
  - `CompositeSupportCube::energy_assessment(gate)`: ψ = `aggregate_support`;
    `evidence_bundle_id = composite_id` (self-referential, CROSS-006).
  - No new fields on any type — the type's own content address satisfies CROSS-006.
  - 3 new `crystal.rs` tests + 4 new `swarm.rs` tests.

* **NormGeneCandidate pipeline integration — Step 5b** — closes the last
  hyphae-to-pipeline integration gap; norm gene candidates are now generated
  and ranked as part of every full pipeline run.

  - `IntegrationRunOptions.enable_norm_candidates: bool` (default false).
  - Pipeline Step 5b: for each accepted assimilation decision, a
    `NormGeneCandidate` is created with `fitness_score = Q16::ONE` (initial
    fitness; `NormFitnessTrace` evolves this via feedback in later phases).
    `evidence_bundle_id = decision.evidence_bundle_id` (CROSS-006: non-ZERO
    causal ref — traces back to the original evidence that justified acceptance).
    All candidates are energy-ranked via `rank_by_energy` before being stored.
  - `IntegrationRunReport.norm_candidates: Vec<NormGeneCandidate>`; count
    participates in `report_id` (content-addressed).
  - `verify_policy_consistency()` extended to cover `norm_candidates[i].policy_id`.
  - `summary()` reports `norm_candidates: N`.
  - 3 new pipeline unit tests (52 total); 2 new `RX:Pipeline` eval scenarios
    (98 total, 712 substrate tests).

* **Void priority ranking — pipeline Step 1b** — every `IntegrationRunReport`
  now ships a severity-ordered void repair queue at zero extra I/O cost.

  - `HostVoid::energy_assessment(gate, policy_id)`: ψ = `severity`; taint/phase
    fixed at `Q16::ONE` (void detection has no coherence dimension at this level);
    `evidence_bundle_id = void_id` — the void's own content address satisfies
    CROSS-006 (non-ZERO evidence ref).
  - `TopologicalVoidMap::priority_ranking(gate) -> Vec<Digest>`: ranks all voids
    by energy D via `rank_by_energy`; ties broken deterministically by `void_id`.
  - Pipeline Step 1b: `void_priority_ranking` is always computed after the HYPHAE
    passive run and stored in `IntegrationRunReport`. `ReportContent` carries
    `void_priority_count` so the void count participates in `report_id`.
  - `summary()` now reports `voids: N (priority ranked)`.
  - 5 new `void_map.rs` unit tests; 2 new `RX:Pipeline` eval scenarios (96 total,
    709 substrate tests).

* **Surgery energy assessment + pipeline Step 3b** — closes the surgical
  intervention planning chain from Metatron diagnostics.

  - `TopologicalSurgeryOption::energy_assessment(gate)`: ψ = `confidence_score`,
    `evidence_bundle_id = diagnostic_id` (CROSS-006 non-ZERO causal ref).
  - Pipeline Step 3b derives surgery options from all Metatron diagnostics,
    energy-ranks them via `rank_by_energy`, and stores the ranked slice in
    `IntegrationRunReport.surgery_options`. Gated by `enable_surgery: bool`
    (default false); requires `enable_metatron` to produce any output.
  - `verify_policy_consistency()` now covers `surgery_options[i].policy_id`.
  - 4 surgery unit tests, 3 new `RX:Pipeline` eval scenarios (94 total,
    704 substrate tests).

* **`from_host_and_composite` removed; `MorphogenicCorpusUpdate` as Step 4d** —

  - `HostTargetDelta::from_host_and_composite` deleted (only callers were its
    own tests; used raw `max_by_key` violating the energy invariant). Its two
    tests migrated to `from_source_cubes` with real `SourceCube` objects.
  - Pipeline Step 4d: `MorphogenicCorpusUpdate::skeleton(cartography_update_id,
    collapse_plan_id, policy_id)` — planning skeleton of the post-collapse corpus.
    Participates in `report_id`, `verify_policy_consistency()`, and `summary()`.
  - 2 new `RX:Pipeline` eval scenarios.

* **JsonlCartographyStore persistence wired into pipeline** — closes the
  last persistence gap; `CorpusCartographyUpdate` can now be durably stored.

  - `CartographyEntryKind::CartographyUpdate` added to `kosmo-core`.
  - `kosmo-pipeline` gains `kosmo-store` dep and a new `persistence` module.
  - `persist_cartography_update(update, path, scope, policy)`: fail-closed on
    `allow_host_write == false`; CROSS-006 satisfied (evidence = `update_id`);
    commit labels `after_cartography_id` + `added_entity_count`.
  - 3 unit tests, 2 new `RX:Pipeline` eval scenarios (89 total).

* **StructuralCrystalCandidate gains `energy_assessment`** — last hyphae
  candidate type to receive energy integration.

  - ψ = `support_score` (ZERO at creation; gate factor collapses to zero if
    the gate rejects). Taint = `Q16::ONE`: quarantined yields are rejected
    at the gate cascade before candidacy (`IsNotQuarantined` constraint).
  - 3 new `crystal.rs` unit tests, 2 new `RX:EnergyRanking` eval scenarios.

* **Phase 4c: HostTargetCollapsePlan wired into run_dry_pipeline** —
  planning-only collapse plan now ships with every `IntegrationRunReport`.

  - `run_dry_pipeline` Step 4c: `HostTargetCollapsePlan::from_delta(&void_fill_delta, policy.id)`.
    Status is always `PlanningOnly` — no execution authority in Phase 5.
  - `IntegrationRunReport` gains `collapse_plan: HostTargetCollapsePlan`.
  - `ReportContent` gains `collapse_plan_id`; the collapse plan participates
    in the report's content address — any plan change alters `report_id`.
  - `verify_policy_consistency()` now asserts `collapse_plan.policy_id == policy.id`.
  - `summary()` reports `collapse: N steps (PlanningOnly)`.
  - 3 new `RX:Pipeline` eval scenarios; total 85 scenarios, 682 substrate tests.

* **MotifCandidate policy alignment + SeamGraph seam coherence wired into ranking** —
  two architectural gaps closed in one weld.

  - `MotifCandidate` gains `policy_id: Digest` (aligns with all other substrate types);
    content addressing (`motif_id`) now includes `policy_id`. `new()` signature updated;
    `energy_assessment(gate)` added: ψ=`support_score`, taint factor from `self.taint`.
    5 tests (3 updated, 2 new).
  - `SourceCube::energy_assessment` gains a `seam_coherence: Q16` parameter; the
    `EnergyFactors::seam` field is no longer hardcoded to `Q16::ONE`.
  - `HostTargetDelta::from_source_cubes` gains `seam_map: &BTreeMap<Digest, Q16>`
    (void_id → seam coherence). Each void's seam coherence multiplies its candidates'
    energy; missing entries default to `Q16::ONE`. A cube with `support=1` but
    `seam=0` collapses to zero energy.
  - Pipeline Step 4b (CubeSwarm) moved after LPCM so LPCM seam data feeds the
    void-fill ranking. `seam_map` built from `lpcm_reports`: coherence = fraction
    of compatible seam edges per void (empty graph → `Q16::ONE`).
  - **`tools/kosmo-eval` extended to 82 scenarios** (was 80): 2 new `RX:EnergyRanking`
    scenarios (`rx-energy-motif-assessment-content-addressed`,
    `rx-energy-seam-penalty-reduces-ranking`).

* **Phase 4 CubeSwarm + HostTargetDelta wired into the pipeline** — closes the
  integration gap where `CubeSwarm` and `HostTargetDelta` existed but were never
  called from `run_dry_pipeline`.

  Step 2b in `run_dry_pipeline`: accepted assimilation decisions are converted
  to `SourceCube`s (ψ=1, taint from intent), assembled into a `CubeSwarm`,
  and ranked via `HostTargetDelta::from_source_cubes` (energy-correct path).
  `IntegrationRunReport` now carries `swarm_composite: CompositeSupportCube`
  and `void_fill_delta: HostTargetDelta` — both content-addressed and
  policy-tagged. `verify_policy_consistency()` covers the new fields.

  - **`tools/kosmo-eval` extended to 80 scenarios** (was 76): 4 new
    `RX:Pipeline` scenarios (swarm+delta in report, empty-workspace delta is
    Clean, policy consistency includes swarm, deterministic across runs).

* **Energy kernel adoption in selection paths** — closes the gap where
  `SourceCube` and `NormGeneCandidate` ranked by raw Q16 scores instead of
  the unified tripolar energy kernel (as called out in the `kosmo-core::energy`
  module-level doc).

  - `SourceCube::energy_assessment(gate, license, foundry)` — ψ=`support_score`,
    ρ=average dimension-profile coverage (coherence), ω=1; taint factor from
    `self.taint`. Returns a content-addressed [`EnergyAssessment`].
  - `NormGeneCandidate::energy_assessment(gate)` — ψ=`fitness_score`, ρ=ω=1;
    gate-collapsed fail-closed (CROSS-010 analogue). Returns an `EnergyAssessment`.
  - `HostTargetDelta::from_source_cubes` — the energy-correct companion to
    `from_host_and_composite`. Groups `SourceCube`s by `target_void_id`, calls
    `energy_assessment` on each, then uses `rank_by_energy` to pick the top
    candidate per void. A quarantined cube with `support_score=1.0` loses to a
    clean cube with `support_score=0.5` — the kernel overrides raw Q16.
  - **`tools/kosmo-eval` extended to 76 scenarios** (was 72): 4 new
    `RX:EnergyRanking` scenarios (quarantine zeroes energy, ranking picks best,
    taint beats higher raw score, norm candidate content-addressed assessment).

* **PSE feedback loop — "Wissen zurück ins Substrat"** — closes the final
  vision link by routing `PromotionOutcome` back into substrate fitness tracking.

  - `FeedbackOutcome` (Accepted/Rejected/Deferred/Skipped) — substrate-side
    mirror of PSE's `PromotionOutcome`, in `kosmo-core` to avoid circular
    dependency. `fitness_signal(energy)` maps: Accepted→energy, Deferred→¼,
    Rejected/Skipped→0 (CROSS-010 analogue).
  - `PromotionFeedback` — content-addressed record in `kosmo-core` binding a
    `PromotionRequestRecord` outcome, candidate confidence, derived
    `fitness_signal`, policy, and `evidence_bundle_id` (CROSS-006). 14 unit tests.
  - `CartographyEntryKind::PromotionFeedback` — new variant allowing feedback
    records to be stored in `CorpusCartographyStore`.
  - `build_promotion_feedback` in `kosmo-pse-bridge` — converts
    `PromotionOutcome` → `FeedbackOutcome` and constructs a `PromotionFeedback`
    from a `PromotionRequestRecord` + `PseBridgeCandidate`.
  - `NormFitnessTrace::observe_from_feedback` in `kosmo-hyphae` — consumes a
    `PromotionFeedback` to append a fitness observation; uses `feedback.id` as
    the evidence reference, closing the loop end-to-end. 3 new tests.
  - **`tools/kosmo-eval` extended to 72 scenarios** (was 68): 4 new
    `RX:FeedbackLoop` scenarios (accepted maps to full energy, rejected gives
    zero fitness, stored in cartography as `CartographyStoreCommit`, full
    chain `build_promotion_feedback` + `observe_from_feedback`).

* **`SystemCube::export_to_kcube` weld** — closes the "Blueprint raus" vision
  link by connecting the dry-run `KcubeExportReport` to the real
  `KcubeExecutor`. The method runs `export_dry_run` first; if
  `op_policy.allow_systemcube_materialization = false` it returns
  `SkippedByReportOnly` without touching the filesystem; otherwise it
  serializes the manifest, export assessment, and all accepted blueprint units
  into a `.kcube` archive via `KcubeExecutor::write`.

  `to_kcube_artifacts` produces three artifact kinds:
  `CartographyManifest` (`manifest.json`), `ValidationClosureReport`
  (`export_report.json`), and `StructuralCrystal` (one file per accepted
  `BlueprintUnit` keyed by `unit_id` hex). 5 new unit tests in
  `kosmo-systemcube`.

* **`PolicyProfile::operator_approved_with_systemcube`** — new constructor in
  `kosmo-core` that sets `allow_systemcube_materialization = true` alongside
  the existing operator-approved gates (host write allowed, no network, Foundry
  + ParseBack still required).

* **`tools/kosmo-eval` extended to 68 scenarios** (was 65): 3 new
  `RX:SystemCubeKcube` scenarios (blocked by default policy, write creates
  archive, archive parses back with correct entry count). `kosmo-eval` now
  depends on `kosmo-systemcube`.

* **Unified tripolar energy kernel** (`kosmo-core::energy`) — the single,
  float-free, content-addressed selection core `D = ψ · ρ · ω`.

  - `TripolarEnergy { psi, rho, omega }` — the three poles (meaning / coherence
    / phase), each clamped to `[0, 1]`; `d()` computes `ψ·ρ·ω` in `Q16` integer
    arithmetic (CROSS-007: no floats).
  - `EnergyFactors` — six `[0, 1]` modulators (`gate`, `taint`, `license`,
    `foundry`, `seam`, `contradiction`) derived fail-closed from the substrate's
    own `GateResult` / `TaintLabel` / `LicenseStatus` / `FoundrySurvival`. Each
    factor can only *reduce* energy; a single zero collapses it.
  - `EnergyKernel` — tripolar core × factor product → final selection energy.
  - `EnergyAssessment` — content-addressed, evidence-bound, `verify_id()`.
  - `rank_by_energy` — deterministic descending ranking, `subject_id` tie-break,
    never silently drops a zero-energy candidate.
  - **Non-bypass invariant (CROSS-010):** energy ranks but never gates. A
    `Reject` zeroes the `gate` factor, so a rejected candidate can never
    out-rank a passing one and a high `D` can never flip a `Reject` into an
    `Accept`. 20 unit tests.

* **Real code topology extraction** (`kosmo-hyphae::code_hdag`) — replaced the
  one-node `CodeHDAG` skeleton with `extract_from_rust_source`, a dependency-free
  lexical extractor that emits real module/import/fn/type/test nodes and
  `Imports`/`Contains`/`Tests`/`Implements` edges. Content-addressed to the
  source line; deterministic (INVARIANT-007). Bridges into the energy kernel via
  `rho_coherence()`, `omega_phase()`, `energy_kernel()`, and `energy_assessment()`
  (ψ is a caller input; ρ and ω are derived from graph structure). New `Contains`
  `HDAGEdgeKind`; `CodeHDAG` content-address now covers full edge wiring. 12 new
  unit tests.

* **Real `.kcube` archive executor** (`kosmo-kcube`) — the host-capability
  bridge that turns `KcubeExportPolicy`-gated artifact lists into real
  `.kcube` files on disk and reads them back.

  Archive format: deterministic framed binary (`KCUBEPM\n` magic, LE-encoded
  sections, artifact bytes sorted by path for bit-exact reproducibility).
  `package_digest = SHA-256(artifact_section)` — the manifest JSON is appended
  as a trailer so the digest covers only the artifacts.

  Policy enforcement: `allow_write=false` → `DeniedByPolicy` (no disk touch);
  artifact kind allowlist checked before any write; `allow_overwrite=false`
  blocks silent replacement; `require_roundtrip_verification=true` (the
  default) re-reads the file and compares artifact-section SHA-256 after write.

  `KcubeExecutor::read` — parses a `.kcube` file back to `KcubePackage` for
  import/verify workflows (`parse_kcube_file` is also public).

  CROSS-006: `evidence_bundle_id ≠ ZERO` is propagated into every report
  variant including `DeniedByPolicy`. CROSS-007: no floats (`written_bytes`,
  `elapsed_ms` are `u64`). INVARIANT-007: `KcubeWriteReport.verify_id()` and
  `KcubePackage.verify_id()` both pass after roundtrip read.

  25 unit tests. No new external dependencies (reuses `kosmo-core`,
  `serde`/`serde_json`).

* **`tools/kosmo-eval` extended to 65 scenarios** (was 60): 5 new `RX:Kcube`
  scenarios (write denied when `allow_write=false`, write+roundtrip pass, content-
  addressed package, overwrite guard, `read` parses manifest). `kosmo-eval` now
  depends on `kosmo-kcube`.

* **`tools/kosmo-eval` extended to 60 scenarios** (was 52): 5 new `RX:Energy`
  scenarios (tripolar exactness, gate non-bypass, quarantine/proprietary/foundry
  zeroing, content-addressing, deterministic ranking) and 3 new `RX:Topology`
  scenarios (real-graph extraction, deterministic extraction, the full
  topology→energy chain). `kosmo-eval` now depends on `kosmo-hyphae`.

* Architecture decisions **AD-015** (tripolar energy kernel + non-bypass
  invariant) and **AD-016** (lexical topology extraction + topology→energy
  bridge).

* **KOSMO-OPS-01 Operationalization Staircase** — R0–RX full implementation
  of the empirical validation benchmark for KOSMO-OPS-01 invariants R1–R9.

  Four new host-capability crates:

  - `crates/kosmo-foundry` — Real Foundry executor. Runs allowlisted `cargo`
    subcommands (check / test / clippy) via `std::process::Command`. Policy
    contract: `ReportOnly` → `SkippedByReportOnly` (zero spawn); command
    denied before spawn; `FoundryExecutionReport` content-addressed.
    8 unit tests.

  - `crates/kosmo-store` — Persistent JSONL CorpusCartography store.
    Implements `CorpusCartographyStore` trait from `kosmo-core`. Append-only
    durable backend with `verify_integrity()` (digest mismatch + sequence gap
    detection). Emergent invariant: `DryRun` (`allow_host_write=false`)
    cannot persist — only `OperatorApproved`. 9 unit tests.

  - `crates/kosmo-parseback` — Real ParseBack executor. Snapshots workspace
    crate topology via `cargo metadata --format-version 1 --no-deps`.
    `TopologySnapshot` and `CrateFingerprint` are content-addressed (SHA-256);
    INVARIANT-007: identical inputs → identical IDs. `diff_snapshots()`
    classifies: `NodeRemoved`/`EdgeRemoved` → Critical, `NodeAdded`/
    `EdgeAdded` → Warning, `NodeModified` → Info (fail-closed worst-wins).
    17 unit tests including real workspace integration.

  - `crates/kosmo-operator` — Operator orchestrator. Wires the R1→R2→R3
    full pipeline: ParseBack pre-snapshot → Foundry execution → ParseBack
    post-snapshot + diff → `ValidationClosureReport` synthesis → optional
    JSONL store persistence (only `OperatorApproved` + `allow_host_write`).
    `OperationPlan` and `OperationReport` are content-addressed.
    8 unit tests including real targeted `cargo check` + temp store round-trip.

  `tools/kosmo-eval` benchmark extended to 52 scenarios:
  - 42 existing R1–R9 data-model scenarios (unchanged)
  - 6 new RX:ParseBackExec scenarios (report-only skip, baseline mismatch,
    severity classification, deterministic snapshot, real workspace pass)
  - 4 new RX:Operator scenarios (report-only inconclusive, content-addressed
    report, full-cycle dry-run, approved-persists-closure)

  All 52 scenarios pass `EXIT 0`. Workspace: 614 tests, 0 failures.

  Architecture decisions added: AD-010 (host-capability crate isolation),
  AD-011 (`allow_host_write` extended to disk persistence), AD-012
  (`cargo metadata` strategy for ParseBack), AD-013 (fail-closed severity
  mapping), AD-014 (OperationReport content-addressed over sub-IDs).

### Fixed

* **`-D warnings` build of `kosmo-core` and `kosmo-operator`** — removed two
  pre-existing unused-import warnings (`EvidenceBundle`/`EvidenceRef` at module
  scope in `cartography.rs`; `FoundryCommandPolicy`/`FoundryEnvironmentPolicy`
  at module scope in `kosmo-operator`, now scoped to the tests that use them)
  that broke `RUSTFLAGS="-D warnings"` builds of those crates.

* **Two stale test assertions in `pse-eval-matrix`** (`agent_exoskeleton.rs`):
  `ablation_aggregate_base_metrics_present` expected the robustness label
  `"requires_real_agent_validation"` but the production code now emits
  `"live_agent_validation_completed"` (set when live proof was completed);
  `trace_feature_design_report_is_present_and_consistent` asserted
  `!migration_plan.productive_agent_validated` but the field is intentionally
  `true` since the live Cerebras proof (commit `48fed88`). Both assertions now
  reflect the actual system state. Workspace test suite is **1315 / 1315**.

* **CHANGELOG `RulePredicate` variant names** corrected: entries `MaxUncertainty`,
  `RequireAttribution`, and `NotHallucinationAttractor` never existed in the
  implementation. Replaced with the actual variant set: `MinStability`,
  `MinKuramoto`, `MaxFreeEnergy`, `MinEvidenceEntries`, `CoherenceGate`,
  `PathInvariant`, `RequiresAgentAttribution`.

* **Clippy warnings** eliminated across `pse-adapter-il`, `mef-core`, `pse-server`,
  and `pse-llm-demo`: replaced indexed loop with iterator in `hdag.rs`,
  `sort_by` → `sort_by_key` in cluster sort, `field_reassign_with_default` in
  `entry_to_proxy_crystal()`, `filter_map` → `map` in the server IL retrieve
  handler (always-`Some` branches collapsed), doc overindentation in
  `il_bridge.rs`.

### Added

* **Pfad B — Constitutional Interceptor** (`crates/pse-constitutional-interceptor`,
  11 unit tests) — action-level governance gate layered over the PSE server.

  New crate `pse-constitutional-interceptor`:
  - `ActionContext { verb, target, description, metadata }` — describes any proposed
    system action (request, write, delete, execute, …).
  - `Decision::Allow | Block { rule_id, reason } | Warn { rule_id, reason }` — the
    three constitutional outcomes.
  - `ConstitutionalEvaluator` — evaluates an `ActionContext` against loaded `RuleAtom`s.
    Two-pass evaluation: Pass 1 scans all triggered rules for Blocking (blocking always
    wins, regardless of rule list order); Pass 2 scans for Required (strict mode →
    Block; non-strict → Warn). Trigger matching is case-insensitive substring over
    `"verb target description"`.
  - `EvaluationReport` — per-rule trigger / decision audit trail returned with every
    evaluation.
  - **Strict mode** auto-activates when the nxalien `EpistemicSignal` is Drifting or
    Diverging — Required rules escalate from Warn to Block automatically.

  Tower middleware (`tools/pse-server/src/constitutional.rs`):
  - `ConstitutionalLayer` / `ConstitutionalService<S>` — wraps the entire app router.
    Requests without `x-nxalien-*` headers pass through transparently.
    Block decision → 403 JSON response; Warn decision → `x-nxalien-warn` header added
    to the upstream response without interrupting the handler.
  - `POST /constitutional/check` — evaluate an `ActionContext` against the server's
    loaded rules. Accepts `{ action, strict_mode? }`, returns
    `{ report, active_rule_count, server_strict_mode }`. Returns 403 on Block, 200
    on Allow or Warn (with warn details inline). Strict mode defaults to the server's
    current epistemic signal state (Drifting/Diverging → strict).
  - The `nxalien_bundle` handler refreshes the evaluator's loaded rule set and updates
    the strict mode flag after every evolution cycle.

* **Pfad C — Multi-Repo Attractor** (`tools/nxalien-cli`) — `nxalien compile`
  extended with `--remote <url>` and `--remote-only` flags.

  - `--remote <url>`: after building the local bundle, POST it to a running PSE server
    at `<url>/nxalien/bundle` via `reqwest::blocking`. Prints the remote server's
    `EpistemicSignal`, IL health (`MemoryHealthReport`), and QTIC statistics from the
    JSON response.
  - `--remote-only`: after the remote POST, skip IL commit and `GraphState` update on
    the local filesystem. Useful for repos that treat the shared PSE server as the
    single central attractor.
  - `RemoteBundleResponse / RemoteSignal / RemoteILHealth` — typed response structs
    (`Deserialize`) for parsing the server JSON reply.
  - Enables multi-repo governance: any repository can contribute governance rules to a
    shared PSE attractor over HTTP, without filesystem access to the central IL store.
  - `reqwest = { workspace = true }` and `pse-exploratory` added to
    `tools/nxalien-cli/Cargo.toml`.

* **Pfad D — Exploratory Ledger** (`crates/pse-exploratory`, 16 unit tests) —
  negative-ψ hypothesis tracking for the nxalien governance pipeline.

  New crate `pse-exploratory`:
  - `EntryStatus::Pending | Landed { grounded_at_run, grounded_psi } |
    Decayed { decayed_at_run }`.
  - `ExploratoryEntry { rule_id, initial_psi, initial_qtic, block_hash_prefix,
    added_at_run, decay_after_runs, status }`.
  - Constants: `EXPLORATORY_PSI_THRESHOLD: f64 = 0.0`,
    `DEFAULT_DECAY_AFTER_RUNS: u64 = 10`.
  - `ExploratoryLedger` — file-backed at `<nxalien_dir>/exploratory.json`:
    * `ingest(rule_id, psi, qtic, block_hash_prefix, run)` — idempotent; only parks
      entries with ψ < 0. Already-Pending entries for the same rule_id are no-ops.
    * `check_landings(grounded, current_run)` — same rule_id reappears with new ψ ≥ 0
      → `Pending → Landed` transition.
    * `tick_decay(current_run)` — Pending entries older than `decay_after_runs` runs
      → `Pending → Decayed` transition.
    * `to_unknown_slots()` — Pending → `Unknown` slot (confidence =
      `(1+ψ).clamp(0, 0.99)`); Decayed → `Stale` slot. Used to surface hypotheses in
      the `[NXALIEN-CONTEXT]` block.
  - `ExploratoryLedgerSummary { pending_count, landed_count, decayed_count, mean_psi }`.

  Integrated into the nxalien pipeline:
  - `nxalien compile`: rules with ψ < 0 marked with `◈` in IL crystal output; these
    are ingested into the ledger; landing/decay checked each run; `exploratory_summary.json`
    written to `.nxalien/`.
  - `nxalien_bundle` handler: updates the exploratory ledger after every bundle
    (check_landings → tick_decay → ingest new negative-ψ entries → save).
  - `GET /exploratory/status` server route: `{ active, summary, pending_unknowns }`.

* **Pfad E — Epistemic Thunderbolt Vector** (`crates/pse-reasoning`, 10 unit tests) —
  D=ψ·ρ·ω guided multi-hop reasoning over the IL knowledge graph.

  New crate `pse-reasoning`:
  - `ThunderboltConfig { max_steps: 6, min_d_threshold: 0.01, top_k_per_step: 32 }`.
  - `ReasoningStep { step_index, crystal_id_hex, d_score, cumulative_d, qtic_class,
    stability_score, is_exploratory }`. `is_exploratory = qtic_class ≤ 1`.
  - `TerminationReason::MaxSteps | MinThreshold | NoNewMatches | EmptyStore`.
  - `ReasoningChain { query, steps, total_d, mean_d, terminated_by,
    has_exploratory_steps }` + helper methods `peak_d()`, `mean_qtic()`, `is_empty()`.
  - `guide(query, store, config) -> ReasoningChain` — Epistemic Thunderbolt Vector:
    1. `text_to_vector8(query)` → initial 8D semantic vector.
    2. `score_tripolar(vec)` → all IL crystals ranked by D = ψ·ρ·ω.
    3. Select highest-D unvisited crystal (loop prevention via `HashSet<String>`).
    4. `crystal_meta(id)` → `(qtic_class, stability_score)` for step annotation.
    5. Advance: `crystal_vector8(id)` → next query vector.
    6. Repeat until `MaxSteps | MinThreshold | NoNewMatches | EmptyStore`.

  Two new methods added to `ILStore` in `adapters/pse-adapter-il`:
  `crystal_vector8(crystal_id_hex) -> Option<Vec<f64>>` and
  `crystal_meta(crystal_id_hex) -> Option<(u8, f64)>`.

  Server route `POST /reasoning/guide` accepts `{ query, max_steps?, min_d_threshold? }`
  and returns the full `ReasoningChain`. Returns `{ active: false }` when the IL store
  is not loaded. Live example: 4-hop chain on a 5-crystal store, `total_d = 0.835`,
  terminated by `MaxSteps`.

* **nxalien — agent-context exoskeleton** (`crates/pse-nxalien-*`, `tools/nxalien-cli`) —
  six new crates + CLI implementing the nxalien governance layer as a fully interwoven
  PSE subsystem. nxalien is not a standalone product; every subsystem is wired to the
  PSE corpus. 26 unit tests + 2 integration tests (all green).

  **Six crates:**

  - `pse-nxalien-types`: canonical governance types — `RuleAtom` (SHA-256 / JCS
    content-addressed, evidence-sorted for hash stability), `UnknownSlot`,
    `Severity` (Advisory / Required / Blocking), `GateOutcome` (Accept / Hold /
    EvidenceOnly / Reject), `NxAlienBundle`, `NxAlienManifest`, `AgentContextCube`,
    `C8Coord`. RuleAtom hashes use `pse_types::content_address` — same substrate
    as `SemanticCrystal` IDs.

  - `pse-nxalien-core`: `canon` (wraps PSE content-addressing), `gate` (8-gate
    conjunctive evaluation: G_evidence / G_scope / G_replay / G_canon / G_delta /
    G_budget / G_governance / G_bridge), `scanner` (project auto-detection:
    Rust / TypeScript / Python with tool-chain recognition).

  - `pse-nxalien-cube`: `HypercubeHdag` — C⁸ directed acyclic graph (8 axes:
    ψ evidence_potential, ρ rule_density, ω temporal_phase, χ connectivity,
    η causality, γ governance, υ uncertainty, λ utility). Edge admission by
    semantic coherence R_A ≥ τ_A and causal drift ε_η. 5D projection to
    PSE-native `FiveDState` via η' = clip(0,1, 0.50η + 0.25γ + 0.25(1−υ)).

  - `pse-nxalien-agent`: `ContextProjector` — renders `[NXALIEN-CONTEXT]` blocks,
    `CLAUDE.md`, `AGENTS.md`, `.rules` for LLM system-prompt injection.

  - `pse-nxalien-pse`: `NxAlienObservationAdapter` implementing
    `pse_graph::ObservationAdapter` — bundles enter PSE through the same pathway
    as Binance / weather / seismo adapters. Phase hint from gate outcome
    (Accept=π/4, Hold=π/2, EvidenceOnly=3π/4, Reject=π) so Mandorla interference
    reflects governance quality. Invariant **I-BRIDGE-001** enforced by a static
    source guard: nxalien crates must never construct `SemanticCrystal` directly.

  - `pse-nxalien-evolve`: attractor-constrained rule evolution —
    * `GraphState` persists the PSE point cloud across compile runs
      (`.nxalien/graph_state.json`) so the attractor centroid accumulates history.
    * `EpistemicSignal` classifies stability as Initialising / Converging / Stable /
      Drifting / Diverging from distance to attractor centroid + free-energy trend
      + live IL health overlay.
    * `EvolutionGuard` prevents unbounded drift (min attractor alignment threshold,
      max severity downgrade steps, evidence requirement).
    * `propose_rule_evolution` / `apply_validated_proposals` — rejected proposals
      become `UnknownSlot`s to keep drift visible.
    * `il_bridge`: every `RuleAtom` committed to `ILStore` as a QTIC-certified
      `SemanticCrystal`. Severity drives `stability_score` (0.50 / 0.75 / 1.00);
      evidence density drives `kuramoto_coherence`; ψ = kuramoto − (1−stability)
      reflects governance quality. Blocking rules with evidence reach **Q5**
      (path-invariant attractor); Advisory rules without evidence sit at **Q3**.
      `load_il_health_and_agenda` reads `MemoryHealthReport` + `EpistemicAgenda`
      and folds them into `EpistemicSignal`: `at_risk_count > 0` overrides Stable →
      Drifting; `mean_qtic < 2.0` → Diverging; healthy IL + Converging PSE →
      Stable. Agenda items with p ≥ 0.50 surface as `UnknownSlot`s in the
      `[NXALIEN-CONTEXT]` block.

  **`nxalien compile` pipeline (one command, PSE workspace):**
  1. Scan project → default `RuleAtom` set (5 rules for Rust/cargo)
  2. `auto_downgrade_rules` — Required without evidence → Advisory
  3. `HypercubeHdag` — 17 nodes, 23 edges (acyclic ✓)
  4. 8-gate evaluation → `NxAlienGateReport` (Accept)
  5. JCS manifest hash + SHA-256 replay hash chain
  6. Ingest bundle into `PersistentGraph` via `NxAlienObservationAdapter`
  7. Each `RuleAtom` → `SemanticCrystal` → `ILStore` (QTIC certificate per rule)
  8. `EpistemicSignal::extract_with_il` — PSE attractor + IL health overlay
  9. `propose_rule_evolution` → `apply_validated_proposals` (guard-constrained)
  10. IL agenda → `UnknownSlot`s → `.nxalien/il_agenda_unknowns.json`
  11. Outputs: `nxalien.manifest.json`, `nxalien.signal.json`, `.nxalien/il/`
      ledger, `nxalien.rules.md`, `nxalien.evolved-rules.json`

  **Live output (PSE workspace, 5 rules):**
  ```
  IL crystals : 5/5  QTIC̄=3.80  gate=✓
    rust-test, no-direct-crystal  → Q5  (Blocking, path-invariant)
    rust-fmt, rust-clippy, minimal-reversible → Q3  (Required, gate-passed)
  PSE signal  : Stable (dist=0.000)  IL: Q̄=3.8 u=0.42 at_risk=0 ⚠
  ```
  The ⚠ correctly identifies that rules without Evidence references have
  mean uncertainty 0.42 > 0.30 (healthy threshold) — the system requests
  evidence before confirming full health.

* **PSE+IL Intelligence Layer** — `adapters/pse-adapter-il` — 10 new modules
  implementing an active-cognition layer over the IL ledger. 191 unit tests total.
  The layer turns the ledger from a passive record-keeper into an epistemic system
  that monitors its own health, manages knowledge lifecycle, enforces constitutional
  constraints, and generates a prioritised action plan toward the knowledge fixpoint.

  - **Direction 1 — Context compression** (`context.rs`): `ContextBudget`
    (max_tokens / top_k / min_qtic_class), `CrystalSummary` (compact 1-2 line
    representation with Pfauenthron++ D score), `ILStore::context_for_query()` —
    budget-filtered `[PSE-CONTEXT]...[/PSE-CONTEXT]` block for LLM system message
    injection. `IndexEntry` gains `question`, `scale_tag`, `agent_id` fields
    (backward-compatible via `#[serde(default)]`).

  - **Direction 2 — Causal graph** (`causal.rs`): `CausalGraph`, `CausalLink`,
    `CausalCause` (Refinement | Sequential | ResonanceProximity | MetatronIsomorphic |
    UserAsserted), link strength ∈ [0, 1]. Persisted in `il_causal.json` alongside the
    ledger index. `ILStore::causal_graph()` provides the full lineage DAG.

  - **Direction 3 — Agent layer** (`agent.rs`): `AgentCausalGraph`, `AgentLink`
    — multi-agent extension tracking crystal provenance per agent and cross-agent
    causal relationships. Crystals committed with `agent_id` are automatically wired.

  - **Direction 4 — Constitutional AI substrate** (`constitutional.rs`):
    `ConstitutionalRule`, `Severity` (Blocking | Required | Advisory),
    `RulePredicate` (composable tree: All / Any / Not / MinQticClass /
    MinStability / MinKuramoto / MaxFreeEnergy / MinEvidenceEntries /
    CoherenceGate / PathInvariant / RequiresAgentAttribution), `ConstitutionalReport`
    (SHA-256 content-addressed per crystal), `ConstitutionalAuditReport`,
    `ConstitutionalFeedback`, `Constitution`.

    Two preset constitutions: `eu_ai_act_minimal()` (EU AI Act Articles 9/13/17)
    and `pse_core_safety()` (4 rules including S4 hallucination attractor gate —
    `NOT(stability > 0.8 AND kuramoto < 0.2)`).

    `ILStore::commit_constitutional()` — blocking pre-commit check; crystals
    violating a Blocking rule are rejected before writing. `is_constitutionally_closed()`
    — knowledge-base-level Q5 fixpoint: all blocking rules pass for all crystals.
    19 unit tests.

  - **Direction 5 — Epistemic health monitoring** (`health.rs`):
    `crystal_uncertainty(qtic_class, stability, coherence) -> f64`:
    `u = 1 − (qtic_weight · stability · coherence)^(1/3)`.
    `CrystalHealthMetrics`, `MemoryHealthReport` (total, mean QTIC class,
    fraction_q4_plus, mean_stability, mean_coherence, mean_uncertainty,
    healthy_count, at_risk_count, attributed_fraction, oldest/newest block).
    `is_healthy()`: `fraction_q4_plus ≥ 0.80 AND mean_uncertainty ≤ 0.30`.
    `ILStore::memory_health()`, `at_risk_crystals(threshold)`,
    `crystal_health(id_prefix)`. 13 unit tests.

  - **Direction 6 — Crystal lifecycle management** (`lifecycle.rs`):
    `DecayModel` (Linear / Exponential / Step, each with `half_life`),
    `LifecycleStatus` (Vital / Aging / Stale / Redundant),
    `CrystalLifecycle` (age_blocks, decay, uncertainty, refresh_score, status),
    `ConsolidationCandidate` (MetatronIsomorphic | SemanticOverlap, with
    retain/deprecate decision), `LifecycleReport`.
    `refresh_score = uncertainty × (1 − decay)` — urgency of re-asking a question.
    `is_lifecycle_closed()`: no stale crystals and no consolidation candidates.
    `ILStore::lifecycle_report(model, sim_threshold, reference_index)`. 18 unit tests.

  - **LLM prompt grounding** (`prompt.rs`): `GroundedPrompt` and `PromptConfig`
    — compose the full LLM system message from a `[PSE-CONTEXT]` block,
    a `[AGENDA]` block, and the base system prompt, with configurable token budgets.

  - **Causal retrieval** (`retrieval.rs`): `CausalRetrievalConfig` (seed_k,
    max_depth, causal_blend α), `CausalRole` (Seed | Ancestor { depth } |
    Descendant { depth }), `CausallyGroundedEntry` (summary + role + semantic_score
    + causal_score + blended score), `CausalRetrievalResult`.
    Score blending: `final = α · D_semantic + (1−α) · D_causal` where
    `D_causal = seed_semantic · path_strength / (1 + hop_count)`.
    `to_annotated_context_block()` → `[PSE-CONTEXT causal=true]` with
    `[SEED]` / `[ANCESTOR depth=N]` / `[DESCENDANT depth=N]` annotations.
    `ILStore::causal_retrieval(query, config)`. 11 unit tests.

  - **Knowledge clustering** (`cluster.rs`): `ClusterConfig` (sim_threshold,
    min_cluster_size), `KnowledgeCluster` (members, centroid, mean_stability,
    mean_uncertainty, causal_density, mean_qtic_class), `BridgeCrystal`
    (crystal_id, bridges: Vec<cluster_id>, cross_cluster_degree),
    `ClusteringReport` (clusters, singletons, bridge_crystals, total_crystals,
    clustered_fraction). Union-Find connected-component algorithm.
    Causal density = direct causal edges / C(|members|, 2).
    `is_unified()`: singletons empty AND clusters.len() ≤ 1.
    `ILStore::cluster_knowledge(config)`. 13 unit tests.

  - **Epistemic agenda** (`agenda.rs`): `AgendaAction` (Refresh / Reinforce /
    Consolidate / Guard / Explore), `AgendaItem` (priority ∈ [0,1], action,
    rationale, expected_uncertainty_delta), `EpistemicAgenda` (items sorted
    by descending priority, diagnosis, items_to_fixpoint), `AgendaConfig`.
    Priority model: blocking constitutional violation → 1.00; bridge at risk
    → 0.90×u; stale causal root → 0.85×refresh; consolidation metatron → 0.70;
    consolidation semantic → 0.60; at-risk non-root → 0.75×u; stale non-root
    → 0.65×refresh; singleton → 0.30.
    `to_context_block(top_k)` → `[AGENDA]...[/AGENDA]` for LLM system message.
    `is_fixpoint()`: items list is empty. 13 unit tests.

    **The four fixpoint conditions** — the IL store is at epistemic fixpoint when
    `constitutional_audit().is_constitutionally_closed()` AND
    `lifecycle_report().is_lifecycle_closed()` AND
    `cluster_knowledge().is_unified()` AND
    `epistemic_agenda().is_fixpoint()` all hold simultaneously.

* **Infinity Ledger (IL) integration** — `adapters/pse-adapter-il` — full
  PSE+IL fusion layer.

  Bundles the private Infinity Ledger distribution as a zip in
  `vendors/infinityledger/` (single-repo requirement: cloning `lashsesh/pse` is
  sufficient). Exposes an `ILStore` that wraps the IL block-chain ledger and
  wires it to PSE's `SemanticCrystal` pipeline.

  Key components added:

  - **`ILStore`** — append-only ledger of crystal blocks (8D vector, topology
    signature, stability score, Metatron canonical hash). `commit_with_feedback()`
    returns a `ValidationFeedback`; `commit()` wraps it for backward compatibility.

  - **`ValidationFeedback`** — `{ block_hash, converged, coherence_potential,
    gate_passed, hdag_node_id, il_stability }`. When `|il_stability −
    original.stability| > 0.02`, `refine_crystal()` is automatically called.

  - **`refine_crystal()`** — IL→PSE feedback loop. Produces a new crystal with
    blended stability `0.7·PSE + 0.3·IL`, a fresh SHA-256 content address, and the
    original in `parent_crystal_ids`. The refined crystal is also committed to IL,
    creating a `refinement` HDAG edge.

  - **`IndexEntry`** gains: `phase: f64`, `hdag_node_id: Option<String>`,
    `stability_score: f64` (serde `default = 0.5`), `metatron_canonical_hash:
    Option<String>` — all backward-compatible with existing ledger files.

* **HDAG v1.0** (`adapters/pse-adapter-il/src/hdag.rs`) — Hierarchical Directed
  Acyclic Graph over the IL ledger. Implements the spec in
  `specs/HDAG_bySebastianKlemm_v1.0.pdf`.

  - **5D resonance tensor** per crystal:
    `[mean_propagation_time, kuramoto_coherence, cheeger_estimate, spectral_gap,
    1−stability_score]` = `[temporal, morphic, relational, topological, entropic]`.
    When Metatron data is present, `cheeger_estimate` and `spectral_gap` are
    replaced by `algebraic_connectivity/n` and `spectral_radius/n`.

  - **Coherence potential** ψ = `kuramoto_coherence − (1 − stability_score)`.
    S_coh class: ψ > −0.1 or Kairos gate passed.

  - **Emergent acyclicity** — edges only added when ψ(target) ≥ ψ(source); no
    timestamp checks required.

  - **Four edge causes**: `sequential_commit`, `resonance_proximity` (‖T_A−T_B‖ ≤
    0.35, both in S_coh), `refinement` (parent_crystal_ids link), `metatron_isomorphic`
    (shared Metatron canonical hash).

  - **Path invariance** (`∮Φ·dl = 0`) — `verify_path_invariance()` using Kahn's
    topological sort and canonical-condensation comparison.

  - **Semantic predecessor search** — `find_semantic_predecessors()` for resonance
    proximity edge wiring.

  - **HDAG statistics**: `edge_count_by_cause()`, `mean_coherence_potential()`,
    `topological_order()`.

* **Pfauenthron++ Unified Retrieval** (`D = ψ · ρ · ω`) — implements the
  tripolar scoring formula from `specs/TheTimelessMonolith_bySebastianKlemm_v1.0.pdf`.

  - `ILStore::score_tripolar(&[f64]) -> Vec<ILMatch>` — multiplicative D = ψ·ρ·ω
    where ψ = IL cosine similarity, ρ = `stability_score`, ω = normalized HDAG
    coherence potential.

  - `pse-llm-demo` uses `pfauenthron_score_all()` instead of the legacy
    `query_similar()`. Logs: `[Unified retrieval: N record(s), top D=X.XXX]`.
    Context label: `[Unified Retrieval — Pfauenthron++ D=ψ·ρ·ω]`.

  - Gabriel4D Funnel: all three axes must be non-trivial — a near-zero on any
    axis collapses the overall D score.

* **pse-server IL/HDAG HTTP routes** (`tools/pse-server`) — four new routes on
  top of the existing four PSE routes (total: eight routes):
  `GET /il/status`, `POST /il/retrieve` (Pfauenthron++),
  `GET /il/hdag/coherence`, `GET /il/hdag/order`.
  IL routes activate only when `PSE_IL_PATH` is set at startup.
  `IngestResponse` gains `il_commits: Vec<ILCommitInfo>` (skip_serializing_if empty).

* **MetatronTopologySignature in HDAG tensor** — `crystal_to_tensor()` uses
  Metatron scan data (`algebraic_connectivity/n`, `spectral_radius/n`) when
  present, giving graph-theoretic precision over heuristic cheeger/spectral
  estimates from the topology signature.

* **QTIC theoretical foundation** documented — `specs/QTIC.pdf` fully mapped
  onto PSE+IL. The full QTIC↔PSE+IL table and Q0–Q5 conformance class
  mapping are documented in `README.md`. Every Q5-conformant crystal is a
  seam-stable, path-invariant, replayable information attractor.

* **PSE-VALIDATION-RUNNER-DOMAIN-01** — Domain validation layer for the PSE workspace.

  Adds a complete L3 domain validation pipeline that runs embedded
  ground-truth benchmark scenarios (seismo/vitals/binance) and derives a
  formal `ValidationConclusion` from real run artifacts.

  Key components:

  - **`pse-bench-gt` JSON output**: `--scenario <seismo|vitals|binance>`,
    `--format json`, `--out <path>` flags added to the `bench_gt` binary.
    Produces machine-readable `BenchGtJsonOutput` with P/R/F1, PSE vs
    STL-zscore vs IsoForest metrics, and `config_hash` / `data_hash`.

  - **`DomainValidationSummary`**: Built from real bench_gt JSON outputs.
    Includes `BaselineComparisonReport` (PSE F1 vs baselines per scenario),
    leakage check, and test-split completion status.

  - **Scoring gate tightened**: `ScoringInputs` gains `domain_test_completed`
    field. `EmpiricalImprovement` requires `domain_test_completed = true`;
    domain available but test not done → `DiagnosticFinding`.

  - **Domain CLI**: `pse-validate run --profile domain --domain-manifest <path>`
    with fail-closed behavior (error if `--domain-manifest` is missing).

  - **`verdict.json`**: Written to every run output directory, records
    conclusion, domain flags, and replay identity.

  - **Command plan**: Domain phases now invoke real
    `cargo run -p pse-bench-gt --bin bench_gt -- --scenario <name> --format json`
    commands (DomainCalibration→seismo, DomainValidation→vitals, DomainTest→binance).

  - **Embedded fixture**: `validation_domains/embedded_ground_truth/manifest.json`
    with three non-overlapping splits (distinct data hashes) for
    seismo/vitals/binance scenarios.

  - **8 new tests**: domain profile requires manifest, missing manifest fails,
    domain summary from records, baseline comparison wins, no domain→no
    empirical improvement, test not completed→diagnostic finding, leakage
    invalidates, verdict.json written.

* **PSE-NCTCS-CONFORMANCE-01** — Null-Centered Toroidal Control Closure
  Layer. New submodule `crates/pse-validation-runner/src/nctcs/` (14
  modules) inside the existing `pse-validation-runner` crate.

  Implements a C0–C4 conformance ladder and produces a content-addressed
  `NctcsClosureBundle` (byte-identical replay). The pipeline:

  ```text
  NctcsRunDescriptor + NctcsClosureInput
    → NullCenterRef         (C0: exogenous, not_phase_state, not_agent)
    → NullProjectionAudit   (K0 ≠ π0(K0): projection distinction)
    → ToroidalPhaseFlowAudit (phase-flow timing, visibility-only)
    → PhaseVisibilityAudit  (C1: phase-gated visibility, coverage ≥ θ)
    → CandidateFormationAudit (C2: candidate_requires_visibility_passed)
    → MaterializationAudit  (C2: no direct fabric→tensor mutation,
                              Dissolution-Grundsatz preserved)
    → TraceReplayContractReport (ReplayIdentity ≥ threshold,
                              replay_ready_required_for_gate_pass)
    → classify_conformance  (two-pass: pre-macro, then with MacroControlState)
    → MacroControlState?    (C4 only, from null_center + tensor + trace
                              — NEVER from resonance or ephemeral fabric)
    → NctcsClosureBundle    (content-addressed, JCS + SHA-256)
  ```

  **Conformance ladder**: `C0FormalTyped` (exogenous null center) →
  `C1PhaseGatedVisibility` (phase-gated candidate visibility) →
  `C2GateBoundMaterialization` (gate-bound tensor revisions) →
  `C3AuditableTensor` (auditable tensor history + trace) →
  `C4MacroControl` (full macro control state).

  **Gate semantics** (fail-closed): `NctcsGateOutcome::Pass` is the
  only materializing outcome; `Hold / Reject / Quarantine / NoUpdate /
  HandoffReady` all produce a non-materializing decision record.
  `ValidationClosureStatus` never reaches `EmpiricalImprovement`
  without a real domain validation result.

  Eight NCTCS metrics registered in `pse-eval-matrix`
  (`nctcs_conformance_class_score`, `nctcs_visibility_candidate_compliance`,
  `nctcs_no_direct_persistence_rate`, `nctcs_gate_bound_revision_rate`,
  `nctcs_trace_replay_contract_rate`, `nctcs_macro_state_validity`,
  `nctcs_coherence_truth_separation_rate`,
  `nctcs_domain_validation_required_compliance`).

  CLI commands added to `pse-validation-runner-cli`:
  `nctcs-close` (full closure pipeline → `nctcs_closure_bundle.json`),
  `nctcs-replay` (byte-identity verification),
  `nctcs-verify` (declared bundle_id recomputation).

  Tests: 10 unit tests, 2 integration tests, 3 negative tests (25 total
  in `nctcs/tests.rs`).

* **PSE-METATRON-MONOLITH-01** — Holistic Eigenmode Closure Layer. New
  submodule `crates/pse-metatron/src/closure/` (11 modules) inside the
  existing `pse-metatron` crate, plus a new `pse-metatron-cli` binary.

  Evaluates a composite fail-closed gate over the full PSE stack and
  produces a content-addressed `HolisticEigenmodeState` only when every
  sub-gate passes:

  ```text
  MetatronRunDescriptor + MetatronClosureInput
    → LocalMonolithProjection[]  (content-addressed per projection)
    → IsomorphicProjectionReport[]  (operator-path + gate-order +
                                     trace + replay dependency checks)
    → SpectralGapStitchReport    (prior_gap, post_gap, delta_gap,
                                  improved_or_preserved)
    → MetatronGateReport         (G_meta = G_nctcs ∧ G_trace ∧ G_replay
                                  ∧ G_iso ∧ G_gap ∧ G_eval ∧ G_drift)
    → MetatronClosureOutcome:
        Closed(HolisticEigenmodeState)  ← gate passed
        Diagnostic(MetatronGateReport)  ← gate failed (fail-closed)
        Rejected(reason)                ← pre-flight policy violation
  ```

  **Gate semantics** (fail-closed): `G_iso` requires at least one
  `IsomorphicProjectionReport` with `passed = true` (vacuously-empty
  does NOT pass). No `HolisticEigenmodeState` with productive status
  is ever produced when `G_meta = 0`.

  **Metatron conformance classes** `M0–M5` classify how many gates of
  the composite passed. `HolisticEigenmodeState` is content-addressed
  (JCS + SHA-256, self-referential `state_id` computed from the
  zero-initialized form). Replay verification zeroes the ID before
  recomputing, matching `build()` — same fix applied to
  `verify_nctcs_bundle`.

  Self-contained `closure/primitives.rs` re-implements `Hash256`,
  `CanonicalNumber`, and `content_address()` using `serde_jcs` +
  `sha2` directly to avoid the cyclic crate dependency
  `pse-traverse → pse-core → pse-cascade → pse-metatron → pse-traverse`.

  `pse-metatron-cli` binary (new tool `tools/pse-metatron-cli/`):
  `inspect` / `project-local` / `isomorphism` / `spectral-gap` /
  `close` / `replay` / `verify` (7 subcommands).

  Tests: 8 unit tests, 2 integration tests, 3 negative tests (13 total
  in `closure/tests.rs`).

* **PSE-TRAVERSE-TPT-MTL-04** — Topological Panoptic Triangulation and
  Möbius-Tripolar Micro-Lift topology layer (conformance class TPTM-5).
  New module `crates/pse-traverse/src/topology/` (feature `topology`)
  and CLI binary `pse-traverse-topology-cli` (14 subcommands).

  Core pipeline: `PhaseSpaceWindow` → `AxisBridgeReport` (I-03 axis
  separation) → `MeshHolo` (seed + evolve under `TopologyGuard`) →
  `MicroFiber[]` (primary + MTL-D1 dual + seam per point) →
  `CarrierReport` (I-06 stateless null-center) →
  `ReinterpretationReport` (Betti numbers → claim candidates) →
  `TptMtlGateReport` (13 fail-closed gates) →
  `TopologicalCrystalCandidate` (not a SemanticCrystal) →
  `TptMtlBundle` → `ReplayManifest` (5-digest replay anchor).

  Ten invariants enforced (I-01 … I-10). MTL-D1 dualization uses f64
  for intermediate arithmetic to avoid rational overflow; results are
  quantized to Fixed(scale=9) before any hashing, preserving
  audit-pathway determinism. All 24 topology integration tests and 218
  total pse-traverse tests pass.

  Ten TPT-MTL metrics registered in `pse-eval-matrix`:
  `tpt_adapter_totality_rate`, `tpt_axis_bridge_validity`,
  `tpt_mesh_determinism_identity`, `tpt_topology_robustness`,
  `tpt_micro_lift_coverage`, `tpt_seam_consistency_rate`,
  `tpt_carrier_continuity`, `tpt_false_crystal_rate`,
  `tpt_trace_completeness`, `tpt_replay_identity`.

* **PHASEMATRIX-HIVEMIND-03** — morphodynamic resonance cell substrate.
  New crate `phase-matrix` and CLI tool `pse-phase-matrix-cli` (binary:
  `phase-matrix`). Implements the spec's full cell-pool → pulses →
  cluster → funnel-graph → morphology → convergence → intent → trace
  → dissolution pipeline as a deterministic `run_cell_substrate_cycle`
  that the runner / replay / verify subcommands all share.
  * Data model: `PhaseCell` (with deterministic `synthetic` factory and
    `PhaseCellRole` covering Sensor / Resonator / Router / Validator /
    MemoryProbe / BoundaryGuard / CandidateEmitter /
    MorphologyRegulator), `CellPool` with matrix-boundary enforcement
    at insertion (foreign-parent cells are rejected), `TridentVector`
    (semantic_density × structural_coherence × temporal_phase →
    activation_potential), `LocalResonanceProcessor` +
    `ResonanceNonlinearity` (Logistic / TanhApprox / SaturatingLinear
    / PiecewiseFixed), `ResonancePulse` with `PhaseBin` quantisation
    (Continuous / KPolar / Tripolar / Quadrupolar), `ResonanceCluster`
    + `ClusterLifecycle` (Proposed / Forming / Active / Stabilized /
    Splitting / Fusing / Decaying / Compacted / Dissolved / Rejected)
    + `ClusterFormationReport`, `FunnelGraph` with four edge families
    (Spatial / Temporal / Semantic / Resonance) and DFS-based
    WHITE/GRAY/BLACK acyclicity validation, `MorphodynamicField`
    (`H = α · Φ + β · µ`) + `ClusterMorphologyEvent` (Grow / Split /
    Fuse / Decay / Replicate / Stabilize / DissolveWorkingState /
    CompactToTrace) + `MorphologyDecision`, `ConvergenceField`,
    `TensionToIntentOperator` + `IntentCandidate` (sorted claim refs),
    `RecursiveFeedbackReport` (Ouroboros loop with bounded learning
    rate), `ClusterTrace` tying every artefact hash together,
    `DissolutionMode` (DropWorkingState / CompactToTrace /
    PersistEvidenceOnly / PersistClusterSummary / ArchiveFullState) +
    `DissolutionReport.validate_trace_preservation` enforcing the
    Dissolution-Grundsatz (working state may be compacted but trace +
    evidence + lifecycle history MUST be preserved),
    `CellToHandoffCandidate`, `PhaseSubnet` /
    `PhaseMatrixNode` / `NodeTrustState`, `MatrixClaim` /
    `TruthMaintenanceReport` / `MatrixBoundaryReport`,
    `CycleReportSummary`, `PhaseMatrixRunDescriptorV3` with
    `CellSubstrateThresholds::permissive()` /
    `CellSubstratePolicies::strict()` / `MatrixGatePolicy::strict()`,
    `ReplayObservation` / `verify_cycle_replay`.
  * Five fail-closed gates: `G_cluster` (phase ∧ coherence ∧ morpho ∧
    purpose ∧ trace), `G_morph` (endo ∧ exo ∧ boundary-safe),
    `G_intent` (tension ∧ convergence ∧ conflict ∧ trace-ready),
    `G_dissolve` (working-state-eligible ∧ trace-persisted ∧
    evidence-persisted ∧ gate-history-persisted), plus the
    matrix-boundary check at the pool layer.
  * `pipeline::run_cell_substrate_cycle` drives the full deterministic
    cycle and returns a `CellSubstrateOutcome` (Completed / Hold /
    Rejected / Compacted / MatrixBoundaryViolation /
    DeterminismViolation). Two runs over the same `(input, rd)` are
    byte-identical.
  * **No `SemanticCrystal` and no `FinalizedEmission`** are
    constructed in any cell-substrate module — the substrate emits
    handoff candidates only; the PSE-Bridge remains the only commit
    path. The `no_commit_artefacts_appear_in_outcome_bytes` test
    guards this invariant against canonical bytes.
  * Feature flags: `cell-substrate` (default-on), `cell-cli`,
    `cell-funnel-graph`, `cell-morphodynamics`, `cell-convergence`,
    `cell-handoff`.
  * Float-free in every gate / score path: `Fixed` (`CanonicalNumber`)
    rationals normalised by gcd, JCS-canonical reports, sorted lists
    before hashing, `BTreeMap`-keyed structures, no wall-clock in the
    audit pathway, no platform RNG.
  * CLI `phase-matrix`: `cluster-cycle`, `cluster-replay`,
    `cluster-verify`, `cell-pool`. Four CLI smoke tests cover the full
    cycle / replay / verify / pool flow.
  * Tests: 34 unit tests + 6 end-to-end integration tests + 4 CLI
    smoke tests.

* **PHASEMATRIX-HIVEMIND-03.1** — Dual-Fabric Field-Tensor Stitch
  Layer. Additive patch on the PHASEMATRIX-HIVEMIND-03 cell-substrate
  implementation. Plugs cleanly into the existing `phase-matrix` crate
  without duplicating or parallelising any existing architecture.
  * **Data model**: `FieldTensorState` (Fabric-T — persistent;
    content-addressed; carries `tensor_revision`, `coupling_matrix_hash`,
    `previous_tensor_hash` chain, `trace_head`), `CouplingMatrix` +
    `CouplingEntry` (five coupling kinds: Structural / Resonance /
    Temporal / Semantic / Boundary), `ResonanceFabricState` (Fabric-H
    — ephemeral; derived deterministically from each
    `CellSubstrateCycleReport`; carries mandatory `trace_hash` per §5.2
    Invariant), `EphemeralResonanceLink` (source / target / resonance_score
    / phase_alignment / ttl_ticks), `StitchCandidate` (proposed coupling
    change; never touches Fabric-T directly), `CouplingUpdate` (accepted
    change; references exactly one `StitcherGateReport`),
    `MirrorConsistencyReport` (MCI per candidate), `TensorDeltaReport`
    (cumulative L1 norm + per-edge max + hypothetical tensor-after hash),
    `StitcherGateReport` (per candidate; all seven sub-gate booleans),
    `FieldTensorTrace` (append-only audit log; sorted before hashing),
    `StitcherReport` (content-addressed outcome; sorted accepted_updates
    / rejected_candidates / gate_reports before hashing),
    `StitcherOutcome` (Completed / Hold / Rejected — all carry the report),
    `StitchRunDescriptor` (replay anchor), `StitchThresholds` /
    `StitchPolicies`, `StitchCycleBundle` (replay-ready artefact containing
    rd, fabric_h, tensor_before, tensor_after, outcome,
    source_cluster_trace_hash).
  * **Key invariants**:
    * Invariant 1 — Fabric-H isolation: Fabric-H MUST NEVER directly
      mutate Fabric-T; all changes route through the StitcherGate.
    * Invariant 2 — StitcherGate is fail-closed:
      `G_stitch = G_conv ∧ G_mci ∧ G_delta ∧ G_budget ∧ G_trace ∧ G_boundary ∧ G_evidence`.
    * Invariant 3 — tensor_revision increments exactly once per
      accepted batch.
    * Invariant 4 — previous_tensor_hash chain is preserved.
    * Invariant 5 — when no updates accepted, tensor_after is
      byte-identical to tensor_before (no trace_head mutation).
    * Invariant 9 — `CouplingUpdate`s sorted before hashing when
      `sort_updates_before_hash = true`.
  * **Pipeline** (`run_stitch_cycle`): validate descriptor → build
    Fabric-H → derive candidates → mirror consistency → tensor delta →
    gate evaluation → collect accepted updates → apply to Fabric-T →
    write FieldTensorTrace → write StitcherReport. Replay path
    (`verify_stitch_replay`) reuses the stored Fabric-H directly
    (bypassing `build_resonance_fabric`) for byte-identity verification.
  * **New modules** in `crates/phase-matrix/src/cell/`:
    `field_tensor`, `resonance_fabric`, `coupling_update`, `stitcher`,
    `stitcher_gate`, `mirror_consistency`, `tensor_delta`,
    `field_tensor_trace`, `stitch_pipeline`.
  * **New CLI subcommands** in `pse-phase-matrix-cli`:
    `stitch-fabric`, `stitch-candidates`, `stitch-gate`, `stitch-apply`,
    `stitch-cycle`, `stitch-replay`, `tensor-inspect`.
  * **New Cargo feature**: `cell-stitch` (default-on; depends on all
    four prior cell features).
  * Float-free in every gate / score path (all `Fixed`); no wall-clock;
    `BTreeMap` for all keyed collections; sorted-before-hashing for all
    lists; JCS-canonical reports.
  * 9 unit tests in `stitch_pipeline.rs`; 5 integration tests in
    `end_to_end.rs`; 3 CLI smoke tests in `cli_smoke.rs`.

* **PSE-EVAL-MATRIX-01 — PHASEMATRIX-HIVEMIND-03.1 closure.** Extended
  the eval matrix so the system stays empirically closed across the
  new stitch layer (additive on top of the HIVEMIND-03 closure):
  * New `WorkloadFamily::DualFabricStitch` (the matrix now lists eleven
    mandatory families) plus `WorkloadSpec::dual_fabric_stitch`
    constructor with hold-correctness / no-false-commit /
    replay-byte-identical success criteria.
  * Six new `CellSubstrateMetricKind` variants for the stitch layer:
    `StitcherGatePassRate`, `CouplingUpdateTraceCoverage`,
    `TensorRevisionMonotonicity`, `MirrorConsistencyCompliance`,
    `StitchReplayIdentity`, `FabricHIsolationRate`.  Three are primary:
    `StitcherGatePassRate`, `StitchReplayIdentity`,
    `FabricHIsolationRate`. `FabricHIsolationRate` is always 1.0 (hard
    invariant; any deviation would be a critical failure).
  * New `dual_fabric_stitch_metric_specs()` (6 metrics) and
    `b9_metric_specs()` (16 metrics: 10 cell-substrate + 6 stitch).
  * New `LayerMask::DUAL_FABRIC_STITCH` bit (1 << 13) and
    `B9_DualFabricStitch` ladder rung (= `B8_PhaseMatrix |
    DUAL_FABRIC_STITCH`), `SystemVariantSpec::dual_fabric_stitch()`
    constructor, and `VariantLadder::full_with_dual_fabric_stitch()`.
  * New `dual-fabric-stitch` preset (B0 / B8 / B9 over the
    `DualFabricStitch` workload, scored against all 16 B9 metrics).
  * `SyntheticTrialExecutor` now emits stitch-layer metric observations
    for `DualFabricStitch` workloads: the six stitch metrics for
    stitch-active variants (B9); the ten cell-substrate metrics for
    substrate-active variants (B8+); `FabricHIsolationRate = 1.0`
    always.
  * `preset_dual_fabric_stitch` exported from `pse-eval-matrix` crate root.
  * 4 new tests in `presets.rs` and `cell_substrate_metrics.rs`.

* **PSE-EVAL-MATRIX-01 — PHASEMATRIX-HIVEMIND-03 closure.** Extended
  the eval matrix so the system stays empirically closed across the
  new substrate:
  * New `WorkloadFamily::MorphoCellSubstrate` (the matrix now lists
    ten mandatory families) plus `WorkloadSpec::morpho_cell_substrate`
    constructor with the standard hold-correctness / no-false-commit /
    replay-byte-identical success criteria.
  * New `cell_substrate_metrics` module with the canonical
    PHASEMATRIX-HIVEMIND-03 metric set (ten metrics:
    `cluster_formation_rate`, `morphology_gate_compliance`,
    `convergence_stability`, `intent_generation_rate`,
    `dissolution_trace_preservation`, `funnel_acyclicity_rate`,
    `matrix_boundary_violation_rate`,
    `working_state_compaction_efficiency`,
    `handoff_candidate_utility`, `substrate_self_coherence`).
  * New `LayerMask::CELL_SUBSTRATE` bit and `B8_PhaseMatrix` ladder
    rung (= `B7_FullStack | CELL_SUBSTRATE`),
    `SystemVariantSpec::phase_matrix_substrate()` constructor, and
    `VariantLadder::full_with_phase_matrix()` for the extended
    nine-rung ladder.
  * New `phase-matrix-substrate` preset (B0 / B7 / B8 over the new
    workload, scored against the full cell-substrate metric set).
  * `SyntheticTrialExecutor` now emits the cell-substrate metric
    observations whenever the workload is `MorphoCellSubstrate`,
    pinned to the fail-closed floor for variants without the
    `CELL_SUBSTRATE` bit and monotonically uplifted for the B8
    variant. Two regression tests guard the uplift on
    `cluster_formation_rate` and the lower-is-better behaviour of
    `matrix_boundary_violation_rate`.

* **PSE-EVAL-MATRIX-01** — empirical benchmark matrix for
  post-symbolic cognition systems. New crate `pse-eval-matrix` and
  CLI tool `pse-eval-matrix-cli` (binary: `pse-eval-matrix`).
  * Data model: `EvaluationSpec` (content-addressed, validatable),
    `SystemVariantSpec` over the B0 → B7 variant ladder with
    explicit `LayerMask` bitset, `WorkloadSpec` over nine mandatory
    families (`StreamEvent` / `AnomalyRegime` / `TraversalPuzzle` /
    `CodeAgentPatch` / `DocSynthesis` / `MemoryReuse` /
    `HorizonFinalization` / `CognitionPanorama` / `MultiAgent`),
    `DatasetManifest` with `calibration` / `validation` / `test`
    splits, `GroundTruthProfile` (synthetic-exact, semi-synthetic
    injection, historical, unit-test oracle, human-adjudicated),
    `MetricSpec` (family / direction / primary flag / aggregation /
    invalidation rules), `MetricObservation`,
    `EvaluationRunLedger` (append-only, hash-chained) with
    `EvaluationRunEntry` and `RunStatus`, `TrialReport` with
    `TrialOutputs` / `GateObservationSet` / `ReplayObservation` /
    `DiagnosticRecord`, `EvaluationSummaryReport`,
    `CapabilityProfile`, `AblationSummary` + `MetricDelta` +
    `AblationConclusion`, `StatisticalSummary`,
    `ReviewerReport` (qualitative rubric), `FailureRecord` /
    `FailureKind` (replay mismatch, false crystal, missed event,
    false handoff, over-hold, under-hold, memory mislead, wormhole
    abuse, calibration leakage), `CalibrationLedgerEntry` /
    `CalibrationProfile` / `CalibrationReason`.
  * Operators: `plan_runs` (deterministic plan), `run_trial` +
    `TrialExecutor` trait (pluggable, with reference
    `SyntheticTrialExecutor`), `init_ledger` / `append_to_ledger` /
    `verify_ledger_chain` (rolling chain hash),
    `verify_trial_replay` (byte-identity check), `score_ledger`
    (aggregates strictly from declared `MetricObservation`s — never
    recomputes), `score_capability_profile` (`U_task / U_replay /
    U_safety / U_cognition / U_efficiency / U_calibration /
    U_robustness` + Safety-Adjusted Utility),
    `safety_adjusted_utility`, `cognition_uplift`,
    `layer_marginal_utility`, `summarize_ablation`,
    `build_ablation_ladder` (eight ablation rungs per §3.2),
    `bootstrap_mean_ci` (deterministic seeded LCG — no platform
    RNG), `exact_binomial_ci`, `paired_mean_diff`,
    `render_markdown_summary`, `render_json_summary`.
  * Three built-in presets (§18): `agent-cognition`,
    `streaming-event-detection`, `post-symbolic-ablation`. Each
    preset stamps a content-addressed spec; the CLI's `init
    --template <preset>` is the canonical entry point.
  * Feature flags: `eval-matrix` (default-on), `eval-cli`,
    `eval-agent`, `eval-cognition`, `eval-streams`,
    `eval-statistics`, `eval-reports`.
  * Float-free in every score / metric / gate hash:
    `CanonicalNumber` only, gcd-normalised rationals to keep i128
    arithmetic safe under composition, `BTreeMap` keyed,
    sorted lists before hashing, JCS-canonical reports, no
    wall-clock timestamps in the audit pathway, no platform RNG.
  * `Schlussformel` (§23) enforced: a system counts as *empirically
    improved* only when `ΔU_task > 0 ∧ ΔU_safety ≥ 0 ∧
    ReplayIdentity = 1 ∧ InvalidRunRate ≤ ε ∧ LMU_target > 0`,
    surfaced as `ConclusionFlag::EmpiricalImprovement` vs.
    `DiagnosticFinding` / `InvalidatedByReplay` /
    `InvalidatedByLeakage`.
  * CLI: `pse-eval-matrix init|validate|plan|run|replay|score|ablate|compare|report`.
  * 49 unit tests + 6 end-to-end integration tests + 4 CLI smoke
    tests; workspace test count rises to **839 / 839** passing.

* **PSE-TRAVERSE-COGNITION-01** — panoptic phase cognition kernel
  layer in `crates/pse-traverse/src/cognition/`:
  * Layered data model `C0–C10`: `CognitionRunDescriptor`,
    `CanonicalCognitionState`, `CognitiveState5D`
    (`ψ, ρ, ω, χ, τ` + derived potential / energy / entropy /
    stability_index), `SingularityDetectorReport`,
    typed `OperatorDeclaration` / `OperatorFamily` / `OperatorType` /
    `IntegrationMode` / `IntegratorKind` / `CognitionSimulationSpec`,
    `SpiralMemoryAddress` / `SpiralMemoryHitSet` / `SpiralSegment`,
    `ConstraintLatticeCognition` / `Resonite` / `Infogene` /
    `AdmissibleRegion` / `InfogenePolicy`,
    `HypercubePuzzleState` / `CognitiveDimension` /
    `PartialAssignment` / `CandidateSet` / `HiddenSingle` /
    `BoundaryContract` / `NegativeTopologyWitness` /
    `EntropyCollapseCertificate`,
    `PhasePanorama` / `Horizon360` / `PhasePath` /
    `AttractorCandidate` / `RecognitionBoundary` /
    `ChoiceGeometryReport`,
    `ScorpioPhaseScheduler` / `ActivationWindow` / `ResonanceOffset` /
    `TransportPolicy` / `VectorTunnelTransport` / `ReasonCode`,
    `GovernedWormhole`,
    `SelfModelTensor` / `ReflexiveModulation` /
    `DualTriggerFeedbackGate`,
    `FixpointCalibrationShell` / `PerformanceTriplet` /
    `ResonanceImpulse` / `CarrierMigrationPlan`,
    `AttractorMap` / `AttractorEntry`,
    `SingularityTriggerReport`,
    `CognitionHandoffGate` / `ProjectionHandoffPolicy` /
    `CognitionCandidate` / `CognitionCandidateBundle`,
    `CognitionReport` / `CognitionHoldReport` /
    `CognitionDiagnostic` / `CognitionRecoveryAction` /
    `CognitionOutcome`.
  * Operators: `null_center_unfold`-style derivations,
    `detect_singularity`, `spiral_memory_query`,
    `build_lattice_minimal`, `evaluate_perkolation`,
    `build_puzzle_minimal`, `build_panorama_minimal`,
    `build_scheduler_minimal`, `admit_wormhole`, `build_self_model`,
    `evaluate_dual_trigger`, `calibrate`, `evaluate_por_acceptance`,
    `evaluate_migration`, `evaluate_singularity_trigger`,
    `CognitionHandoffGate::evaluate`.
  * `pipeline::run_cognition` — total reference pipeline (per §16:
    canonicalize → 5D state → spiral memory query → constraint
    lattice → hypercube puzzle → perkolation → scheduler → panorama
    → wormholes → self-model → dual-trigger feedback → fixpoint
    calibration → carrier migration → attractor ranking →
    singularity trigger → handoff gate → bundle-or-hold → report →
    replay).
  * `replay_hash_of` / `assert_replay_match` for byte-identity audit.
  * Feature flags: `cognition` (default-on), `cognition-cli`,
    `cognition-simulation`, `cognition-spiral-memory`,
    `cognition-hypercube`, `cognition-scorpio-phase`,
    `cognition-wormholes`, `cognition-calibration`,
    `cognition-projection-handoff`.
  * Float-free everywhere: gate-relevant scalars are `Fixed`
    (`CanonicalNumber`); rationals are normalised by gcd to keep
    i128 arithmetic safe under composition; keyed structures are
    `BTreeMap`; lists are sorted before hashing; reports are
    JCS-canonicalised.
  * **No `SemanticCrystal` and no `FinalizedEmission`** are
    constructed in any cognition module — the kernel hands a
    `CognitionCandidateBundle` to projection-v0.2, which alone may
    finalise; the PSE-Bridge remains the only commit path.
* **`pse-traverse-cognition-cli`** tool binary (binary name
  `pse-traverse-cognition`) with the spec's twelve subcommands
  (§18): `inspect`, `observe`, `state5`, `memory-query`, `lattice`,
  `puzzle`, `panorama`, `calibrate`, `trigger`, `bundle`, `replay`,
  `verify`. Golden fixtures in
  `tools/pse-traverse-cognition-cli/tests/fixtures/` and
  end-to-end CLI smoke tests for every subcommand.
* **PSE-TRAVERSE-HORIZON-03** — null-centered horizon geometry layer
  in `crates/pse-traverse/src/horizon/`:
  * Data model: `HorizonRunDescriptorV3`, `HorizonThresholdsV3`,
    `HorizonPoliciesV3`, `HorizonFailurePolicy`, `CarrierPolicyV3`,
    `HorizonWindowPolicyV3`, `ProjectionConePolicyV3`,
    `CausalPolicyV3`, `DualityPolicyV3`, `CrossingPolicyV3`,
    `RationalFixed`, `EpochRange`, `HorizonEvidenceRef`,
    `HorizonError`.
  * Operators: `NullCenterUnfold`, `PhaseRayLift` (hypertorus T^n,
    default n = 4), `HorizonVisibility`, `ProjectionConeCheck`,
    `CausalOrderCheck`, `CollapseEmissionDualityCheck`,
    `HorizonCrossingGate`, combined gate
    `G_v0.3 = G_projection_v2 ∧ G_cross ∧ ReplayReady`.
  * Reports & artefacts: `HorizonChart` (content-addressed),
    `PhaseRay`, `EventHorizonWindowV3` /
    `HorizonWindowReportV3`, `ProjectionCone` /
    `ProjectionConeReport`, `CausalAdmissibilityReport` (with
    `CausalViolation`), `DualityReport`, `HorizonCrossingReport`,
    `FinalizedEmissionV3`, `HorizonHoldReport`, `HorizonV3Outcome`
    (`Finalized` / `Hold` / `WaitForHorizon` / `RefineCone` /
    `NeedsCarrierMigration` / `Recondense` / `ProjectionOnly` /
    `InvalidInput` / `DeterminismViolation`),
    `HorizonCertificate`, `replay_hash_of`, `assert_replay_match`.
  * `pipeline::run_horizon_v3` — total reference pipeline
    (canonicalize → null-resolve → unfold → ray-lift →
    window-evaluate → cone-check → causal-check → duality-check →
    crossing-gate → projection-v0.2 merge → finalize-or-hold →
    certify → replay).
  * Feature flags: `horizon` (default-on), `horizon-cli`,
    `horizon-projection-v2`, `horizon-pse-bridge`,
    `horizon-adapters`.
  * Float-free in every audit path: gate-relevant scalars are
    `CanonicalNumber` (`Fixed`); rationals are `RationalFixed` with
    decimal-string i128 serialisation; keyed collections are
    `BTreeMap`; ray / window / kind lists are sorted before
    hashing; every report is JCS-canonicalised.
  * No `SemanticCrystal` is constructed in any horizon module — the
    PSE-Bridge remains the only commit path.
* **`pse-traverse-horizon-cli`** tool binary (binary name
  `pse-traverse-horizon`) implementing the spec's seven subcommands:
  `inspect`, `chart`, `rays`, `crossing`, `finalize` (refuses when
  `G_v0.3 = 0`), `replay` (byte-identity check), `verify`
  (certificate-chain audit). Golden fixtures in
  `tools/pse-traverse-horizon-cli/tests/fixtures/`. End-to-end CLI
  smoke tests cover every subcommand.
* GitHub Actions CI workflow (`.github/workflows/ci.yml`):
  fmt, clippy, build (Linux / macOS / Windows), test, doc, and a
  non-blocking `cargo audit` job.
* Dependabot configuration for weekly Cargo and monthly Actions
  updates, grouped by patch / minor.
* `CONTRIBUTING.md` with the determinism / replay ground rules,
  PR checklist, and adapter recipe.
* `SECURITY.md` with vulnerability-reporting flow, in-scope
  components, threat model, and primitive inventory.
* `CHANGELOG.md` (this file).

### Changed

* Workspace builds and tests now run warning-free under
  `RUSTFLAGS="-D warnings"`.
* `Cargo.lock` is now committed. The workspace ships binaries
  (`pse-cli`, `pse-demo`, `pse-traverse-cli`,
  `pse-traverse-horizon`, `pse-traverse-cognition`,
  `pse-eval-matrix`, `pse-bench-bbo`) where a reproducible build is
  a hard requirement.
* `cargo fmt --all` applied across the workspace; CI now enforces it.
* README and CHANGELOG document the eval matrix, cognition and
  horizon layers alongside the existing signature and dynamics
  layers; workspace test suite now reports **839 / 839** passing.

### Fixed

* Various clippy warnings: removed dead `crystal_count` in the
  Binance adapter test, replaced an `if_same_then_else` in the
  Metatron platonic classifier (the `is_iso ⇒ is_sub` collapse was
  redundant), used `std::f64::consts::PI` instead of an inline literal
  in `pse-traverse`, removed a placeholder `assert!(true)` test in
  `pse-cli` and replaced it with a real one, annotated the
  NaN-handling `!(hi > lo)` in the IsoForest baseline.

## [0.1.0] — 2026-05

Initial public iteration. Highlights below — see `README.md`
"What's new since the last README" for the full strand-letter log
(E through P, plus the signature and dynamics layers).

### Added (top-level)

* Engine architecture, strands E through N: real Mandorla / cascade,
  5D state, `CrystalAdapter`, resonance fingerprint query,
  resonance-landscape-aware TRITON.
* Operator algebra: `compose / dual / bridge / query / interpolate`.
* Falsification: `Shuffle`, `BlockBootstrap`, `PhaseRandomize`.
* AdaptiveCalibrator (P.3): rolling-history quantile thresholds,
  opt-in.
* `state.last_gate` diagnostic surface (P.2).
* PSE Traversal Agent v0.1 (`pse-traverse` crate): full
  ProblemSpec → FieldCube → DoFGraph → CollapsePlan → Candidate →
  GateReport → PSE-bridge pipeline, fail-closed.
* PSE-TRAVERSE-SIGNATURE-01: signature layer
  (`StructuralOperator` → `Signature` → `SignatureDiagnostics` →
  `SignatureGate`), `BlueprintSearch`, `NonDominatedFrontier`,
  `SearchLedger`, `SearchAutopilot`.
* PSE-TRAVERSE-DYNAMICS-01: morphodynamic tick engine
  (`CanonicalNumber`, `Hash256`/`StableId`, `BaseState`/`LiftedState`,
  `FieldAbsorber`, `GuidanceField`, `MorphodynamicCompressor`,
  `TransitionProof`, `DynamicGate`, `dynamic_tick`/`dynamic_run`).
* Ten domain adapters (Binance, ENTSO-E, Seismo, Weather, AirQuality,
  IoT, Syslog, Vitals, Tabular, ModelMon).
* Four tool binaries (`pse-bench-gt`, `pse-bench-bbo`, `pse-audit`,
  `pse-demo`).
* `pse-traverse-cli` with `inspect / plan / run / replay / search /
  dynamics` subcommands.
* `docs/POST_SYMBOLIC.md`, `docs/COMPLIANCE.md`.

[Unreleased]: https://github.com/lashsesh/pse/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lashsesh/pse/releases/tag/v0.1.0
