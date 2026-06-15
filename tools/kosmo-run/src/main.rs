//! `kosmo-run` — the agent runner.
//!
//! Drives the closed-loop Kosmocrates agent over a workspace: it runs the
//! pipeline, distills the ranked `ActionItem` queue, and asks an LLM backend
//! (Claude or any OpenAI-compatible endpoint such as Cerebras) to synthesize a
//! patch for each top action. By default it is **dry-run**: patches are
//! proposed, content-addressed and reported, but never written to disk.
//!
//! ```text
//! kosmo-run [OPTIONS] [PATH]
//!
//!   --provider <p>        claude | cerebras | mock | env   (default: env auto-detect)
//!   --model <m>           override the model slug
//!   --max-steps <n>       synthesize at most N actions      (default: 5)
//!   --min-confidence <p>  skip patches below P percent      (default: 50)
//!   --all                 enable all optional pipeline layers
//!   --capacity <n>        SystemCube D-density denominator  (default: 100)
//!   --json                emit the AgentRunReport as JSON
//!   --no-color            disable ANSI colour
//!   -h, --help            this help
//!
//! Keys are read from the environment: ANTHROPIC_API_KEY, CEREBRAS_API_KEY,
//! or KOSMO_LLM_API_KEY. The `mock` provider needs no key — use it to try the
//! loop offline.
//! ```

mod doors;
mod prose_bench;
mod pruefstand;
mod realize_bench;
mod reforge;
mod steward;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use kosmo_agent::{AgentOptions, AgentRunReport, AgentSession, CargoFoundryValidator};
use kosmo_core::{
    assess_wish, assess_wish_layered, Digest, FoundryCheckKind, FoundryCheckSpec,
    FoundryCommandPolicy, FoundryEnvironmentPolicy, FoundryExecutionOutcome, FoundryExecutionPlan,
    FoundryOutcome, FoundrySandboxKind, FoundrySandboxSpec, FoundryTimeoutPolicy, GateResult,
    KcubeArtifactKind, KcubeExportPolicy, KcubeWriteOutcome, ObservedTopology, ParseBackScanScope,
    PolicyProfile, PrecedenceOrder, RenderAnomaly, StagedClosureReport, StageStanding,
    StratumClosure, VentureSession, Wish, WishAssessment, WishClosureStatus, WishCube, WishFacet,
    WishFacetKind, Q16,
};
use kosmo_foundry::FoundryExecutor;
use kosmo_hyphae::codematrix::CodeMatrixFingerprint;
use kosmo_hyphae::{
    promotable, FacetBundleObservation, NormInjectionSpec, NormLearningConfig, SourceLanguage,
};
use kosmo_intent::{
    companion_suggestions, compile_venture, compile_wish, compile_wish_with_norms, CubeMeshReading,
    is_reserved_wish_word, observe_workspace_deep, observe_workspace_runtime,
    observe_workspace_runtime_diag, observe_workspace_service, observe_workspace_service_diag,
    observe_workspace_validated, parse_atelier_command, AtelierCommand, ChatIntent, DraftSlot,
    IndexSelection, IntentExtractor, KeywordIntentExtractor, NormCatalog, SuggestionSource,
    WishDraft, WishSession,
};
use kosmo_intent_llm::{LlmIntentExtractor, LlmWishRefiner};
use kosmo_kcube::KcubeExecutor;
use kosmo_parseback::{diff_snapshots, ParseBackExecutor, TopologySnapshot};
use kosmo_pipeline::{
    landscape_geometry, measure_landscape, propose_wishes, run_workspace_pipeline, ActionItem,
    ActionItemKind, IntegrationRunOptions, LandscapeStanding, WishProposal,
};
use kosmo_pse_bridge::MemoryRecall;
use kosmo_sandbox::{RunSpec, Sandbox};
use kosmo_store::NormStore;
use kosmo_synthesizer::consensus::{ConsensusConfig, ResonanceReading};
use kosmo_synthesizer::{
    ActionSynthesizer, ContextualSynthesizer, FacetScaffolder, FileChangeKind, GroundedSynthesizer,
    MockSynthesizer, SourceSnippet, SynthesisRequest,
};
use kosmo_synthesizer_llm::{LlmConfig, LlmSynthesizer, SwarmSynthesizer};
use pse_adapter_kosmo::LedgerRecall;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";

#[derive(Clone)]
struct Args {
    path: String,
    provider: String,
    model: Option<String>,
    max_steps: u32,
    min_confidence_pct: u32,
    all_layers: bool,
    capacity: u32,
    json: bool,
    color: bool,
    apply: bool,
    commit: bool,
    /// Wish mode: a plain-prose wish to measure the workspace against.
    wish: Option<String>,
    scaffold: bool,
    validated: bool,
    /// Render the wish as a layered hypercube whose strata fill from transparent
    /// to solid (Run 3). A modifier on `--wish`; with `--apply`, one render block
    /// per descent iteration — the 3-D-printer film.
    layers: bool,
    /// Drive the descent as a staged closure pipeline (Solve→Gate→Coagula),
    /// solidifying stratum by stratum bottom-up (Run 4). Implies `--layers`.
    staged: bool,
    /// Mesh the wish-cube against the workspace's real SystemCube D-Density
    /// (Run 8): the two gears — wish solidity vs. topology density — read
    /// side by side, surfacing over-fit (wish solid, topology sparse).
    mesh: bool,
    /// Opt out of the graduated default human render (Run 10): print only the
    /// terse verdict — the flat assessment (read-only) or the descent summary
    /// (`--apply`) — without the layered hypercube, Konus focus, or staged film.
    /// Human render only; the `--json` machine contract is unaffected.
    flat: bool,
    provider_set: bool,
    /// Path to a JSON file that the convergence trajectory is written to (and
    /// resumed from, if the file already exists and matches the current wish).
    wish_session: Option<String>,
    /// Run the empirical Prüfstand: descend a reference corpus of known-good
    /// (and deliberately broken) systems and report the fidelity.
    pruefstand: bool,
    /// Path to an Infinity-Ledger store: synthesis is then grounded in the
    /// anchored memory (Pfauenthron recall per action). Missing path = error.
    ledger: Option<String>,
    /// How many recalled crystals to attach per action (default 5).
    ground_top: u32,
    /// Consensus ensemble size: n perspectives per action (0 = off).
    swarm: u32,
    /// Landscape mode: map every substrate finding the wish vocabulary can
    /// express into a ranked wish-proposal landscape (read-only).
    landscape: bool,
    /// Adopt the top-N open proposals as a wish (descends with --apply).
    adopt: u32,
    /// Also compute the landscape's spectral shape: coupled-proposal
    /// clusters and articulation singularities (read-only, advisory).
    geometry: bool,
    /// Adopt cluster #i (1-based, from the geometry) as ONE wish
    /// (descends with --apply). 0 = unset.
    adopt_cluster: u32,
    /// Directory of the norm store (`norms.jsonl` + `observations.jsonl`).
    /// Arms promoted norms in the wish grammar and records realized descents
    /// as learning observations. Always caller-pathed, never a home dir.
    norms: Option<String>,
    /// Operator governance: inject a norm from a JSON spec file (arrives
    /// trigger-less; arm it with --promote-norm).
    inject_norm: Option<String>,
    /// Operator governance: arm a stored norm's prose trigger (hex norm id;
    /// requires --trigger).
    promote_norm: Option<String>,
    /// The trigger word for --promote-norm.
    trigger: Option<String>,
    /// Chat mode: a one-shot utterance, routed to an existing mode by a
    /// total intent extractor (keyword rules; LLM-first when a real
    /// provider is chosen, falling back to keywords on any failure).
    chat: Option<String>,
    /// Atelier mode: path of a durable WishDraft (JSON). Each invocation is
    /// one shaping round (--chat carries the utterance); "realize" descends.
    atelier: Option<String>,
    /// Venture mode: a JSON spec of dependent wish stages, orchestrated as
    /// one whole-system fabrication (writes only with --apply).
    venture: Option<String>,
    /// Reforge mode: the external-empiricism bench — re-forge known system
    /// tools from oracle-collected wish specs (requires a real provider).
    reforge: bool,
    /// Write the reforge report (content-addressed JSON) to this file.
    reforge_report: Option<String>,
    /// Realization benchmark: drive a curated behavioural corpus through the
    /// real generative loop and measure the realization rate (real provider).
    realize_bench: bool,
    /// Write the realization-benchmark report (content-addressed JSON) here.
    realize_bench_report: Option<String>,
    /// Service-synthesis smoke: drive ONE HTTP-service wish through the real
    /// loop — the artifact is started as a server and probed over HTTP.
    realize_service: bool,
    /// Prose→spec benchmark: run natural-language utterances through the intent
    /// extractor + compiler and score the facets against a hand-written truth.
    prose_bench: bool,
    /// Multi-crate smoke: drive a Run wish onto a two-crate workspace (a bin +
    /// a library it calls), realized across the crate boundary.
    realize_multicrate: bool,
    /// Doors mode: print this binary's complete docking surface — every
    /// door with inputs, governance and needs, content-addressed.
    doors: bool,
    /// Federate: comma-separated catalog JSON files (emitted by other
    /// surfaces' doors) merged into one ecosystem inventory. Each file is
    /// verified by content address before it is trusted. Implies --doors.
    doors_merge: Option<String>,
    /// Foundry door: run the loop's own allowlisted cargo checks (comma
    /// list: build,test,lint,typecheck) as a directed invocation.
    foundry: Option<String>,
    /// Witness door: execute the workspace binary with this comma-separated
    /// argv under the sandbox witness; print the content-addressed evidence.
    witness: Option<String>,
    /// ParseBack door: capture the workspace topology snapshot; with a
    /// baseline file, report severity-ranked drift against it.
    parseback: bool,
    /// Baseline file for --parseback (written once; delete to rebaseline).
    parseback_baseline: Option<String>,
    /// KCube door: export the workspace's SystemCube blueprint as a real,
    /// roundtrip-verified .kcube archive into this directory.
    kcube: Option<String>,
    /// Codematrix door: per-source 5D fingerprints + most resonant pairs
    /// (advisory — ranks, never gates).
    codematrix: bool,
    /// Steward mode: self-husbandry — survey the workspace's own landscape
    /// and, under --apply, descend the open chores inside the fence.
    steward: bool,
    /// The operator's fence: comma-separated facet classes the steward may
    /// husband (e.g. "doc,test"). Nothing is fenced by default.
    fence: Option<String>,
    /// Cap the steward's chore list per run (0 = uncapped).
    steward_max: u32,
    /// Write the steward report (content-addressed JSON) to this file.
    steward_report: Option<String>,
    /// Durable venture progress (JSON): written after every stage, resumed
    /// on the next invocation. The venture identity must match.
    venture_session: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            path: ".".into(),
            provider: "env".into(),
            model: None,
            max_steps: 5,
            min_confidence_pct: 50,
            all_layers: false,
            capacity: 100,
            json: false,
            color: true,
            apply: false,
            commit: false,
            wish: None,
            scaffold: false,
            validated: false,
            layers: false,
            staged: false,
            mesh: false,
            flat: false,
            provider_set: false,
            wish_session: None,
            pruefstand: false,
            ledger: None,
            ground_top: 5,
            swarm: 0,
            landscape: false,
            adopt: 0,
            geometry: false,
            adopt_cluster: 0,
            norms: None,
            inject_norm: None,
            promote_norm: None,
            trigger: None,
            chat: None,
            atelier: None,
            venture: None,
            venture_session: None,
            reforge: false,
            reforge_report: None,
            realize_bench: false,
            realize_bench_report: None,
            realize_service: false,
            prose_bench: false,
            realize_multicrate: false,
            doors: false,
            doors_merge: None,
            foundry: None,
            witness: None,
            parseback: false,
            parseback_baseline: None,
            kcube: None,
            codematrix: false,
            steward: false,
            fence: None,
            steward_max: 0,
            steward_report: None,
        }
    }
}

fn print_help() {
    println!(
        "kosmo-run — Kosmocrates agent runner\n\n\
USAGE:\n    kosmo-run [OPTIONS] [PATH]\n\n\
OPTIONS:\n\
    --provider <p>        claude | cerebras | mock | env  (default: env auto-detect)\n\
    --model <m>           override the model slug\n\
    --max-steps <n>       synthesize at most N actions     (default: 5)\n\
    --min-confidence <p>  skip patches below P percent      (default: 50)\n\
    --swarm <n>           consensus ensemble: n lensed perspectives per\n\
                          action (2-6), scored by structural agreement; a\n\
                          divergent ensemble lands below the confidence\n\
                          gate instead of being emitted. Needs a real\n\
                          provider (not mock).\n\
    --all                 enable all optional pipeline layers\n\
    --capacity <n>        SystemCube D-density denominator  (default: 100)\n\
    --apply               WRITE validated patches to the workspace (cargo\n\
                          check+test each; rolls back any that fail). Default\n\
                          is dry-run: nothing is written.\n\
    --commit              After each accepted patch, run git add -A && git commit\n\
                          (requires --apply; each patch lands as its own commit).\n\
\n\
  WISH MODE (deterministic, offline — no LLM, no key):\n\
    --wish \"<prose>\"      compile a plain-language wish and measure the\n\
                          workspace against it; prints met/missing facets\n\
    --validated           observe green tests too (runs the suite; heavier)\n\
                          (run probes accept a tail budget: \"hi=>out~hi,ms<50\"\n\
                          — the program must answer AND stay under 50ms)\n\
    --scaffold            also print the file changes that would close the gap\n\
    --layers              render the wish as a hypercube: 5 strata whose opacity\n\
                          fills from transparent to solid (Run 3; a --wish modifier)\n\
    --staged              descend as a staged closure pipeline (Solve\u{2192}Gate\u{2192}\n\
                          Coagula), solidifying stratum by stratum bottom-up\n\
                          (Run 4; implies --layers)\n\
    --mesh                read the two gears: wish solidity vs. the workspace's\n\
                          observed structural density \u{2014} surfaces over-fit (Run 8)\n\
    --flat                opt out of the default cube view: print only the terse\n\
                          verdict, no layered hypercube/Konus/staged film (Run 10)\n\
    --wish-session <path> write the convergence trajectory as JSON to <path>;\n\
                          if <path> already exists and matches the wish, resume\n\
                          from the prior session (auditable, replayable)\n\
\n\
    (wish + --apply descends: scaffold \u{2192} write \u{2192} re-observe until\n\
     realized; add --provider to let the LLM build facets the scaffolder can't)\n\
\n\
    --json                emit the report as JSON\n\
    --no-color            disable ANSI colour\n\
    -h, --help            show this help\n\
\n\
  PR\u{00dc}FSTAND MODE (empirical fidelity harness):\n\
    --pruefstand          descend a built-in reference corpus of known-good and\n\
                          deliberately broken systems; report fidelity and exit\n\
                          non-zero if any verdict is wrong (--validated runs the\n\
                          behavioural scenarios' suites too)\n\
\n\
  LANDSCAPE MODE (the findings become the wish menu):\n\
    --landscape           run the substrate pipeline and project every finding\n\
                          the wish vocabulary can express into a ranked\n\
                          wish-proposal landscape: met/open/beyond-vocabulary,\n\
                          each with severity and provenance. Read-only.\n\
    --adopt <n>           adopt the top-N open proposals as ONE wish (weighted\n\
                          by severity). Without --apply: prints the wish.\n\
                          With --apply: descends it (deterministic scaffolds).\n\
    --geometry            also compute the landscape's spectral shape:\n\
                          conductance-bounded clusters of coupled proposals\n\
                          (subject 45 / kind 30 / severity-proximity 25) and\n\
                          the singular proposals whose removal disconnects\n\
                          the landscape. Advisory; the landscape itself is\n\
                          unchanged.\n\
    --adopt-cluster <i>   adopt geometry cluster #i (1-based) as ONE coherent\n\
                          wish instead of a blind top-k. Without --apply:\n\
                          prints it. With --apply: descends it.\n\
\n\
  MEMORY (anchored knowledge from the promotion ledger):\n\
    --ledger <path>       ground every synthesis in the Infinity-Ledger memory:\n\
                          per action, the top recalled crystals (Pfauenthron\n\
                          D = \u{3c8}\u{b7}\u{3c1}\u{b7}\u{3c9}) ride along as advisory context and each\n\
                          patch cites the crystals it received. Read-only; a\n\
                          missing ledger is a hard error, never a silent skip.\n\
    --ground-top <n>      recalled crystals per action (default: 5)\n\
\n\
  NORMS (learned archetypes — the catalog starts empty, always):\n\
    --norms <dir>         norm store directory. In wish mode: promoted norm\n\
                          triggers expand in the prose grammar, and every\n\
                          realized descent (with --apply) is recorded as a\n\
                          learning observation; a facet shape realized \u{2265}3x\n\
                          across \u{2265}2 workspaces becomes a stored, UNARMED norm.\n\
    --inject-norm <file>  operator governance: add a norm from a JSON spec\n\
                          (facet templates over the {{name}} placeholder only —\n\
                          path-like templates are rejected). Arrives unarmed.\n\
    --promote-norm <id> --trigger <word>\n\
                          operator governance: arm a stored norm's prose\n\
                          trigger. Reserved grammar words are refused.\n\
\n\
  CHAT (one-shot front door — no REPL):\n\
    --chat \"<utterance>\"  route a plain utterance to an existing mode:\n\
                          wish / descend / landscape (+geometry) / adopt /\n\
                          adopt-cluster / status / norm governance hints.\n\
                          Routing is TOTAL: anything unrecognized becomes a\n\
                          measurable wish, never a template. Deterministic\n\
                          keyword rules by default; with a real --provider\n\
                          the model routes first and falls back to keywords\n\
                          on any failure. --apply/--ledger/--norms compose.\n\
\n\
  ATELIER (shape a wish over rounds before realizing it):\n\
    --atelier <draft.json>  open (or create) a durable wish draft; each\n\
                          invocation is ONE round, the utterance comes via\n\
                          --chat. Your dictated facets enter the wish;\n\
                          machine proposals (companions; model suggestions\n\
                          with a real --provider) stay PENDING until you\n\
                          accept them — the machine proposes, you dispose.\n\
                          Rounds: --chat \"<prose>\" adds facets;\n\
                          \"accept 3,4\" / \"reject 5\" / \"drop 2\" are\n\
                          verdicts on the numbered list; \"realize\" freezes\n\
                          the wish and descends (writes only with --apply).\n\
\n\
  VENTURE (a whole system of dependent wishes):\n\
    --venture <spec.json> orchestrate a venture: stages carry wishes as\n\
                          prose (promoted norm triggers work), \"after\"\n\
                          lists prerequisite stage indices. Stages descend\n\
                          in dependency order, each under full gates; a\n\
                          failed stage blocks its dependents. Read-only\n\
                          preview without --apply.\n\
    --venture-session <f> durable progress: written after every stage,\n\
                          resumed on the next run (identity-checked).\n\
\n\
  REFORGE (external empiricism — requires a real provider):\n\
    --reforge             re-forge known system tools (expr, factor,\n\
                          basename) from wish specs whose expectations are\n\
                          collected from the REAL binaries on this machine\n\
                          at run time; the forged tool is executed and must\n\
                          answer like the oracle, within a time budget.\n\
                          One command, reproducible by anyone with a key.\n\
    --reforge-report <f>  write the content-addressed JSON report to <f>.\n\
\n\
  REALIZE-BENCH (does the generative loop actually work? — requires a real provider):\n\
    --realize-bench       drive a curated corpus of behavioural wishes through\n\
                          the REAL provider descent and measure the fraction\n\
                          that reach REALIZED — judged by executing the forged\n\
                          program, never by the model's word. Provider-agnostic\n\
                          (cloud API or a local model via KOSMO_LLM_BASE_URL);\n\
                          reports realization rate, iterations and token cost.\n\
                          A measurement, not a gate (always exits 0).\n\
    --realize-bench-report <f>  write the content-addressed JSON report to <f>.\n\
    --realize-service     drive ONE HTTP-service wish through the same loop: the\n\
                          artifact is started as a SERVER and probed over HTTP\n\
                          (the served witness), proving the loop realizes more\n\
                          than CLIs. Real provider; a measurement, exits 0.\n\
    --prose-bench         measure the prose->spec front door: natural-language\n\
                          utterances -> intent extractor + compiler -> facets,\n\
                          scored against ground truth. Offline (keyword router),\n\
                          or via the LLM extractor with --provider. Exits 0.\n\
    --realize-multicrate  drive ONE Run wish onto a two-crate workspace (a bin +\n\
                          the library it calls): the logic is realized in the\n\
                          engine crate, reached across the boundary, judged by\n\
                          execution. Real provider; a measurement, exits 0.\n\
\n\
  STEWARD (self-husbandry — the machine proposes, the operator disposes):\n\
    --steward             survey the workspace's own wish landscape and name\n\
                          the open chores inside the fence. Read-only without\n\
                          --apply; with --apply, each fenced chore descends as\n\
                          its own evidence-bound wish (deterministic scaffolds;\n\
                          --provider/--ledger/--norms compose as in wish mode).\n\
    --fence <classes>     the facet classes the steward may husband, comma-\n\
                          separated (e.g. doc,test). NOTHING is fenced by\n\
                          default — husbandry without a fence is refused, and\n\
                          widening the fence is an explicit operator act.\n\
    --steward-max <n>     cap the chore list per run (default: uncapped)\n\
    --steward-report <f>  write the content-addressed JSON report to <f>\n\
                          (host-path-free; fit for an unattended nightly run).\n\
\n\
  DOORS (the binary's self-description):\n\
    --doors               print this binary's complete docking surface: every\n\
                          door with its inputs, write power and needs —\n\
                          content-addressed, deterministic, pinned by test\n\
                          against the parser (--json for the machine form).\n\
    --doors-merge <files> federate: merge other surfaces' emitted catalogs\n\
                          (comma-separated JSON files, e.g. from\n\
                          GET /api/doors) into one ecosystem inventory; each\n\
                          file is verified by content address — a catalog\n\
                          that does not recompute is refused.\n\
\n\
  ORGANS (directed doors over the substrate — one door per run):\n\
    --foundry <kinds>     run the loop's own gate executor alone: allowlisted\n\
                          cargo checks (build,test,lint,typecheck), worst-wins\n\
                          outcome, content-addressed evidence. Exit 6 on fail.\n\
    --witness \"<argv>\"    execute the workspace binary once under the sandbox\n\
                          witness (60s budget, cwd-confined) and print the\n\
                          content-addressed evidence. Exit 7 unless clean.\n\
    --parseback           capture the workspace topology snapshot (crates,\n\
                          files, dependency edges — content-addressed);\n\
    --parseback-baseline <f>  persist the snapshot once and report severity-\n\
                          ranked drift on later runs (delete to rebaseline).\n\
    --kcube <dir>         export the workspace's SystemCube blueprint as a\n\
                          real, roundtrip-verified .kcube archive (refuses\n\
                          silent overwrite). Exit 8 if not written.\n\
    --codematrix          per-source 5D fingerprints (relationality, cohesion,\n\
                          topology, symmetry, entropy) + most resonant pairs.\n\
                          Advisory: ranks, never gates.\n\n\
ENVIRONMENT:\n\
    ANTHROPIC_API_KEY / CEREBRAS_API_KEY / KOSMO_LLM_API_KEY   provider key\n\
    ANTHROPIC_MODEL / CEREBRAS_MODEL / KOSMO_LLM_MODEL         model override\n\
    KOSMO_LLM_PROVIDER, KOSMO_LLM_BASE_URL                     custom endpoint\n\n\
EXAMPLES:\n\
    CEREBRAS_API_KEY=sk-... kosmo-run --provider cerebras .\n\
    ANTHROPIC_API_KEY=sk-... kosmo-run --provider claude --max-steps 3 ./crate\n\
    kosmo-run --provider mock --all .        # offline, no key required\n\
    kosmo-run --provider mock --apply --commit .   # apply + commit each patch\n\
    kosmo-run --wish \"a crate kosmo-api and a function handle\" --scaffold .   # offline"
    );
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut args = Args::default();
    let mut argv = std::env::args().skip(1);
    let mut saw_path = false;

    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(None);
            }
            "--provider" => {
                args.provider = argv
                    .next()
                    .ok_or("--provider needs a value")?
                    .to_lowercase();
                args.provider_set = true;
            }
            "--model" => {
                args.model = Some(argv.next().ok_or("--model needs a value")?);
            }
            "--max-steps" => {
                args.max_steps = argv
                    .next()
                    .ok_or("--max-steps needs a value")?
                    .parse()
                    .map_err(|_| "--max-steps must be a number")?;
            }
            "--min-confidence" => {
                args.min_confidence_pct = argv
                    .next()
                    .ok_or("--min-confidence needs a value")?
                    .parse()
                    .map_err(|_| "--min-confidence must be a number 0-100")?;
            }
            "--capacity" => {
                args.capacity = argv
                    .next()
                    .ok_or("--capacity needs a value")?
                    .parse()
                    .map_err(|_| "--capacity must be a number")?;
            }
            "--all" => args.all_layers = true,
            "--apply" => args.apply = true,
            "--commit" => args.commit = true,
            "--wish" => {
                args.wish = Some(argv.next().ok_or("--wish needs a value")?);
            }
            "--scaffold" => args.scaffold = true,
            "--validated" => args.validated = true,
            "--layers" => args.layers = true,
            "--staged" => {
                args.staged = true;
                args.layers = true; // staged descent always renders its strata
            }
            "--mesh" => args.mesh = true,
            "--flat" => args.flat = true,
            "--wish-session" => {
                args.wish_session = Some(argv.next().ok_or("--wish-session needs a value")?);
            }
            "--pruefstand" | "--testbench" => args.pruefstand = true,
            "--swarm" => {
                args.swarm = argv
                    .next()
                    .ok_or("--swarm needs a value")?
                    .parse()
                    .map_err(|_| "--swarm must be a number (2-6)")?;
            }
            "--landscape" => args.landscape = true,
            "--adopt" => {
                args.adopt = argv
                    .next()
                    .ok_or("--adopt needs a value")?
                    .parse()
                    .map_err(|_| "--adopt must be a number")?;
            }
            "--geometry" => args.geometry = true,
            "--adopt-cluster" => {
                args.adopt_cluster = argv
                    .next()
                    .ok_or("--adopt-cluster needs a value")?
                    .parse()
                    .map_err(|_| "--adopt-cluster must be a number")?;
                if args.adopt_cluster == 0 {
                    return Err("--adopt-cluster takes a 1-based cluster index".into());
                }
            }
            "--ledger" => {
                args.ledger = Some(argv.next().ok_or("--ledger needs a value")?);
            }
            "--norms" => {
                args.norms = Some(argv.next().ok_or("--norms needs a directory")?);
            }
            "--inject-norm" => {
                args.inject_norm = Some(argv.next().ok_or("--inject-norm needs a JSON file")?);
            }
            "--promote-norm" => {
                args.promote_norm = Some(argv.next().ok_or("--promote-norm needs a norm id")?);
            }
            "--trigger" => {
                args.trigger = Some(argv.next().ok_or("--trigger needs a word")?);
            }
            "--chat" => {
                args.chat = Some(argv.next().ok_or("--chat needs an utterance")?);
            }
            "--atelier" => {
                args.atelier = Some(argv.next().ok_or("--atelier needs a draft file path")?);
            }
            "--venture" => {
                args.venture = Some(argv.next().ok_or("--venture needs a spec file path")?);
            }
            "--reforge" => args.reforge = true,
            "--reforge-report" => {
                args.reforge_report =
                    Some(argv.next().ok_or("--reforge-report needs a file path")?);
            }
            "--realize-bench" => args.realize_bench = true,
            "--realize-service" => args.realize_service = true,
            "--prose-bench" => args.prose_bench = true,
            "--realize-multicrate" => args.realize_multicrate = true,
            "--realize-bench-report" => {
                args.realize_bench_report = Some(
                    argv.next()
                        .ok_or("--realize-bench-report needs a file path")?,
                );
            }
            "--venture-session" => {
                args.venture_session =
                    Some(argv.next().ok_or("--venture-session needs a file path")?);
            }
            "--doors" => args.doors = true,
            "--doors-merge" => {
                args.doors_merge = Some(
                    argv.next()
                        .ok_or("--doors-merge needs catalog files (comma-separated)")?,
                );
            }
            "--foundry" => {
                args.foundry = Some(
                    argv.next()
                        .ok_or("--foundry needs check kinds (e.g. build,test)")?,
                );
            }
            "--witness" => {
                args.witness = Some(
                    argv.next()
                        .ok_or("--witness needs a comma-separated argv")?,
                );
            }
            "--parseback" => args.parseback = true,
            "--parseback-baseline" => {
                args.parseback_baseline = Some(
                    argv.next()
                        .ok_or("--parseback-baseline needs a file path")?,
                );
            }
            "--kcube" => {
                args.kcube = Some(argv.next().ok_or("--kcube needs an output directory")?);
            }
            "--codematrix" => args.codematrix = true,
            "--steward" => args.steward = true,
            "--fence" => {
                args.fence = Some(argv.next().ok_or("--fence needs facet classes")?);
            }
            "--steward-max" => {
                args.steward_max = argv
                    .next()
                    .ok_or("--steward-max needs a value")?
                    .parse()
                    .map_err(|_| "--steward-max must be a number")?;
            }
            "--steward-report" => {
                args.steward_report =
                    Some(argv.next().ok_or("--steward-report needs a file path")?);
            }
            "--ground-top" => {
                args.ground_top = argv
                    .next()
                    .ok_or("--ground-top needs a value")?
                    .parse()
                    .map_err(|_| "--ground-top must be a number")?;
            }
            "--json" => args.json = true,
            "--no-color" => args.color = false,
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            path => {
                args.path = path.to_string();
                saw_path = true;
            }
        }
    }
    let _ = saw_path;
    Ok(Some(args))
}

