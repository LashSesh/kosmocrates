//! Kosmocrates substrate CLI.
//!
//! Analyses any Rust workspace and produces a priority-ranked action queue from
//! the full HYPHAE → Metatron → SystemCube → Crystal → PSE pipeline.
//!
//! USAGE:
//!     kosmo-substrate [OPTIONS] [PATH]
//!
//! Run `kosmo-substrate --help` for the full option list.

use std::path::PathBuf;
use std::process;

use kosmo_core::PolicyProfile;
use kosmo_pipeline::{
    ActionItemKind, IntegrationRunOptions, IntegrationRunReport, WorkspacePipelineSession,
};

// ─── CLI args ────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct Args {
    path: String,
    store: Option<PathBuf>,
    all_layers: bool,
    metatron: bool,
    lpcm: bool,
    systemcube: bool,
    surgery: bool,
    crystals: bool,
    norms: bool,
    motifs: bool,
    pse: bool,
    operator: bool,
    capacity: u32,
    output: OutputFormat,
    fail_on_reject: bool,
    fail_on_warn: bool,
}

#[derive(Debug, PartialEq)]
enum OutputFormat {
    Text,
    Json,
    Markdown,
    Summary,
}

fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut args = Args {
        path: ".".into(),
        store: None,
        all_layers: false,
        metatron: false,
        lpcm: false,
        systemcube: false,
        surgery: false,
        crystals: false,
        norms: false,
        motifs: false,
        pse: false,
        operator: false,
        capacity: 100,
        output: OutputFormat::Text,
        fail_on_reject: false,
        fail_on_warn: false,
    };

    let mut i = 0usize;
    while i < raw.len() {
        match raw[i].as_str() {
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            "-V" | "--version" => {
                println!("kosmo-substrate {}", env!("CARGO_PKG_VERSION"));
                process::exit(0);
            }
            "--all" | "--all-layers" => args.all_layers = true,
            "--metatron" => args.metatron = true,
            "--lpcm" => args.lpcm = true,
            "--systemcube" => {
                args.systemcube = true;
                args.metatron = true;
            }
            "--surgery" => {
                args.surgery = true;
                args.metatron = true;
            }
            "--crystals" => args.crystals = true,
            "--norms" => args.norms = true,
            "--motifs" => args.motifs = true,
            "--pse" => args.pse = true,
            "--operator" => args.operator = true,
            "--fail-on-reject" => args.fail_on_reject = true,
            "--fail-on-warn" => {
                args.fail_on_warn = true;
                args.fail_on_reject = true;
            }
            "--store" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--store requires a path argument".into());
                }
                args.store = Some(expand_tilde(&raw[i]));
            }
            "--output" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--output requires a format argument (text|json|summary)".into());
                }
                args.output = match raw[i].as_str() {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    "markdown" => OutputFormat::Markdown,
                    "summary" => OutputFormat::Summary,
                    other => {
                        return Err(format!(
                            "unknown output format '{other}'; use text, json, markdown, or summary"
                        ))
                    }
                };
            }
            "--capacity" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--capacity requires a number argument".into());
                }
                args.capacity = raw[i].parse().map_err(|_| {
                    format!("--capacity must be a positive integer, got '{}'", raw[i])
                })?;
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown flag '{flag}'; run --help for usage"));
            }
            path => args.path = path.to_string(),
        }
        i += 1;
    }
    Ok(args)
}

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(format!("{}{}", home, &path[1..]));
        }
    }
    PathBuf::from(path)
}

