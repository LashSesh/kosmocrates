//! Kosmocrates TUI — interactive workspace topology dashboard.
//!
//! Three-pane layout: scrollable action queue │ detail view │ workspace stats.
//!
//! ```text
//! USAGE:
//!     kosmo-tui [OPTIONS] [PATH]
//!
//! Keybindings: q/Esc=quit  r=rerun  p=promote  l=landscape  ↑↓/jk=navigate  PgUp/PgDn=page  g=top  G=bottom
//! ```
//!
//! `p` offers the workspace's gated candidates to the PSE engine — ensemble
//! batch under the substrate calibration, **in-memory only** (no ledger, no
//! archive: the host-writing surfaces stay on `kosmo-promote`/HTTP where paths
//! are explicit). The promotion line shows verdicts, engine state, and the
//! QTIC class (capped at Q3 without an Infinity-Ledger anchor).

use std::io;
use std::path::PathBuf;
use std::process;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};

use kosmo_core::{GateResult, PolicyProfile};
use kosmo_pipeline::{
    ActionItem, ActionItemKind, IntegrationRunOptions, IntegrationRunReport,
    WorkspacePipelineSession,
};

// ─── Args ────────────────────────────────────────────────────────────────────

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
}

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(format!("{}{}", home, &path[1..]));
        }
    }
    PathBuf::from(path)
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
    };
    let mut i = 0usize;
    while i < raw.len() {
        match raw[i].as_str() {
            "-h" | "--help" => {
                println!(concat!(
                    "Kosmocrates TUI — interactive workspace topology dashboard\n",
                    "\n",
                    "USAGE:\n",
                    "    kosmo-tui [OPTIONS] [PATH]\n",
                    "\n",
                    "OPTIONS: same flags as kosmo-substrate\n",
                    "    --store <path>   Crystal store path (persists CAD library)\n",
                    "    --all            Enable all analysis layers\n",
                    "    --metatron / --lpcm / --systemcube / --surgery\n",
                    "    --crystals / --norms / --motifs / --pse\n",
                    "    --operator       OperatorApproved policy\n",
                    "    --capacity <n>   SystemCube capacity (default 100)\n",
                    "\n",
                    "KEYBINDINGS:\n",
                    "    q / Esc          Quit\n",
                    "    r                Re-run analysis\n",
                    "    p                Promote: offer the gated candidates to the\n",
                    "                     PSE engine (ensemble batch, substrate\n",
                    "                     calibration, in-memory — no host writes)\n",
                    "    ↑↓ / j k         Navigate action queue\n",
                    "    PgUp / PgDn      Page up / down\n",
                    "    g                Jump to top\n",
                    "    G                Jump to bottom\n",
                ));
                process::exit(0);
            }
            "-V" | "--version" => {
                println!("kosmo-tui {}", env!("CARGO_PKG_VERSION"));
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
            "--store" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--store requires a path".into());
                }
                args.store = Some(expand_tilde(&raw[i]));
            }
            "--capacity" => {
                i += 1;
                if i >= raw.len() {
                    return Err("--capacity requires a number".into());
                }
                args.capacity = raw[i]
                    .parse()
                    .map_err(|_| format!("--capacity must be a number, got '{}'", raw[i]))?;
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
        opts.enable_crystal_candidates = true;
    }
    opts
}

// ─── Promotion (substrate→core, in-memory) ───────────────────────────────────

/// Outcome summary of one in-memory promotion run.
#[derive(Debug)]
struct PromotionSummary {
    offered: usize,
    accepted: usize,
    deferred: usize,
    rejected: usize,
    engine_state: String,
    kairos: Option<bool>,
    /// QTIC class of the first accepted crystal (Q3 ceiling without IL anchor).
    qtic_top: Option<u8>,
}

/// One row of the TUI wish landscape: standing mark, facet, severity.
#[derive(Debug)]
struct LandscapeRow {
    mark: &'static str,
    facet: String,
    severity: f64,
    rationale: String,
}