fn build_synthesizer(args: &Args) -> Result<Arc<dyn ActionSynthesizer>, String> {
    let model = args.model.clone();
    let apply_model = |c: LlmConfig| -> LlmConfig {
        match &model {
            Some(m) => c.with_model(m.clone()),
            None => c,
        }
    };

    let swarmed = |inner: LlmSynthesizer| -> Arc<dyn ActionSynthesizer> {
        if args.swarm > 0 {
            Arc::new(SwarmSynthesizer::new(Arc::new(inner), args.swarm as usize))
        } else {
            Arc::new(inner)
        }
    };

    match args.provider.as_str() {
        "mock" => {
            if args.swarm > 0 {
                return Err(
                    "--swarm needs a real provider (claude | cerebras | env): the mock \
                     synthesizer answers identically n times, which is consensus theater"
                        .to_string(),
                );
            }
            Ok(Arc::new(MockSynthesizer::confident()))
        }
        "claude" | "anthropic" => {
            let key = env_key(&["ANTHROPIC_API_KEY", "KOSMO_LLM_API_KEY"]).ok_or_else(|| {
                "provider=claude requires ANTHROPIC_API_KEY (or KOSMO_LLM_API_KEY)".to_string()
            })?;
            let cfg = apply_model(LlmConfig::claude(key));
            Ok(swarmed(
                LlmSynthesizer::new(cfg).map_err(|e| e.to_string())?,
            ))
        }
        "cerebras" => {
            let key = env_key(&["CEREBRAS_API_KEY", "KOSMO_LLM_API_KEY"]).ok_or_else(|| {
                "provider=cerebras requires CEREBRAS_API_KEY (or KOSMO_LLM_API_KEY)".to_string()
            })?;
            let cfg = apply_model(LlmConfig::cerebras(key));
            Ok(swarmed(
                LlmSynthesizer::new(cfg).map_err(|e| e.to_string())?,
            ))
        }
        "env" | "auto" | "" => {
            // First contact without a key is an invitation, not a dead end:
            // most of the system runs offline.
            let synth = LlmSynthesizer::from_env().map_err(|e| {
                format!(
                    "{e}\n\nno LLM provider is required to start — these run offline:\n\
                     \x20 kosmo-run --landscape .            map what the workspace is missing\n\
                     \x20 kosmo-run --wish \"a module x\" .    measure a wish (add --apply to build)\n\
                     \x20 kosmo-run --chat \"status\" .        ask in plain words\n\
                     \x20 kosmo-run --atelier wish.json --chat \"a module x\" .   shape a wish over rounds"
                )
            })?;
            Ok(swarmed(synth))
        }
        other => Err(format!(
            "unknown provider '{other}' (expected claude | cerebras | mock | env)"
        )),
    }
}

/// Arm an optional LLM fallback with the anchored memory: with `--ledger`,
/// every fallback request is grounded in recalled knowledge before the LLM
/// sees it ([`GroundedSynthesizer`]). The deterministic scaffolder needs no
/// memory — it builds exactly. Shared by wish mode and landscape adoption.
fn arm_fallback(
    args: &Args,
    inner: Option<Arc<dyn ActionSynthesizer>>,
) -> Result<Option<Arc<dyn ActionSynthesizer>>, String> {
    Ok(match (inner, open_recall(args)?) {
        (Some(inner), Some(recall)) => Some(Arc::new(GroundedSynthesizer::new(
            inner,
            recall,
            args.ground_top as usize,
        ))),
        (None, Some(recall)) => {
            eprintln!(
                "kosmo-run: --ledger {} attached but no LLM fallback is active \
                 (add --provider); the deterministic scaffolder runs memory-free",
                recall.source()
            );
            None
        }
        (inner, None) => inner,
    })
}

/// The optional LLM fallback for facet realization: built only when a
/// provider was explicitly chosen, armed with memory via [`arm_fallback`],
/// and wrapped in a descent context (`ContextualSynthesizer`) so every
/// facet's prompt carries the symbols earlier facets created and every
/// patch passes the Mikro/Meso gates. Deterministic-only when `None`.
fn wish_fallback(args: &Args) -> Result<Option<Arc<dyn ActionSynthesizer>>, String> {
    Ok(
        bare_wish_fallback(args)?.map(|inner| -> Arc<dyn ActionSynthesizer> {
            Arc::new(ContextualSynthesizer::new(inner, args.path.clone()))
        }),
    )
}

fn bare_wish_fallback(args: &Args) -> Result<Option<Arc<dyn ActionSynthesizer>>, String> {
    let inner = if args.provider_set {
        match build_synthesizer(args) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("kosmo-run: LLM fallback disabled ({e}); deterministic only");
                None
            }
        }
    } else {
        None
    };
    arm_fallback(args, inner)
}

/// Open the anchored memory when `--ledger` was given. Hard error on a
/// missing or unreadable store — memory explicitly asked for must exist
/// (the read-only contract of `LedgerRecall::open`).
fn open_recall(args: &Args) -> Result<Option<Arc<dyn MemoryRecall>>, String> {
    match &args.ledger {
        Some(p) => {
            let recall = LedgerRecall::open(Path::new(p))?;
            Ok(Some(Arc::new(recall)))
        }
        None => Ok(None),
    }
}

fn env_key(names: &[&str]) -> Option<String> {
    for n in names {
        if let Ok(v) = std::env::var(n) {
            if !v.trim().is_empty() {
                return Some(v);
            }
        }
    }
    None
}

fn pct(q: Q16) -> i64 {
    // Q16::ONE.raw() == 65536; map to an integer percentage (no floats).
    q.raw() * 100 / Q16::ONE.raw()
}

fn kind_label(kind: &ActionItemKind) -> &'static str {
    match kind {
        ActionItemKind::FillVoid { .. } => "FillVoid",
        ActionItemKind::RepairTopology { .. } => "RepairTopology",
        ActionItemKind::PromoteToPse { .. } => "PromoteToPse",
        ActionItemKind::ReviewCrystal { .. } => "ReviewCrystal",
        ActionItemKind::ApplyNorm { .. } => "ApplyNorm",
        ActionItemKind::RealizeWishFacet { .. } => "RealizeWishFacet",
    }
}