fn print_help() {
    println!(
        "{}",
        concat!(
        "Kosmocrates Substrate — workspace topology analysis + ranked action queue\n",
        "\n",
        "USAGE:\n",
        "    kosmo-substrate [OPTIONS] [PATH]\n",
        "\n",
        "ARGS:\n",
        "    [PATH]           Workspace root to analyse (default: current directory)\n",
        "\n",
        "OPTIONS:\n",
        "    --store <path>   Crystal store for cross-session CAD library persistence\n",
        "                     Automatically loads prior crystals and persists new ones.\n",
        "                     Supports ~ expansion.\n",
        "    --all            Enable all optional analysis layers\n",
        "    --metatron       Enable Metatron topology diagnostics\n",
        "    --lpcm           Enable LPCM passive seam reports\n",
        "    --systemcube     Enable SystemCube blueprint export (implies --metatron)\n",
        "    --surgery        Enable topology surgery options (implies --metatron)\n",
        "    --crystals       Enable structural crystal candidates + CAD library\n",
        "    --norms          Enable norm gene candidates\n",
        "    --motifs         Enable motif candidates\n",
        "    --pse            Enable PSE bridge candidates\n",
        "    --operator       Use OperatorApproved policy (enables host writes/crystal persist)\n",
        "    --capacity <n>   SystemCube D-density denominator (default: 100)\n",
        "    --output <fmt>   Output format: text (default) | json | markdown | summary\n",
        "    --fail-on-reject Exit 1 if gate result is Reject\n",
        "    --fail-on-warn   Exit 1 if gate result is Reject or Warn\n",
        "    -h, --help       Print this help\n",
        "    -V, --version    Print version\n",
        "\n",
        "EXAMPLES:\n",
        "    kosmo-substrate .\n",
        "    kosmo-substrate ./my-project --all --operator --store ~/.kosmo/cadlib.jsonl\n",
        "    kosmo-substrate ./my-project --crystals --output json > report.json\n",
        "    kosmo-substrate ./my-project --output summary --fail-on-reject\n",
    )
    );
}

// ─── ANSI colours ────────────────────────────────────────────────────────────

fn use_color() -> bool {
    std::env::var("NO_COLOR").is_err() && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
}

fn c(code: &str, text: &str) -> String {
    if use_color() {
        format!("\x1b[{}m{}\x1b[0m", code, text)
    } else {
        text.to_string()
    }
}
fn green(s: &str) -> String {
    c("32", s)
}
fn yellow(s: &str) -> String {
    c("33", s)
}
fn red(s: &str) -> String {
    c("31", s)
}
fn cyan(s: &str) -> String {
    c("36", s)
}
fn bold(s: &str) -> String {
    c("1", s)
}
fn dim(s: &str) -> String {
    c("2", s)
}

/// Visual length of a string — strips ANSI escape sequences before measuring.
fn vlen(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for ch in s.chars() {
        if ch == '\x1b' {
            in_escape = true;
            continue;
        }
        if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
            continue;
        }
        len += 1;
    }
    len
}

// ─── Formatting helpers ───────────────────────────────────────────────────────

fn hex8(d: &kosmo_core::Digest) -> String {
    d.to_hex()[..16].to_string()
}

fn q16_fmt(raw: i64) -> String {
    // Display as decimal fraction with 4 places: raw / 65536.0
    let i = raw / 65536;
    let f = (raw.unsigned_abs() % 65536) * 10000 / 65536;
    format!("{}.{:04}", i, f)
}

fn gate_str(r: &kosmo_core::GateResult) -> String {
    match r {
        kosmo_core::GateResult::Pass => green("✓ Pass"),
        kosmo_core::GateResult::Warn { .. } => yellow("⚠ Warn"),
        kosmo_core::GateResult::Reject { .. } => red("✗ Reject"),
        kosmo_core::GateResult::Downgrade { .. } => yellow("↓ Downgrade"),
    }
}

fn kind_label(kind: &ActionItemKind) -> &'static str {
    match kind {
        ActionItemKind::FillVoid { .. } => "FillVoid      ",
        ActionItemKind::RepairTopology { .. } => "RepairTopology",
        ActionItemKind::PromoteToPse { .. } => "PromoteToPse  ",
        ActionItemKind::ReviewCrystal { .. } => "ReviewCrystal ",
        ActionItemKind::ApplyNorm { .. } => "ApplyNorm     ",
        ActionItemKind::RealizeWishFacet { .. } => "RealizeWish   ",
    }
}

fn kind_color(kind: &ActionItemKind, s: &str) -> String {
    match kind {
        ActionItemKind::FillVoid { .. } => cyan(s),
        ActionItemKind::RepairTopology { .. } => yellow(s),
        ActionItemKind::PromoteToPse { .. } => green(s),
        ActionItemKind::ReviewCrystal { .. } => yellow(s),
        ActionItemKind::ApplyNorm { .. } => green(s),
        ActionItemKind::RealizeWishFacet { .. } => green(s),
    }
}