/// Summary of the last `l` landscape run.
#[derive(Debug)]
struct LandscapeSummary {
    proposals: usize,
    met: usize,
    open: usize,
    beyond_observation: usize,
    beyond_vocabulary: usize,
    /// The highest peaks (top 3 by severity).
    top: Vec<LandscapeRow>,
    /// The spectral shape of the open landscape, pre-rendered: one line per
    /// coherent cluster, then one per singular proposal.
    geometry: Vec<String>,
}

/// Map the workspace's findings into the wish landscape — read-only
/// (the adopting/descending surfaces stay on `kosmo-run` where `--apply`
/// is explicit).
fn run_landscape(path: &str) -> Result<LandscapeSummary, String> {
    use kosmo_pipeline::{
        measure_landscape, propose_wishes, run_workspace_pipeline, LandscapeStanding,
    };
    let policy = PolicyProfile::default_report_only();
    let options = IntegrationRunOptions::report_only();
    let report = run_workspace_pipeline(path, &options, &policy)
        .map_err(|e| format!("pipeline failed: {e}"))?;
    let landscape = propose_wishes(&report.hyphae_result.host_cube.void_map.voids);
    let observed = kosmo_intent::observe_workspace_deep(path).ok();
    let standing = measure_landscape(&landscape, observed.as_ref());

    let count = |s: LandscapeStanding| standing.iter().filter(|x| **x == s).count();
    let top = landscape
        .proposals
        .iter()
        .zip(&standing)
        .take(3)
        .map(|(p, s)| LandscapeRow {
            mark: match s {
                LandscapeStanding::Met => "✓",
                LandscapeStanding::Open => "✗",
                LandscapeStanding::BeyondObservation => "∅",
                LandscapeStanding::Unmeasured => "?",
            },
            facet: format!("{:?} {}", p.facet.kind, p.facet.key),
            severity: p.severity.to_f64(),
            rationale: p.rationale.clone(),
        })
        .collect();
    // The spectral shape of the OPEN landscape — the same standing
    // definition as the CLI and HTTP surfaces.
    let open_proposals: Vec<kosmo_pipeline::WishProposal> = landscape
        .proposals
        .iter()
        .zip(&standing)
        .filter(|(_, s)| **s == LandscapeStanding::Open)
        .map(|(p, _)| p.clone())
        .collect();
    let geo = kosmo_pipeline::landscape_geometry(&open_proposals, 6);
    let mut geometry: Vec<String> = geo
        .clusters
        .iter()
        .enumerate()
        .map(|(i, cl)| {
            let facets: Vec<String> = cl
                .members
                .iter()
                .map(|&m| {
                    format!(
                        "{:?} {}",
                        open_proposals[m].facet.kind, open_proposals[m].facet.key
                    )
                })
                .collect();
            format!(
                "cluster {} [mass {:.2}] {} — {}",
                i + 1,
                cl.severity_mass.to_f64(),
                cl.subjects.join(", "),
                facets.join(", ")
            )
        })
        .collect();
    for sng in &geo.singular {
        geometry.push(format!(
            "singular: {:?} {} (coupling {:.2})",
            open_proposals[sng.index].facet.kind,
            open_proposals[sng.index].facet.key,
            sng.coupling_mass.to_f64()
        ));
    }

    Ok(LandscapeSummary {
        proposals: landscape.proposals.len(),
        met: count(LandscapeStanding::Met),
        open: count(LandscapeStanding::Open),
        beyond_observation: count(LandscapeStanding::BeyondObservation),
        beyond_vocabulary: landscape.unmapped.len(),
        top,
        geometry,
    })
}

