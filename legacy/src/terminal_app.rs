use directories::ProjectDirs;
use egui::{
    text::{LayoutJob, TextFormat},
    Color32, FontId, Frame, Key, Margin, Modifiers, Rounding, ScrollArea, Stroke, Vec2,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::{
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};
use vte::Parser as VteParser;

use crate::features::{git::GitInfo, jump::JumpDb, suggest::Suggester};
use crate::pty::PtySession;
use crate::vte_proc::{line_to_layout_job, strip_ansi, OutputProcessor, StyledChar};

// ── colour palette ──────────────────────────────────────────────────────────
const BG: Color32 = Color32::from_rgb(18, 18, 28);
const BLOCK_BG: Color32 = Color32::from_rgb(24, 24, 38);
const BLOCK_BORDER: Color32 = Color32::from_rgb(55, 55, 85);
const INPUT_BG: Color32 = Color32::from_rgb(22, 22, 36);
const STATUS_BG: Color32 = Color32::from_rgb(14, 14, 24);
const TEXT: Color32 = Color32::from_rgb(220, 220, 220);
const DIM: Color32 = Color32::from_rgb(100, 100, 130);
const GREEN: Color32 = Color32::from_rgb(78, 201, 148);
const RED: Color32 = Color32::from_rgb(255, 107, 107);
const YELLOW: Color32 = Color32::from_rgb(220, 220, 170);
const CYAN: Color32 = Color32::from_rgb(78, 201, 176);
const BLUE: Color32 = Color32::from_rgb(86, 156, 214);
const ORANGE: Color32 = Color32::from_rgb(206, 145, 120);

const FONT: f32 = 14.0;

// ── data types ───────────────────────────────────────────────────────────────
pub struct CommandBlock {
    pub command: String,
    /// Rendered, coloured output lines (processed by VTE).
    pub output: Vec<Vec<StyledChar>>,
    pub exit_code: i32,
    pub cwd: PathBuf,
    pub duration_ms: u64,
    pub is_running: bool,
    /// True while the child has entered alternate-screen mode.
    pub is_fullscreen: bool,
}

// ── app ───────────────────────────────────────────────────────────────────────
pub struct TerminalApp {
    // shell
    pty: PtySession,
    startup_done: bool,
    /// Leftover bytes that don't yet form a complete line.
    partial_line: Vec<u8>,
    block_start: Option<Instant>,

    // VTE state (persists across lines so colour carries over)
    vte_parser: VteParser,
    vte_proc: OutputProcessor,

    // state
    blocks: Vec<CommandBlock>,
    input: String,
    cwd: PathBuf,

    // history / suggestions
    suggester: Suggester,
    history_nav: Option<usize>,
    history_draft: String,
    suggestion: Option<String>,

    // git
    git_info: Option<GitInfo>,
    git_refresh: Instant,

    // palette
    palette_open: bool,
    palette_query: String,
    palette_items: Vec<String>,
    palette_filtered: Vec<String>,
    palette_sel: usize,

    // jump
    jump_db: JumpDb,

    // misc
    matcher: SkimMatcherV2,
    scroll_to_bottom: bool,
}

impl TerminalApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = BG;
        visuals.override_text_color = Some(TEXT);
        cc.egui_ctx.set_visuals(visuals);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(4.0, 2.0);
        cc.egui_ctx.set_style(style);

        let pty = PtySession::new(40, 200).expect("failed to start shell");
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let history = load_history();
        let suggester = Suggester::new(history);
        let jump_db = JumpDb::load().unwrap_or_else(|_| JumpDb::load().unwrap());
        let git_info = crate::features::git::get_info(&cwd);

        Self {
            pty,
            startup_done: false,
            partial_line: Vec::new(),
            block_start: None,
            vte_parser: VteParser::new(),
            vte_proc: OutputProcessor::new(),
            blocks: Vec::new(),
            input: String::new(),
            cwd,
            suggester,
            history_nav: None,
            history_draft: String::new(),
            suggestion: None,
            git_info,
            git_refresh: Instant::now(),
            palette_open: false,
            palette_query: String::new(),
            palette_items: Vec::new(),
            palette_filtered: Vec::new(),
            palette_sel: 0,
            jump_db,
            matcher: SkimMatcherV2::default(),
            scroll_to_bottom: false,
        }
    }

    // ── PTY output processing ─────────────────────────────────────────────────

    fn process_pty(&mut self) {
        let mut received = Vec::new();
        while let Ok(bytes) = self.pty.rx.try_recv() {
            received.extend_from_slice(&bytes);
        }
        if received.is_empty() { return; }

        self.partial_line.extend_from_slice(&received);

        // Split into complete lines (terminated by \n), keep the remainder.
        let mut start = 0;
        let buf = std::mem::take(&mut self.partial_line);
        for (i, &b) in buf.iter().enumerate() {
            if b == b'\n' {
                let raw = &buf[start..i]; // does not include the \n
                self.handle_raw_line(raw);
                start = i + 1;
            }
        }
        // leftover bytes that don't have a \n yet
        self.partial_line = buf[start..].to_vec();
    }

    fn handle_raw_line(&mut self, raw: &[u8]) {
        let raw_str = String::from_utf8_lossy(raw);
        // trim trailing \r if present
        let trimmed = raw_str.trim_end_matches('\r');
        let clean = strip_ansi(trimmed);

        if clean == crate::pty::PROMPT_MARKER {
            self.on_prompt_ready();
            return;
        }
        if let Some(meta) = clean.strip_prefix(crate::pty::META_MARKER) {
            self.on_meta(meta);
            return;
        }

        if !self.startup_done {
            // Discard output before the first prompt marker
            return;
        }

        let is_running = self.blocks.last().map(|b| b.is_running).unwrap_or(false);
        if !is_running { return; }

        // Feed the raw bytes (with ANSI intact) through the VTE processor.
        for &b in raw {
            self.vte_parser.advance(&mut self.vte_proc, b);
        }
        // Newline (which we stripped for line detection)
        self.vte_parser.advance(&mut self.vte_proc, b'\n');

        // Transfer any fully-completed lines to the current block.
        let new_lines = self.vte_proc.take_completed();
        if let Some(block) = self.blocks.last_mut() {
            block.is_fullscreen = self.vte_proc.alternate_screen;
            block.output.extend(new_lines);
        }
    }

    fn on_meta(&mut self, meta: &str) {
        // format: /path/to/dir:exit_code
        let mut parts = meta.rsplitn(2, ':');
        if let (Some(code_s), Some(path)) = (parts.next(), parts.next()) {
            let exit_code: i32 = code_s.trim().parse().unwrap_or(0);
            self.cwd = PathBuf::from(path.trim());
            if let Some(b) = self.blocks.last_mut() {
                if b.is_running { b.exit_code = exit_code; }
            }
        }
    }

    fn on_prompt_ready(&mut self) {
        if !self.startup_done {
            self.startup_done = true;
        } else if let Some(b) = self.blocks.last_mut() {
            if b.is_running {
                // Flush any partial line from the VTE processor
                let remaining = self.vte_proc.take_all();
                b.output.extend(remaining);
                // Trim trailing blank lines
                while b.output.last().map(|l: &Vec<StyledChar>| {
                    l.iter().all(|sc| sc.ch == ' ' || sc.ch == '\0')
                }).unwrap_or(false) {
                    b.output.pop();
                }
                b.is_running = false;
                b.is_fullscreen = false;
                b.duration_ms = self.block_start.take()
                    .map(|t| t.elapsed().as_millis() as u64)
                    .unwrap_or(0);
            }
        }
        // Reset VTE state for the next command
        self.vte_parser = VteParser::new();
        self.vte_proc = OutputProcessor::new();

        self.jump_db.add(&self.cwd);
        self.git_info = crate::features::git::get_info(&self.cwd);
        self.scroll_to_bottom = true;
    }

    // ── commands ──────────────────────────────────────────────────────────────

    fn submit(&mut self) {
        let cmd = self.input.trim().to_string();
        if cmd.is_empty() { return; }

        let actual = if let Some(q) = cmd.strip_prefix("z ").map(str::trim) {
            self.jump_db.query(q)
                .map(|p| format!("cd '{}'", p.display()))
                .unwrap_or_else(|| cmd.clone())
        } else {
            cmd.clone()
        };

        self.blocks.push(CommandBlock {
            command: cmd.clone(),
            output: Vec::new(),
            exit_code: 0,
            cwd: self.cwd.clone(),
            duration_ms: 0,
            is_running: true,
            is_fullscreen: false,
        });
        self.block_start = Some(Instant::now());
        self.suggester.add(cmd);
        self.input.clear();
        self.suggestion = None;
        self.history_nav = None;
        self.scroll_to_bottom = true;
        let _ = self.pty.send_command(&actual);
    }

    fn history_up(&mut self) {
        let h = self.suggester.history();
        if h.is_empty() { return; }
        match self.history_nav {
            None => { self.history_draft = self.input.clone(); self.history_nav = Some(h.len() - 1); }
            Some(0) => {}
            Some(ref mut i) => { *i -= 1; }
        }
        if let Some(i) = self.history_nav {
            self.input = self.suggester.history()[i].clone();
        }
    }

    fn history_down(&mut self) {
        let h = self.suggester.history();
        match self.history_nav {
            None => {}
            Some(i) if i + 1 >= h.len() => {
                self.history_nav = None;
                self.input = self.history_draft.clone();
            }
            Some(ref mut i) => {
                *i += 1;
                let idx = *i;
                self.input = self.suggester.history()[idx].clone();
            }
        }
    }

    fn update_suggestion(&mut self) {
        self.suggestion = self.suggester.suggest(&self.input);
    }

    fn open_palette(&mut self) {
        self.palette_items = self.suggester.history().iter().rev().cloned().collect();
        self.palette_query.clear();
        self.palette_filtered = self.palette_items.clone();
        self.palette_sel = 0;
        self.palette_open = true;
    }

    fn palette_filter(&mut self) {
        if self.palette_query.is_empty() {
            self.palette_filtered = self.palette_items.clone();
        } else {
            let q = self.palette_query.clone();
            let mut scored: Vec<(i64, String)> = self.palette_items.iter()
                .filter_map(|item| self.matcher.fuzzy_match(item, &q).map(|s| (s, item.clone())))
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            self.palette_filtered = scored.into_iter().map(|(_, s)| s).collect();
        }
        self.palette_sel = 0;
    }

    // ── key handling ──────────────────────────────────────────────────────────

    fn handle_keys(&mut self, ctx: &egui::Context) {
        let is_running = self.blocks.last().map(|b| b.is_running).unwrap_or(false);

        if is_running {
            // Always forward all input to the PTY while any command is running.
            // This handles TUI programs (claude, vim, htop) that may or may not
            // use alternate-screen mode.
            ctx.input_mut(|i| {
                for ev in &i.events.clone() {
                    if let egui::Event::Text(text) = ev {
                        let _ = self.pty.writer.write_all(text.as_bytes());
                        let _ = self.pty.writer.flush();
                    }
                }
                i.events.retain(|ev| !matches!(ev, egui::Event::Text(_)));

                macro_rules! fwd {
                    ($bytes:expr) => {
                        let _ = self.pty.writer.write_all($bytes);
                        let _ = self.pty.writer.flush();
                    };
                }
                if i.consume_key(Modifiers::NONE, Key::Enter)      { fwd!(b"\r"); }
                if i.consume_key(Modifiers::NONE, Key::Escape)      { fwd!(b"\x1b"); }
                if i.consume_key(Modifiers::NONE, Key::Backspace)   { fwd!(b"\x7f"); }
                if i.consume_key(Modifiers::NONE, Key::Tab)         { fwd!(b"\t"); }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp)     { fwd!(b"\x1b[A"); }
                if i.consume_key(Modifiers::NONE, Key::ArrowDown)   { fwd!(b"\x1b[B"); }
                if i.consume_key(Modifiers::NONE, Key::ArrowRight)  { fwd!(b"\x1b[C"); }
                if i.consume_key(Modifiers::NONE, Key::ArrowLeft)   { fwd!(b"\x1b[D"); }
                if i.consume_key(Modifiers::NONE, Key::Home)        { fwd!(b"\x1b[H"); }
                if i.consume_key(Modifiers::NONE, Key::End)         { fwd!(b"\x1b[F"); }
                if i.consume_key(Modifiers::NONE, Key::Delete)      { fwd!(b"\x1b[3~"); }
                if i.consume_key(Modifiers::SHIFT, Key::Tab)        { fwd!(b"\x1b[Z"); }
                if i.consume_key(Modifiers::CTRL, Key::C)           { fwd!(b"\x03"); }
                if i.consume_key(Modifiers::CTRL, Key::D)           { fwd!(b"\x04"); }
                for (k, b) in CTRL_KEYS {
                    if i.consume_key(Modifiers::CTRL, *k) {
                        fwd!(&[*b]);
                    }
                }
            });
            return;
        }

        // Normal mode: no command running.
        let mut do_submit = false;
        let mut do_quit = false;
        ctx.input_mut(|i| {
            if i.consume_key(Modifiers::NONE, Key::Enter) { do_submit = true; }
            if i.consume_key(Modifiers::CTRL, Key::Q) { do_quit = true; }
            if i.consume_key(Modifiers::CTRL, Key::C) {
                self.input.clear();
                self.suggestion = None;
            }
            if i.consume_key(Modifiers::CTRL, Key::D) && self.input.is_empty() {
                let _ = self.pty.send_signal("eof");
            }
            if i.consume_key(Modifiers::CTRL, Key::R) {
                if self.palette_open { self.palette_open = false; }
                else { self.open_palette(); }
            }
            if i.consume_key(Modifiers::NONE, Key::Escape) { self.palette_open = false; }
            if !self.palette_open {
                if i.consume_key(Modifiers::NONE, Key::Tab) {
                    if let Some(sug) = self.suggestion.take() {
                        self.input.push_str(&sug);
                        self.update_suggestion();
                    }
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp)   { self.history_up();   self.update_suggestion(); }
                if i.consume_key(Modifiers::NONE, Key::ArrowDown)  { self.history_down(); self.update_suggestion(); }
            }
        });

        if do_quit { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        if do_submit { self.submit(); }
    }

    // ── rendering ─────────────────────────────────────────────────────────────

    fn render_status(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("status")
            .frame(Frame::none().fill(STATUS_BG).inner_margin(Margin::symmetric(10.0, 4.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let cwd = shorten_path(&self.cwd.display().to_string());
                    ui.label(egui::RichText::new(cwd).color(CYAN).strong().monospace());

                    if let Some(g) = &self.git_info {
                        ui.label(egui::RichText::new("  ").color(DIM));
                        let col = if g.is_dirty { YELLOW } else { GREEN };
                        ui.label(egui::RichText::new(format!(" {}", g.branch)).color(col).monospace());
                        if g.is_dirty { ui.label(egui::RichText::new("*").color(YELLOW).monospace()); }
                        if g.ahead  > 0 { ui.label(egui::RichText::new(format!(" ↑{}", g.ahead)).color(CYAN).monospace()); }
                        if g.behind > 0 { ui.label(egui::RichText::new(format!(" ↓{}", g.behind)).color(RED).monospace()); }
                    }

                    let is_fs = self.blocks.last().map(|b| b.is_fullscreen).unwrap_or(false);
                    if is_fs {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("⌨ INTERACTIVE – all keys forwarded  Ctrl+C to interrupt").color(YELLOW).small());
                        });
                    } else {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("Ctrl+Q quit  Ctrl+C interrupt  Ctrl+R history  Tab accept").color(DIM).small());
                        });
                    }
                });
            });
    }

    fn render_input_panel(&mut self, ctx: &egui::Context) {
        let is_running    = self.blocks.last().map(|b| b.is_running).unwrap_or(false);
        let is_fullscreen = self.blocks.last().map(|b| b.is_fullscreen).unwrap_or(false);

        egui::TopBottomPanel::bottom("input_panel")
            .frame(Frame::none().fill(INPUT_BG).inner_margin(Margin::symmetric(10.0, 6.0)))
            .show(ctx, |ui| {
                if is_running {
                    let msg = if is_fullscreen {
                        "⌨ Interactive – all keys forwarded to running program  (Ctrl+C to interrupt)"
                    } else {
                        "⌨ Running – keyboard forwarded to process  (Ctrl+C to interrupt)"
                    };
                    ui.label(egui::RichText::new(msg).color(YELLOW).italics());
                    return;
                }

                // Ghost suggestion row
                if let Some(sug) = &self.suggestion {
                    if !sug.is_empty() {
                        let mut job = LayoutJob::default();
                        let ghost_fmt = TextFormat { font_id: FontId::monospace(FONT), color: Color32::from_rgb(60, 60, 80), ..Default::default() };
                        job.append("$ ", 0.0, TextFormat { font_id: FontId::monospace(FONT), color: DIM, ..Default::default() });
                        job.append(&self.input, 0.0, TextFormat { font_id: FontId::monospace(FONT), color: DIM, ..Default::default() });
                        job.append(sug, 0.0, ghost_fmt);
                        ui.label(job);
                    }
                }

                ui.horizontal(|ui| {
                    let prompt_color = if is_running { YELLOW } else { GREEN };
                    ui.label(egui::RichText::new("$ ").color(prompt_color).strong().monospace());

                    let prev = self.input.clone();
                    let r = egui::TextEdit::singleline(&mut self.input)
                        .id(egui::Id::new("main_input"))
                        .frame(false)
                        .desired_width(f32::INFINITY)
                        .font(egui::FontSelection::FontId(FontId::monospace(FONT)))
                        .text_color(TEXT)
                        .layouter(&mut |ui, s, wrap| {
                            let job = highlight_job(s, wrap);
                            ui.fonts(|f| f.layout_job(job))
                        })
                        .show(ui);

                    if !self.palette_open { r.response.request_focus(); }

                    if self.input != prev { self.history_nav = None; self.update_suggestion(); }
                });

            });
    }

    fn render_blocks(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().fill(BG).inner_margin(Margin::symmetric(10.0, 8.0)))
            .show(ctx, |ui| {
                let scroll = ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .stick_to_bottom(self.scroll_to_bottom);
                self.scroll_to_bottom = false;

                scroll.show(ui, |ui| {
                    if !self.startup_done {
                        ui.label(egui::RichText::new("  Starting shell…").color(DIM).italics());
                        return;
                    }
                    if self.blocks.is_empty() {
                        ui.label(egui::RichText::new("  No commands yet — type one below.").color(DIM).italics());
                        return;
                    }
                    for block in &self.blocks {
                        render_block(ui, block);
                        ui.add_space(6.0);
                    }
                });
            });
    }

    fn render_palette(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let w = (screen.width() * 0.65).min(720.0);
        let h = (screen.height() * 0.6).min(420.0);
        let pos = egui::pos2(screen.center().x - w / 2.0, screen.center().y - h / 2.0);

        egui::Window::new("History Search")
            .fixed_pos(pos)
            .fixed_size([w, h])
            .collapsible(false)
            .frame(Frame::window(&ctx.style()).fill(Color32::from_rgb(28, 28, 44)))
            .show(ctx, |ui| {
                let prev = self.palette_query.clone();
                let r = egui::TextEdit::singleline(&mut self.palette_query)
                    .desired_width(f32::INFINITY)
                    .hint_text("fuzzy search history…")
                    .font(egui::FontSelection::FontId(FontId::monospace(FONT)))
                    .show(ui);
                r.response.request_focus();
                if self.palette_query != prev { self.palette_filter(); }

                ui.separator();

                ui.input(|i| {
                    if i.key_pressed(Key::ArrowUp) && self.palette_sel > 0 { self.palette_sel -= 1; }
                    if i.key_pressed(Key::ArrowDown) {
                        let max = self.palette_filtered.len().saturating_sub(1);
                        if self.palette_sel < max { self.palette_sel += 1; }
                    }
                    if i.key_pressed(Key::Enter) {
                        if let Some(sel) = self.palette_filtered.get(self.palette_sel).cloned() {
                            self.input = sel;
                            self.palette_open = false;
                            self.update_suggestion();
                        }
                    }
                });

                ScrollArea::vertical().show(ui, |ui| {
                    for (i, item) in self.palette_filtered.clone().iter().enumerate() {
                        let selected = i == self.palette_sel;
                        let bg = if selected { Color32::from_rgb(50, 50, 80) } else { Color32::TRANSPARENT };
                        let label = egui::RichText::new(item)
                            .monospace()
                            .color(if selected { Color32::WHITE } else { Color32::LIGHT_GRAY });
                        let r = Frame::none()
                            .fill(bg)
                            .inner_margin(Margin::symmetric(6.0, 2.0))
                            .show(ui, |ui| ui.label(label))
                            .response;
                        if r.double_clicked() {
                            self.input = item.clone();
                            self.palette_open = false;
                            self.update_suggestion();
                        }
                    }
                });
            });
    }
}