// ─── Text output ─────────────────────────────────────────────────────────────

const WIDTH: usize = 72;

fn ruler() -> String {
    dim(&"─".repeat(WIDTH))
}

fn print_report_text(path: &str, report: &IntegrationRunReport, run_count: u32) {
    // ── Header ────────────────────────────────────────────────────────────────
    let policy_label = if report.policy_id == PolicyProfile::operator_approved().id {
        "OperatorApproved"
    } else {
        "ReportOnly"
    };
    let inner = WIDTH; // inner content width (between │ borders)
    println!(
        "{}",
        bold(&format!(
            "╭─ Kosmocrates ─{:─<width$}╮",
            "",
            width = inner - 14
        ))
    );

    let box_row = |label: &str, value: String| {
        let prefix = format!("  {}  ", bold(label));
        let content = format!("{}{}", prefix, value);
        let pad = inner.saturating_sub(2 + vlen(&content));
        println!("│{}{}│", content, " ".repeat(pad));
    };

    box_row("workspace", cyan(path).to_string());
    box_row("policy   ", policy_label.to_string());
    box_row("gate     ", gate_str(&report.final_result));
    box_row(
        "report id",
        dim(&report.report_id.to_hex()[..24]).to_string(),
    );
    if run_count > 0 {
        box_row("session  ", format!("run #{run_count}"));
    }
    println!("{}", bold(&format!("╰{:─<width$}╯", "", width = inner + 2)));
    println!();

    // ── Workspace stats ───────────────────────────────────────────────────────
    let void_count = report.void_priority_ranking.len();
    let def_sev = report.deficiency_vector.total_severity;
    let source_count =
        report.hyphae_result.host_cube.void_map.voids.len() + report.source_cubes.len();
    println!("{}", bold("Workspace"));
    println!("  {:<24} {}", dim("source files"), source_count);
    println!(
        "  {:<24} {}  {}",
        dim("voids detected"),
        void_count,
        if void_count > 0 {
            format!("(total severity {})", q16_fmt(def_sev.raw()))
        } else {
            String::new()
        }
    );
    println!(
        "  {:<24} {} entries",
        dim("deficiency"),
        report.deficiency_vector.entries.len()
    );
    println!();

    // ── Action queue ─────────────────────────────────────────────────────────
    let items = report.action_items();
    println!("{}", bold(&format!("Action Queue  {} items", items.len())));
    if items.is_empty() {
        println!("  {}", dim("(no actions — workspace is clean)"));
    } else {
        println!("{}", ruler());
        for (i, item) in items.iter().enumerate().take(20) {
            let score_s = format!("[{}]", q16_fmt(item.priority_score.raw()));
            let label = kind_color(&item.kind, kind_label(&item.kind));
            // Truncate description to fit terminal width
            let desc_max = WIDTH.saturating_sub(5 + 8 + 15);
            let desc = if item.description.len() > desc_max {
                format!("{}…", &item.description[..desc_max.saturating_sub(1)])
            } else {
                item.description.clone()
            };
            println!("  {:>3}  {}  {}  {}", i + 1, dim(&score_s), label, desc);
        }
        if items.len() > 20 {
            println!(
                "  {} {} more items (use --output json for full list)",
                dim("…"),
                items.len() - 20
            );
        }
    }
    println!();

    // ── Crystal CAD library ───────────────────────────────────────────────────
    if !report.crystal_candidates.is_empty()
        || report.persisted_crystal_count > 0
        || !report.resonite_map.is_empty()
    {
        println!("{}", bold("Crystal CAD Library"));
        println!(
            "  {:<28} {}",
            dim("certified this run"),
            green(&report.certified_crystals.len().to_string())
        );
        println!(
            "  {:<28} {}",
            dim("resonite pairs"),
            report.resonite_map.len()
        );
        println!(
            "  {:<28} {}",
            dim("persisted to store"),
            report.persisted_crystal_count
        );
        println!();
    }

    // ── Optional layers ───────────────────────────────────────────────────────
    let has_opt = !report.metatron_diagnostics.is_empty()
        || !report.lpcm_reports.is_empty()
        || !report.surgery_options.is_empty()
        || report.systemcube_export.is_some()
        || !report.pse_candidates.is_empty()
        || !report.norm_candidates.is_empty()
        || !report.motif_candidates.is_empty();

    if has_opt {
        println!("{}", bold("Optional Layers"));
        if !report.metatron_diagnostics.is_empty() {
            println!(
                "  {:<28} {} diagnostics  {} lifts  {} ambiguities",
                dim("metatron"),
                report.metatron_diagnostics.len(),
                report.lift_reports.len(),
                report.ambiguity_profiles.len()
            );
        }
        if !report.lpcm_reports.is_empty() {
            println!(
                "  {:<28} {} reports",
                dim("lpcm"),
                report.lpcm_reports.len()
            );
        }
        if !report.surgery_options.is_empty() {
            println!(
                "  {:<28} {} options",
                dim("surgery"),
                report.surgery_options.len()
            );
        }
        if let Some(ref sc) = report.systemcube_export {
            println!(
                "  {:<28} mode={:?}  compat={}  contradiction_energy={}",
                dim("systemcube"),
                sc.mode,
                q16_fmt(sc.compatibility.compatibility_score.raw()),
                q16_fmt(sc.contradiction_energy.total_energy.raw())
            );
        }
        if !report.pse_candidates.is_empty() {
            println!(
                "  {:<28} {} candidates",
                dim("pse bridge"),
                report.pse_candidates.len()
            );
        }
        if !report.norm_candidates.is_empty() {
            println!(
                "  {:<28} {} genes  {} fitness traces",
                dim("norm candidates"),
                report.norm_candidates.len(),
                report.norm_fitness_traces.len()
            );
        }
        if !report.motif_candidates.is_empty() {
            println!(
                "  {:<28} {} patterns",
                dim("motif candidates"),
                report.motif_candidates.len()
            );
        }
        println!();
    }

    // ── Top void list ─────────────────────────────────────────────────────────
    if !report.void_priority_ranking.is_empty() {
        println!("{}", bold("Void Priority Ranking"));
        for (i, vid) in report.void_priority_ranking.iter().enumerate().take(10) {
            println!("  {:>3}  {}", i + 1, dim(&hex8(vid)));
        }
        if report.void_priority_ranking.len() > 10 {
            println!(
                "  {}  … {} more",
                dim(""),
                report.void_priority_ranking.len() - 10
            );
        }
        println!();
    }
}