fn gate_text(gate: &GateResult) -> String {
    match gate {
        GateResult::Pass => "PASS".into(),
        GateResult::Warn { message } => format!("WARN ({message})"),
        GateResult::Reject { reason } => format!("REJECT ({reason})"),
        GateResult::Downgrade { from, to, reason } => {
            format!("DOWNGRADE {from:?}→{to:?} ({reason})")
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.replace('\n', " ");
    if s.chars().count() <= max {
        s
    } else {
        let kept: String = s.chars().take(max).collect();
        format!("{kept}…")
    }
}

fn render_text(report: &AgentRunReport, synth_name: &str, color: bool) {
    let c = |code: &'static str| if color { code } else { "" };

    println!(
        "{}{}Kosmocrates agent run{}  {}{}{}",
        c(BOLD),
        c(CYAN),
        c(RESET),
        c(DIM),
        report.workspace_path,
        c(RESET)
    );
    let gate = gate_text(&report.pipeline_gate);
    let gate_color = match report.pipeline_gate {
        GateResult::Pass => c(GREEN),
        GateResult::Warn { .. } => c(YELLOW),
        _ => c(RED),
    };
    println!(
        "  synthesizer {}{}{}   pipeline gate {}{}{}   run #{}",
        c(CYAN),
        synth_name,
        c(RESET),
        gate_color,
        gate,
        c(RESET),
        report.pipeline_run_number
    );
    println!(
        "  {} actions available · {} synthesized · {} skipped (low confidence) · {} materialized · {} lines proposed",
        report.total_actions_available,
        report.steps_synthesized,
        report.steps_skipped_low_confidence,
        report.steps_materialized,
        report.total_lines_proposed
    );

    if report.steps.is_empty() {
        println!(
            "\n  {}no patches synthesized above the confidence threshold{}",
            c(DIM),
            c(RESET)
        );
        return;
    }

    for step in &report.steps {
        let conf = pct(step.synthesis.confidence);
        println!(
            "\n{}#{}{} {}{}{}  {}{}%{} confidence  ·  {} file(s), {} line(s)  ·  {} tok",
            c(BOLD),
            step.step_number,
            c(RESET),
            c(YELLOW),
            kind_label(&step.action.kind),
            c(RESET),
            c(GREEN),
            conf,
            c(RESET),
            step.synthesis.patch.file_changes.len(),
            step.synthesis.patch.total_lines(),
            step.synthesis.tokens_used,
        );
        println!("    action  {}", truncate(&step.action.description, 100));
        println!("    why     {}", truncate(&step.synthesis.rationale, 100));
        if !step.synthesis.grounding_crystal_ids.is_empty() {
            println!(
                "    memory  {}grounded by {} anchored crystal(s): {}{}",
                c(CYAN),
                step.synthesis.grounding_crystal_ids.len(),
                truncate(&step.synthesis.grounding_crystal_ids.join(", "), 80),
                c(RESET)
            );
        }
        if let Some(hint) = &step.synthesis.test_hint {
            println!("    verify  {}{}{}", c(DIM), hint, c(RESET));
        }
        for fc in &step.synthesis.patch.file_changes {
            println!(
                "      {}{:?}{} {}  ({} ln)",
                c(CYAN),
                fc.kind,
                c(RESET),
                fc.path.to_string_lossy(),
                fc.line_count()
            );
        }
        let mat = match &step.materialization {
            Some(m) if m.applied => "applied",
            Some(_) => "dry-run (not written)",
            None => "not materialized",
        };
        println!("    status  {}{}{}", c(DIM), mat, c(RESET));
        if let Some(ref attempt) = step.materialization {
            if let Some(ref sha) = attempt.commit_sha {
                println!(
                    "    commit  {}{}{}",
                    c(CYAN),
                    &sha[..sha.len().min(12)],
                    c(RESET)
                );
            }
        }
    }
}

// ─── Session persistence ─────────────────────────────────────────────────────

/// Load a prior [`WishSession`] from `path`, but only if it belongs to `wish`.
/// Returns `None` on any I/O or parse failure — the caller falls back to a fresh
/// session, which is always the safe choice.
fn load_prior_session(path: &str, wish: &Wish) -> Option<WishSession> {
    let text = fs::read_to_string(path).ok()?;
    let session: WishSession = serde_json::from_str(&text).ok()?;
    if session.wish().id == wish.id {
        Some(session)
    } else {
        None // different wish — start fresh, don't silently merge trajectories
    }
}

/// Serialize `session` as pretty-printed JSON and write it to `path`.
fn save_session(path: &str, session: &WishSession) -> Result<(), String> {
    let json = serde_json::to_string_pretty(session)
        .map_err(|e| format!("failed to serialize session: {e}"))?;
    fs::write(path, json).map_err(|e| format!("failed to write session to {path}: {e}"))?;
    Ok(())
}

// ─── Wish mode ──────────────────────────────────────────────────────────────

/// Whether a wish requires validated observation (running the suite). True iff
/// it carries a [`WishFacetKind::Behavior`] facet — those are satisfied only by
/// a passing spec-test, never by lexical presence.
fn wish_needs_validation(wish: &Wish) -> bool {
    wish.predicates
        .iter()
        .any(|p| p.facet.kind == WishFacetKind::Behavior)
}

/// Whether a wish requires runtime observation (executing the built artifact).
/// True iff it carries a [`WishFacetKind::Run`] facet — those are satisfied only
/// by running the program under the sandbox, never by reading source.
fn wish_needs_runtime(wish: &Wish) -> bool {
    wish.predicates
        .iter()
        .any(|p| p.facet.kind == WishFacetKind::Run)
}

/// Whether a wish requires service observation (starting a server and probing
/// it). True iff it carries a [`WishFacetKind::Service`] facet.
fn wish_needs_service(wish: &Wish) -> bool {
    wish.predicates
        .iter()
        .any(|p| p.facet.kind == WishFacetKind::Service)
}

/// Deterministic, offline front door: compile a prose wish, observe the
/// workspace, and report the distance to the wish (which facets are present,
/// which are missing). With `--scaffold`, also print the changes that would
/// close the gap. No LLM and no key required.
/// Landscape mode: run the substrate pipeline, project its findings into the
/// wish vocabulary ([`propose_wishes`]), measure every proposal against the
/// observed topology, and render the ranked landscape. `--adopt <n>` turns
/// the top open proposals into ONE severity-weighted wish — printed by
/// default, descended under `--apply` (deterministic scaffolds only).
fn run_landscape_mode(args: &Args) -> Result<ExitCode, String> {
    if args.adopt > 0 && args.adopt_cluster > 0 {
        return Err("--adopt and --adopt-cluster are mutually exclusive (one wish per run)".into());
    }
    let policy = PolicyProfile::default_report_only();
    let options = if args.all_layers {
        IntegrationRunOptions::all_layers(args.capacity)
    } else {
        IntegrationRunOptions::report_only()
    };
    let report = run_workspace_pipeline(&args.path, &options, &policy)
        .map_err(|e| format!("pipeline failed on {}: {e}", args.path))?;
    let voids = &report.hyphae_result.host_cube.void_map.voids;
    let landscape = propose_wishes(voids);

    // Measure: which proposals are already met, which targets can the wish
    // world even see? (A non-Rust module is honest residue, not a stalling
    // wish.) Observation needs a cargo workspace — fail-soft to "unmeasured".
    let observed = observe_workspace_deep(&args.path).ok();
    let standing = measure_landscape(&landscape, observed.as_ref());

    // Geometry (opt-in): the spectral shape of the OPEN landscape — coupled
    // clusters and articulation singularities. Strictly additive: without
    // --geometry / --adopt-cluster nothing below changes.
    let open_proposals: Vec<WishProposal> = landscape
        .proposals
        .iter()
        .zip(&standing)
        .filter(|(_, s)| **s == LandscapeStanding::Open)
        .map(|(p, _)| p.clone())
        .collect();
    let geometry = (args.geometry || args.adopt_cluster > 0)
        .then(|| landscape_geometry(&open_proposals, GEOMETRY_MAX_CLUSTERS));

    let met = standing
        .iter()
        .filter(|s| **s == LandscapeStanding::Met)
        .count();
    let open = standing
        .iter()
        .filter(|s| **s == LandscapeStanding::Open)
        .count();
    let invisible = standing
        .iter()
        .filter(|s| **s == LandscapeStanding::BeyondObservation)
        .count();

    if args.json {
        let rows: Vec<serde_json::Value> = landscape
            .proposals
            .iter()
            .zip(&standing)
            .map(|(p, s)| {
                serde_json::json!({
                    "facet_kind": format!("{:?}", p.facet.kind),
                    "facet_key": p.facet.key,
                    "severity": format!("{:?}", p.severity),
                    "subject": p.subject,
                    "rationale": p.rationale,
                    "standing": s.label(),
                })
            })
            .collect();
        let mut doc = serde_json::json!({
            "path": args.path,
            "report_id": report.report_id.to_hex(),
            "proposals": rows,
            "met": met,
            "open": open,
            "beyond_observation": invisible,
            "beyond_vocabulary": landscape.unmapped.len(),
        });
        if let Some(geo) = &geometry {
            let facet_label = |i: usize| {
                format!(
                    "{:?} {}",
                    open_proposals[i].facet.kind, open_proposals[i].facet.key
                )
            };
            doc["geometry"] = serde_json::json!({
                "clusters": geo.clusters.iter().map(|cl| serde_json::json!({
                    "facets": cl.members.iter().map(|&i| facet_label(i)).collect::<Vec<_>>(),
                    "subjects": cl.subjects,
                    "severity_mass": format!("{:.2}", cl.severity_mass.to_f64()),
                })).collect::<Vec<_>>(),
                "singular": geo.singular.iter().map(|s| serde_json::json!({
                    "facet": facet_label(s.index),
                    "coupling_mass": format!("{:.2}", s.coupling_mass.to_f64()),
                })).collect::<Vec<_>>(),
            });
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?
        );
    } else {
        let c = |code: &'static str| if args.color { code } else { "" };
        println!(
            "{}Kosmocrates wish landscape{}  {}{}{}",
            c(BOLD),
            c(RESET),
            c(DIM),
            args.path,
            c(RESET)
        );
        println!(
            "  {} proposal(s) · {}{} met{} · {}{} open{} · {} beyond observation · {} beyond vocabulary",
            landscape.proposals.len(),
            c(GREEN),
            met,
            c(RESET),
            c(YELLOW),
            open,
            c(RESET),
            invisible,
            landscape.unmapped.len()
        );
        for (p, s) in landscape.proposals.iter().zip(&standing) {
            let (mark, color) = match s {
                LandscapeStanding::Met => ("\u{2713}", GREEN),
                LandscapeStanding::Open => ("\u{2717}", YELLOW),
                LandscapeStanding::BeyondObservation => ("\u{2205}", DIM),
                LandscapeStanding::Unmeasured => ("?", DIM),
            };
            println!(
                "    {}{}{} sev={:>5} {:?} {}  {}\u{2190} {}{}",
                c(color),
                mark,
                c(RESET),
                format!("{:.2}", p.severity.to_f64()),
                p.facet.kind,
                p.facet.key,
                c(DIM),
                p.rationale,
                c(RESET)
            );
        }
        if !landscape.unmapped.is_empty() {
            println!(
                "  {}beyond the wish vocabulary today ({} finding(s)):{}",
                c(DIM),
                landscape.unmapped.len(),
                c(RESET)
            );
            for u in landscape.unmapped.iter().take(5) {
                println!(
                    "    {}\u{2014} {} @ {}{}",
                    c(DIM),
                    u.kind_label,
                    u.location,
                    c(RESET)
                );
            }
        }
        if let Some(geo) = &geometry {
            println!(
                "  {}geometry{}: {} coherent cluster(s) over {} open proposal(s)",
                c(BOLD),
                c(RESET),
                geo.clusters.len(),
                open_proposals.len()
            );
            for (idx, cl) in geo.clusters.iter().enumerate() {
                println!(
                    "    cluster {} [mass {:.2}]  {}",
                    idx + 1,
                    cl.severity_mass.to_f64(),
                    cl.subjects.join(", ")
                );
                for &m in &cl.members {
                    println!(
                        "      {:?} {}",
                        open_proposals[m].facet.kind, open_proposals[m].facet.key
                    );
                }
            }
            for s in &geo.singular {
                println!(
                    "    {}singular{}: {:?} {} (coupling {:.2}) — removing it disconnects the landscape",
                    c(YELLOW),
                    c(RESET),
                    open_proposals[s.index].facet.kind,
                    open_proposals[s.index].facet.key,
                    s.coupling_mass.to_f64()
                );
            }
        }
    }

    // ── Adoption: a slice of the landscape becomes ONE wish ──────────────────
    if args.adopt > 0 {
        let adopted: Vec<&WishProposal> = landscape
            .proposals
            .iter()
            .zip(&standing)
            .filter(|(_, s)| **s == LandscapeStanding::Open)
            .map(|(p, _)| p)
            .take(args.adopt as usize)
            .collect();
        let label = format!("landscape: top {} of {}", adopted.len(), args.path);
        return adopt_and_descend(args, &adopted, label, report.report_id);
    }
    if args.adopt_cluster > 0 {
        let geo = geometry
            .as_ref()
            .expect("adopt_cluster > 0 forces geometry computation");
        let idx = (args.adopt_cluster - 1) as usize;
        let Some(cluster) = geo.clusters.get(idx) else {
            return Err(format!(
                "--adopt-cluster {}: the open landscape has {} cluster(s)",
                args.adopt_cluster,
                geo.clusters.len()
            ));
        };
        let adopted: Vec<&WishProposal> = cluster
            .members
            .iter()
            .map(|&i| &open_proposals[i])
            .collect();
        let label = format!(
            "landscape cluster {} ({}) of {}",
            args.adopt_cluster,
            cluster.subjects.join("+"),
            args.path
        );
        return adopt_and_descend(args, &adopted, label, report.report_id);
    }

    Ok(ExitCode::SUCCESS)
}

/// Maximum cluster count offered by `--geometry` — an operator-attention
/// scale; the actual count *emerges* (tight clusters refuse to shatter).
const GEOMETRY_MAX_CLUSTERS: usize = 6;

/// Turn `adopted` proposals into ONE severity-weighted, evidence-bound wish:
/// print it, and under `--apply` descend it with the same armament as wish
/// mode (deterministic scaffolds first, provider-gated LLM fallback,
/// memory-grounded under `--ledger`). Shared by `--adopt` and
/// `--adopt-cluster`.
fn adopt_and_descend(
    args: &Args,
    adopted: &[&WishProposal],
    label: String,
    evidence: Digest,
) -> Result<ExitCode, String> {
    if adopted.is_empty() {
        println!("\nnothing open to adopt — the landscape is either met or beyond reach");
        return Ok(ExitCode::SUCCESS);
    }
    let predicates = adopted
        .iter()
        .map(|p| kosmo_core::WishPredicate::weighted(p.facet.clone(), p.severity));
    // Evidence: the wish is bound to the diagnosis that proposed it.
    let wish = Wish::new(label, predicates, Digest::ZERO, evidence);
    println!("\n{} adopted as one wish:", adopted.len());
    for p in adopted {
        println!(
            "    {:?} {}  (weight {:.2})",
            p.facet.kind,
            p.facet.key,
            p.severity.to_f64()
        );
    }
    if !args.apply {
        println!("  (read-only — add --apply to descend the adopted wish)");
        return Ok(ExitCode::SUCCESS);
    }
    let fallback = wish_fallback(args)?;
    let session = descend_to_wish(
        &args.path,
        &wish,
        evidence,
        false,
        8,
        fallback.as_deref(),
        None,
    )?;
    print!("{}", descent_report(&session, args.color));
    let realized = session
        .latest()
        .map(|a| matches!(a.status, WishClosureStatus::Realized))
        .unwrap_or(false);
    // The system's own proposals are sightings too: an adopted descent
    // records a norm-learning observation like a spoken wish does.
    if let Some(dir) = args.norms.as_deref() {
        match NormStore::open(dir) {
            Ok(mut store) => record_norm_observation(
                &mut store,
                &args.path,
                &wish,
                realized,
                evidence,
                &PolicyProfile::operator_approved(),
            ),
            Err(e) => eprintln!("norms: could not open store: {e}"),
        }
    }
    Ok(if realized {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(4)
    })
}

// ─── Chat front door (one-shot utterance → existing modes) ──────────────────

/// Compact audit label for the routing echo line.
fn intent_label(intent: &ChatIntent) -> String {
    match intent {
        ChatIntent::MakeWish { .. } => "make-wish".into(),
        ChatIntent::DescendWish { .. } => "descend-wish".into(),
        ChatIntent::ShowLandscape { geometry } => {
            format!(
                "show-landscape{}",
                if *geometry { " +geometry" } else { "" }
            )
        }
        ChatIntent::AdoptLandscape { top } => format!("adopt top-{top}"),
        ChatIntent::AdoptCluster { index } => format!("adopt cluster {index}"),
        ChatIntent::ShowStatus => "status".into(),
        ChatIntent::InjectNorm => "inject-norm".into(),
    }
}

/// The shared-transport config for advisory model work (chat routing, wish
/// refinement) — only when the operator explicitly chose a real provider.
/// The mock provider yields none (mock advice would be theater).
fn llm_config_for(args: &Args) -> Option<kosmo_llm::LlmConfig> {
    if !args.provider_set {
        return None;
    }
    let config =
        match args.provider.as_str() {
            "claude" => env_key(&["ANTHROPIC_API_KEY", "KOSMO_LLM_API_KEY"])
                .map(kosmo_llm::LlmConfig::claude),
            "cerebras" => env_key(&["CEREBRAS_API_KEY", "KOSMO_LLM_API_KEY"])
                .map(kosmo_llm::LlmConfig::cerebras),
            "env" => kosmo_llm::config_from_env().ok(),
            _ => None,
        }?;
    Some(match &args.model {
        Some(m) => config.with_model(m.clone()),
        None => config,
    })
}

/// The router: deterministic keyword rules by default; when the operator
/// explicitly chose a real provider, the model routes first — and the
/// extractor itself falls back to the keyword rules on any failure, so
/// routing stays total either way.
fn chat_extractor(args: &Args) -> Box<dyn IntentExtractor> {
    match llm_config_for(args) {
        Some(config) => Box::new(LlmIntentExtractor::new(config)),
        None => Box::new(KeywordIntentExtractor),
    }
}

/// One-shot chat: extract the intent (total — the fallback is the wish
/// door), echo the routing for auditability, and delegate to the existing
/// mode. Chat never escalates: writes still require the explicit `--apply`.
fn run_chat_mode(args: &Args) -> Result<ExitCode, String> {
    let utterance = args.chat.as_deref().unwrap_or("");
    let extractor = chat_extractor(args);
    let intent = extractor.extract(utterance);
    if !args.json {
        println!(
            "chat[{}] \u{2192} {}",
            extractor.name(),
            intent_label(&intent)
        );
    }
    match intent {
        ChatIntent::MakeWish { prose } => {
            let mut sub = args.clone();
            sub.wish = Some(prose);
            run_wish_mode(&sub)
        }
        ChatIntent::DescendWish { prose } => {
            let mut sub = args.clone();
            sub.wish = Some(prose);
            if !sub.apply && !sub.json {
                println!("(the utterance asks to build — measuring only; add --apply to descend)");
            }
            run_wish_mode(&sub)
        }
        ChatIntent::ShowLandscape { geometry } => {
            let mut sub = args.clone();
            sub.landscape = true;
            sub.geometry = sub.geometry || geometry;
            run_landscape_mode(&sub)
        }
        ChatIntent::AdoptLandscape { top } => {
            let mut sub = args.clone();
            sub.landscape = true;
            sub.adopt = top.max(1);
            run_landscape_mode(&sub)
        }
        ChatIntent::AdoptCluster { index } => {
            let mut sub = args.clone();
            sub.landscape = true;
            sub.adopt_cluster = index.max(1);
            run_landscape_mode(&sub)
        }
        ChatIntent::ShowStatus => {
            let mut sub = args.clone();
            sub.landscape = true;
            if !sub.json {
                println!("status \u{2014} the system's measured standing:");
                // The cockpit lines: every armed organ reports in one glance.
                if let Some(dir) = args.norms.as_deref() {
                    match NormStore::open(dir) {
                        Ok(store) => {
                            let armed =
                                store.norms().iter().filter(|n| n.trigger.is_some()).count();
                            println!(
                                "  norms: {} known ({} armed) \u{b7} {} observation(s)",
                                store.norms().len(),
                                armed,
                                store.observations().len()
                            );
                        }
                        Err(e) => println!("  norms: store unreadable ({e})"),
                    }
                }
                if let Some(recall) = open_recall(args)? {
                    println!("  memory: {}", recall.source());
                }
            }
            run_landscape_mode(&sub)
        }
        ChatIntent::InjectNorm => {
            println!("norm injection is an operator act with a spec file — chat carries no paths.");
            println!("  use: kosmo-run --norms <dir> --inject-norm <spec.json>");
            println!("  then: kosmo-run --norms <dir> --promote-norm <id> --trigger <word>");
            Ok(ExitCode::SUCCESS)
        }
    }
}

// ─── Wish atelier (shape a wish over rounds) ────────────────────────────────

/// Load a draft from disk (verifying its content address — a corrupt draft
/// is a hard error) or start a fresh one.
fn load_draft(path: &str) -> Result<WishDraft, String> {
    if !Path::new(path).exists() {
        return Ok(WishDraft::new(Digest::ZERO));
    }
    let text = fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let draft: WishDraft =
        serde_json::from_str(&text).map_err(|e| format!("parse draft {path}: {e}"))?;
    if !draft.verify_id() {
        return Err(format!(
            "draft {path} fails its content address — refusing a tampered draft"
        ));
    }
    Ok(draft)
}

fn save_draft(path: &str, draft: &WishDraft) -> Result<(), String> {
    let json = serde_json::to_string_pretty(draft).map_err(|e| format!("serialize draft: {e}"))?;
    fs::write(path, json).map_err(|e| format!("write {path}: {e}"))
}

/// Map a display selection (continuous 1-based numbering over accepted then
/// pending) to 0-based indices of ONE list; mismatched slots become honest
/// feedback lines instead of silent skips.
fn resolve_selection(
    draft: &WishDraft,
    selection: &IndexSelection,
    want_pending: bool,
) -> (Vec<usize>, Vec<String>) {
    match selection {
        IndexSelection::All => {
            let len = if want_pending {
                draft.pending.len()
            } else {
                draft.accepted.len()
            };
            ((0..len).collect(), vec![])
        }
        IndexSelection::These(numbers) => {
            let mut indices = Vec::new();
            let mut notes = Vec::new();
            for &n in numbers {
                match draft.resolve(n) {
                    Some(DraftSlot::Pending(i)) if want_pending => indices.push(i),
                    Some(DraftSlot::Accepted(i)) if !want_pending => indices.push(i),
                    Some(DraftSlot::Pending(_)) => {
                        notes.push(format!("{n} is a proposal — use accept/reject, not drop"))
                    }
                    Some(DraftSlot::Accepted(_)) => {
                        notes.push(format!("{n} is already in the wish — use drop to retract"))
                    }
                    None => notes.push(format!("{n} is not on the list")),
                }
            }
            (indices, notes)
        }
    }
}

/// Render the draft: dialogue, the numbered wish-so-far (measured ✓/✗ when
/// an observation is available), the numbered proposals, open questions.
fn render_draft(
    draft: &WishDraft,
    observed: Option<&kosmo_core::ObservedTopology>,
    path: &str,
    color: bool,
) -> String {
    let c = |code: &'static str| if color { code } else { "" };
    let mut out = String::new();
    out.push_str(&format!(
        "{}{}Kosmocrates wish atelier{}  {}{}{}\n",
        c(BOLD),
        c(CYAN),
        c(RESET),
        c(DIM),
        path,
        c(RESET)
    ));
    if draft.prose_history.is_empty() {
        out.push_str("  (an empty draft — speak a wish: --chat \"a module …\")\n");
    } else {
        out.push_str(&format!(
            "  dialogue ({} round(s)):\n",
            draft.prose_history.len()
        ));
        for (i, prose) in draft.prose_history.iter().enumerate() {
            out.push_str(&format!("    {}. \u{201c}{}\u{201d}\n", i + 1, prose));
        }
    }
    let mut number = 0usize;
    if !draft.accepted.is_empty() {
        out.push_str(&format!("  wish so far ({}):\n", draft.accepted.len()));
        for f in &draft.accepted {
            number += 1;
            let mark = match observed {
                Some(obs) if obs.contains(f) => format!("{}\u{2713}{}", c(GREEN), c(RESET)),
                Some(_) => format!("{}\u{2717}{}", c(RED), c(RESET)),
                None => "?".to_string(),
            };
            out.push_str(&format!("    {number:>2} {mark} {:?} {}\n", f.kind, f.key));
        }
    }
    if !draft.pending.is_empty() {
        out.push_str(&format!(
            "  proposed ({}) {}— \u{201c}accept <n>\u{201d} / \u{201c}reject <n>\u{201d}:{}\n",
            draft.pending.len(),
            c(DIM),
            c(RESET)
        ));
        for s in &draft.pending {
            number += 1;
            let source = match &s.source {
                SuggestionSource::Observation => "substrate".to_string(),
                SuggestionSource::Model { label } => label.clone(),
            };
            out.push_str(&format!(
                "    {number:>2} {}?{} {:?} {} {}\u{2014} {} [{}]{}\n",
                c(YELLOW),
                c(RESET),
                s.facet.kind,
                s.facet.key,
                c(DIM),
                s.rationale,
                source,
                c(RESET)
            ));
        }
    }
    if !draft.questions.is_empty() {
        out.push_str("  questions:\n");
        for q in &draft.questions {
            out.push_str(&format!("    {}\u{2014} {}{}\n", c(DIM), q, c(RESET)));
        }
    }
    if !draft.accepted.is_empty() {
        out.push_str(&format!(
            "  {}(\u{201c}realize\u{201d} freezes the wish and descends; writes only with --apply){}\n",
            c(DIM),
            c(RESET)
        ));
    }
    out
}

/// Freeze the draft and descend toward it — the only place an atelier round
/// touches the workspace, and only under `--apply`. Without `--apply`:
/// measure and report, the wish-mode contract.
fn realize_draft(args: &Args, draft: &WishDraft) -> Result<ExitCode, String> {
    let wish = draft.to_wish();
    if wish.predicate_count() == 0 {
        println!("the draft holds no accepted facets yet — speak a wish first");
        return Ok(ExitCode::from(1));
    }
    let validated = args.validated || wish_needs_validation(&wish);
    if !args.apply {
        let observed = if wish_needs_service(&wish) {
            observe_workspace_service(args.path.as_str())
        } else if wish_needs_runtime(&wish) {
            observe_workspace_runtime(args.path.as_str())
        } else if validated {
            observe_workspace_validated(args.path.as_str())
        } else {
            observe_workspace_deep(args.path.as_str())
        }
        .map_err(|e| format!("could not observe {}: {e}", args.path))?;
        let assessment = assess_wish(&wish, &observed, draft.evidence_bundle_id);
        print!("{}", wish_report(&wish, &assessment, args.color));
        println!("  (read-only — add --apply to descend the realized wish)");
        return Ok(match assessment.status {
            WishClosureStatus::Realized | WishClosureStatus::Vacuous => ExitCode::SUCCESS,
            _ => ExitCode::from(1),
        });
    }
    let fallback = wish_fallback(args)?;
    let session = descend_to_wish(
        &args.path,
        &wish,
        draft.evidence_bundle_id,
        validated,
        8,
        fallback.as_deref(),
        None,
    )?;
    print!("{}", descent_report(&session, args.color));
    let realized = session.latest().is_some_and(|a| {
        matches!(
            a.status,
            WishClosureStatus::Realized | WishClosureStatus::Vacuous
        )
    });
    // A realized atelier descent is a learning observation like any other.
    if let Some(dir) = args.norms.as_deref() {
        match NormStore::open(dir) {
            Ok(mut store) => record_norm_observation(
                &mut store,
                &args.path,
                &wish,
                realized,
                draft.evidence_bundle_id,
                &PolicyProfile::operator_approved(),
            ),
            Err(e) => eprintln!("norms: could not open store: {e}"),
        }
    }
    Ok(if realized {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

/// One atelier round: load the draft, apply the utterance (prose, verdict,
/// show, or realize), persist, and render the new state.
fn run_atelier_mode(args: &Args) -> Result<ExitCode, String> {
    let draft_path = args.atelier.as_deref().expect("dispatch checked");
    let mut draft = load_draft(draft_path)?;
    let utterance = args.chat.as_deref().unwrap_or("");
    let command = parse_atelier_command(utterance);

    // One observation per round: measures the wish-so-far and filters
    // companion proposals. Fail-soft — a non-cargo dir just shows '?'.
    let observed = observe_workspace_deep(&args.path).ok();

    let mut feedback: Vec<String> = Vec::new();
    let mutated = match command {
        AtelierCommand::Show => false,
        AtelierCommand::Realize => return realize_draft(args, &draft),
        AtelierCommand::Speak(prose) => {
            let catalog = match args.norms.as_deref() {
                Some(dir) => {
                    let store = NormStore::open(dir).map_err(|e| e.to_string())?;
                    NormCatalog::from_norms(store.norms()).map_err(|e| e.to_string())?
                }
                None => NormCatalog::empty(),
            };
            draft = draft.speak(&prose, &catalog);
            let companions = companion_suggestions(&draft, observed.as_ref());
            draft = draft.propose(companions);
            // Model refinement is advisory and provider-gated; its absence
            // costs suggestions, never the round.
            if let Some(config) = llm_config_for(args) {
                let refiner = LlmWishRefiner::new(config);
                let outcome = refiner.refine(&draft);
                if let Some(note) = &outcome.note {
                    feedback.push(note.clone());
                }
                draft = draft
                    .propose(outcome.suggestions)
                    .with_questions(outcome.questions);
            }
            true
        }
        AtelierCommand::Accept(selection) => {
            let (indices, notes) = resolve_selection(&draft, &selection, true);
            feedback.extend(notes);
            draft = draft.accept_pending(&indices);
            true
        }
        AtelierCommand::Reject(selection) => {
            let (indices, notes) = resolve_selection(&draft, &selection, true);
            feedback.extend(notes);
            draft = draft.reject_pending(&indices);
            true
        }
        AtelierCommand::Drop(selection) => {
            let (indices, notes) = resolve_selection(&draft, &selection, false);
            feedback.extend(notes);
            draft = draft.retract_accepted(&indices);
            true
        }
    };
    if mutated {
        save_draft(draft_path, &draft)?;
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&draft).map_err(|e| e.to_string())?
        );
    } else {
        for note in &feedback {
            println!("  note: {note}");
        }
        print!(
            "{}",
            render_draft(&draft, observed.as_ref(), draft_path, args.color)
        );
    }
    Ok(ExitCode::SUCCESS)
}

// ─── Venture (a whole system of dependent wishes) ──────────────────────────

/// Load a venture session from disk, verifying both its content address and
/// that it belongs to `expected` — a session for a different venture is a
/// hard error, never a silent restart.
fn load_venture_session(path: &str, expected: &Digest) -> Result<Option<VentureSession>, String> {
    if !Path::new(path).exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let session: VentureSession =
        serde_json::from_str(&text).map_err(|e| format!("parse venture session {path}: {e}"))?;
    if !session.venture().verify_id() {
        return Err(format!(
            "venture session {path} fails its content address — refusing a tampered session"
        ));
    }
    if &session.venture().venture_id != expected {
        return Err(format!(
            "venture session {path} belongs to a different venture \
             (the spec changed?) — refusing to mix histories"
        ));
    }
    Ok(Some(session))
}

fn save_venture_session(path: &str, session: &VentureSession) -> Result<(), String> {
    let json = serde_json::to_string_pretty(session)
        .map_err(|e| format!("serialize venture session: {e}"))?;
    fs::write(path, json).map_err(|e| format!("write {path}: {e}"))
}

/// Render the venture's staircase: order, standings, facet counts.
fn render_venture(session: &VentureSession, color: bool) -> String {
    let c = |code: &'static str| if color { code } else { "" };
    let venture = session.venture();
    let mut out = String::new();
    out.push_str(&format!(
        "{}{}Kosmocrates venture{}  \u{201c}{}\u{201d}  ({} stage(s))\n",
        c(BOLD),
        c(CYAN),
        c(RESET),
        venture.label,
        venture.stage_count()
    ));
    let order = venture.execution_order();
    let order_labels: Vec<&str> = order
        .iter()
        .map(|&i| venture.stages[i].label.as_str())
        .collect();
    out.push_str(&format!("  order: {}\n", order_labels.join(" \u{2192} ")));
    for (i, stage) in venture.stages.iter().enumerate() {
        let standing = session.standings()[i];
        let (mark, col) = match standing {
            StageStanding::Realized => ("\u{2713}", GREEN),
            StageStanding::Failed => ("\u{2717}", RED),
            StageStanding::Blocked => ("\u{2298}", RED),
            StageStanding::Pending => ("\u{00b7}", DIM),
        };
        let deps = if stage.after.is_empty() {
            String::new()
        } else {
            format!(
                "  {}after {:?}{}",
                c(DIM),
                stage.after.iter().map(|d| d + 1).collect::<Vec<_>>(),
                c(RESET)
            )
        };
        out.push_str(&format!(
            "  {:>2} {}{}{} {}  [{}]  {} facet(s){}\n",
            i + 1,
            c(col),
            mark,
            c(RESET),
            stage.label,
            standing.label(),
            stage.wish.predicate_count(),
            deps
        ));
    }
    out
}

/// Orchestrate a venture: compile the spec, resume or start the session,
/// and (under `--apply`) descend the stages in dependency order — each
/// under the full armament, each outcome persisted before the next begins.
fn run_venture_mode(args: &Args) -> Result<ExitCode, String> {
    let spec_path = args.venture.as_deref().expect("dispatch checked");
    let bytes = fs::read(spec_path).map_err(|e| format!("read {spec_path}: {e}"))?;
    let evidence = Digest::of_bytes(&bytes);
    let catalog = match args.norms.as_deref() {
        Some(dir) => {
            let store = NormStore::open(dir).map_err(|e| e.to_string())?;
            NormCatalog::from_norms(store.norms()).map_err(|e| e.to_string())?
        }
        None => NormCatalog::empty(),
    };
    let spec_text = String::from_utf8_lossy(&bytes);
    let venture = compile_venture(&spec_text, &catalog, Digest::ZERO, evidence)?;

    let mut session = match args.venture_session.as_deref() {
        Some(path) => load_venture_session(path, &venture.venture_id)?
            .unwrap_or_else(|| VentureSession::new(venture)),
        None => VentureSession::new(venture),
    };

    if !args.apply {
        // Read-only preview: the staircase plus each stage measured against
        // the workspace (fail-soft when unobservable).
        if !args.json {
            print!("{}", render_venture(&session, args.color));
            if let Ok(observed) = observe_workspace_deep(args.path.as_str()) {
                for stage in &session.venture().stages {
                    let a = assess_wish(&stage.wish, &observed, stage.wish.evidence_bundle_id);
                    println!(
                        "     {} measured: {}/{} met",
                        stage.label, a.met_count, a.total_count
                    );
                }
            }
            println!("  (read-only — add --apply to erect the venture)");
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?
            );
        }
        return Ok(ExitCode::SUCCESS);
    }

    let fallback = wish_fallback(args)?;
    while let Some(i) = session.next_ready() {
        let stage = session.venture().stages[i].clone();
        if !args.json {
            println!(
                "stage {}/{} \u{201c}{}\u{201d} \u{2014} descending \u{2026}",
                i + 1,
                session.venture().stage_count(),
                stage.label
            );
        }
        let validated = args.validated || wish_needs_validation(&stage.wish);
        let wish_session = descend_to_wish(
            &args.path,
            &stage.wish,
            stage.wish.evidence_bundle_id,
            validated,
            8,
            fallback.as_deref(),
            None,
        )?;
        let realized = wish_session.latest().is_some_and(|a| {
            matches!(
                a.status,
                WishClosureStatus::Realized | WishClosureStatus::Vacuous
            )
        });
        if let Some(a) = wish_session.latest() {
            if !args.json {
                println!(
                    "  \u{2192} {} (met {}/{}, {} observation(s))",
                    if realized { "REALIZED" } else { "NOT REALIZED" },
                    a.met_count,
                    a.total_count,
                    wish_session.iterations()
                );
            }
        }
        session.mark(
            i,
            if realized {
                StageStanding::Realized
            } else {
                StageStanding::Failed
            },
        );
        // Every realized stage is a learning sighting like any other.
        if realized {
            if let Some(dir) = args.norms.as_deref() {
                match NormStore::open(dir) {
                    Ok(mut store) => record_norm_observation(
                        &mut store,
                        &args.path,
                        &stage.wish,
                        realized,
                        stage.wish.evidence_bundle_id,
                        &PolicyProfile::operator_approved(),
                    ),
                    Err(e) => eprintln!("norms: could not open store: {e}"),
                }
            }
        }
        // Persist progress before the next stage — the resumability seam.
        if let Some(path) = args.venture_session.as_deref() {
            save_venture_session(path, &session)?;
        }
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?
        );
    } else {
        print!("{}", render_venture(&session, args.color));
        println!(
            "venture: {}/{} realized{}",
            session.realized_count(),
            session.venture().stage_count(),
            if session.is_complete() {
                " \u{2713}"
            } else {
                ""
            }
        );
    }
    Ok(if session.is_complete() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

// ─── Reforge (the external-empiricism bench) ────────────────────────────────

/// Run the reforge bench: oracle-collected wishes, re-forged with the
/// provider-backed fallback, judged at execution. Refuses to run without a
/// real provider — and refuses the mock outright (forging theater).
fn run_reforge_mode(args: &Args) -> Result<ExitCode, String> {
    if args.provider == "mock" {
        return Err(
            "--reforge needs a real provider (claude | cerebras | env): the mock \
             synthesizer cannot implement behaviour, and pretending otherwise \
             would be forging theater"
                .to_string(),
        );
    }
    let synthesizer = build_synthesizer(args)?;
    let fallback = arm_fallback(args, Some(synthesizer))?
        .ok_or("--reforge needs a real provider (claude | cerebras | env)")?;
    let report = reforge::run_reforge(fallback.as_ref(), &args.provider);
    if args.json {
        println!("{}", report.to_json());
    } else {
        print!("{}", report.render(args.color));
    }
    if let Some(path) = args.reforge_report.as_deref() {
        fs::write(path, report.to_json()).map_err(|e| format!("write {path}: {e}"))?;
        if !args.json {
            println!("  report written to {path}");
        }
    }
    Ok(if report.is_faithful() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(5)
    })
}

/// `--realize-bench`: fire the real generative loop at a curated behavioural
/// corpus and measure the realization rate (and token cost) — judged by
/// execution. Provider-agnostic (the same corpus runs against a cloud API
/// or a local OpenAI-compatible model); mock is refused. It is an
/// instrument, not a gate: completion exits 0 and the rate is the finding.
fn run_realize_bench_mode(args: &Args) -> Result<ExitCode, String> {
    if args.provider == "mock" {
        return Err(
            "--realize-bench needs a real provider (claude | cerebras | env): the mock \
             synthesizer cannot implement behaviour, so a benchmark of it would measure \
             only the deterministic scaffolder — which the Prüfstand already proves"
                .to_string(),
        );
    }
    let synthesizer = build_synthesizer(args)?;
    let armed = arm_fallback(args, Some(synthesizer))?.ok_or(
        "--realize-bench needs a real provider (claude | cerebras | env) — set a key or \
         KOSMO_LLM_BASE_URL for a local model",
    )?;
    if !args.json {
        eprintln!(
            "kosmo-run: firing {} CLI + {} service task(s) through the real loop — this \
             calls the provider repeatedly and may take a while…",
            realize_bench::reference_corpus().len(),
            realize_bench::service_corpus().len()
        );
    }
    let report = realize_bench::run_realize_bench(armed, &args.provider, args.model.clone());
    if args.json {
        println!("{}", report.to_json());
    } else {
        print!("{}", report.render(args.color));
    }
    if let Some(path) = args.realize_bench_report.as_deref() {
        fs::write(path, report.to_json()).map_err(|e| format!("write {path}: {e}"))?;
        if !args.json {
            println!("  report written to {path}");
        }
    }
    // A measurement, not a gate: completion is success; the rate is the finding.
    Ok(ExitCode::SUCCESS)
}

/// `--realize-service`: the service-dimension counterpart to `--realize-bench`.
/// Drive ONE HTTP-service wish through the real descent and report whether the
/// artifact, **started as a server and probed over HTTP**, answered. Requires a
/// real provider (mock cannot implement a server). A measurement, exits 0.
fn run_service_smoke_mode(args: &Args) -> Result<ExitCode, String> {
    if args.provider == "mock" {
        return Err(
            "--realize-service needs a real provider (claude | cerebras | env): the mock \
             synthesizer cannot implement a server"
                .to_string(),
        );
    }
    let synthesizer = build_synthesizer(args)?;
    let armed = arm_fallback(args, Some(synthesizer))?.ok_or(
        "--realize-service needs a real provider (claude | cerebras | env) — set a key or \
         KOSMO_LLM_BASE_URL for a local model",
    )?;
    let max_iters: u32 = std::env::var("KOSMO_REALIZE_MAX_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(8);
    if !args.json {
        eprintln!(
            "kosmo-run: firing 1 HTTP-service wish through the real loop — starts a server \
             and probes it over HTTP…"
        );
    }
    let (realized, iterations, tokens) = realize_bench::run_service_smoke(armed, max_iters);
    if args.json {
        println!(
            "{{\"service_smoke\":{{\"realized\":{realized},\"iterations\":{iterations},\"tokens\":{tokens}}}}}"
        );
    } else {
        println!(
            "service realization: {} · {} iterations · {} tokens",
            if realized { "REALIZED" } else { "unrealized" },
            iterations,
            tokens
        );
    }
    // A measurement, not a gate: completion is success; the verdict is the finding.
    Ok(ExitCode::SUCCESS)
}

/// `--prose-bench`: measure the prose→spec front door. Run natural-language
/// utterances through the intent extractor (the LLM router under `--provider`,
/// else the deterministic keyword router) and `compile_wish`, scoring the facets
/// against a hand-written ground truth. No workspace, no realization — a cheap
/// single-call probe of the *other* axis. A measurement, exits 0.
fn run_prose_bench_mode(args: &Args) -> Result<ExitCode, String> {
    let extractor = chat_extractor(args);
    if !args.json {
        eprintln!(
            "kosmo-run: firing {} prose task(s) through the {} extractor…",
            prose_bench::prose_corpus().len(),
            extractor.name()
        );
    }
    let report = prose_bench::run_prose_bench(extractor.as_ref());
    print!("{}", report.render(args.color));
    // A measurement, not a gate: completion is success; the rate is the finding.
    Ok(ExitCode::SUCCESS)
}

/// `--realize-multicrate`: the multi-crate counterpart to `--realize-service`.
/// Drive one Run wish onto a two-crate workspace (a bin + the library it calls)
/// and report whether the loop realized the logic *across the crate boundary*,
/// judged by executing the bin. Requires a real provider. A measurement, exits 0.
fn run_multicrate_smoke_mode(args: &Args) -> Result<ExitCode, String> {
    if args.provider == "mock" {
        return Err(
            "--realize-multicrate needs a real provider (claude | cerebras | env): the mock \
             synthesizer cannot implement the library logic"
                .to_string(),
        );
    }
    let synthesizer = build_synthesizer(args)?;
    let armed = arm_fallback(args, Some(synthesizer))?.ok_or(
        "--realize-multicrate needs a real provider (claude | cerebras | env) — set a key or \
         KOSMO_LLM_BASE_URL for a local model",
    )?;
    let max_iters: u32 = std::env::var("KOSMO_REALIZE_MAX_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(8);
    if !args.json {
        eprintln!(
            "kosmo-run: firing 1 multi-crate wish through the real loop — the logic must land \
             in the engine crate, reached across the boundary…"
        );
    }
    let (realized, iterations, tokens) = realize_bench::run_multicrate_smoke(armed, max_iters);
    if args.json {
        println!(
            "{{\"multicrate_smoke\":{{\"realized\":{realized},\"iterations\":{iterations},\"tokens\":{tokens}}}}}"
        );
    } else {
        println!(
            "multi-crate realization: {} · {} iterations · {} tokens",
            if realized { "REALIZED" } else { "unrealized" },
            iterations,
            tokens
        );
    }
    // A measurement, not a gate: completion is success; the verdict is the finding.
    Ok(ExitCode::SUCCESS)
}

// ─── Steward (self-husbandry under an operator-named fence) ─────────────────

/// `--steward`: survey the workspace's wish landscape, name the open chores
/// inside the operator's `--fence`, and under `--apply` descend each chore
/// as its own evidence-bound wish — the same armament as wish mode and
/// landscape adoption, one descent and one norm observation per chore.
/// Read-only without `--apply`; `--apply` without a fence is refused
/// (nothing is fenced by default). The report is content-addressed and
/// host-path-free, fit for an unattended nightly run.
fn run_steward_mode(args: &Args) -> Result<ExitCode, String> {
    let fence = args
        .fence
        .as_deref()
        .map(steward::Fence::parse)
        .transpose()?;
    if args.apply && fence.is_none() {
        return Err(
            "--steward --apply needs --fence <classes>: nothing is fenced by default — \
             the operator names what the steward may husband (e.g. --fence doc,test)"
                .into(),
        );
    }

    // The same diagnosis the landscape door runs — the steward adds no new
    // eyes, only governed hands.
    let policy = PolicyProfile::default_report_only();
    let options = if args.all_layers {
        IntegrationRunOptions::all_layers(args.capacity)
    } else {
        IntegrationRunOptions::report_only()
    };
    let report = run_workspace_pipeline(&args.path, &options, &policy)
        .map_err(|e| format!("pipeline failed on {}: {e}", args.path))?;
    let voids = &report.hyphae_result.host_cube.void_map.voids;
    let landscape = propose_wishes(voids);
    let observed = observe_workspace_deep(&args.path).ok();
    let standing = measure_landscape(&landscape, observed.as_ref());

    let cap = (args.steward_max > 0).then_some(args.steward_max as usize);
    let chores: Vec<&WishProposal> = match &fence {
        Some(f) => steward::fenced_open(&landscape, &standing, f, cap),
        None => Vec::new(),
    };
    let mut sreport = steward::StewardReport::survey(
        workspace_tag(&args.path),
        fence.as_ref(),
        &landscape,
        &standing,
        &chores,
        args.apply,
    );

    if args.apply {
        let fallback = wish_fallback(args)?;
        let mut norm_store = match args.norms.as_deref() {
            Some(dir) => match NormStore::open(dir) {
                Ok(store) => Some(store),
                Err(e) => {
                    eprintln!("norms: could not open store: {e}");
                    None
                }
            },
            None => None,
        };
        for p in &chores {
            let wish = Wish::new(
                format!("steward: {:?} {}", p.facet.kind, p.facet.key),
                [kosmo_core::WishPredicate::weighted(
                    p.facet.clone(),
                    p.severity,
                )],
                Digest::ZERO,
                report.report_id,
            );
            // A failed chore is recorded and the round continues — an
            // unattended steward reports failures, it doesn't abandon the
            // remaining fenced work over one of them.
            let (realized, iterations) = match descend_to_wish(
                &args.path,
                &wish,
                report.report_id,
                false,
                8,
                fallback.as_deref(),
                None,
            ) {
                Ok(session) => (
                    session
                        .latest()
                        .is_some_and(|a| matches!(a.status, WishClosureStatus::Realized)),
                    session.iterations(),
                ),
                Err(e) => {
                    eprintln!("steward: chore {:?} {}: {e}", p.facet.kind, p.facet.key);
                    (false, 0)
                }
            };
            // Husbanded chores are sightings too: each descent records a
            // norm-learning observation like a spoken wish does.
            if let Some(store) = norm_store.as_mut() {
                record_norm_observation(
                    store,
                    &args.path,
                    &wish,
                    realized,
                    report.report_id,
                    &PolicyProfile::operator_approved(),
                );
            }
            sreport.chores.push(steward::ChoreOutcome {
                kind: format!("{:?}", p.facet.kind),
                key: p.facet.key.clone(),
                wish_id: wish.id.to_hex(),
                realized,
                iterations,
            });
        }
    }

    if args.json {
        println!("{}", sreport.to_json());
    } else {
        print!("{}", sreport.render(args.color));
    }
    if let Some(path) = args.steward_report.as_deref() {
        fs::write(path, sreport.to_json()).map_err(|e| format!("write {path}: {e}"))?;
        if !args.json {
            println!("  report written to {path}");
        }
    }
    Ok(if sreport.is_faithful() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(4)
    })
}

// ─── Substrate organ doors (Spreizung II) ───────────────────────────────────

/// `--foundry <kinds>`: the loop's own gate executor as a directed door —
/// the same allowlisted cargo commands, timeout discipline and
/// content-addressed evidence the agent uses, invoked alone. `DryRun` is
/// the least power that executes (`ReportOnly` is inert by design); the
/// commands never touch sources (cargo's `target/` cache aside).
fn run_foundry_mode(args: &Args) -> Result<ExitCode, String> {
    let spec = args.foundry.as_deref().unwrap_or_default();
    let mut checks = Vec::new();
    for word in spec.split(',') {
        let word = word.trim().to_lowercase();
        if word.is_empty() {
            continue;
        }
        let kind = match word.as_str() {
            "build" => FoundryCheckKind::Build,
            "test" => FoundryCheckKind::Test,
            "lint" | "clippy" => FoundryCheckKind::Lint,
            "typecheck" | "check" => FoundryCheckKind::TypeCheck,
            other => {
                return Err(format!(
                    "--foundry: '{other}' is not a runnable check (the vocabulary: \
                     build, test, lint, typecheck)"
                ));
            }
        };
        checks.push(FoundryCheckSpec::new(kind, "workspace", true));
    }
    if checks.is_empty() {
        return Err("--foundry names no check (e.g. --foundry build,test)".into());
    }
    let policy = PolicyProfile::dry_run();
    let root_digest = workspace_tag(&args.path);
    let plan = FoundryExecutionPlan::new(
        policy.id,
        root_digest,
        Digest::of_bytes(spec.as_bytes()),
        FoundrySandboxSpec::new(FoundrySandboxKind::LocalDryRun, root_digest),
        checks,
        FoundryCommandPolicy::default_cargo_policy(),
        FoundryTimeoutPolicy::new(600_000, 2_400_000),
        FoundryEnvironmentPolicy::locked(),
    );
    let report = FoundryExecutor::new(args.path.as_str()).execute(&plan, &policy, root_digest);
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
        );
    } else {
        let c = |code: &'static str| if args.color { code } else { "" };
        println!(
            "{}Kosmocrates foundry — the gate executor{}",
            c(BOLD),
            c(RESET)
        );
        for r in &report.check_results {
            match &r.outcome {
                FoundryOutcome::Passed => {
                    println!(
                        "  {}\u{2713}{} {:?} passed",
                        c(GREEN),
                        c(RESET),
                        r.check_kind
                    )
                }
                FoundryOutcome::Failed { exit_code, .. } => println!(
                    "  {}\u{2717}{} {:?} failed (exit {})",
                    c(RED),
                    c(RESET),
                    r.check_kind,
                    exit_code
                ),
                other => println!("  \u{2013} {:?}: {:?}", r.check_kind, other),
            }
        }
        println!(
            "  outcome: {:?} \u{b7} {} ms \u{b7} evidence {}{}\u{2026}{}",
            report.outcome,
            report.elapsed_ms,
            c(DIM),
            &report.evidence_bundle_id.to_hex()[..12],
            c(RESET)
        );
    }
    Ok(
        if matches!(report.outcome, FoundryExecutionOutcome::Passed) {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(6)
        },
    )
}

/// `--witness "<argv>"`: one execution of the workspace's binary under the
/// sandbox witness (cwd-confined, 60s budget, output capped but
/// digest-complete) — the raw, content-addressed evidence of a run.
fn run_witness_mode(args: &Args) -> Result<ExitCode, String> {
    let argv: Vec<String> = args
        .witness
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let manifest = Path::new(&args.path).join("Cargo.toml");
    if !manifest.exists() {
        return Err(format!(
            "--witness needs a cargo workspace ({} has no Cargo.toml)",
            args.path
        ));
    }
    let mut run_args = vec![
        "run".to_string(),
        "--quiet".to_string(),
        "--manifest-path".to_string(),
        manifest.to_string_lossy().into_owned(),
        "--".to_string(),
    ];
    run_args.extend(argv);
    let sandbox = Sandbox::new()
        .with_cwd(Path::new(&args.path))
        .with_timeout(std::time::Duration::from_secs(60));
    let w = sandbox.run(&RunSpec::new("cargo", run_args));
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "verdict": format!("{:?}", w.verdict),
                "exit_code": w.exit_code,
                "duration_ms": w.duration.as_millis() as u64,
                "stdout_digest": w.stdout_digest.to_hex(),
                "truncated": w.truncated,
                "stdout": w.stdout,
                "stderr": w.stderr,
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        let c = |code: &'static str| if args.color { code } else { "" };
        println!(
            "{}Kosmocrates witness — one execution, content-addressed{}",
            c(BOLD),
            c(RESET)
        );
        println!(
            "  verdict {:?} \u{b7} exit {:?} \u{b7} {} ms{}",
            w.verdict,
            w.exit_code,
            w.duration.as_millis(),
            if w.truncated {
                " \u{b7} output truncated (the digest still covers all of it)"
            } else {
                ""
            }
        );
        println!("  stdout digest {}", w.stdout_digest.to_hex());
        for line in w.stdout.lines().take(20) {
            println!("  | {line}");
        }
        if !w.stderr.trim().is_empty() {
            for line in w.stderr.lines().take(10) {
                println!("  ! {line}");
            }
        }
    }
    Ok(if w.succeeded() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(7)
    })
}