impl eframe::App for TerminalApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_pty();

        if self.git_refresh.elapsed() > Duration::from_secs(5) {
            self.git_info = crate::features::git::get_info(&self.cwd);
            self.git_refresh = Instant::now();
        }

        self.handle_keys(ctx);

        self.render_status(ctx);
        self.render_input_panel(ctx);
        self.render_blocks(ctx);
        if self.palette_open { self.render_palette(ctx); }

        // Repaint frequently while anything is running or during startup
        if !self.startup_done || self.blocks.last().map(|b| b.is_running).unwrap_or(false) {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.jump_db.save();
        save_history(self.suggester.history());
    }
}

// ── block rendering ──────────────────────────────────────────────────────────

fn render_block(ui: &mut egui::Ui, block: &CommandBlock) {
    let border_col = if block.is_running {
        Color32::from_rgb(80, 100, 180)
    } else if block.exit_code == 0 {
        BLOCK_BORDER
    } else {
        Color32::from_rgb(120, 50, 50)
    };

    Frame::none()
        .fill(BLOCK_BG)
        .stroke(Stroke::new(1.0, border_col))
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            // Header row
            ui.horizontal(|ui| {
                let cmd_job = highlight_job(&block.command, f32::INFINITY);
                ui.label(cmd_job);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if block.is_running {
                        let label = if block.is_fullscreen { "⌨ interactive…" } else { "⟳ running…" };
                        ui.label(egui::RichText::new(label).color(BLUE).small().monospace());
                    } else {
                        let (icon, col) = if block.exit_code == 0 { ("✓", GREEN) } else { ("✗", RED) };
                        ui.label(egui::RichText::new(
                            format!("{} {}  {}", icon, block.exit_code, fmt_dur(block.duration_ms))
                        ).color(col).small().monospace());
                    }
                });
            });

            // CWD
            let cwd = shorten_path(&block.cwd.display().to_string());
            ui.label(egui::RichText::new(format!("  {cwd}")).color(DIM).small().monospace());

            if !block.output.is_empty() {
                ui.add(egui::Separator::default().spacing(4.0));
                for line in &block.output {
                    let job = line_to_layout_job(line, FONT);
                    ui.label(job);
                }
            }
        });
}

