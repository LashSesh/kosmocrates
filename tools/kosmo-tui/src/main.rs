//! Kosmocrates TUI — interactive workspace topology dashboard.
//!
//! Three-pane layout: scrollable action queue │ detail view │ workspace stats.
//!
//! USAGE:
//!     kosmo-tui [OPTIONS] [PATH]
//!
//! Keybindings: q/Esc=quit  r=rerun  ↑↓/jk=navigate  PgUp/PgDn=page  g=top  G=bottom

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
            "--metatron"   => args.metatron = true,
            "--lpcm"       => args.lpcm = true,
            "--systemcube" => { args.systemcube = true; args.metatron = true; }
            "--surgery"    => { args.surgery = true; args.metatron = true; }
            "--crystals"   => args.crystals = true,
            "--norms"      => args.norms = true,
            "--motifs"     => args.motifs = true,
            "--pse"        => args.pse = true,
            "--operator"   => args.operator = true,
            "--store" => {
                i += 1;
                if i >= raw.len() { return Err("--store requires a path".into()); }
                args.store = Some(expand_tilde(&raw[i]));
            }
            "--capacity" => {
                i += 1;
                if i >= raw.len() { return Err("--capacity requires a number".into()); }
                args.capacity = raw[i].parse()
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

// ─── App state ───────────────────────────────────────────────────────────────

enum AppPhase {
    Analysing,
    Ready { report: IntegrationRunReport, items: Vec<ActionItem> },
    Error(String),
}

struct App {
    path: String,
    session: WorkspacePipelineSession,
    phase: AppPhase,
    selected: usize,
    offset: usize,
}

impl App {
    fn new(path: String, options: IntegrationRunOptions, policy: PolicyProfile) -> Self {
        Self {
            path,
            session: WorkspacePipelineSession::new(options, policy),
            phase: AppPhase::Analysing,
            selected: 0,
            offset: 0,
        }
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
        if n == 0 { return; }
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
        if n > 0 { self.selected = n - 1; }
    }
}

// ─── Formatting helpers ───────────────────────────────────────────────────────

fn q16_fmt(raw: i64) -> String {
    let i = raw / 65536;
    let f = ((raw.unsigned_abs() % 65536) * 10000 / 65536) as u64;
    format!("{}.{:04}", i, f)
}

fn hex16(d: &kosmo_core::Digest) -> String {
    d.to_hex()[..16].to_string()
}

fn gate_color(r: &GateResult) -> Color {
    match r {
        GateResult::Pass              => Color::Green,
        GateResult::Warn { .. }       => Color::Yellow,
        GateResult::Reject { .. }     => Color::Red,
        GateResult::Downgrade { .. }  => Color::Yellow,
    }
}

fn gate_label(r: &GateResult) -> &'static str {
    match r {
        GateResult::Pass              => "✓ Pass",
        GateResult::Warn { .. }       => "⚠ Warn",
        GateResult::Reject { .. }     => "✗ Reject",
        GateResult::Downgrade { .. }  => "↓ Downgrade",
    }
}

fn kind_label(kind: &ActionItemKind) -> &'static str {
    match kind {
        ActionItemKind::FillVoid { .. }         => "FillVoid  ",
        ActionItemKind::RepairTopology { .. }   => "Repair    ",
        ActionItemKind::PromoteToPse { .. }     => "PromotePSE",
        ActionItemKind::ReviewCrystal { .. }    => "Review    ",
        ActionItemKind::ApplyNorm { .. }        => "ApplyNorm ",
        ActionItemKind::RealizeWishFacet { .. } => "WishFacet ",
    }
}

fn kind_color(kind: &ActionItemKind) -> Color {
    match kind {
        ActionItemKind::FillVoid { .. }         => Color::Cyan,
        ActionItemKind::RepairTopology { .. }   => Color::Yellow,
        ActionItemKind::PromoteToPse { .. }     => Color::Green,
        ActionItemKind::ReviewCrystal { .. }    => Color::Yellow,
        ActionItemKind::ApplyNorm { .. }        => Color::Green,
        ActionItemKind::RealizeWishFacet { .. } => Color::Green,
    }
}