/// `--parseback [--parseback-baseline <f>]`: the topology eye as a door —
/// a content-addressed snapshot of the workspace (crates, files,
/// dependency edges) and, against a stored baseline, the severity-ranked
/// drift. The baseline is written once and never silently replaced.
fn run_parseback_mode(args: &Args) -> Result<ExitCode, String> {
    let executor = ParseBackExecutor::new(PathBuf::from(&args.path));
    let now = executor
        .snapshot(&ParseBackScanScope::FullWorkspace)
        .map_err(|e| format!("parseback: could not snapshot {}: {e:?}", args.path))?;
    let c = |code: &'static str| if args.color { code } else { "" };
    let summary = |snap: &TopologySnapshot| {
        format!(
            "{} crate(s), {} dependency edge(s) \u{b7} snapshot {}\u{2026}",
            snap.crate_count(),
            snap.dep_edges.len(),
            &snap.snapshot_id.to_hex()[..12]
        )
    };
    match args.parseback_baseline.as_deref() {
        None => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&now).map_err(|e| e.to_string())?
                );
            } else {
                println!(
                    "{}Kosmocrates parseback — the topology eye{}",
                    c(BOLD),
                    c(RESET)
                );
                println!("  {}", summary(&now));
                println!("  (give --parseback-baseline <file> to persist and diff)");
            }
        }
        Some(p) if !Path::new(p).exists() => {
            let body = serde_json::to_string_pretty(&now).map_err(|e| e.to_string())?;
            fs::write(p, body).map_err(|e| format!("parseback: write {p}: {e}"))?;
            if !args.json {
                println!(
                    "{}Kosmocrates parseback — the topology eye{}",
                    c(BOLD),
                    c(RESET)
                );
                println!("  {}", summary(&now));
                println!("  baseline written to {p} — future runs report drift against it");
            }
        }
        Some(p) => {
            let pre: TopologySnapshot = serde_json::from_str(
                &fs::read_to_string(p).map_err(|e| format!("parseback: read {p}: {e}"))?,
            )
            .map_err(|e| format!("parseback: {p} is not a topology snapshot: {e}"))?;
            let deltas = diff_snapshots(&pre, &now);
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "baseline": pre.snapshot_id.to_hex(),
                        "current": now.snapshot_id.to_hex(),
                        "deltas": deltas.iter().map(|d| serde_json::json!({
                            "kind": format!("{:?}", d.change_kind),
                            "severity": format!("{:?}", d.severity),
                            "description": d.description,
                        })).collect::<Vec<_>>(),
                    }))
                    .map_err(|e| e.to_string())?
                );
            } else {
                println!(
                    "{}Kosmocrates parseback — the topology eye{}",
                    c(BOLD),
                    c(RESET)
                );
                println!(
                    "  baseline {}\u{2026} \u{2192} current {}\u{2026}",
                    &pre.snapshot_id.to_hex()[..12],
                    &now.snapshot_id.to_hex()[..12]
                );
                if deltas.is_empty() {
                    println!("  {}\u{2713} no topology drift{}", c(GREEN), c(RESET));
                } else {
                    for d in &deltas {
                        println!("  [{:?}] {}", d.severity, d.description);
                    }
                }
                println!("  (the baseline is never silently replaced — delete {p} to rebaseline)");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// `--kcube <dir>`: the blueprint exporter as a door — run the full
/// diagnosis, build the workspace's SystemCube, and export it as a real,
/// roundtrip-verified `.kcube` archive. The explicit flag is the
/// operator's materialization approval; silent overwrite is refused.
fn run_kcube_mode(args: &Args) -> Result<ExitCode, String> {
    let out_dir = args.kcube.as_deref().expect("dispatch checked");
    fs::create_dir_all(out_dir).map_err(|e| format!("kcube: create {out_dir}: {e}"))?;
    let policy = PolicyProfile::operator_approved_with_systemcube();
    let options = IntegrationRunOptions::all_layers(args.capacity);
    let report = run_workspace_pipeline(&args.path, &options, &policy)
        .map_err(|e| format!("pipeline failed on {}: {e}", args.path))?;
    let cube = report
        .systemcube
        .as_ref()
        .ok_or("kcube: the pipeline yielded no SystemCube")?;
    let export_policy = KcubeExportPolicy::write_once(
        policy.id,
        Digest::of_bytes(out_dir.as_bytes()),
        vec![
            KcubeArtifactKind::CartographyManifest,
            KcubeArtifactKind::ValidationClosureReport,
            KcubeArtifactKind::StructuralCrystal,
        ],
    );
    let executor = KcubeExecutor::new(out_dir);
    let w = cube.export_to_kcube(
        &executor,
        args.capacity,
        &export_policy,
        &policy,
        report.report_id,
        1,
    );
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&w).map_err(|e| e.to_string())?
        );
    } else {
        let c = |code: &'static str| if args.color { code } else { "" };
        println!(
            "{}Kosmocrates kcube — the blueprint archive{}",
            c(BOLD),
            c(RESET)
        );
        match &w.outcome {
            KcubeWriteOutcome::Written => {
                let file = kosmo_kcube::kcube_file_name(
                    &format!("systemcube-{}", &cube.cube_id.to_hex()[..16]),
                    1,
                );
                println!(
                    "  {}\u{2713}{} written \u{b7} {} \u{b7} package {}\u{2026} \u{b7} {} bytes \u{b7} roundtrip {}",
                    c(GREEN),
                    c(RESET),
                    Path::new(out_dir).join(file).display(),
                    &w.package_id.to_hex()[..12],
                    w.written_bytes,
                    if w.roundtrip.is_some() {
                        "verified"
                    } else {
                        "not verified"
                    }
                );
            }
            other => {
                println!("  {}\u{2717}{} {:?}", c(RED), c(RESET), other);
                for d in &w.diagnostics {
                    println!("    {d}");
                }
            }
        }
    }
    Ok(if matches!(w.outcome, KcubeWriteOutcome::Written) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(8)
    })
}