/// Offer the workspace's gated candidates to the PSE engine — ensemble batch
/// under the substrate calibration, in-memory only (no host writes; the
/// persisting surfaces stay on `kosmo-promote`/HTTP where paths are explicit).
fn run_promotion(path: &str) -> Result<PromotionSummary, String> {
    use kosmo_core::ImplementationMode;
    use kosmo_pipeline::run_workspace_pipeline;
    use kosmo_pse_bridge::{
        PromotionOutcome, PseBridgeCandidateKind, PseBridgePolicy, PseBridgeRateLimit,
    };
    use pse_adapter_kosmo::{offer_batch, KosmoBridgeAdapter};

    let pipeline_policy = PolicyProfile::default_report_only();
    let options = IntegrationRunOptions::all_layers(8);
    let report = run_workspace_pipeline(path, &options, &pipeline_policy)
        .map_err(|e| format!("pipeline failed: {e}"))?;

    let kinds = vec![
        PseBridgeCandidateKind::CertifiedCrystal,
        PseBridgeCandidateKind::StructuralObservation,
        PseBridgeCandidateKind::TopologyObservation,
    ];
    let candidates: Vec<_> = report
        .pse_candidates
        .iter()
        .filter(|c| kinds.contains(&c.kind))
        .cloned()
        .collect();

    let profile = PolicyProfile {
        mode: ImplementationMode::DryRun,
        ..PolicyProfile::default()
    };
    let bridge_policy =
        PseBridgePolicy::allow(pipeline_policy.id, kinds, PseBridgeRateLimit::strict());
    let adapter = KosmoBridgeAdapter::new(path)
        .with_allowed_kinds(bridge_policy.allowed_candidate_kinds.clone());

    // The substrate calibration (see kosmo-promote::Calibration): kairos as
    // the discriminant, the 8-fold conjunctive gate fully armed.
    let mut config = pse_types::Config::preset_planning();
    config.consensus.consensus_threshold = 0.0;
    config.consensus.mirror_consistency_eta = 0.0;
    let mut state = pse_core::GlobalState::new(&config);

    let offers = offer_batch(
        &mut state,
        &config,
        &profile,
        &bridge_policy,
        &candidates,
        &adapter,
    );

    let mut summary = PromotionSummary {
        offered: candidates.len(),
        accepted: 0,
        deferred: 0,
        rejected: 0,
        engine_state: format!("{:?}", state.engine_state),
        kairos: state.last_gate.as_ref().map(|g| g.kairos),
        qtic_top: None,
    };
    for offer in &offers {
        match &offer.outcome {
            PromotionOutcome::Accepted => {
                summary.accepted += 1;
                if summary.qtic_top.is_none() {
                    summary.qtic_top = offer.qtic.as_ref().map(|q| q.class_u8());
                }
            }
            PromotionOutcome::Deferred => summary.deferred += 1,
            PromotionOutcome::Rejected { .. } => summary.rejected += 1,
            PromotionOutcome::SkippedByPolicy | PromotionOutcome::SkippedByReportOnly => {}
        }
    }
    Ok(summary)
}

// ─── App state ───────────────────────────────────────────────────────────────

// Held as a single live value in the TUI's `App`; never stored in bulk, so the
// size gap between `Ready` and the other variants costs nothing in practice.
#[allow(clippy::large_enum_variant)]
enum AppPhase {
    Analysing,
    Ready {
        report: IntegrationRunReport,
        items: Vec<ActionItem>,
    },
    Error(String),
}

struct App {
    path: String,
    session: WorkspacePipelineSession,
    phase: AppPhase,
    selected: usize,
    offset: usize,
    /// Result of the last `p` promotion run (in-memory offer), if any.
    promotion: Option<Result<PromotionSummary, String>>,
    /// Result of the last `l` landscape mapping, if any.
    landscape: Option<Result<LandscapeSummary, String>>,
}

impl App {
    fn new(path: String, options: IntegrationRunOptions, policy: PolicyProfile) -> Self {
        Self {
            path,
            session: WorkspacePipelineSession::new(options, policy),
            phase: AppPhase::Analysing,
            selected: 0,
            offset: 0,
            promotion: None,
            landscape: None,
        }
    }

    fn run_promotion(&mut self) {
        self.promotion = Some(run_promotion(&self.path));
    }

    fn run_landscape(&mut self) {
        self.landscape = Some(run_landscape(&self.path));
    }