// ─── Summary output ───────────────────────────────────────────────────────────

fn print_report_summary(path: &str, report: &IntegrationRunReport) {
    let gate = match &report.final_result {
        kosmo_core::GateResult::Pass => "PASS",
        kosmo_core::GateResult::Warn { .. } => "WARN",
        kosmo_core::GateResult::Reject { .. } => "REJECT",
        kosmo_core::GateResult::Downgrade { .. } => "DOWNGRADE",
    };
    println!(
        "{gate} | workspace={path} | voids={} | actions={} | crystals={} | report={}",
        report.void_priority_ranking.len(),
        report.action_items().len(),
        report.certified_crystals.len(),
        &report.report_id.to_hex()[..16],
    );
}

// ─── Markdown output ─────────────────────────────────────────────────────────

fn kind_md(kind: &ActionItemKind) -> &'static str {
    match kind {
        ActionItemKind::FillVoid { .. } => "FillVoid",
        ActionItemKind::RepairTopology { .. } => "RepairTopology",
        ActionItemKind::PromoteToPse { .. } => "PromoteToPse",
        ActionItemKind::ReviewCrystal { .. } => "ReviewCrystal",
        ActionItemKind::ApplyNorm { .. } => "ApplyNorm",
        ActionItemKind::RealizeWishFacet { .. } => "RealizeWishFacet",
    }
}

fn gate_md(r: &kosmo_core::GateResult) -> &'static str {
    match r {
        kosmo_core::GateResult::Pass => "✅ Pass",
        kosmo_core::GateResult::Warn { .. } => "⚠️ Warn",
        kosmo_core::GateResult::Reject { .. } => "❌ Reject",
        kosmo_core::GateResult::Downgrade { .. } => "⬇️ Downgrade",
    }
}