/// `--codematrix`: the 5D fingerprint lens as a door — per-source axes
/// (relationality, functional cohesion, topology, symmetry, entropy) and
/// the most resonant pairs. Strictly advisory: it ranks, it never gates
/// (CROSS-010); the floats below are display-only.
fn run_codematrix_mode(args: &Args) -> Result<ExitCode, String> {
    let root = Path::new(&args.path);
    let mut prints: Vec<(String, CodeMatrixFingerprint)> = Vec::new();
    collect_fingerprints(root, root, 0, &mut prints);
    prints.sort_by(|a, b| a.0.cmp(&b.0));
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "sources": prints.iter().map(|(loc, fp)| serde_json::json!({
                    "location": loc,
                    "axes_raw": fp.axes().map(|q| q.raw()),
                    "richness_raw": fp.richness().raw(),
                })).collect::<Vec<_>>(),
            }))
            .map_err(|e| e.to_string())?
        );
        return Ok(ExitCode::SUCCESS);
    }
    let c = |code: &'static str| if args.color { code } else { "" };
    println!(
        "{}Kosmocrates codematrix — the 5D fingerprint lens (advisory){}",
        c(BOLD),
        c(RESET)
    );
    if prints.is_empty() {
        println!("  no fingerprintable sources under {}", args.path);
        return Ok(ExitCode::SUCCESS);
    }
    println!(
        "  {} source(s) \u{b7} axes: relationality \u{b7} cohesion \u{b7} topology \u{b7} symmetry \u{b7} entropy",
        prints.len()
    );
    for (loc, fp) in &prints {
        let [r, f, t, s, e] = fp.axes();
        println!(
            "  {loc}  r={:.2} f={:.2} t={:.2} s={:.2} e={:.2}",
            r.to_f64(),
            f.to_f64(),
            t.to_f64(),
            s.to_f64(),
            e.to_f64()
        );
    }
    if prints.len() >= 2 && prints.len() <= 64 {
        let mut pairs: Vec<(Q16, &str, &str)> = Vec::new();
        for i in 0..prints.len() {
            for j in (i + 1)..prints.len() {
                pairs.push((
                    prints[i].1.resonance(&prints[j].1),
                    prints[i].0.as_str(),
                    prints[j].0.as_str(),
                ));
            }
        }
        pairs.sort_by(|a, b| b.0.raw().cmp(&a.0.raw()).then(a.1.cmp(b.1)));
        println!("  most resonant pairs:");
        for (res, a, b) in pairs.iter().take(5) {
            println!("    {:.2}  {a} \u{2194} {b}", res.to_f64());
        }
    } else if prints.len() > 64 {
        println!(
            "  {}(pairwise resonance skipped above 64 sources){}",
            c(DIM),
            c(RESET)
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// Bounded, deterministic source walk for the codematrix lens (the same
/// skip list as the language detector; entries sorted; capped).
fn collect_fingerprints(
    base: &Path,
    dir: &Path,
    depth: u32,
    out: &mut Vec<(String, CodeMatrixFingerprint)>,
) {
    const SKIP: &[&str] = &[
        ".git",
        "target",
        "node_modules",
        ".hg",
        ".svn",
        "vendor",
        "vendors",
    ];
    if depth > 8 || out.len() >= 400 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.path());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            let skip = p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| SKIP.contains(&n));
            if !skip {
                collect_fingerprints(base, &p, depth + 1, out);
            }
        } else {
            let loc = p
                .strip_prefix(base)
                .unwrap_or(&p)
                .to_string_lossy()
                .into_owned();
            if SourceLanguage::from_path(&loc).is_none() {
                continue;
            }
            let Ok(content) = fs::read_to_string(&p) else {
                continue;
            };
            if content.len() > 512 * 1024 {
                continue;
            }
            if let Some(fp) = CodeMatrixFingerprint::from_auto(
                Digest::of_bytes(content.as_bytes()),
                &loc,
                &content,
            ) {
                out.push((loc, fp));
            }
        }
    }
}

// ─── Norm organ (learned archetypes) ────────────────────────────────────────

/// Operator governance: `--inject-norm <file>` / `--promote-norm <id>
/// --trigger <word>`. The explicit flag IS the operator's approval — these
/// commands exist only as deliberate CLI invocations, never inside the agent
/// loop — so the store append runs under `operator_approved` regardless of
/// `--apply` (which governs *workspace* writes, not governance acts).
fn run_norm_admin(args: &Args) -> Result<ExitCode, String> {
    let dir = args
        .norms
        .as_deref()
        .ok_or("--inject-norm / --promote-norm require --norms <dir>")?;
    let mut store = NormStore::open(dir).map_err(|e| e.to_string())?;
    let policy = PolicyProfile::operator_approved();

    if let Some(ref file) = args.inject_norm {
        // The spec file's bytes ARE the evidence for the injected norm.
        let bytes = fs::read(file).map_err(|e| format!("read {file}: {e}"))?;
        let evidence = Digest::of_bytes(&bytes);
        let spec: NormInjectionSpec =
            serde_json::from_slice(&bytes).map_err(|e| format!("parse {file}: {e}"))?;
        let source = Path::new(file)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("injected")
            .to_string();
        let norm = spec
            .into_norm(evidence, policy.id, source)
            .map_err(|e| format!("invalid norm spec: {e}"))?;
        store
            .append_norm(&norm, &policy)
            .map_err(|e| e.to_string())?;
        println!(
            "injected norm {} ({}, {} facet template(s))",
            norm.norm_id.to_hex(),
            norm.name,
            norm.template.len()
        );
        println!(
            "  unarmed — activate with: --norms {dir} --promote-norm {} --trigger <word>",
            norm.norm_id.to_hex()
        );
        return Ok(ExitCode::SUCCESS);
    }

    let id_hex = args.promote_norm.as_deref().expect("dispatch checked");
    let word = args
        .trigger
        .as_deref()
        .ok_or("--promote-norm requires --trigger <word>")?;
    // The wish grammar lives in kosmo-intent; the reserved-word check is ours.
    if is_reserved_wish_word(word) {
        return Err(format!(
            "trigger '{word}' is reserved by the wish grammar — choose another word"
        ));
    }
    let norm_id = Digest::from_hex(id_hex)
        .ok_or_else(|| format!("--promote-norm: '{id_hex}' is not a norm id (hex digest)"))?;
    let promoted = store
        .promote(norm_id, word, &policy)
        .map_err(|e| e.to_string())?;
    println!(
        "promoted norm {} — trigger '{}' now expands {} facet template(s) in wish prose",
        promoted.norm_id.to_hex(),
        promoted.trigger.as_deref().unwrap_or(word),
        promoted.template.len()
    );
    Ok(ExitCode::SUCCESS)
}

/// The workspace's identity tag for learning observations: a digest of its
/// canonical path — no raw host path ever enters a durable artifact.
fn workspace_tag(path: &str) -> Digest {
    let canonical = fs::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string());
    Digest::of_bytes(canonical.as_bytes())
}

/// Source languages present under `root` (recursive, bounded), via the
/// fail-closed `SourceLanguage::from_path` detector. Vendor/VCS/build dirs
/// are skipped.
fn workspace_languages(root: &Path) -> Vec<String> {
    const SKIP: &[&str] = &[".git", "target", "node_modules", ".hg", ".svn", "vendor"];
    fn walk(dir: &Path, depth: u32, langs: &mut std::collections::BTreeSet<&'static str>) {
        if depth > 8 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if !SKIP.contains(&name.as_ref()) {
                    walk(&path, depth + 1, langs);
                }
            } else if let Some(lang) = SourceLanguage::from_path(&path.to_string_lossy()) {
                langs.insert(lang.as_str());
            }
        }
    }
    let mut langs = std::collections::BTreeSet::new();
    walk(root, 0, &mut langs);
    langs.into_iter().map(String::from).collect()
}

/// Record the finished descent as a learning observation and run the
/// promotion scan. Only called under `--apply` (the store requires
/// `allow_host_write`). Learning failures are reported, never fatal — the
/// descent's own verdict stands either way.
fn record_norm_observation(
    store: &mut NormStore,
    path: &str,
    wish: &Wish,
    realized: bool,
    evidence: Digest,
    policy: &PolicyProfile,
) {
    let facets: Vec<WishFacet> = wish.predicates.iter().map(|p| p.facet.clone()).collect();
    if facets.is_empty() {
        return; // a vacuous wish observes nothing
    }
    let obs = FacetBundleObservation::new(
        workspace_tag(path),
        facets,
        workspace_languages(Path::new(path)),
        realized,
        evidence,
        policy.id,
    );
    if let Err(e) = store.append_observation(&obs, policy) {
        eprintln!("norms: could not record observation: {e}");
        return;
    }
    // Every recording re-scans the corpus: shapes that crossed the
    // thresholds become stored — but UNARMED — norms.
    let proposals = promotable(
        store.observations(),
        &NormLearningConfig::default(),
        policy.id,
    );
    for p in proposals {
        if store.get(&p.norm.norm_id).is_some() {
            continue; // already learned
        }
        match store.append_norm(&p.norm, policy) {
            Ok(()) => {
                println!(
                    "norm learned: {} — {}",
                    p.norm.norm_id.to_hex(),
                    p.norm.description
                );
                println!(
                    "  unarmed — activate with: --norms <dir> --promote-norm {} --trigger <word>",
                    p.norm.norm_id.to_hex()
                );
            }
            Err(e) => eprintln!("norms: could not store learned norm: {e}"),
        }
    }
}

fn run_wish_mode(args: &Args) -> Result<ExitCode, String> {
    let prose = args.wish.as_deref().unwrap_or("");
    // Bind the wish's identity to its prose — content-addressed, deterministic.
    let evidence = Digest::of_bytes(prose.as_bytes());
    // With a norm store, promoted triggers join the grammar; without one this
    // is the untouched deterministic front door (and an empty catalog is
    // pinned byte-identical to it).
    let mut norm_store = match args.norms.as_deref() {
        Some(dir) => Some(NormStore::open(dir).map_err(|e| e.to_string())?),
        None => None,
    };
    let wish = match &norm_store {
        Some(store) => {
            let catalog = NormCatalog::from_norms(store.norms()).map_err(|e| e.to_string())?;
            compile_wish_with_norms(prose, &catalog, Digest::ZERO, evidence)
        }
        None => compile_wish(prose, Digest::ZERO, evidence),
    };

    // A behaviour facet is satisfiable only by a *passing* test, so any wish
    // that carries one forces validated observation (run the suite), whether or
    // not --validated was given — the keystone demands it.
    let validated = args.validated || wish_needs_validation(&wish);

    // Run 10 — graduate the cube view to the default human render. The layered
    // hypercube (with its Konus focus and Run 9 honesty verdict) and the
    // staged-closure film now ride every human wish-run; `--flat` is the opt-out
    // to the terse verdict that used to be the default. The headline summary
    // (flat assessment / descent report) always prints either way, so scripts
    // and the verdict strings keep their contract. The machine channel (--json)
    // is untouched — it still selects serialization by explicit flag.
    let show_layers = args.layers || args.staged || !args.flat;
    let show_staged = args.staged || !args.flat;

    // --apply turns wish mode into a descent: observe → scaffold → apply →
    // re-observe, until the wish is realized. This WRITES to the workspace.
    if args.apply {
        // LLM fallback for facets the deterministic scaffolder can't build —
        // provider-gated, memory-armed under --ledger (see wish_fallback).
        let fallback = wish_fallback(args)?;
        let prior = args
            .wish_session
            .as_deref()
            .and_then(|p| load_prior_session(p, &wish));
        let session = if args.staged {
            descend_staged(&args.path, &wish, evidence, validated, 8, fallback.as_deref(), prior)?
        } else {
            descend_to_wish(&args.path, &wish, evidence, validated, 8, fallback.as_deref(), prior)?
        };
        if let Some(ref sp) = args.wish_session {
            save_session(sp, &session)?;
        }
        if args.json {
            if args.mesh {
                let d = observe_workspace_deep(&args.path)
                    .map(|o| topology_density(&o, args.capacity))
                    .unwrap_or(Q16::ZERO);
                let reading = session
                    .latest_cube()
                    .map(|c| CubeMeshReading::read(c, d, evidence));
                let json = serde_json::to_string_pretty(&reading)
                    .map_err(|e| format!("failed to serialize mesh reading: {e}"))?;
                println!("{json}");
            } else if args.staged {
                let report = StagedClosureReport::from_descent(
                    session.cubes(),
                    &session.layered_trace(),
                    evidence,
                );
                let json = serde_json::to_string_pretty(&report)
                    .map_err(|e| format!("failed to serialize staged report: {e}"))?;
                println!("{json}");
            } else if args.layers {
                let json = serde_json::to_string_pretty(session.cubes())
                    .map_err(|e| format!("failed to serialize cubes: {e}"))?;
                println!("{json}");
            } else {
                let json = serde_json::to_string_pretty(session.assessments())
                    .map_err(|e| format!("failed to serialize assessments: {e}"))?;
                println!("{json}");
            }
        } else {
            // The topology gear: one read-only re-observation after the descent,
            // shared by the cube-mode honesty verdict (Run 9) and --mesh (Run 8).
            let mesh_density = if show_layers || args.mesh {
                observe_workspace_deep(&args.path)
                    .ok()
                    .map(|o| topology_density(&o, args.capacity))
            } else {
                None
            };
            // The descent summary is the headline scripts gate on — always shown.
            print!("{}", descent_report(&session, args.color));
            // Run 10 — the cube view rides every human descent by default: the
            // layered hypercube (Konus focus + Run 9 honesty verdict) and, atop a
            // staged descent, the Solve→Gate→Coagula film. --flat suppresses both.
            if show_layers {
                print!(
                    "{}",
                    layered_descent_report(&session, mesh_density, args.color)
                );
                if show_staged {
                    let report = StagedClosureReport::from_descent(
                        session.cubes(),
                        &session.layered_trace(),
                        evidence,
                    );
                    print!("{}", staged_closure_render(&report, args.color));
                }
            }
            if args.mesh {
                if let Some(c) = session.latest_cube() {
                    let reading =
                        CubeMeshReading::read(c, mesh_density.unwrap_or(Q16::ZERO), evidence);
                    print!("{}", mesh_report(&reading, args.color));
                }
            }
        }
        let realized = session.latest().is_some_and(|a| {
            matches!(
                a.status,
                WishClosureStatus::Realized | WishClosureStatus::Vacuous
            )
        });
        // Learning: a finished --apply descent is one facet-bundle sighting.
        // --apply maps to operator_approved, the policy the store requires.
        if let Some(ref mut store) = norm_store {
            record_norm_observation(
                store,
                &args.path,
                &wish,
                realized,
                evidence,
                &PolicyProfile::operator_approved(),
            );
        }
        return Ok(if realized {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        });
    }

    let observed = if wish_needs_service(&wish) {
        observe_workspace_service(args.path.as_str())
    } else if wish_needs_runtime(&wish) {
        observe_workspace_runtime(args.path.as_str())
    } else if validated {
        observe_workspace_validated(args.path.as_str())
    } else {
        observe_workspace_deep(args.path.as_str())
    }
    .map_err(|e| format!("could not observe {}: {e}", args.path))?;

    let assessment = assess_wish(&wish, &observed, evidence);

    // Persist a single-step session when requested — even without --apply this
    // gives the caller an auditable record of the current workspace state.
    if let Some(ref sp) = args.wish_session {
        let mut one_step = WishSession::new(wish.clone(), evidence);
        one_step.observe(&observed);
        save_session(sp, &one_step)?;
    }

    if args.json {
        if args.mesh {
            let cube = assess_wish_layered(&wish, &observed, evidence);
            let d = topology_density(&observed, args.capacity);
            let reading = CubeMeshReading::read(&cube, d, evidence);
            let json = serde_json::to_string_pretty(&reading)
                .map_err(|e| format!("failed to serialize mesh reading: {e}"))?;
            println!("{json}");
        } else if args.layers {
            let cube = assess_wish_layered(&wish, &observed, evidence);
            let json = serde_json::to_string_pretty(&cube)
                .map_err(|e| format!("failed to serialize cube: {e}"))?;
            println!("{json}");
        } else {
            let json = serde_json::to_string_pretty(&assessment)
                .map_err(|e| format!("failed to serialize assessment: {e}"))?;
            println!("{json}");
        }
    } else {
        // The flat verdict is the headline scripts gate on — always shown.
        print!("{}", wish_report(&wish, &assessment, args.color));
        // The topology gear, computed once from this observation (free — no
        // re-observe), drives both the cube-mode honesty verdict and --mesh.
        let mesh_density = if show_layers || args.mesh {
            Some(topology_density(&observed, args.capacity))
        } else {
            None
        };
        // Run 10 — the layered hypercube (Konus + Run 9 honesty verdict) rides
        // every human read-only run by default; --flat falls back to the verdict.
        if show_layers {
            let mut one = WishSession::new(wish.clone(), evidence);
            one.observe_layered(&observed);
            print!("{}", layered_descent_report(&one, mesh_density, args.color));
        }
        if args.mesh {
            let cube = assess_wish_layered(&wish, &observed, evidence);
            let reading = CubeMeshReading::read(&cube, mesh_density.unwrap_or(Q16::ZERO), evidence);
            print!("{}", mesh_report(&reading, args.color));
        }
        if args.scaffold && !assessment.unmet_facets.is_empty() {
            print!(
                "{}",
                scaffold_report(&args.path, &assessment.unmet_facets, args.color)
            );
        }
    }

    // Exit 0 only when the wish is realized — so scripts can gate on it.
    match assessment.status {
        WishClosureStatus::Realized | WishClosureStatus::Vacuous => Ok(ExitCode::SUCCESS),
        _ => Ok(ExitCode::from(1)),
    }
}