// ── syntax highlighting for the input ────────────────────────────────────────

fn highlight_job(input: &str, _wrap: f32) -> LayoutJob {
    use crate::features::highlight::highlight;
    let spans = highlight(input);
    let mut job = LayoutJob::default();
    for span in &spans {
        let color = ratatui_to_egui(span.style.fg);
        job.append(
            span.content.as_ref(),
            0.0,
            TextFormat { font_id: FontId::monospace(FONT), color, ..Default::default() },
        );
    }
    if job.sections.is_empty() {
        job.append(input, 0.0, TextFormat { font_id: FontId::monospace(FONT), color: TEXT, ..Default::default() });
    }
    job
}

fn ratatui_to_egui(c: Option<ratatui::style::Color>) -> Color32 {
    use ratatui::style::Color as RC;
    match c {
        Some(RC::Green)      => GREEN,
        Some(RC::Red)        => RED,
        Some(RC::Yellow)     => YELLOW,
        Some(RC::Cyan)       => CYAN,
        Some(RC::Blue)       => BLUE,
        Some(RC::Magenta)    => Color32::from_rgb(197, 134, 192),
        Some(RC::LightGreen) => Color32::from_rgb(106, 153, 85),
        Some(RC::LightYellow) | Some(RC::LightBlue) | Some(RC::LightCyan) => ORANGE,
        Some(RC::White)      => TEXT,
        Some(RC::DarkGray)   => DIM,
        _                    => TEXT,
    }
}

