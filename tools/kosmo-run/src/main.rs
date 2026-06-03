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

use std::process::ExitCode;
use std::sync::Arc;

use kosmo_agent::{AgentOptions, AgentRunReport, AgentSession, CargoFoundryValidator};
use kosmo_core::{GateResult, PolicyProfile, Q16};
use kosmo_pipeline::{ActionItemKind, IntegrationRunOptions};
use kosmo_synthesizer::{ActionSynthesizer, MockSynthesizer};
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
    --json                emit the AgentRunReport as JSON\n\
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
    kosmo-run --provider mock --apply --commit .   # apply + commit each patch"
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

fn run() -> Result<ExitCode, String> {
    let args = match parse_args()? {
        Some(a) => a,
        None => return Ok(ExitCode::SUCCESS),
    };

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