/// Render a wish assessment as human-readable text (returned, not printed, so
/// it is unit-testable).
fn wish_report(wish: &Wish, a: &WishAssessment, color: bool) -> String {
    let c = |code: &'static str| if color { code } else { "" };
    let (label, col) = match a.status {
        WishClosureStatus::Realized => ("REALIZED ✓", c(GREEN)),
        WishClosureStatus::Approaching => ("APPROACHING", c(YELLOW)),
        WishClosureStatus::Unstarted => ("UNSTARTED", c(RED)),
        WishClosureStatus::Vacuous => ("VACUOUS (no predicates)", c(DIM)),
    };
    let mut out = String::new();
    out.push_str(&format!(
        "{}{}Kosmocrates wish{}\n",
        c(BOLD),
        c(CYAN),
        c(RESET)
    ));
    out.push_str(&format!("  \u{201c}{}\u{201d}\n", wish.label));
    out.push_str(&format!(
        "  status {}{}{}   met {}/{}\n",
        col,
        label,
        c(RESET),
        a.met_count,
        a.total_count
    ));
    if a.unmet_facets.is_empty() {
        out.push_str(&format!(
            "  {}all wished facets are present.{}\n",
            c(GREEN),
            c(RESET)
        ));
    } else {
        out.push_str("  missing:\n");
        for f in &a.unmet_facets {
            out.push_str(&format!(
                "    {}\u{2717}{} {:?} {}\n",
                c(RED),
                c(RESET),
                f.kind,
                f.key
            ));
        }
    }
    out
}

/// Render the deterministic scaffold (dry run) for the unmet facets — the
/// `FacetScaffolder`'s proposed file changes, never written to disk here.
fn scaffold_report(path: &str, unmet: &[WishFacet], color: bool) -> String {
    let c = |code: &'static str| if color { code } else { "" };
    let mut out = String::new();
    out.push_str(&format!(
        "\n{}scaffold{} {}(dry run — changes that would close the gap){}\n",
        c(BOLD),
        c(RESET),
        c(DIM),
        c(RESET)
    ));
    for facet in unmet {
        let action = ActionItem {
            action_id: Digest::ZERO,
            priority_score: Q16::ONE,
            kind: ActionItemKind::RealizeWishFacet {
                facet: facet.clone(),
            },
            description: format!("realize {:?} {}", facet.kind, facet.key),
            policy_id: Digest::ZERO,
        };
        let req = SynthesisRequest::new(action, path.to_string());
        match FacetScaffolder.synthesize(&req) {
            Ok(result) if !result.patch.is_empty() => {
                for fc in &result.patch.file_changes {
                    out.push_str(&format!(
                        "  {}{:?}{} {} ({} ln)  \u{2192} {:?} {}\n",
                        c(CYAN),
                        fc.kind,
                        c(RESET),
                        fc.path.to_string_lossy(),
                        fc.line_count(),
                        facet.kind,
                        facet.key
                    ));
                }
            }
            Ok(_) => out.push_str(&format!(
                "  {}\u{2014}{} no deterministic scaffold for {:?} {}\n",
                c(DIM),
                c(RESET),
                facet.kind,
                facet.key
            )),
            Err(e) => out.push_str(&format!("  scaffold error for {}: {e}\n", facet.key)),
        }
    }
    out
}

/// Realize every unmet facet and write the result to disk (relative to `root`).
/// Deterministic first: the [`FacetScaffolder`] builds structural facets exactly.
/// For facets it cannot build (e.g. a dependency edge), an optional `fallback`
/// synthesizer is consulted — the LLM end of the same `Wish → Patch` contract.
/// Returns the number of files written. The only place wish mode touches the
/// filesystem, and only under `--apply`.
/// Read the workspace's source files into snippets so a repair request can show
/// the model exactly what exists — what to fix, rename, or delete. Bounded so a
/// large tree cannot blow up the prompt; `target/` and dotfiles are skipped.
fn workspace_snippets(root: &Path) -> Vec<SourceSnippet> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<SourceSnippet>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.path());
        for e in entries {
            if out.len() >= 24 {
                return;
            }
            let p = e.path();
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name == "target" || name.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                walk(&p, root, out);
            } else if matches!(p.extension().and_then(|x| x.to_str()), Some("rs" | "toml")) {
                if let Ok(text) = fs::read_to_string(&p) {
                    let content = if text.len() > 4096 {
                        let mut end = 4096;
                        while !text.is_char_boundary(end) {
                            end -= 1;
                        }
                        format!("{}\n… (truncated)", &text[..end])
                    } else {
                        text
                    };
                    let rel = p.strip_prefix(root).unwrap_or(&p).to_path_buf();
                    out.push(SourceSnippet {
                        path: rel,
                        content,
                        relevance_score: Q16::ONE,
                    });
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

fn apply_synthesis(
    root: &Path,
    unmet: &[WishFacet],
    fallback: Option<&dyn ActionSynthesizer>,
    repair: Option<&str>,
) -> std::io::Result<usize> {
    let mut written = 0;
    // During a repair, show the model the current files so it can see what to
    // fix or delete. Normal iterations stay context-light to preserve the
    // baseline behaviour the bench measures.
    let repair_snippets = if repair.is_some() {
        workspace_snippets(root)
    } else {
        Vec::new()
    };
    // A wish opts into crate dependencies via KOSMO_DEPS_ALLOWED; off by default,
    // so synthesis stays std-only (fast, no crates.io fetch).
    let deps_allowed = std::env::var("KOSMO_DEPS_ALLOWED").is_ok();
    for facet in unmet {
        let action = ActionItem {
            action_id: Digest::ZERO,
            priority_score: Q16::ONE,
            kind: ActionItemKind::RealizeWishFacet {
                facet: facet.clone(),
            },
            description: format!("realize {:?} {}", facet.kind, facet.key),
            policy_id: Digest::ZERO,
        };
        let mut req = SynthesisRequest::new(action, root.to_string_lossy().to_string())
            .with_deps_allowed(deps_allowed);
        if let Some(err) = repair {
            req = req
                .with_repair_diagnostics(vec![err.to_string()])
                .with_snippets(repair_snippets.clone());
        }

        // Deterministic scaffolder first; consult the model only if it built
        // nothing — except in a repair, where only the model can clean up its
        // own mess (delete a stray binary, rewrite a broken file).
        let mut changes = if repair.is_some() {
            Vec::new()
        } else {
            FacetScaffolder
                .synthesize(&req)
                .map(|r| r.patch.file_changes)
                .unwrap_or_default()
        };
        if changes.is_empty() {
            if let Some(synth) = fallback {
                match synth.synthesize(&req) {
                    Ok(result) => {
                        // Run 6: when a swarm chose this patch, surface the
                        // Ophanim/Konus/Monolith resonance — the operator watches
                        // the ensemble converge (sealed as a replayable reading).
                        if let Some(report) = &result.consensus {
                            let reading = ResonanceReading::seal(
                                report,
                                &ConsensusConfig::default(),
                                Digest::of_bytes(facet.key.as_bytes()),
                            );
                            eprintln!(
                                "  \u{2299} ophanim resonance: {} perspectives \u{00b7} d_total={:.3} \u{03b8}={:.3} \u{21d2} {}",
                                reading.perspectives(),
                                reading.d_total.to_f64(),
                                reading.theta.to_f64(),
                                if reading.convergent {
                                    "CONVERGENT"
                                } else {
                                    "DIVERGENT"
                                }
                            );
                        }
                        changes = result.patch.file_changes;
                    }
                    // Surface the error instead of silently swallowing it: a
                    // swallowed transport error is indistinguishable from "the
                    // model produced nothing", which is exactly what masked the
                    // provider misconfiguration this benchmark is meant to catch.
                    Err(e) => {
                        eprintln!(
                            "  fallback synthesis failed for {}: {}",
                            facet.key, e.message
                        )
                    }
                }
            }
        }
        for fc in &changes {
            let target = root.join(&fc.path);
            match fc.kind {
                // Honour delete ops — essential to a repair (removing the stray
                // binary that collided). The old writer turned a delete into an
                // empty file, which would not have resolved the collision.
                FileChangeKind::Delete => {
                    fs::remove_file(&target).ok();
                }
                _ => {
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&target, &fc.content)?;
                }
            }
            written += 1;
        }
    }
    Ok(written)
}

/// The curriculum target for a staged descent: the unmet facets of the
/// *shallowest* non-solid stratum (solidify Existence before chasing a Run
/// probe). Falls back to the flat unmet set when there is no cube yet or every
/// non-empty stratum is already solid. Ranks/orders effort; it never forbids a
/// facet (CROSS-010).
fn staged_target(cube: Option<&WishCube>, flat_unmet: &[WishFacet]) -> Vec<WishFacet> {
    if let Some(c) = cube {
        for view in &c.layers {
            if !view.is_empty_layer() && !view.is_solid() {
                return view.unmet_facets.clone();
            }
        }
    }
    flat_unmet.to_vec()
}

/// Descend toward a wish: observe → scaffold/synthesize → re-observe until
/// realized (flat targeting — the established loop). See [`descend_staged`] for
/// the layered Solve→Gate→Coagula variant.
fn descend_to_wish(
    path: &str,
    wish: &Wish,
    evidence: Digest,
    validated: bool,
    max_iters: u32,
    fallback: Option<&dyn ActionSynthesizer>,
    prior: Option<WishSession>,
) -> Result<WishSession, String> {
    descend_inner(path, wish, evidence, validated, max_iters, fallback, prior, false)
}

/// Descend as a **staged closure pipeline** (Run 4): synthesis targets the
/// shallowest non-solid stratum first, so the wish coagulates bottom-up (the
/// "print → debind → sinter" curriculum). Same observation/contraction contract
/// as [`descend_to_wish`]; only the targeting order differs. The session it
/// returns carries the cube film [`StagedClosureReport::from_descent`] folds.
fn descend_staged(
    path: &str,
    wish: &Wish,
    evidence: Digest,
    validated: bool,
    max_iters: u32,
    fallback: Option<&dyn ActionSynthesizer>,
    prior: Option<WishSession>,
) -> Result<WishSession, String> {
    descend_inner(path, wish, evidence, validated, max_iters, fallback, prior, true)
}

/// Drive the workspace toward `wish` by repeated observe → assess → scaffold →
/// apply, until it is realized, no further progress is possible, or `max_iters`
/// is reached. Returns the [`WishSession`] carrying the full convergence
/// trajectory — the attractor descent, executed. `prior` resumes an earlier
/// descent; `staged` selects the bottom-up curriculum (see [`staged_target`]).
#[allow(clippy::too_many_arguments)]
fn descend_inner(
    path: &str,
    wish: &Wish,
    evidence: Digest,
    validated: bool,
    max_iters: u32,
    fallback: Option<&dyn ActionSynthesizer>,
    prior: Option<WishSession>,
    staged: bool,
) -> Result<WishSession, String> {
    let mut session = prior.unwrap_or_else(|| WishSession::new(wish.clone(), evidence));
    let mut iter = 0u32;
    // The unmet facets from the last *successful* observation. On a later observe
    // failure these drive the repair attempt (we have no fresh assessment then).
    let mut last_unmet: Vec<WishFacet> = Vec::new();
    loop {
        // Directed diagnostics from this observation: a build failure, a probe
        // that ran but produced the wrong result, or a service that never
        // answered (or answered wrong). Fed back next iteration so the model
        // repairs directly instead of guessing.
        let mut run_diag: Option<String> = None;
        let observation = if wish_needs_service(wish) {
            match observe_workspace_service_diag(path) {
                Ok((observed, diag)) => {
                    run_diag = diag;
                    Ok(observed)
                }
                Err(e) => Err(e),
            }
        } else if wish_needs_runtime(wish) {
            match observe_workspace_runtime_diag(path) {
                Ok((observed, diag)) => {
                    run_diag = diag;
                    Ok(observed)
                }
                Err(e) => Err(e),
            }
        } else if validated {
            observe_workspace_validated(path)
        } else {
            observe_workspace_deep(path)
        };
        let observed = match observation {
            Ok(o) => o,
            Err(e) => {
                // A prior synthesis iteration left the workspace unobservable —
                // e.g. an unparseable Cargo.toml (duplicate binary names). A
                // failure on the very first observation (a workspace that was
                // never usable) is fatal. Otherwise self-heal: feed the error
                // back so the model repairs its own mess, and only give up —
                // gracefully, at the last good state — once the budget is spent
                // or there is nothing left to drive a repair.
                if iter == 0 {
                    return Err(format!("could not observe {path}: {e}"));
                }
                if iter >= max_iters || last_unmet.is_empty() {
                    break;
                }
                let diag = format!("could not build/observe the workspace: {e}");
                let repaired =
                    apply_synthesis(Path::new(path), &last_unmet, fallback, Some(diag.as_str()))
                        .map_err(|e| e.to_string())?;
                if repaired == 0 {
                    break;
                }
                iter += 1;
                continue;
            }
        };

        // Always render the layered cube alongside the flat assessment — cubes
        // are cheap and give every descent a replayable per-stratum film.
        session.observe_layered(&observed);
        let assessment = session.latest().expect("just observed").clone();
        let done = matches!(
            assessment.status,
            WishClosureStatus::Realized | WishClosureStatus::Vacuous
        );
        let unmet = assessment.unmet_facets.clone();

        if done || unmet.is_empty() || iter >= max_iters {
            break;
        }
        last_unmet = unmet.clone();
        // Staged descent solidifies the shallowest non-solid stratum first, then
        // focuses it foundation-first via the precedence lens (Run 5: realize the
        // crate before its modules, both endpoints before a dependency edge). The
        // flat descent attacks the whole unmet set in its existing order, so the
        // bench-measured path stays byte-identical.
        let target = if staged {
            let stratum = staged_target(session.latest_cube(), &unmet);
            let stratum = if stratum.is_empty() {
                unmet.clone()
            } else {
                stratum
            };
            PrecedenceOrder::focus(&stratum, wish.id, evidence).ordered_facets()
        } else {
            unmet.clone()
        };
        let written = apply_synthesis(Path::new(path), &target, fallback, run_diag.as_deref())
            .map_err(|e| e.to_string())?;
        if written == 0 {
            break; // nothing scaffoldable — can't make progress, fail-closed
        }
        iter += 1;
    }
    Ok(session)
}

/// Render the descent trajectory: one line per iteration plus the verdict.
fn descent_report(session: &WishSession, color: bool) -> String {
    let c = |code: &'static str| if color { code } else { "" };
    let mut out = String::new();
    out.push_str(&format!(
        "{}{}Kosmocrates wish — descent{}\n",
        c(BOLD),
        c(CYAN),
        c(RESET)
    ));
    out.push_str(&format!("  \u{201c}{}\u{201d}\n", session.wish().label));
    for (i, a) in session.assessments().iter().enumerate() {
        let (label, col) = match a.status {
            WishClosureStatus::Realized => ("REALIZED \u{2713}", c(GREEN)),
            WishClosureStatus::Approaching => ("APPROACHING", c(YELLOW)),
            WishClosureStatus::Unstarted => ("UNSTARTED", c(RED)),
            WishClosureStatus::Vacuous => ("VACUOUS", c(DIM)),
        };
        out.push_str(&format!(
            "  iter {}: met {}/{}  {}{}{}\n",
            i,
            a.met_count,
            a.total_count,
            col,
            label,
            c(RESET)
        ));
    }
    let realized = session.latest().is_some_and(|a| {
        matches!(
            a.status,
            WishClosureStatus::Realized | WishClosureStatus::Vacuous
        )
    });
    if realized {
        out.push_str(&format!(
            "  {}\u{2713} wish realized.{}\n",
            c(GREEN),
            c(RESET)
        ));
    } else if let Some(a) = session.latest() {
        if !a.unmet_facets.is_empty() {
            out.push_str("  still missing:\n");
            for f in &a.unmet_facets {
                out.push_str(&format!(
                    "    {}\u{2717}{} {:?} {}\n",
                    c(RED),
                    c(RESET),
                    f.kind,
                    f.key
                ));
            }
        }
    }
    out
}

/// A unicode bar filled to `opacity` (0 → empty, ONE → full) over `width` cells.
fn opacity_bar(opacity: Q16, width: usize) -> String {
    let filled = ((opacity.to_f64() * width as f64).round() as usize).min(width);
    let mut s = String::with_capacity(width * 3);
    for _ in 0..filled {
        s.push('\u{2588}'); // █
    }
    for _ in filled..width {
        s.push('\u{2591}'); // ░
    }
    s
}

/// The render word for one stratum: solid / rendering / transparent / empty.
fn layer_state_word(view: &kosmo_core::WishLayerView) -> &'static str {
    if view.is_empty_layer() {
        "—"
    } else if view.is_solid() {
        "solid"
    } else if view.met_count > 0 {
        "rendering"
    } else {
        "transparent"
    }
}

/// The topology gear's judgement of a *realized* wish (Runs 9 / 11 / 12), as one
/// line — `None` until the wish has fully solidified. Calibrated by confidence:
///
/// - **dense backing** → `genuine`, a cut diamond.
/// - **sparse backing under a deep (earned) claim** → an `over-fit suspect`, not
///   a verdict: a passing test or run is execution-earned, so sparse topology is
///   only a weak proxy for a *stub* probe — the line names that residual risk
///   instead of falsely declaring the (genuinely realized) wish a hologram.
/// - **sparse backing under a shallow declarative claim** → honest: a small wish
///   in a small workspace, no alarm.
///
/// Definitive structural holograms (a stratum floating above a hollow base) are
/// surfaced separately by the caller and keep the wish from solidifying at all,
/// so they never reach here. Advisory (CROSS-010).
fn honesty_verdict(cube: &WishCube, d: Q16, color: bool) -> Option<String> {
    if cube.structural_solidity != Q16::ONE {
        return None;
    }
    let c = |code: &'static str| if color { code } else { "" };
    let q = Q16::ratio(1, 4).unwrap_or(Q16::ZERO);
    let deepest = cube.solid_frontier();
    let claim = deepest.map(|l| l.label()).unwrap_or("nothing");
    let deep = deepest
        .map(|l| l.rank() >= kosmo_core::WishLayer::Verified.rank())
        .unwrap_or(false);
    let line = if d.at_least(q) {
        format!(
            "  {}\u{2713} genuine \u{2014} wish solid and topology dense ({:.3}): a cut diamond{}\n",
            c(GREEN),
            d.to_f64(),
            c(RESET)
        )
    } else if deep {
        format!(
            "  {}\u{26a0} over-fit suspect \u{2014} a {} claim stands over a sparse topology ({:.3}); confirm the probe is substantive, not a stub (a hologram passes too){}\n",
            c(YELLOW),
            claim,
            d.to_f64(),
            c(RESET)
        )
    } else {
        format!(
            "  {}\u{00b7} thin but shallow \u{2014} only {} was claimed; a sparse topology ({:.3}) is honest for so shallow a wish{}\n",
            c(DIM),
            claim,
            d.to_f64(),
            c(RESET)
        )
    };
    Some(line)
}

/// Render the latest cube as a 3-D-printer: one bar per stratum, filled to its
/// opacity, plus the geomean-solidity gauge and the per-layer convergence
/// verdict — the human watches the hologram become a solid diamond. When
/// `mesh_density` is `Some` and the wish has fully solidified, the topology gear
/// judges it via [`honesty_verdict`] — calibrated by confidence (Run 12): dense
/// backing is genuine, a sparse deep claim is only a *suspect* (name the stub
/// risk, don't cry hologram), a sparse shallow claim is honest. Advisory.
fn layered_descent_report(
    session: &WishSession,
    mesh_density: Option<Q16>,
    color: bool,
) -> String {
    let c = |code: &'static str| if color { code } else { "" };
    let mut out = String::new();
    out.push_str(&format!(
        "{}{}Kosmocrates wish \u{2014} hypercube render{}\n",
        c(BOLD),
        c(CYAN),
        c(RESET)
    ));
    out.push_str(&format!("  \u{201c}{}\u{201d}\n", session.wish().label));
    let Some(cube) = session.latest_cube() else {
        out.push_str("  (no render yet)\n");
        return out;
    };
    for view in &cube.layers {
        let col = if view.is_solid() {
            c(GREEN)
        } else if view.met_count > 0 {
            c(YELLOW)
        } else {
            c(DIM)
        };
        out.push_str(&format!(
            "  {:<10} {}{}{}  {}{:<11}{} {}/{}  {}(opacity {:.3}){}\n",
            view.layer.label(),
            col,
            opacity_bar(view.opacity, 12),
            c(RESET),
            col,
            layer_state_word(view),
            c(RESET),
            view.met_count,
            view.total_count,
            c(DIM),
            view.opacity.to_f64(),
            c(RESET),
        ));
    }
    let frontier = cube.solid_frontier().map(|l| l.label()).unwrap_or("none");
    out.push_str(&format!(
        "  {}\u{2500}\u{2500} solidity(geomean) {:.3} \u{00b7} overall {:.3} \u{00b7} frontier: {}{}\n",
        c(DIM),
        cube.structural_solidity.to_f64(),
        cube.overall_opacity.to_f64(),
        frontier,
        c(RESET)
    ));
    if cube.has_floating_layer() {
        out.push_str(&format!(
            "  {}\u{26a0} a stratum renders above a still-transparent base (over-fit suspect){}\n",
            c(YELLOW),
            c(RESET)
        ));
    }
    // The descent-side Konus: the foundation facet the Solve stage targets next.
    if let Some(next_stratum) = cube
        .layers
        .iter()
        .find(|l| !l.is_empty_layer() && !l.is_solid())
    {
        let focus = PrecedenceOrder::focus(
            &next_stratum.unmet_facets,
            cube.wish_id,
            cube.evidence_bundle_id,
        );
        if let Some(f) = focus.focal() {
            out.push_str(&format!(
                "  {}focus \u{2192} {:?} {}{}\n",
                c(CYAN),
                f.kind,
                f.key,
                c(RESET)
            ));
        }
    }
    // Per-layer convergence verdict — the resolution a flat scalar cannot give.
    let trace = session.layered_trace();
    for a in &trace.anomalies {
        let msg = match a {
            RenderAnomaly::MaskedDeepRegression { layer, step } => format!(
                "masked deep regression in {} at iter {}",
                layer.label(),
                step
            ),
            RenderAnomaly::SetOutOfOrder {
                deeper,
                ungrounded_below,
            } => format!(
                "{} set before {} is solid",
                deeper.label(),
                ungrounded_below.label()
            ),
        };
        out.push_str(&format!("  {}\u{2717} {}{}\n", c(RED), msg, c(RESET)));
    }
    if trace.is_strictly_contractive() {
        out.push_str(&format!(
            "  {}\u{2713} every stratum contractive \u{2014} the cut is clean{}\n",
            c(GREEN),
            c(RESET)
        ));
    }
    // The topology gear's calibrated judgement of a realized wish (Run 12).
    if let (Some(d), Some(cube)) = (mesh_density, session.latest_cube()) {
        if let Some(line) = honesty_verdict(cube, d, color) {
            out.push_str(&line);
        }
    }
    out
}