// ── ctrl-key table for pass-through ──────────────────────────────────────────

const CTRL_KEYS: &[(Key, u8)] = &[
    (Key::A, 0x01), (Key::B, 0x02), (Key::E, 0x05), (Key::F, 0x06),
    (Key::G, 0x07), (Key::H, 0x08), (Key::K, 0x0B), (Key::L, 0x0C),
    (Key::N, 0x0E), (Key::O, 0x0F), (Key::P, 0x10), (Key::Q, 0x11),
    (Key::R, 0x12), (Key::S, 0x13), (Key::T, 0x14), (Key::U, 0x15),
    (Key::V, 0x16), (Key::W, 0x17), (Key::X, 0x18), (Key::Y, 0x19),
    (Key::Z, 0x1A),
];

// ── helpers ───────────────────────────────────────────────────────────────────

fn shorten_path(p: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if p.starts_with(&home) { return format!("~{}", &p[home.len()..]); }
    }
    p.to_string()
}

fn fmt_dur(ms: u64) -> String {
    if ms < 1000 { format!("{}ms", ms) }
    else if ms < 60_000 { format!("{:.1}s", ms as f64 / 1000.0) }
    else { format!("{}m{}s", ms / 60_000, (ms % 60_000) / 1000) }
}

fn load_history() -> Vec<String> {
    history_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.lines().map(String::from).collect())
        .unwrap_or_default()
}

fn save_history(h: &[String]) {
    if let Some(path) = history_path() {
        if let Some(p) = path.parent() { let _ = std::fs::create_dir_all(p); }
        let _ = std::fs::write(path, h.join("\n"));
    }
}

fn history_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "myterm").map(|d| d.data_dir().join("history"))
}