fn print_report_markdown(path: &str, report: &IntegrationRunReport, run_count: u32) {
    let policy_label = if report.policy_id == PolicyProfile::operator_approved().id {
        "OperatorApproved"
    } else {
        "ReportOnly"
    };
    let generated = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();

    println!("# Kosmocrates Analysis Report\n");
    println!("> **workspace:** `{path}`  ");
    println!("> **policy:** {policy_label}  ");
    println!("> **gate:** {}  ", gate_md(&report.final_result));
    println!("> **report id:** `{}`  ", &report.report_id.to_hex()[..32]);
    if run_count > 0 {
        println!("> **session:** run #{run_count}  ");
    }
    println!("> **generated:** {generated}  ");
    println!();
    println!("---\n");

    // ── Workspace ─────────────────────────────────────────────────────────────
    let void_count = report.void_priority_ranking.len();
    let def_sev = report.deficiency_vector.total_severity;
    let source_count =
        report.hyphae_result.host_cube.void_map.voids.len() + report.source_cubes.len();
    println!("## Workspace\n");
    println!("| Metric | Value |");
    println!("|--------|-------|");
    println!("| Source files | {source_count} |");
    println!("| Voids detected | {void_count} |");
    println!("| Total severity | {} |", q16_fmt(def_sev.raw()));
    println!(
        "| Deficiency entries | {} |",
        report.deficiency_vector.entries.len()
    );
    println!();

    // ── Action queue ─────────────────────────────────────────────────────────
    let items = report.action_items();
    println!("## Action Queue ({} items)\n", items.len());
    if items.is_empty() {
        println!("_No actions — workspace is clean._\n");
    } else {
        println!("| # | Score | Kind | Description |");
        println!("|---|------:|------|-------------|");
        for (i, item) in items.iter().enumerate().take(50) {
            let desc = item.description.replace('|', "\\|");
            println!(
                "| {} | {} | {} | {} |",
                i + 1,
                q16_fmt(item.priority_score.raw()),
                kind_md(&item.kind),
                desc,
            );
        }
        if items.len() > 50 {
            println!();
            println!(
                "_… {} more items. Use `--output json` for the full list._",
                items.len() - 50
            );
        }
        println!();
    }

    // ── Crystal CAD library ───────────────────────────────────────────────────
    if !report.crystal_candidates.is_empty()
        || report.persisted_crystal_count > 0
        || !report.resonite_map.is_empty()
    {
        println!("## Crystal CAD Library\n");
        println!("| Metric | Value |");
        println!("|--------|-------|");
        println!(
            "| Certified this run | {} |",
            report.certified_crystals.len()
        );
        println!("| Resonite pairs | {} |", report.resonite_map.len());
        println!(
            "| Persisted to store | {} |",
            report.persisted_crystal_count
        );
        println!();
    }

    // ── Optional layers ───────────────────────────────────────────────────────
    let has_opt = !report.metatron_diagnostics.is_empty()
        || !report.lpcm_reports.is_empty()
        || !report.surgery_options.is_empty()
        || report.systemcube_export.is_some()
        || !report.pse_candidates.is_empty()
        || !report.norm_candidates.is_empty()
        || !report.motif_candidates.is_empty();

    if has_opt {
        println!("## Optional Layers\n");
        println!("| Layer | Details |");
        println!("|-------|---------|");
        if !report.metatron_diagnostics.is_empty() {
            println!(
                "| Metatron | {} diagnostics, {} lifts, {} ambiguities |",
                report.metatron_diagnostics.len(),
                report.lift_reports.len(),
                report.ambiguity_profiles.len()
            );
        }
        if !report.lpcm_reports.is_empty() {
            println!("| LPCM | {} reports |", report.lpcm_reports.len());
        }
        if !report.surgery_options.is_empty() {
            println!("| Surgery | {} options |", report.surgery_options.len());
        }
        if let Some(ref sc) = report.systemcube_export {
            println!(
                "| SystemCube | mode={:?}, compat={}, contradiction_energy={} |",
                sc.mode,
                q16_fmt(sc.compatibility.compatibility_score.raw()),
                q16_fmt(sc.contradiction_energy.total_energy.raw())
            );
        }
        if !report.pse_candidates.is_empty() {
            println!(
                "| PSE Bridge | {} candidates |",
                report.pse_candidates.len()
            );
        }
        if !report.norm_candidates.is_empty() {
            println!(
                "| Norm Candidates | {} genes, {} fitness traces |",
                report.norm_candidates.len(),
                report.norm_fitness_traces.len()
            );
        }
        if !report.motif_candidates.is_empty() {
            println!(
                "| Motif Candidates | {} patterns |",
                report.motif_candidates.len()
            );
        }
        println!();
    }

    // ── Void priority ranking ─────────────────────────────────────────────────
    if !report.void_priority_ranking.is_empty() {
        println!("## Void Priority Ranking\n");
        for (i, vid) in report.void_priority_ranking.iter().enumerate().take(20) {
            println!("{}. `{}`", i + 1, &vid.to_hex()[..16]);
        }
        if report.void_priority_ranking.len() > 20 {
            println!();
            println!(
                "_… {} more voids._",
                report.void_priority_ranking.len() - 20
            );
        }
        println!();
    }

    // ── Footer ────────────────────────────────────────────────────────────────
    println!("---");
    println!();
    println!(
        "_Generated by [kosmo-substrate](https://github.com/LashSesh/kosmocrates) v{}_",
        env!("CARGO_PKG_VERSION")
    );
}