    fn run_pipeline(&mut self) {
        self.phase = AppPhase::Analysing;
        match self.session.run(&self.path) {
            Ok(report) => {
                let items = report.action_items();
                self.phase = AppPhase::Ready { report, items };
                self.selected = 0;
                self.offset = 0;
            }
            Err(e) => {
                self.phase = AppPhase::Error(e.to_string());
            }
        }
    }

    fn item_count(&self) -> usize {
        match &self.phase {
            AppPhase::Ready { items, .. } => items.len(),
            _ => 0,
        }
    }

    fn selected_item(&self) -> Option<&ActionItem> {
        match &self.phase {
            AppPhase::Ready { items, .. } => items.get(self.selected),
            _ => None,
        }
    }

    fn scroll_down(&mut self, list_height: usize) {
        let max = self.item_count().saturating_sub(1);
        if self.selected < max {
            self.selected += 1;
            if self.selected >= self.offset + list_height {
                self.offset += 1;
            }
        }
    }

    fn scroll_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.offset {
                self.offset = self.selected;
            }
        }
    }

    fn page_down(&mut self, list_height: usize) {
        let n = self.item_count();
        if n == 0 {
            return;
        }
        self.selected = (self.selected + list_height).min(n - 1);
        if self.selected >= self.offset + list_height {
            self.offset = self.selected.saturating_sub(list_height - 1);
        }
    }

    fn page_up(&mut self, list_height: usize) {
        self.selected = self.selected.saturating_sub(list_height);
        if self.selected < self.offset {
            self.offset = self.selected;
        }
    }

    fn go_top(&mut self) {
        self.selected = 0;
        self.offset = 0;
    }

    fn go_bottom(&mut self) {
        let n = self.item_count();
        if n > 0 {
            self.selected = n - 1;
        }
    }
}

// ─── Formatting helpers ───────────────────────────────────────────────────────

fn q16_fmt(raw: i64) -> String {
    let i = raw / 65536;
    let f = (raw.unsigned_abs() % 65536) * 10000 / 65536;
    format!("{}.{:04}", i, f)
}

fn hex16(d: &kosmo_core::Digest) -> String {
    d.to_hex()[..16].to_string()
}

fn gate_color(r: &GateResult) -> Color {
    match r {
        GateResult::Pass => Color::Green,
        GateResult::Warn { .. } => Color::Yellow,
        GateResult::Reject { .. } => Color::Red,
        GateResult::Downgrade { .. } => Color::Yellow,
    }
}

fn gate_label(r: &GateResult) -> &'static str {
    match r {
        GateResult::Pass => "✓ Pass",
        GateResult::Warn { .. } => "⚠ Warn",
        GateResult::Reject { .. } => "✗ Reject",
        GateResult::Downgrade { .. } => "↓ Downgrade",
    }
}

fn kind_label(kind: &ActionItemKind) -> &'static str {
    match kind {
        ActionItemKind::FillVoid { .. } => "FillVoid  ",
        ActionItemKind::RepairTopology { .. } => "Repair    ",
        ActionItemKind::PromoteToPse { .. } => "PromotePSE",
        ActionItemKind::ReviewCrystal { .. } => "Review    ",
        ActionItemKind::ApplyNorm { .. } => "ApplyNorm ",
        ActionItemKind::RealizeWishFacet { .. } => "WishFacet ",
    }
}

fn kind_color(kind: &ActionItemKind) -> Color {
    match kind {
        ActionItemKind::FillVoid { .. } => Color::Cyan,
        ActionItemKind::RepairTopology { .. } => Color::Yellow,
        ActionItemKind::PromoteToPse { .. } => Color::Green,
        ActionItemKind::ReviewCrystal { .. } => Color::Yellow,
        ActionItemKind::ApplyNorm { .. } => Color::Green,
        ActionItemKind::RealizeWishFacet { .. } => Color::Green,
    }
}