fn target_id(kind: &ActionItemKind) -> String {
    match kind {
        ActionItemKind::FillVoid { void_id }                     => hex16(void_id),
        ActionItemKind::RepairTopology { surgery_option_id }     => hex16(surgery_option_id),
        ActionItemKind::PromoteToPse { candidate_id }            => hex16(candidate_id),
        ActionItemKind::ReviewCrystal { candidate_id }           => hex16(candidate_id),
        ActionItemKind::ApplyNorm { norm_candidate_id, .. }      => hex16(norm_candidate_id),
        ActionItemKind::RealizeWishFacet { facet }               => facet.key.clone(),
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
            if report.policy_id == PolicyProfile::operator_approved().id => "OperatorApproved",
        _ => "ReportOnly",
    };

    let line = Line::from(vec![
        Span::styled(" Kosmocrates", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
        Span::styled(app.path.as_str(), Style::default().fg(Color::Cyan)),
        Span::styled("  │  policy: ", Style::default().fg(Color::DarkGray)),
        Span::raw(policy_label),
        Span::styled("  │  gate: ", Style::default().fg(Color::DarkGray)),
        Span::styled(gate_text, Style::default().fg(gate_col).add_modifier(Modifier::BOLD)),
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
        AppPhase::Analysing            => " Action Queue ".to_string(),
        AppPhase::Error(_)             => " Action Queue ".to_string(),
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
                    Paragraph::new(indicator)
                        .style(Style::default().fg(Color::DarkGray)),
                    ind_area,
                );
            }

            f.render_widget(List::new(visible), inner);
        }
        AppPhase::Analysing => {
            f.render_widget(
                Paragraph::new("  Analysing…")
                    .style(Style::default().fg(Color::DarkGray)),
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
            Paragraph::new("(select an item)")
                .style(Style::default().fg(Color::DarkGray)),
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
        let source_count = report.hyphae_result.host_cube.void_map.voids.len()
            + report.source_cubes.len();
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
            spans.push(Span::styled("  │  crystals stored: ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                format!("{}", report.persisted_crystal_count),
                Style::default().fg(Color::Green),
            ));
        }

        if !report.certified_crystals.is_empty() {
            spans.push(Span::styled("  │  certified: ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                format!("{}", report.certified_crystals.len()),
                Style::default().fg(Color::Green),
            ));
        }

        f.render_widget(Paragraph::new(Line::from(spans)), inner);
    }
}

fn render_status_bar(f: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":quit  ", Style::default().fg(Color::DarkGray)),
        Span::styled("r", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":rerun  ", Style::default().fg(Color::DarkGray)),
        Span::styled("↑↓/jk", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled("PgUp/PgDn", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":page  ", Style::default().fg(Color::DarkGray)),
        Span::styled("g/G", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(":top/bottom", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn ui(f: &mut Frame, app: &App, list_height: usize) {
    let vertical = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Min(5),    // action list + detail
        Constraint::Length(3), // workspace stats
        Constraint::Length(1), // key hints
    ])
    .split(f.area());

    render_header(f, vertical[0], app);

    let horizontal = Layout::horizontal([
        Constraint::Percentage(58),
        Constraint::Percentage(42),
    ])
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
                    KeyCode::Down  | KeyCode::Char('j') => app.scroll_down(list_height),
                    KeyCode::Up    | KeyCode::Char('k') => app.scroll_up(),
                    KeyCode::Char('g') => app.go_top(),
                    KeyCode::Char('G') => app.go_bottom(),
                    KeyCode::PageDown  => app.page_down(list_height),
                    KeyCode::PageUp    => app.page_up(list_height),
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