// ─── Build options ────────────────────────────────────────────────────────────

fn build_options(args: &Args) -> IntegrationRunOptions {
    if args.all_layers {
        let opts = IntegrationRunOptions::all_layers(args.capacity);
        return if let Some(ref p) = args.store {
            opts.with_crystal_store_path(p)
        } else {
            opts
        };
    }

    let mut opts = IntegrationRunOptions {
        enable_metatron: args.metatron,
        enable_lpcm: args.lpcm,
        enable_systemcube: args.systemcube,
        systemcube_capacity: args.capacity,
        lpcm_seam_threshold: kosmo_core::Q16::ZERO,
        enable_surgery: args.surgery,
        enable_motif_candidates: args.motifs,
        enable_norm_candidates: args.norms,
        enable_crystal_candidates: args.crystals,
        enable_pse_candidates: args.pse,
        prior_feedback: vec![],
        prior_motifs: vec![],
        prior_motif_min_support: kosmo_core::Q16::HALF,
        prior_crystals: vec![],
        crystal_store_path: None,
    };

    if let Some(ref p) = args.store {
        opts = opts.with_crystal_store_path(p);
        // Crystal store implies we want crystal candidates.
        opts.enable_crystal_candidates = true;
    }

    opts
}

// ─── main ────────────────────────────────────────────────────────────────────

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{} {}", red("error:"), e);
            eprintln!("Run {} for usage.", bold("kosmo-substrate --help"));
            process::exit(2);
        }
    };

    let policy = if args.operator {
        PolicyProfile::operator_approved()
    } else {
        PolicyProfile::default_report_only()
    };

    let options = build_options(&args);

    // Ensure crystal store parent directory exists.
    if let Some(ref store_path) = options.crystal_store_path {
        if let Some(parent) = store_path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
    }

    let mut session = WorkspacePipelineSession::new(options, policy);

    if args.output == OutputFormat::Text || args.output == OutputFormat::Markdown {
        eprintln!("{} {}…", dim("analysing"), cyan(&args.path));
    }

    let report = match session.run(&args.path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} {}", red("error:"), e);
            process::exit(2);
        }
    };

    match args.output {
        OutputFormat::Text => {
            print_report_text(&args.path, &report, session.run_count());
        }
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(j) => println!("{j}"),
            Err(e) => {
                eprintln!("{} serialization failed: {e}", red("error:"));
                process::exit(2);
            }
        },
        OutputFormat::Markdown => {
            print_report_markdown(&args.path, &report, session.run_count());
        }
        OutputFormat::Summary => {
            print_report_summary(&args.path, &report);
        }
    }

    // Exit code.
    let should_fail = match &report.final_result {
        kosmo_core::GateResult::Reject { .. } => args.fail_on_reject,
        kosmo_core::GateResult::Warn { .. } => args.fail_on_warn,
        _ => false,
    };
    if should_fail {
        process::exit(1);
    }
}