fn target_id(kind: &ActionItemKind) -> String {
    match kind {
        ActionItemKind::FillVoid { void_id } => hex16(void_id),
        ActionItemKind::RepairTopology { surgery_option_id } => hex16(surgery_option_id),
        ActionItemKind::PromoteToPse { candidate_id } => hex16(candidate_id),
        ActionItemKind::ReviewCrystal { candidate_id } => hex16(candidate_id),
        ActionItemKind::ApplyNorm {
            norm_candidate_id, ..
        } => hex16(norm_candidate_id),
        ActionItemKind::RealizeWishFacet { facet } => facet.key.clone(),
    }
}

// ─── Rendering ───────────────────────────────────────────────────────────────

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let (gate_text, gate_col, run_suffix) = match &app.phase {
        AppPhase::Analysing => ("Analysing…".to_string(), Color::DarkGray, String::new()),
        AppPhase::Ready { report, .. } => (
            gate_label(&report.final_result).to_string(),
            gate_color(&report.final_result),
            format!("  │  run #{}", app.session.run_count()),
        ),
        AppPhase::Error(_) => ("Error".to_string(), Color::Red, String::new()),
    };

    let policy_label = match &app.phase {
        AppPhase::Ready { report, .. }
            if report.policy_id == PolicyProfile::operator_approved().id =>
        {
            "OperatorApproved"
        }
        _ => "ReportOnly",
    };

    let line = Line::from(vec![
        Span::styled(
            " Kosmocrates",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
        Span::styled(app.path.as_str(), Style::default().fg(Color::Cyan)),
        Span::styled("  │  policy: ", Style::default().fg(Color::DarkGray)),
        Span::raw(policy_label),
        Span::styled("  │  gate: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            gate_text,
            Style::default().fg(gate_col).add_modifier(Modifier::BOLD),
        ),
        Span::styled(run_suffix, Style::default().fg(Color::DarkGray)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(line), inner);
}

fn render_action_list(f: &mut Frame, area: Rect, app: &App, list_height: usize) {
    let title = match &app.phase {
        AppPhase::Ready { items, .. } => format!(" Action Queue ({}) ", items.len()),
        AppPhase::Analysing => " Action Queue ".to_string(),
        AppPhase::Error(_) => " Action Queue ".to_string(),
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    match &app.phase {
        AppPhase::Ready { items, .. } => {
            let desc_max = inner.width.saturating_sub(30) as usize;
            let visible: Vec<ListItem> = items
                .iter()
                .enumerate()
                .skip(app.offset)
                .take(list_height)
                .map(|(i, item)| {
                    let score = q16_fmt(item.priority_score.raw());
                    let kl = kind_label(&item.kind);
                    let desc = if item.description.len() > desc_max {
                        format!("{}…", &item.description[..desc_max.saturating_sub(1)])
                    } else {
                        item.description.clone()
                    };
                    let line = Line::from(vec![
                        Span::styled(
                            format!("{:>4} ", i + 1),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("[{}] ", score),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(kl, Style::default().fg(kind_color(&item.kind))),
                        Span::raw("  "),
                        Span::raw(desc),
                    ]);
                    if i == app.selected {
                        ListItem::new(line).style(
                            Style::default()
                                .bg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        ListItem::new(line)
                    }
                })
                .collect();

            // Scroll position indicator in bottom-right of the inner area
            if !items.is_empty() && inner.height > 0 && inner.width > 10 {
                let pct = (app.selected * 100) / items.len().max(1);
                let indicator = format!(" {}% ", pct);
                let x = inner.x + inner.width.saturating_sub(indicator.len() as u16 + 1);
                let y = inner.y + inner.height - 1;
                let ind_area = Rect::new(x, y, indicator.len() as u16, 1);
                f.render_widget(
                    Paragraph::new(indicator).style(Style::default().fg(Color::DarkGray)),
                    ind_area,
                );
            }

            f.render_widget(List::new(visible), inner);
        }
        AppPhase::Analysing => {
            f.render_widget(
                Paragraph::new("  Analysing…").style(Style::default().fg(Color::DarkGray)),
                inner,
            );
        }
        AppPhase::Error(e) => {
            f.render_widget(
                Paragraph::new(e.as_str())
                    .style(Style::default().fg(Color::Red))
                    .wrap(Wrap { trim: false }),
                inner,
            );
        }
    }
}

fn render_detail(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Detail ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(item) = app.selected_item() else {
        f.render_widget(
            Paragraph::new("(select an item)").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    };

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Kind:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                kind_label(&item.kind).trim(),
                Style::default()
                    .fg(kind_color(&item.kind))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Score:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(q16_fmt(item.priority_score.raw())),
        ]),
        Line::from(vec![
            Span::styled("Target:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(target_id(&item.kind), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Action:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                item.action_id.to_hex()[..16].to_string(),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "Description:",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let wrap_width = inner.width.saturating_sub(4) as usize;
    let mut buf = String::new();
    for word in item.description.split_whitespace() {
        if buf.is_empty() {
            buf = word.to_string();
        } else if buf.len() + 1 + word.len() <= wrap_width {
            buf.push(' ');
            buf.push_str(word);
        } else {
            lines.push(Line::raw(format!("  {}", buf)));
            buf = word.to_string();
        }
    }
    if !buf.is_empty() {
        lines.push(Line::raw(format!("  {}", buf)));
    }

    if let ActionItemKind::ApplyNorm { name, .. } = &item.kind {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("Norm:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(name.as_str()),
        ]));
    }

    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_stats(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Workspace ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let AppPhase::Ready { report, .. } = &app.phase {
        let source_count =
            report.hyphae_result.host_cube.void_map.voids.len() + report.source_cubes.len();
        let def_sev = report.deficiency_vector.total_severity;

        let mut spans = vec![
            Span::styled("files: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{source_count}")),
            Span::styled("  │  voids: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", report.void_priority_ranking.len())),
            Span::styled("  │  severity: ", Style::default().fg(Color::DarkGray)),
            Span::raw(q16_fmt(def_sev.raw())),
            Span::styled("  │  deficiency: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", report.deficiency_vector.entries.len())),
        ];

        if report.persisted_crystal_count > 0 {
            spans.push(Span::styled(
                "  │  crystals stored: ",
                Style::default().fg(Color::DarkGray),
            ));
            spans.push(Span::styled(
                format!("{}", report.persisted_crystal_count),
                Style::default().fg(Color::Green),
            ));
        }

        if !report.certified_crystals.is_empty() {
            spans.push(Span::styled(
                "  │  certified: ",
                Style::default().fg(Color::DarkGray),
            ));
            spans.push(Span::styled(
                format!("{}", report.certified_crystals.len()),
                Style::default().fg(Color::Green),
            ));
        }

        let mut lines = vec![Line::from(spans)];
        if let Some(promotion) = &app.promotion {
            lines.push(promotion_line(promotion));
        }
        if let Some(landscape) = &app.landscape {
            lines.extend(landscape_lines_render(landscape));
        }
        f.render_widget(Paragraph::new(lines), inner);
    }
}

/// One-line summary of the last `p` promotion run.
fn promotion_line(promotion: &Result<PromotionSummary, String>) -> Line<'static> {
    match promotion {
        Ok(s) => {
            let accepted_style = if s.accepted > 0 {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow)
            };
            let mut spans = vec![
                Span::styled("promotion: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{} offered", s.offered)),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{} accepted", s.accepted), accepted_style),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{} deferred", s.deferred)),
            ];
            if s.rejected > 0 {
                spans.push(Span::styled("  │  ", Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled(
                    format!("{} rejected", s.rejected),
                    Style::default().fg(Color::Red),
                ));
            }
            if let Some(kairos) = s.kairos {
                spans.push(Span::styled(
                    "  │  kairos: ",
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled(
                    if kairos { "✓" } else { "✗" },
                    if kairos {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Yellow)
                    },
                ));
            }
            if let Some(q) = s.qtic_top {
                spans.push(Span::styled(
                    "  │  QTIC: ",
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled(
                    format!("Q{q}"),
                    Style::default().fg(Color::Cyan),
                ));
            }
            spans.push(Span::styled(
                format!("  │  engine: {}", s.engine_state),
                Style::default().fg(Color::DarkGray),
            ));
            Line::from(spans)
        }
        Err(e) => Line::from(vec![
            Span::styled("promotion: ", Style::default().fg(Color::DarkGray)),
            Span::styled(e.clone(), Style::default().fg(Color::Red)),
        ]),
    }
}

