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

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use kosmo_agent::{AgentOptions, AgentRunReport, AgentSession, CargoFoundryValidator};
use kosmo_core::{
    assess_wish, Digest, GateResult, PolicyProfile, Q16, Wish, WishAssessment, WishClosureStatus,
    WishFacet, WishFacetKind,
};
use kosmo_intent::{compile_wish, observe_workspace_deep, observe_workspace_validated, WishSession};
use kosmo_pipeline::{ActionItem, ActionItemKind, IntegrationRunOptions};
use kosmo_synthesizer::{ActionSynthesizer, FacetScaffolder, MockSynthesizer, SynthesisRequest};
use kosmo_synthesizer_llm::{LlmConfig, LlmSynthesizer};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";

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
    provider_set: bool,
    /// Path to a JSON file that the convergence trajectory is written to (and
    /// resumed from, if the file already exists and matches the current wish).
    wish_session: Option<String>,
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
            provider_set: false,
            wish_session: None,
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
    --scaffold            also print the file changes that would close the gap\n\
    --wish-session <path> write the convergence trajectory as JSON to <path>;\n\
                          if <path> already exists and matches the wish, resume\n\
                          from the prior session (auditable, replayable)\n\
\n\
    (wish + --apply descends: scaffold \u{2192} write \u{2192} re-observe until\n\
     realized; add --provider to let the LLM build facets the scaffolder can't)\n\
\n\
    --json                emit the report as JSON\n\
    --no-color            disable ANSI colour\n\
    -h, --help            show this help\n\n\
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
                args.provider = argv.next().ok_or("--provider needs a value")?.to_lowercase();
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
            "--wish-session" => {
                args.wish_session = Some(argv.next().ok_or("--wish-session needs a value")?);
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

    match args.provider.as_str() {
        "mock" => Ok(Arc::new(MockSynthesizer::confident())),
        "claude" | "anthropic" => {
            let key = env_key(&["ANTHROPIC_API_KEY", "KOSMO_LLM_API_KEY"]).ok_or_else(|| {
                "provider=claude requires ANTHROPIC_API_KEY (or KOSMO_LLM_API_KEY)".to_string()
            })?;
            let cfg = apply_model(LlmConfig::claude(key));
            Ok(Arc::new(LlmSynthesizer::new(cfg).map_err(|e| e.to_string())?))
        }
        "cerebras" => {
            let key = env_key(&["CEREBRAS_API_KEY", "KOSMO_LLM_API_KEY"]).ok_or_else(|| {
                "provider=cerebras requires CEREBRAS_API_KEY (or KOSMO_LLM_API_KEY)".to_string()
            })?;
            let cfg = apply_model(LlmConfig::cerebras(key));
            Ok(Arc::new(LlmSynthesizer::new(cfg).map_err(|e| e.to_string())?))
        }
        "env" | "auto" | "" => {
            let synth = LlmSynthesizer::from_env().map_err(|e| e.to_string())?;
            Ok(Arc::new(synth))
        }
        other => Err(format!(
            "unknown provider '{other}' (expected claude | cerebras | mock | env)"
        )),
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
                println!("    commit  {}{}{}", c(CYAN), &sha[..sha.len().min(12)], c(RESET));
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

/// Deterministic, offline front door: compile a prose wish, observe the
/// workspace, and report the distance to the wish (which facets are present,
/// which are missing). With `--scaffold`, also print the changes that would
/// close the gap. No LLM and no key required.
fn run_wish_mode(args: &Args) -> Result<ExitCode, String> {
    let prose = args.wish.as_deref().unwrap_or("");
    // Bind the wish's identity to its prose — content-addressed, deterministic.
    let evidence = Digest::of_bytes(prose.as_bytes());
    let wish = compile_wish(prose, Digest::ZERO, evidence);

    // A behaviour facet is satisfiable only by a *passing* test, so any wish
    // that carries one forces validated observation (run the suite), whether or
    // not --validated was given — the keystone demands it.
    let validated = args.validated || wish_needs_validation(&wish);

    // --apply turns wish mode into a descent: observe → scaffold → apply →
    // re-observe, until the wish is realized. This WRITES to the workspace.
    if args.apply {
        // LLM fallback for facets the deterministic scaffolder can't build —
        // only when a provider was explicitly chosen (else: deterministic only).
        let fallback = if args.provider_set {
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
        let prior = args
            .wish_session
            .as_deref()
            .and_then(|p| load_prior_session(p, &wish));
        let session = descend_to_wish(
            &args.path,
            &wish,
            evidence,
            validated,
            8,
            fallback.as_deref(),
            prior,
        )?;
        if let Some(ref sp) = args.wish_session {
            save_session(sp, &session)?;
        }
        if args.json {
            let json = serde_json::to_string_pretty(session.assessments())
                .map_err(|e| format!("failed to serialize assessments: {e}"))?;
            println!("{json}");
        } else {
            print!("{}", descent_report(&session, args.color));
        }
        let realized = session.latest().map_or(false, |a| {
            matches!(
                a.status,
                WishClosureStatus::Realized | WishClosureStatus::Vacuous
            )
        });
        return Ok(if realized {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        });
    }

    let observed = if validated {
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
        let json = serde_json::to_string_pretty(&assessment)
            .map_err(|e| format!("failed to serialize assessment: {e}"))?;
        println!("{json}");
    } else {
        print!("{}", wish_report(&wish, &assessment, args.color));
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
    out.push_str(&format!("{}{}Kosmocrates wish{}\n", c(BOLD), c(CYAN), c(RESET)));
    out.push_str(&format!("  \u{201c}{}\u{201d}\n", wish.label));
    out.push_str(&format!(
        "  status {}{}{}   met {}/{}\n",
        col, label, c(RESET), a.met_count, a.total_count
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
            out.push_str(&format!("    {}\u{2717}{} {:?} {}\n", c(RED), c(RESET), f.kind, f.key));
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
fn apply_synthesis(
    root: &Path,
    unmet: &[WishFacet],
    fallback: Option<&dyn ActionSynthesizer>,
) -> std::io::Result<usize> {
    let mut written = 0;
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
        let req = SynthesisRequest::new(action, root.to_string_lossy().to_string());

        // Deterministic scaffolder first; consult the LLM only if it built nothing.
        let mut changes = FacetScaffolder
            .synthesize(&req)
            .map(|r| r.patch.file_changes)
            .unwrap_or_default();
        if changes.is_empty() {
            if let Some(synth) = fallback {
                if let Ok(result) = synth.synthesize(&req) {
                    changes = result.patch.file_changes;
                }
            }
        }
        for fc in &changes {
            let target = root.join(&fc.path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target, &fc.content)?;
            written += 1;
        }
    }
    Ok(written)
}

/// Drive the workspace toward `wish` by repeated observe → assess → scaffold →
/// apply, until it is realized, no further progress is possible, or `max_iters`
/// is reached. Returns the [`WishSession`] carrying the full convergence
/// trajectory — the attractor descent, executed.
///
/// `prior` resumes an earlier descent: the loaded session's assessments are
/// prepended to the trajectory and the loop continues from the current state.
fn descend_to_wish(
    path: &str,
    wish: &Wish,
    evidence: Digest,
    validated: bool,
    max_iters: u32,
    fallback: Option<&dyn ActionSynthesizer>,
    prior: Option<WishSession>,
) -> Result<WishSession, String> {
    let mut session = prior.unwrap_or_else(|| WishSession::new(wish.clone(), evidence));
    let mut iter = 0u32;
    loop {
        let observed = if validated {
            observe_workspace_validated(path)
        } else {
            observe_workspace_deep(path)
        }
        .map_err(|e| format!("could not observe {path}: {e}"))?;

        let assessment = session.observe(&observed);
        let done = matches!(
            assessment.status,
            WishClosureStatus::Realized | WishClosureStatus::Vacuous
        );
        let unmet = assessment.unmet_facets.clone();

        if done || unmet.is_empty() || iter >= max_iters {
            break;
        }
        let written =
            apply_synthesis(Path::new(path), &unmet, fallback).map_err(|e| e.to_string())?;
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
            i, a.met_count, a.total_count, col, label, c(RESET)
        ));
    }
    let realized = session.latest().map_or(false, |a| {
        matches!(
            a.status,
            WishClosureStatus::Realized | WishClosureStatus::Vacuous
        )
    });
    if realized {
        out.push_str(&format!("  {}\u{2713} wish realized.{}\n", c(GREEN), c(RESET)));
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

fn run() -> Result<ExitCode, String> {
    let args = match parse_args()? {
        Some(a) => a,
        None => return Ok(ExitCode::SUCCESS),
    };

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
    let min_confidence = Q16::ratio(args.min_confidence_pct.min(100) as u64, 100)
        .unwrap_or(Q16::HALF);

    let options = AgentOptions {
        max_steps: args.max_steps,
        min_confidence,
        dry_run: !args.apply,
        pipeline_options,
        commit_to_git: args.commit && args.apply,
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

        match descend_to_wish(root.to_str().unwrap(), &wish, evidence, false, 8, None, None) {
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
        assert_eq!(apply_synthesis(&root, &dep, None).unwrap(), 0);
        // With a synthesizer that proposes a change → the fallback is consulted.
        let mock = MockSynthesizer::confident().with_change(FileChange::create("FALLBACK.txt", "x\n"));
        let n = apply_synthesis(&root, &dep, Some(&mock)).unwrap();
        assert_eq!(n, 1);
        assert!(root.join("FALLBACK.txt").exists());

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

        match descend_to_wish(root.to_str().unwrap(), &wish, evidence, false, 8, None, None) {
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

        match descend_to_wish(root.to_str().unwrap(), &wish, evidence, false, 8, None, None) {
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
    fn behavior_wish_forces_validation() {
        let w = compile_wish("a behavior add(2,3)=>5", Digest::ZERO, Digest::ZERO);
        assert!(wish_needs_validation(&w), "behaviour wish must force validation");
        let w2 = compile_wish("a crate foo", Digest::ZERO, Digest::ZERO);
        assert!(!wish_needs_validation(&w2), "a non-behaviour wish must not");
    }

    #[test]
    fn descend_realizes_behavior_when_impl_is_correct() {
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
        let loaded = load_prior_session(&session_path, &wish)
            .expect("should load because wish id matches");
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