/// Render a staged closure report: the Solve→Gate→Coagula state of each stratum
/// and where the print head has set (Run 4).
fn staged_closure_render(report: &StagedClosureReport, color: bool) -> String {
    let c = |code: &'static str| if color { code } else { "" };
    let mut out = String::new();
    out.push_str(&format!(
        "{}{}staged closure \u{2014} Solve\u{2192}Gate\u{2192}Coagula{}\n",
        c(BOLD),
        c(CYAN),
        c(RESET)
    ));
    for (layer, state) in &report.strata {
        let (word, col) = match state {
            StratumClosure::Coagulated => ("coagulated", c(GREEN)),
            StratumClosure::Gated { .. } => ("gated", c(YELLOW)),
            StratumClosure::Solving { .. } => ("solving", c(YELLOW)),
            StratumClosure::Pending => ("pending", c(DIM)),
            StratumClosure::Fractured { .. } => ("fractured", c(RED)),
        };
        out.push_str(&format!(
            "  {:<10} {}{}{}\n",
            layer.label(),
            col,
            word,
            c(RESET)
        ));
    }
    if report.fully_coagulated {
        out.push_str(&format!(
            "  {}\u{2713} fully coagulated \u{2014} the diamond is cut{}\n",
            c(GREEN),
            c(RESET)
        ));
    } else {
        let frontier = report.frontier.map(|l| l.label()).unwrap_or("none");
        out.push_str(&format!(
            "  {}print head at: {}{}\n",
            c(DIM),
            frontier,
            c(RESET)
        ));
    }
    out
}

/// Run 8 — the topology gear: the workspace's structural density, the count of
/// structural facets the parser observed over `capacity`, clamped to `ONE`. Read
/// straight from the observation — no analysis pipeline, no host writes — so a
/// read-only mesh never mutates the workspace. (The SystemCube's void-fill
/// D-density needs a host-write-enabled, operator-approved analysis, unsafe for
/// a read-only mesh; this is the safe, wish-independent topology measure: it
/// counts ALL observed structure, so a wish that reads solid over a sparse
/// topology stands out as an over-fit shell.)
fn topology_density(observed: &ObservedTopology, capacity: u32) -> Q16 {
    let count = observed.facets().count() as u64;
    let raw = Q16::ratio(count, capacity.max(1) as u64).unwrap_or(Q16::ZERO);
    if raw.at_least(Q16::ONE) {
        Q16::ONE
    } else {
        raw
    }
}

/// Render the two gears (Zahnräder) in contact: the wish-cube's structural
/// solidity against the system-cube's D-Density. Meshed when both have turned
/// past the threshold together; divergent when the wish reads solid while the
/// topology stays sparse — an over-fit shell. Advisory only (CROSS-010).
fn mesh_report(reading: &CubeMeshReading, color: bool) -> String {
    let c = |code: &'static str| if color { code } else { "" };
    let q = Q16::ratio(1, 4).unwrap_or(Q16::ZERO);
    let mut out = String::new();
    out.push_str(&format!(
        "{}{}Kosmocrates \u{2014} two gears (wish \u{27f7} topology){}\n",
        c(BOLD),
        c(CYAN),
        c(RESET)
    ));
    out.push_str(&format!(
        "  wish solidity   {}{}{}  {:.3}\n",
        c(CYAN),
        opacity_bar(reading.wish_solidity, 12),
        c(RESET),
        reading.wish_solidity.to_f64()
    ));
    out.push_str(&format!(
        "  topology dense  {}{}{}  {:.3}\n",
        c(CYAN),
        opacity_bar(reading.system_d_density, 12),
        c(RESET),
        reading.system_d_density.to_f64()
    ));
    if reading.in_mesh(q) {
        out.push_str(&format!(
            "  {}\u{2699} meshed \u{2014} both gears turning toward the diamond{}\n",
            c(GREEN),
            c(RESET)
        ));
    } else if reading.wish_solidity.at_least(q) && !reading.system_d_density.at_least(q) {
        out.push_str(&format!(
            "  {}\u{2699} divergent \u{2014} wish solid but topology sparse (over-fit suspect){}\n",
            c(YELLOW),
            c(RESET)
        ));
    } else {
        out.push_str(&format!(
            "  {}\u{2699} turning \u{2014} gears not yet meshed{}\n",
            c(DIM),
            c(RESET)
        ));
    }
    out
}