fn render_status_bar(f: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(
            " q",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":quit  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":rerun  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "p",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":promote  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "l",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":landscape  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "↑↓/jk",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "PgUp/PgDn",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":page  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "g/G",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":top/bottom", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn ui(f: &mut Frame, app: &App, list_height: usize) {
    // The workspace pane grows by one line when a promotion summary exists.
    let landscape_lines = match &app.landscape {
        Some(Ok(s)) => 1 + s.top.len() as u16,
        Some(Err(_)) => 1,
        None => 0,
    };
    let stats_height = (if app.promotion.is_some() { 4 } else { 3 }) + landscape_lines;
    let vertical = Layout::vertical([
        Constraint::Length(3),            // header
        Constraint::Min(5),               // action list + detail
        Constraint::Length(stats_height), // workspace stats (+ promotion line)
        Constraint::Length(1),            // key hints
    ])
    .split(f.area());

    render_header(f, vertical[0], app);

    let horizontal = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(vertical[1]);

    render_action_list(f, horizontal[0], app, list_height);
    render_detail(f, horizontal[1], app);
    render_stats(f, vertical[2], app);
    render_status_bar(f, vertical[3]);
}

// ─── TUI event loop ──────────────────────────────────────────────────────────

fn run_tui(mut app: App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Draw the "Analysing…" frame before blocking on the pipeline.
    terminal.draw(|f| {
        let h = f.area().height.saturating_sub(10) as usize;
        ui(f, &app, h);
    })?;

    app.run_pipeline();

    loop {
        let list_height = terminal.size()?.height.saturating_sub(10) as usize;

        terminal.draw(|f| ui(f, &app, list_height))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('r') => {
                        terminal.draw(|f| {
                            let h = f.area().height.saturating_sub(10) as usize;
                            ui(f, &app, h);
                        })?;
                        app.run_pipeline();
                    }
                    KeyCode::Char('p') => {
                        terminal.draw(|f| {
                            let h = f.area().height.saturating_sub(10) as usize;
                            ui(f, &app, h);
                        })?;
                        app.run_promotion();
                    }
                    KeyCode::Char('l') => {
                        terminal.draw(|f| {
                            let h = f.area().height.saturating_sub(10) as usize;
                            ui(f, &app, h);
                        })?;
                        app.run_landscape();
                    }
                    KeyCode::Down | KeyCode::Char('j') => app.scroll_down(list_height),
                    KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
                    KeyCode::Char('g') => app.go_top(),
                    KeyCode::Char('G') => app.go_bottom(),
                    KeyCode::PageDown => app.page_down(list_height),
                    KeyCode::PageUp => app.page_up(list_height),
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// ─── main ────────────────────────────────────────────────────────────────────

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("Run kosmo-tui --help for usage.");
            process::exit(2);
        }
    };

    let policy = if args.operator {
        PolicyProfile::operator_approved()
    } else {
        PolicyProfile::default_report_only()
    };

    let options = build_options(&args);

    if let Some(ref store_path) = options.crystal_store_path {
        if let Some(parent) = store_path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
    }

    let app = App::new(args.path, options, policy);

    if let Err(e) = run_tui(app) {
        eprintln!("terminal error: {e}");
        process::exit(1);
    }
}

/// Render the last `l` landscape mapping: one summary line plus the top
/// peaks (mark · facet · severity · provenance).
fn landscape_lines_render(landscape: &Result<LandscapeSummary, String>) -> Vec<Line<'static>> {
    match landscape {
        Ok(s) => {
            let open_style = if s.open > 0 {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            let mut lines = vec![Line::from(vec![
                Span::styled("landscape: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{} proposal(s)", s.proposals)),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{} met", s.met), Style::default().fg(Color::Green)),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{} open", s.open), open_style),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!(
                    "{} beyond obs · {} beyond vocab",
                    s.beyond_observation, s.beyond_vocabulary
                )),
            ])];
            for row in &s.top {
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(row.mark.to_string(), Style::default().fg(Color::Yellow)),
                    Span::raw(format!(" sev={:.2} {} ", row.severity, row.facet)),
                    Span::styled(
                        format!("← {}", row.rationale),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
            for line in &s.geometry {
                lines.push(Line::from(vec![
                    Span::styled("  ◆ ", Style::default().fg(Color::Cyan)),
                    Span::raw(line.clone()),
                ]));
            }
            lines
        }
        Err(e) => vec![Line::from(vec![
            Span::styled("landscape: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("error: {e}"), Style::default().fg(Color::Red)),
        ])],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The TUI's promotion core against a real polyglot workspace: in-memory
    /// offer under the substrate calibration must crystallize, and without an
    /// Infinity-Ledger anchor the QTIC class honestly caps at Q3.
    #[test]
    fn tui_promotion_core_crystallizes_in_memory() {
        let root = std::env::temp_dir()
            .join("kosmo-tui-tests")
            .join("promotion");
        std::fs::remove_dir_all(&root).ok();
        let ws = root.join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        let files: [(&str, &str); 4] = [
            (
                "fib.py",
                "import os\n\ndef a(x):\n    return x\n\ndef b(y):\n    return y\n",
            ),
            (
                "fib.rs",
                "use std::io;\npub fn a(x: u32) -> u32 { x }\npub fn b(y: u32) -> u32 { y }\n",
            ),
            (
                "fib.go",
                "package main\nimport \"fmt\"\nfunc a(n int) int { return n }\nfunc b(n int) int { return n }\n",
            ),
            (
                "fib.js",
                "function a(x) {\n  return x;\n}\nfunction b(y) {\n  return y;\n}\n",
            ),
        ];
        for (name, content) in files {
            std::fs::write(ws.join(name), content).unwrap();
        }

        let summary = run_promotion(&ws.to_string_lossy()).expect("promotion must run");
        assert!(summary.offered > 0, "candidates must be offered");
        assert!(
            summary.accepted > 0,
            "the ensemble must crystallize (accepted {}, deferred {}, engine {})",
            summary.accepted,
            summary.deferred,
            summary.engine_state
        );
        assert_eq!(summary.kairos, Some(true), "the conjunctive gate fired");
        assert_eq!(
            summary.qtic_top,
            Some(3),
            "in-memory promotion caps at Q3 (no IL anchor from the TUI)"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn tui_promotion_core_reports_pipeline_errors() {
        let err = run_promotion("/definitely/not/a/workspace").unwrap_err();
        assert!(
            err.contains("pipeline failed"),
            "must name the failure: {err}"
        );
    }

    #[test]
    fn tui_landscape_core_maps_and_measures() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-tui-landscape-{nanos}"));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub mod router;\n").unwrap();
        std::fs::write(
            root.join("src/router.rs"),
            "pub fn dispatch(p: &str) -> u32 { p.len() as u32 }\n",
        )
        .unwrap();

        let s = run_landscape(&root.to_string_lossy()).expect("landscape must run");
        assert!(s.proposals > 0, "findings project to proposals");
        assert!(s.open >= 1, "the undocumented module is open");
        assert!(!s.top.is_empty(), "the peaks render");
        assert!(s.top[0].severity > 0.0);
        // The spectral shape rides along: router's doc+test form one cluster.
        assert!(!s.geometry.is_empty(), "geometry lines present");
        assert!(
            s.geometry[0].contains("router"),
            "the router cluster leads: {:?}",
            s.geometry
        );

        // The render path stays total over both arms.
        assert!(!landscape_lines_render(&Ok(s)).is_empty());
        assert!(!landscape_lines_render(&Err("boom".into())).is_empty());

        std::fs::remove_dir_all(&root).ok();
    }
}