fn run() -> Result<ExitCode, String> {
    let args = match parse_args()? {
        Some(a) => a,
        None => return Ok(ExitCode::SUCCESS),
    };

    // Doors: the binary's self-description — the machine-true catalog of
    // its own docking surface. With --doors-merge, other surfaces' emitted
    // catalogs federate into one ecosystem inventory: each file is trusted
    // only if its content addresses recompute (fail-closed).
    if args.doors || args.doors_merge.is_some() {
        let mut catalogs = vec![doors::catalog()];
        if let Some(files) = args.doors_merge.as_deref() {
            for f in files.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let text =
                    fs::read_to_string(f).map_err(|e| format!("doors-merge: read {f}: {e}"))?;
                let foreign: kosmo_core::DoorCatalog = serde_json::from_str(&text)
                    .map_err(|e| format!("doors-merge: {f} is not a door catalog: {e}"))?;
                if !foreign.verify() {
                    return Err(format!(
                        "doors-merge: {f} fails content-address verification — a catalog \
                         that does not recompute is not trusted"
                    ));
                }
                catalogs.push(foreign);
            }
        }
        let catalog = kosmo_core::DoorCatalog::merge(catalogs);
        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&catalog).map_err(|e| e.to_string())?
            );
        } else {
            print!("{}", doors::render(&catalog, args.color));
        }
        return Ok(ExitCode::SUCCESS);
    }

    // Substrate organ doors (Spreizung II): foundry / witness / parseback /
    // kcube / codematrix — directed doors over organs that used to be
    // reachable only as side effects of --apply/--all. One door per run.
    let organ_doors = usize::from(args.foundry.is_some())
        + usize::from(args.witness.is_some())
        + usize::from(args.parseback)
        + usize::from(args.kcube.is_some())
        + usize::from(args.codematrix);
    if organ_doors > 0 {
        let other_door = args.pruefstand
            || args.reforge
            || args.steward
            || args.landscape
            || args.wish.is_some()
            || args.chat.is_some()
            || args.atelier.is_some()
            || args.venture.is_some()
            || args.inject_norm.is_some()
            || args.promote_norm.is_some();
        if organ_doors > 1 || other_door {
            return Err(
                "--foundry / --witness / --parseback / --kcube / --codematrix are one door \
                 per run (and exclusive with the other doors)"
                    .into(),
            );
        }
        if args.foundry.is_some() {
            return run_foundry_mode(&args);
        }
        if args.witness.is_some() {
            return run_witness_mode(&args);
        }
        if args.parseback {
            return run_parseback_mode(&args);
        }
        if args.kcube.is_some() {
            return run_kcube_mode(&args);
        }
        return run_codematrix_mode(&args);
    }

    // The Prüfstand descends a built-in reference corpus and reports fidelity:
    // every known-good system must be accepted, every broken one rejected.
    if args.pruefstand {
        let report = pruefstand::run_corpus(pruefstand::reference_corpus(), args.validated);
        print!("{}", pruefstand::render(&report, args.color));
        return Ok(if report.is_faithful() {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(3)
        });
    }

    // Reforge: the external-empiricism bench. Requires a real provider —
    // re-forging implements behaviour, which the deterministic scaffolder
    // cannot, and a mock would be theater.
    if args.reforge {
        return run_reforge_mode(&args);
    }

    // Realization benchmark: the instrument that measures whether the
    // generative loop actually works. Requires a real provider — a mock
    // would measure only the scaffolder, which the Prüfstand already covers.
    if args.realize_bench {
        return run_realize_bench_mode(&args);
    }

    // The service-dimension counterpart: realize an HTTP service (started as a
    // server, probed over HTTP) — proving the loop reaches past CLIs.
    if args.realize_service {
        return run_service_smoke_mode(&args);
    }

    // The prose→spec benchmark: natural language → facets, scored offline (or
    // through the LLM extractor with --provider). A measurement, exits 0.
    if args.prose_bench {
        return run_prose_bench_mode(&args);
    }

    // The multi-crate smoke: realize logic in a library crate, reached across
    // the boundary by the bin — the frontier of "umfang".
    if args.realize_multicrate {
        return run_multicrate_smoke_mode(&args);
    }

    // Steward: self-husbandry. Survey the workspace's own landscape; under
    // --apply, husband the open chores inside the operator-named fence.
    if args.steward {
        if args.wish.is_some()
            || args.atelier.is_some()
            || args.chat.is_some()
            || args.venture.is_some()
            || args.landscape
        {
            return Err(
                "--steward is exclusive with --wish / --atelier / --chat / --venture / \
                 --landscape (one door per run)"
                    .into(),
            );
        }
        return run_steward_mode(&args);
    }

    // Norm governance commands are standalone operator acts: inject a spec,
    // or arm a trigger. They run and exit before any other mode.
    if args.inject_norm.is_some() || args.promote_norm.is_some() {
        return run_norm_admin(&args);
    }

    // Venture: a whole system of dependent wishes, orchestrated stage by
    // stage. Exclusive with the single-wish doors.
    if args.venture.is_some() {
        if args.wish.is_some() || args.atelier.is_some() || args.chat.is_some() {
            return Err(
                "--venture is exclusive with --wish / --atelier / --chat (one door per run)".into(),
            );
        }
        return run_venture_mode(&args);
    }

    // Atelier: one shaping round on a durable wish draft (--chat carries
    // the utterance; without one, the round is "show").
    if args.atelier.is_some() {
        if args.wish.is_some() {
            return Err("--atelier and --wish are mutually exclusive".into());
        }
        return run_atelier_mode(&args);
    }

    // Chat: one utterance, routed onto an existing mode. Wins over the
    // direct mode flags; combining it with --wish is ambiguous, so refused.
    if args.chat.is_some() {
        if args.wish.is_some() {
            return Err("--chat and --wish are mutually exclusive".into());
        }
        return run_chat_mode(&args);
    }

    // Landscape mode: the substrate's findings, projected into the wish
    // vocabulary as a ranked proposal landscape. Read-only unless --adopt
    // (+ --apply) turns the top of the landscape into a descent.
    if args.landscape {
        return run_landscape_mode(&args);
    }

    // Wish mode is deterministic and offline (no LLM, no key): compile the
    // prose, observe the workspace, and report the distance to the wish.
    if args.wish.is_some() {
        return run_wish_mode(&args);
    }

    let synthesizer = build_synthesizer(&args)?;
    let synth_name = synthesizer.name().to_string();

    let pipeline_options = if args.all_layers {
        IntegrationRunOptions::all_layers(args.capacity)
    } else {
        IntegrationRunOptions::report_only()
    };
    let min_confidence =
        Q16::ratio(args.min_confidence_pct.min(100) as u64, 100).unwrap_or(Q16::HALF);

    let options = AgentOptions {
        max_steps: args.max_steps,
        min_confidence,
        dry_run: !args.apply,
        pipeline_options,
        commit_to_git: args.commit && args.apply,
        grounding_top: args.ground_top,
    };
    // --apply escalates to OperatorApproved (host writes permitted, gated by
    // per-patch cargo validation + rollback). Default stays report-only.
    let policy = if args.apply {
        PolicyProfile::operator_approved()
    } else {
        PolicyProfile::default_report_only()
    };

    let mut session = AgentSession::new(options, policy, synthesizer);
    if args.apply {
        session = session.with_validator(Arc::new(CargoFoundryValidator::new()));
    }
    if let Some(recall) = open_recall(&args)? {
        if !args.json {
            println!(
                "memory  {} (top {} recalled crystal(s) per action)",
                recall.source(),
                args.ground_top
            );
        }
        session = session.with_recall(recall);
    }
    let report = session.run(&args.path).map_err(|e| e.to_string())?;

    if args.json {
        let json = serde_json::to_string_pretty(&report)
            .map_err(|e| format!("failed to serialize report: {e}"))?;
        println!("{json}");
    } else {
        render_text(&report, &synth_name, args.color);
    }

    // Exit non-zero if the underlying pipeline gate rejected.
    match report.pipeline_gate {
        GateResult::Reject { .. } => Ok(ExitCode::from(2)),
        _ => Ok(ExitCode::SUCCESS),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("kosmo-run: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::ObservedTopology;

    /// Serializes the heavy descent tests (each spawns nested `cargo`, the
    /// service one also starts servers). Under a full `cargo test --workspace`
    /// run they would otherwise pile concurrent toolchain processes up; one at a
    /// time keeps peak load — and thus the chance of a spawn failing — low.
    /// Poison-tolerant so a panic in one does not cascade.
    static HEAVY: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn heavy() -> std::sync::MutexGuard<'static, ()> {
        HEAVY.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn assess_against(prose: &str, present: &[WishFacet]) -> (Wish, WishAssessment) {
        let wish = compile_wish(prose, Digest::ZERO, Digest::ZERO);
        let mut observed = ObservedTopology::empty();
        for f in present {
            observed.insert(f.clone());
        }
        let a = assess_wish(&wish, &observed, Digest::ZERO);
        (wish, a)
    }

    #[test]
    fn wish_report_realized_when_facet_present() {
        let (w, a) = assess_against("a crate foo", &[WishFacet::crate_("foo")]);
        let out = wish_report(&w, &a, false);
        assert!(out.contains("REALIZED"), "got: {out}");
        assert!(out.contains("1/1"));
    }

    #[test]
    fn wish_report_lists_missing_facet() {
        let (w, a) = assess_against("a crate foo", &[]);
        let out = wish_report(&w, &a, false);
        assert!(out.contains("missing"));
        assert!(out.contains("foo"));
    }

    // ── Run 3 / Run 4: hypercube render + staged closure ──────────────────

    /// A wish spanning Existence (crate), Wiring (contract) and Live (run).
    fn layered_test_wish() -> Wish {
        use kosmo_core::WishPredicate;
        Wish::new(
            "spanning",
            [
                WishPredicate::require(WishFacet::crate_("kosmo-api")),
                WishPredicate::require(WishFacet::new(
                    WishFacetKind::Contract,
                    "handle(Req)->Resp",
                )),
                WishPredicate::require(WishFacet::new(WishFacetKind::Run, "ping=>out~pong")),
            ],
            Digest::ZERO,
            Digest::of_bytes(b"ev"),
        )
    }

    #[test]
    fn layered_descent_report_renders_all_five_strata() {
        let mut session = WishSession::new(layered_test_wish(), Digest::of_bytes(b"ev"));
        let mut observed = ObservedTopology::empty();
        observed.insert(WishFacet::crate_("kosmo-api")); // existence solid
        session.observe_layered(&observed);

        let out = layered_descent_report(&session, None, false);
        for label in ["existence", "shape", "wiring", "verified", "live"] {
            assert!(out.contains(label), "missing stratum {label}: {out}");
        }
        assert!(out.contains("opacity"));
        assert!(out.contains("solid"), "existence should read solid: {out}");
        assert!(out.contains("frontier: existence"), "got: {out}");
    }

    #[test]
    fn layered_report_flags_floating_layer() {
        let mut session = WishSession::new(layered_test_wish(), Digest::of_bytes(b"ev"));
        let mut observed = ObservedTopology::empty();
        observed.insert(WishFacet::new(WishFacetKind::Run, "ping=>out~pong")); // live over hollow base
        session.observe_layered(&observed);

        let out = layered_descent_report(&session, None, false);
        assert!(
            out.contains("over-fit suspect"),
            "should warn about a floating layer: {out}"
        );
    }

    #[test]
    fn staged_target_picks_shallowest_unsolid_stratum() {
        let wish = layered_test_wish();
        let mut observed = ObservedTopology::empty();
        observed.insert(WishFacet::crate_("kosmo-api")); // existence solid; wiring+live unmet
        let cube = assess_wish_layered(&wish, &observed, Digest::of_bytes(b"ev"));
        let flat_unmet = vec![
            WishFacet::new(WishFacetKind::Contract, "handle(Req)->Resp"),
            WishFacet::new(WishFacetKind::Run, "ping=>out~pong"),
        ];
        let target = staged_target(Some(&cube), &flat_unmet);
        assert_eq!(
            target,
            vec![WishFacet::new(WishFacetKind::Contract, "handle(Req)->Resp")],
            "the curriculum targets wiring (the contract) before the live run probe"
        );
    }

    #[test]
    fn staged_closure_render_shows_coagulation() {
        let mut session = WishSession::new(layered_test_wish(), Digest::of_bytes(b"ev"));
        let mut observed = ObservedTopology::empty();
        observed.insert(WishFacet::crate_("kosmo-api"));
        session.observe_layered(&observed);
        let report = StagedClosureReport::from_descent(
            session.cubes(),
            &session.layered_trace(),
            Digest::of_bytes(b"ev"),
        );

        let out = staged_closure_render(&report, false);
        assert!(out.contains("Solve"), "header names the pipeline: {out}");
        assert!(out.contains("existence"));
        assert!(out.contains("coagulated"), "existence coagulated: {out}");
    }

    #[test]
    fn layered_report_shows_konus_focus() {
        // Nothing observed → the shallowest non-solid stratum is existence, and
        // the lens focuses on its foundation (the crate).
        let mut session = WishSession::new(layered_test_wish(), Digest::of_bytes(b"ev"));
        session.observe_layered(&ObservedTopology::empty());
        let out = layered_descent_report(&session, None, false);
        assert!(out.contains("focus \u{2192}"), "render shows the lens focus: {out}");
        assert!(out.contains("kosmo-api"), "focal foundation is the crate: {out}");
    }

    #[test]
    fn mesh_report_flags_overfit_and_mesh() {
        // A fully-solid wish cube (every facet observed present).
        let wish = layered_test_wish();
        let observed =
            ObservedTopology::from_facets(wish.predicates.iter().map(|p| p.facet.clone()));
        let cube = assess_wish_layered(&wish, &observed, Digest::of_bytes(b"ev"));
        assert_eq!(cube.structural_solidity, Q16::ONE, "the wish gear is solid");

        // Wish solid (1.0) but topology sparse (0.05) → over-fit divergence.
        let overfit =
            CubeMeshReading::read(&cube, Q16::ratio(5, 100).unwrap(), Digest::of_bytes(b"ev"));
        let out = mesh_report(&overfit, false);
        assert!(out.contains("two gears"));
        assert!(
            out.contains("over-fit suspect"),
            "a sparse topology under a solid wish is over-fit: {out}"
        );

        // Both gears past the threshold → meshed.
        let meshed =
            CubeMeshReading::read(&cube, Q16::ratio(80, 100).unwrap(), Digest::of_bytes(b"ev"));
        assert!(mesh_report(&meshed, false).contains("meshed"));
    }

    #[test]
    fn honesty_verdict_calibrates_by_confidence() {
        use kosmo_core::WishPredicate;
        let cube_of = |wish: Wish| {
            let obs =
                ObservedTopology::from_facets(wish.predicates.iter().map(|p| p.facet.clone()));
            let mut s = WishSession::new(wish, Digest::of_bytes(b"ev"));
            s.observe_layered(&obs);
            s.latest_cube().cloned().expect("a cube")
        };

        // A DEEP wish (claims Live), solid over a sparse topology: a passing probe
        // is execution-earned, so the verdict is only a calibrated SUSPECT that
        // names the real residual risk (a stub) — never a false hologram verdict.
        let deep = cube_of(layered_test_wish());
        let suspect = honesty_verdict(&deep, Q16::ratio(5, 100).unwrap(), false).unwrap();
        assert!(suspect.contains("over-fit suspect"), "{suspect}");
        assert!(suspect.contains("stub") && suspect.contains("live"), "{suspect}");
        assert!(
            !suspect.contains("a hologram, not a diamond"),
            "the instrument does not falsely accuse earned work: {suspect}"
        );

        // Dense topology → genuine, at any depth.
        let genuine = honesty_verdict(&deep, Q16::ratio(80, 100).unwrap(), false).unwrap();
        assert!(genuine.contains("genuine"), "{genuine}");

        // A SHALLOW wish (existence only), solid over the SAME sparse topology, is
        // honest — a small wish in a small workspace, no alarm.
        let shallow = cube_of(Wish::new(
            "shallow",
            [WishPredicate::require(WishFacet::crate_("solo"))],
            Digest::ZERO,
            Digest::of_bytes(b"ev"),
        ));
        let thin = honesty_verdict(&shallow, Q16::ratio(5, 100).unwrap(), false).unwrap();
        assert!(thin.contains("thin but shallow"), "{thin}");
        assert!(!thin.contains("over-fit"), "no alarm for a shallow wish: {thin}");

        // An unrealized wish gets no verdict at all.
        let mut partial = WishSession::new(layered_test_wish(), Digest::of_bytes(b"ev"));
        let mut o = ObservedTopology::empty();
        o.insert(WishFacet::crate_("kosmo-api"));
        partial.observe_layered(&o);
        assert!(honesty_verdict(
            partial.latest_cube().unwrap(),
            Q16::ratio(5, 100).unwrap(),
            false
        )
        .is_none());

        // The render wires it in: a realized session surfaces the verdict line.
        let mut s = WishSession::new(layered_test_wish(), Digest::of_bytes(b"ev"));
        s.observe_layered(&ObservedTopology::from_facets(
            layered_test_wish().predicates.iter().map(|p| p.facet.clone()),
        ));
        assert!(layered_descent_report(&s, Some(Q16::ratio(5, 100).unwrap()), false)
            .contains("over-fit suspect"));
    }

    #[test]
    fn scaffold_report_proposes_changes_for_missing_crate() {
        let out = scaffold_report(".", &[WishFacet::crate_("demo_crate")], false);
        assert!(out.contains("scaffold"));
        assert!(out.contains("demo_crate"));
    }

    #[test]
    fn descend_realizes_symbol_wish_on_temp_workspace() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-run-descent-{nanos}"));
        fs::create_dir_all(root.join("demo/src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"demo\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        fs::write(
            root.join("demo/Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(root.join("demo/src/lib.rs"), "// demo\n").unwrap();

        let prose = "a function alpha and a function beta";
        let evidence = Digest::of_bytes(prose.as_bytes());
        let wish = compile_wish(prose, Digest::ZERO, evidence);

        match descend_to_wish(
            root.to_str().unwrap(),
            &wish,
            evidence,
            false,
            8,
            None,
            None,
        ) {
            Ok(session) => {
                let last = session.latest().expect("at least one observation");
                assert!(
                    matches!(last.status, WishClosureStatus::Realized),
                    "descent should converge, got {:?} ({}/{})",
                    last.status,
                    last.met_count,
                    last.total_count
                );
                // observe (unmet) → apply → observe (met): at least two steps.
                assert!(session.iterations() >= 2);
            }
            Err(e) => eprintln!("observe unavailable, skipping: {e}"),
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn descend_realizes_doc_wish() {
        let _guard = heavy();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-run-doc-{nanos}"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        // The function exists but is undocumented — the substrate's
        // MissingDocFiber finding, expressed as a wish.
        fs::write(root.join("src/lib.rs"), "pub fn helper() -> u32 { 1 }\n").unwrap();

        let prose = "docs for helper";
        let evidence = Digest::of_bytes(prose.as_bytes());
        let wish = compile_wish(prose, Digest::ZERO, evidence);

        match descend_to_wish(
            root.to_str().unwrap(),
            &wish,
            evidence,
            false,
            8,
            None,
            None,
        ) {
            Ok(session) => {
                let last = session.latest().expect("at least one observation");
                assert!(
                    matches!(last.status, WishClosureStatus::Realized),
                    "doc descent should converge, got {:?} ({}/{})",
                    last.status,
                    last.met_count,
                    last.total_count
                );
                assert!(session.iterations() >= 2, "unmet → scaffold → met");
                let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
                assert!(
                    lib.lines().next().unwrap().starts_with("/// `helper`"),
                    "the doc stub landed above the item: {lib}"
                );
            }
            Err(e) => eprintln!("observe unavailable, skipping: {e}"),
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn doc_wish_on_documented_item_is_realized_without_writing() {
        let (w, a) = assess_against("docs of helper", &[WishFacet::doc("helper")]);
        let out = wish_report(&w, &a, false);
        assert!(out.contains("REALIZED"), "got: {out}");
        assert!(matches!(a.status, WishClosureStatus::Realized));
    }

    #[test]
    fn apply_synthesis_routes_unscaffoldable_facet_to_fallback() {
        use kosmo_synthesizer::FileChange;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-run-fallback-{nanos}"));
        fs::create_dir_all(&root).unwrap();

        // A dependency edge is not deterministically scaffoldable.
        let dep = vec![WishFacet::dependency("a", "b")];
        // No fallback → the scaffolder builds nothing, so nothing is written.
        assert_eq!(apply_synthesis(&root, &dep, None, None).unwrap(), 0);
        // With a synthesizer that proposes a change → the fallback is consulted.
        let mock =
            MockSynthesizer::confident().with_change(FileChange::create("FALLBACK.txt", "x\n"));
        let n = apply_synthesis(&root, &dep, Some(&mock), None).unwrap();
        assert_eq!(n, 1);
        assert!(root.join("FALLBACK.txt").exists());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn descent_survives_a_layer_that_breaks_the_manifest() {
        use kosmo_core::WishPredicate;
        use kosmo_synthesizer::FileChange;
        let _g = heavy(); // the descent spawns cargo

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-run-resilient-{nanos}"));
        fs::create_dir_all(root.join("src")).unwrap();
        // A package whose own default binary is named `hello` (package name +
        // src/main.rs). Observable and buildable as it stands.
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
        )
        .unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();

        // A lone dependency facet is not deterministically scaffoldable, so the
        // fallback synthesizer is consulted (see
        // `apply_synthesis_routes_unscaffoldable_facet_to_fallback`).
        let evidence = Digest::of_bytes(b"resilient-descent");
        let wish = Wish::new(
            "dependency a->b",
            [WishPredicate::require(WishFacet::dependency("a", "b"))],
            Digest::ZERO,
            evidence,
        );

        // The fallback plays a model that *also* sprays a second binary named
        // `hello`; `cargo metadata` then refuses to parse the manifest, so the
        // next observation fails partway through the descent.
        let saboteur = MockSynthesizer::confident()
            .with_change(FileChange::create("src/bin/hello.rs", "fn main() {}\n"));

        // Before the fix this propagated `Err` and the whole descent — with any
        // correct work inside it — was discarded. The descent must now survive:
        // a single bad layer ends it at its last good state, it does not void it.
        let result = descend_to_wish(
            root.to_str().unwrap(),
            &wish,
            evidence,
            false,
            3,
            Some(&saboteur),
            None,
        );

        assert!(
            result.is_ok(),
            "a mid-descent manifest break must not void the descent: {:?}",
            result.err()
        );
        let session = result.unwrap();
        assert!(
            root.join("src/bin/hello.rs").exists(),
            "the saboteur layer must actually have been applied"
        );
        // Only the first observation succeeds; each later one fails because the
        // saboteur re-breaks the manifest on every repair, so no further
        // assessment is recorded. The point: the descent stays Ok and terminates
        // (it neither errors out nor hangs) even when it cannot heal.
        assert!(
            session.iterations() <= 2,
            "an unrecoverable break must still terminate cleanly (got {} iterations)",
            session.iterations()
        );
        let last = session.latest().expect("the first observation survives");
        assert!(
            !matches!(last.status, WishClosureStatus::Realized),
            "an unbuildable workspace is not a realized wish, got {:?}",
            last.status
        );

        fs::remove_dir_all(&root).ok();
    }

    /// Breaks the manifest on the first call (a duplicate binary), then — once
    /// the build error is fed back as a repair — deletes the offending file, and
    /// finally proposes nothing more.
    struct BreakThenHeal {
        calls: std::sync::atomic::AtomicUsize,
    }
    impl ActionSynthesizer for BreakThenHeal {
        fn synthesize(
            &self,
            request: &SynthesisRequest,
        ) -> Result<kosmo_synthesizer::SynthesisResult, kosmo_synthesizer::SynthesisError> {
            use kosmo_synthesizer::{FileChange, Patch, SynthesisResult};
            use std::sync::atomic::Ordering;
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            let changes = match n {
                0 => vec![FileChange::create("src/bin/hello.rs", "fn main() {}\n")],
                1 => vec![FileChange::delete("src/bin/hello.rs")],
                _ => vec![],
            };
            let patch = Patch::new(request.request_id, changes, "break-then-heal");
            Ok(SynthesisResult::new(patch, "test repair", Q16::ONE))
        }
        fn name(&self) -> &str {
            "break-then-heal"
        }
    }

    #[test]
    fn descent_self_heals_a_broken_manifest_and_continues() {
        use kosmo_core::WishPredicate;
        let _g = heavy(); // the descent spawns cargo

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-run-selfheal-{nanos}"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
        )
        .unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();

        let evidence = Digest::of_bytes(b"self-heal");
        let wish = Wish::new(
            "dependency a->b",
            [WishPredicate::require(WishFacet::dependency("a", "b"))],
            Digest::ZERO,
            evidence,
        );

        // First layer breaks the manifest; when the build error is fed back, the
        // second layer deletes the offending binary.
        let healer = BreakThenHeal {
            calls: std::sync::atomic::AtomicUsize::new(0),
        };

        let result = descend_to_wish(
            root.to_str().unwrap(),
            &wish,
            evidence,
            false,
            8,
            Some(&healer),
            None,
        );
        assert!(
            result.is_ok(),
            "a self-healing descent must not error: {:?}",
            result.err()
        );
        let session = result.unwrap();

        // The stray binary the first layer added was deleted by the repair…
        assert!(
            !root.join("src/bin/hello.rs").exists(),
            "the repair layer should have deleted the colliding binary"
        );
        // …and the workspace became observable again, so the descent advanced
        // past the break — a second assessment was recorded.
        assert!(
            session.iterations() >= 2,
            "the descent should re-observe after healing (got {} iterations)",
            session.iterations()
        );

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn descend_realizes_dependency_wish() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-run-dep-descent-{nanos}"));
        for name in ["a", "b"] {
            fs::create_dir_all(root.join("crates").join(name).join("src")).unwrap();
            fs::write(
                root.join("crates").join(name).join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
            )
            .unwrap();
            fs::write(root.join("crates").join(name).join("src/lib.rs"), "// x\n").unwrap();
        }
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/a\", \"crates/b\"]\nresolver = \"2\"\n",
        )
        .unwrap();

        let prose = "dependency a->b";
        let evidence = Digest::of_bytes(prose.as_bytes());
        let wish = compile_wish(prose, Digest::ZERO, evidence);

        match descend_to_wish(
            root.to_str().unwrap(),
            &wish,
            evidence,
            false,
            8,
            None,
            None,
        ) {
            Ok(session) => {
                let last = session.latest().expect("at least one observation");
                assert!(
                    matches!(last.status, WishClosureStatus::Realized),
                    "dependency wish should converge, got {:?} ({}/{})",
                    last.status,
                    last.met_count,
                    last.total_count
                );
            }
            Err(e) => eprintln!("observe (cargo metadata) unavailable, skipping: {e}"),
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn descend_realizes_typed_contract_wish() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-run-contract-{nanos}"));
        fs::create_dir_all(root.join("demo/src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"demo\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        fs::write(
            root.join("demo/Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(root.join("demo/src/lib.rs"), "// demo\n").unwrap();

        // Built-in types so the lexical descent's round-trip is unambiguous.
        let prose = "a contract add(i32,i32)->i32";
        let evidence = Digest::of_bytes(prose.as_bytes());
        let wish = compile_wish(prose, Digest::ZERO, evidence);
        assert!(
            wish.predicates
                .iter()
                .any(|p| p.facet.kind == kosmo_core::WishFacetKind::Contract),
            "prose should compile to a Contract facet"
        );

        match descend_to_wish(
            root.to_str().unwrap(),
            &wish,
            evidence,
            false,
            8,
            None,
            None,
        ) {
            Ok(session) => {
                let last = session.latest().expect("at least one observation");
                assert!(
                    matches!(last.status, WishClosureStatus::Realized),
                    "typed-contract wish should converge, got {:?} ({}/{})",
                    last.status,
                    last.met_count,
                    last.total_count
                );
            }
            Err(e) => eprintln!("observe unavailable, skipping: {e}"),
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn descend_realizes_crud_archetype() {
        // One prose phrase ("a crud user") fans out into a 4-facet bundle —
        // module + two typed handlers + capability — all structural, so the
        // deterministic scaffolder converges it offline (no LLM, no validation).
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-run-crud-{nanos}"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "// demo\n").unwrap();

        let prose = "a crud user";
        let evidence = Digest::of_bytes(prose.as_bytes());
        let wish = compile_wish(prose, Digest::ZERO, evidence);
        assert_eq!(wish.predicate_count(), 4, "crud should fan out to 4 facets");

        match descend_to_wish(
            root.to_str().unwrap(),
            &wish,
            evidence,
            false,
            8,
            None,
            None,
        ) {
            Ok(session) => {
                let last = session.latest().expect("at least one observation");
                assert!(
                    matches!(last.status, WishClosureStatus::Realized),
                    "crud archetype should converge, got {:?} ({}/{})",
                    last.status,
                    last.met_count,
                    last.total_count
                );
            }
            Err(e) => eprintln!("observe unavailable, skipping: {e}"),
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn descend_realizes_composition() {
        // A typed data-flow wire: parse() -> String -> eval(String). The
        // scaffolder writes two type-compatible stubs; the observer derives the
        // composition from their contracts, so it converges offline.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-run-comp-{nanos}"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "// demo\n").unwrap();

        let prose = "a composition parse>>String>>eval";
        let evidence = Digest::of_bytes(prose.as_bytes());
        let wish = compile_wish(prose, Digest::ZERO, evidence);

        match descend_to_wish(
            root.to_str().unwrap(),
            &wish,
            evidence,
            false,
            8,
            None,
            None,
        ) {
            Ok(session) => {
                let last = session.latest().expect("at least one observation");
                assert!(
                    matches!(last.status, WishClosureStatus::Realized),
                    "composition wish should converge, got {:?} ({}/{})",
                    last.status,
                    last.met_count,
                    last.total_count
                );
                let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
                assert!(lib.contains("pub fn parse() -> String"), "got:\n{lib}");
                assert!(lib.contains("pub fn eval(_a0: String)"), "got:\n{lib}");
            }
            Err(e) => eprintln!("observe unavailable, skipping: {e}"),
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn descend_validates_behavioral_composition() {
        let _heavy = heavy();
        // Beam 1 of the runtime floor: a two-stage pipeline parse -> eval,
        // validated by the COMPOSED spec test `assert_eq!(eval(parse("2+3")), 5)`.
        // Acceptance over generation, applied to the wire — both directions.
        let correct = "pub fn parse(s: &str) -> Vec<i32> { s.split('+').map(|x| x.trim().parse().unwrap()).collect() }\n\
                       pub fn eval(v: Vec<i32>) -> i32 { v.iter().sum() }\n";
        let wrong = "pub fn parse(s: &str) -> Vec<i32> { s.split('+').map(|x| x.trim().parse().unwrap()).collect() }\n\
                     pub fn eval(v: Vec<i32>) -> i32 { v.iter().sum::<i32>() + 1 }\n";
        let prose = "a flow parse(\"2+3\")>>eval=>5";

        for (idx, (lib, want_realized)) in [(correct, true), (wrong, false)].iter().enumerate() {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("kosmo-run-flow-{idx}-{nanos}"));
            fs::create_dir_all(root.join("src")).unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname = \"calc\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            )
            .unwrap();
            fs::write(root.join("src/lib.rs"), lib).unwrap();

            let evidence = Digest::of_bytes(prose.as_bytes());
            let wish = compile_wish(prose, Digest::ZERO, evidence);
            match descend_to_wish(root.to_str().unwrap(), &wish, evidence, true, 8, None, None) {
                Ok(session) => {
                    let last = session.latest().expect("an observation");
                    let realized = matches!(last.status, WishClosureStatus::Realized);
                    assert_eq!(
                        realized, *want_realized,
                        "pipeline verdict wrong for lib:\n{lib}\nstatus {:?} ({}/{})",
                        last.status, last.met_count, last.total_count
                    );
                }
                Err(e) => eprintln!("validated observe unavailable, skipping: {e}"),
            }
            fs::remove_dir_all(&root).ok();
        }
    }

    #[test]
    fn descend_realizes_run_probe() {
        let _heavy = heavy();
        // Beam 3 of the runtime floor: the built artifact is EXECUTED and its
        // stdout probed. `a run add,2,3=>out~5` over a binary that prints the
        // sum realizes; an empty `main` (prints nothing) is rejected.
        let correct = "fn main() { let a: Vec<String> = std::env::args().skip(1).collect(); \
                       if a.first().map(|s| s.as_str()) == Some(\"add\") { \
                       let s: i32 = a[1..].iter().filter_map(|x| x.parse::<i32>().ok()).sum(); \
                       println!(\"{s}\"); } }\n";
        let empty = "fn main() {}\n";
        let prose = "a run add,2,3=>out~5";

        for (idx, (main_rs, want_realized)) in [(correct, true), (empty, false)].iter().enumerate()
        {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("kosmo-run-runprobe-{idx}-{nanos}"));
            fs::create_dir_all(root.join("src")).unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname = \"calc\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            )
            .unwrap();
            fs::write(root.join("src/main.rs"), main_rs).unwrap();

            let evidence = Digest::of_bytes(prose.as_bytes());
            let wish = compile_wish(prose, Digest::ZERO, evidence);
            // validated=false, but the Run facet forces runtime observation.
            match descend_to_wish(
                root.to_str().unwrap(),
                &wish,
                evidence,
                false,
                4,
                None,
                None,
            ) {
                Ok(session) => {
                    let last = session.latest().expect("an observation");
                    let realized = matches!(last.status, WishClosureStatus::Realized);
                    assert_eq!(
                        realized, *want_realized,
                        "run-probe verdict wrong for main:\n{main_rs}\nstatus {:?} ({}/{})",
                        last.status, last.met_count, last.total_count
                    );
                    // The probe marker was scaffolded into the bin either way.
                    let m = fs::read_to_string(root.join("src/main.rs")).unwrap();
                    assert!(m.contains("// kosmo:run: add,2,3=>out~5"), "marker:\n{m}");
                }
                Err(e) => eprintln!("runtime observe unavailable, skipping: {e}"),
            }
            fs::remove_dir_all(&root).ok();
        }
    }

    #[test]
    fn descend_realizes_service_probe() {
        let _heavy = heavy();
        // Beam 5 of the runtime floor: the artifact is STARTED AS A SERVER and
        // probed over HTTP. A std-only server that binds KOSMO_PORT and answers
        // 200 realizes `a service GET:/health=>200`; an empty `main` (binds
        // nothing) is rejected.
        let server = r#"use std::io::{Read, Write};
use std::net::TcpListener;
fn main() {
    let port = std::env::var("KOSMO_PORT").unwrap();
    let l = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();
    for s in l.incoming() {
        let mut s = s.unwrap();
        let mut b = [0u8; 512];
        let _ = s.read(&mut b);
        let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
    }
}
"#;
        let empty = "fn main() {}\n";
        let prose = "a service GET:/health=>200";

        for (idx, (main_rs, want_realized)) in [(server, true), (empty, false)].iter().enumerate() {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("kosmo-run-svc-{idx}-{nanos}"));
            fs::create_dir_all(root.join("src")).unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname = \"srv\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            )
            .unwrap();
            fs::write(root.join("src/main.rs"), main_rs).unwrap();

            let evidence = Digest::of_bytes(prose.as_bytes());
            let wish = compile_wish(prose, Digest::ZERO, evidence);
            match descend_to_wish(
                root.to_str().unwrap(),
                &wish,
                evidence,
                false,
                4,
                None,
                None,
            ) {
                Ok(session) => {
                    let last = session.latest().expect("an observation");
                    let realized = matches!(last.status, WishClosureStatus::Realized);
                    assert_eq!(
                        realized, *want_realized,
                        "service-probe verdict wrong (status {:?}, {}/{})",
                        last.status, last.met_count, last.total_count
                    );
                    let m = fs::read_to_string(root.join("src/main.rs")).unwrap();
                    assert!(
                        m.contains("// kosmo:service: GET:/health=>200"),
                        "marker:\n{m}"
                    );
                }
                Err(e) => eprintln!("service observe unavailable, skipping: {e}"),
            }
            fs::remove_dir_all(&root).ok();
        }
    }

    #[test]
    fn descend_targets_function_into_member_crate() {
        let _heavy = heavy();
        // Crate-targeting: "helper@beta" must land in crates/beta, not the root.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-run-target-{nanos}"));
        for name in ["alpha", "beta"] {
            fs::create_dir_all(root.join("crates").join(name).join("src")).unwrap();
            fs::write(
                root.join("crates").join(name).join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
            )
            .unwrap();
            fs::write(root.join("crates").join(name).join("src/lib.rs"), "// x\n").unwrap();
        }
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/alpha\", \"crates/beta\"]\nresolver = \"2\"\n",
        )
        .unwrap();

        let prose = "a function helper@beta";
        let evidence = Digest::of_bytes(prose.as_bytes());
        let wish = compile_wish(prose, Digest::ZERO, evidence);

        match descend_to_wish(
            root.to_str().unwrap(),
            &wish,
            evidence,
            false,
            8,
            None,
            None,
        ) {
            Ok(session) => {
                let last = session.latest().expect("at least one observation");
                assert!(
                    matches!(last.status, WishClosureStatus::Realized),
                    "crate-targeted wish should converge, got {:?} ({}/{})",
                    last.status,
                    last.met_count,
                    last.total_count
                );
                // The function landed in beta, not alpha, not the root.
                let beta = fs::read_to_string(root.join("crates/beta/src/lib.rs")).unwrap();
                assert!(beta.contains("pub fn helper"), "beta should have helper");
                let alpha = fs::read_to_string(root.join("crates/alpha/src/lib.rs")).unwrap();
                assert!(
                    !alpha.contains("pub fn helper"),
                    "alpha must NOT have helper"
                );
                assert!(
                    !root.join("src/lib.rs").exists(),
                    "root must not be touched"
                );
            }
            Err(e) => eprintln!("observe (cargo metadata) unavailable, skipping: {e}"),
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn behavior_wish_forces_validation() {
        let w = compile_wish("a behavior add(2,3)=>5", Digest::ZERO, Digest::ZERO);
        assert!(
            wish_needs_validation(&w),
            "behaviour wish must force validation"
        );
        let w2 = compile_wish("a crate foo", Digest::ZERO, Digest::ZERO);
        assert!(!wish_needs_validation(&w2), "a non-behaviour wish must not");
    }

    #[test]
    fn descend_realizes_behavior_when_impl_is_correct() {
        let _heavy = heavy();
        // The keystone, demonstrated offline: given a CORRECT implementation,
        // a behaviour wish converges to Realized — the loop scaffolds the
        // spec-test and observes it green. (With a wrong/todo!() body it would
        // honestly stall at Approaching; see kosmo-intent::behavior_facets.)
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-run-behavior-{nanos}"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
        )
        .unwrap();

        let prose = "a behavior add(2,3)=>5";
        let evidence = Digest::of_bytes(prose.as_bytes());
        let wish = compile_wish(prose, Digest::ZERO, evidence);

        // validated = true (a behaviour wish forces it); no LLM fallback — the
        // implementation is already correct, so the loop only scaffolds the
        // spec-test and re-observes it green.
        match descend_to_wish(root.to_str().unwrap(), &wish, evidence, true, 8, None, None) {
            Ok(session) => {
                let last = session.latest().expect("at least one observation");
                assert!(
                    matches!(last.status, WishClosureStatus::Realized),
                    "behaviour wish should converge with a correct impl, got {:?} ({}/{})",
                    last.status,
                    last.met_count,
                    last.total_count
                );
            }
            Err(e) => eprintln!("cargo test unavailable, skipping: {e}"),
        }
        fs::remove_dir_all(&root).ok();
    }

    // ── Session persistence ───────────────────────────────────────────────

    #[test]
    fn wish_session_json_roundtrip() {
        let prose = "a crate canary";
        let evidence = Digest::of_bytes(prose.as_bytes());
        let wish = compile_wish(prose, Digest::ZERO, evidence);
        let mut session = WishSession::new(wish.clone(), evidence);
        // iter 0: unmet — canary not present
        session.observe(&ObservedTopology::empty());
        // iter 1: realized — canary present
        let observed = ObservedTopology::from_facets([WishFacet::crate_("canary")]);
        session.observe(&observed);

        let json = serde_json::to_string_pretty(&session).unwrap();
        let back: WishSession = serde_json::from_str(&json).unwrap();
        assert_eq!(back.iterations(), 2);
        assert_eq!(back.wish().id, wish.id);
        assert!(matches!(
            back.latest().unwrap().status,
            WishClosureStatus::Realized
        ));
    }

    #[test]
    fn wish_session_saved_and_loaded_from_file() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let session_path = std::env::temp_dir()
            .join(format!("kosmo-run-session-{nanos}.json"))
            .to_string_lossy()
            .to_string();

        let prose = "a crate sparrow";
        let evidence = Digest::of_bytes(prose.as_bytes());
        let wish = compile_wish(prose, Digest::ZERO, evidence);

        // Build a two-step session and save it.
        let mut session = WishSession::new(wish.clone(), evidence);
        session.observe(&ObservedTopology::empty());
        let observed = ObservedTopology::from_facets([WishFacet::crate_("sparrow")]);
        session.observe(&observed);
        save_session(&session_path, &session).expect("save must succeed");

        // Load it back.
        let loaded =
            load_prior_session(&session_path, &wish).expect("should load because wish id matches");
        assert_eq!(loaded.iterations(), 2);
        assert!(matches!(
            loaded.latest().unwrap().status,
            WishClosureStatus::Realized
        ));

        // A different wish must not be accepted.
        let other_wish = compile_wish("a crate other", Digest::ZERO, evidence);
        assert!(
            load_prior_session(&session_path, &other_wish).is_none(),
            "different wish id must be rejected"
        );

        fs::remove_file(&session_path).ok();
    }
}
