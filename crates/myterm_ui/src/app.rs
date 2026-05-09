use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use feature_fzf::FzfState;
use feature_git::GitInfo;
use feature_jump::JumpDb;
use feature_predict::{CommandIndex, SignatureDb};
use feature_suggest::Suggester;
use myterm_settings::Settings;
use myterm_vte::strip_ansi;

use crate::blocks::{render_blocks, BlockFlash};
use crate::input::{render_input_panel, InputAction};
use crate::layout::{classify_drop, split_rect, zone_to_split, DropZone, PaneLayout, SplitDir};
use crate::palette::render_palette;
use crate::pane::{PaneId, TerminalPane};
use crate::plugin::Feature;
use crate::predict_bar::render_predict_bar;
use crate::status::render_status_bar;
use crate::tabs::{render_tab_bar, TabAction, TabDragState};
use crate::theme::{apply_visuals, ResolvedPalette};

pub struct TerminalApp {
    panes: Vec<TerminalPane>,
    layout: PaneLayout,
    active: PaneId,

    settings: Settings,
    palette: ResolvedPalette,

    suggester: Suggester,
    jump_db: JumpDb,
    git_cache: Option<GitInfo>,
    git_timer: Instant,

    fzf: FzfState,
    flash_states: HashMap<usize, BlockFlash>,

    predict_index: Arc<CommandIndex>,
    predict_sigs: Arc<SignatureDb>,

    drag: TabDragState,
    features: Vec<Box<dyn Feature>>,
}

impl TerminalApp {
    pub fn new(
        cc: &eframe::CreationContext,
        settings: Settings,
        features: Vec<Box<dyn Feature>>,
    ) -> anyhow::Result<Self> {
        let palette = ResolvedPalette::from_settings(&settings);
        apply_visuals(&palette, &cc.egui_ctx);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::Vec2::new(4.0, 2.0);
        cc.egui_ctx.set_style(style);

        let history = load_history();
        let suggester = Suggester::new(history);
        let jump_db = JumpDb::load().unwrap_or_default();

        let first_pane = TerminalPane::spawn("")?;
        let first_id = first_pane.id;
        let cwd = first_pane.cwd.clone();
        let git_cache = feature_git::get_info(&cwd);

        Ok(Self {
            layout: PaneLayout::Leaf(first_id),
            active: first_id,
            panes: vec![first_pane],
            settings,
            palette,
            suggester,
            jump_db,
            git_cache,
            git_timer: Instant::now(),
            fzf: FzfState::new(),
            flash_states: HashMap::new(),
            predict_index: Arc::new(CommandIndex::build()),
            predict_sigs: Arc::new(SignatureDb::load()),
            drag: TabDragState::default(),
            features,
        })
    }

    // ── pane helpers ──────────────────────────────────────────────────────────

    fn pane_idx(&self, id: PaneId) -> usize {
        self.panes
            .iter()
            .position(|p| p.id == id)
            .expect("pane must exist")
    }

    fn new_pane(&mut self) -> PaneId {
        match TerminalPane::spawn("") {
            Ok(pane) => {
                let id = pane.id;
                self.panes.push(pane);
                id
            }
            Err(_) => self.active,
        }
    }

    fn close_pane(&mut self, id: PaneId) {
        self.panes.retain(|p| p.id != id);
        if let Some(layout) = self.layout.clone().remove(id) {
            self.layout = layout;
            if self.active == id {
                self.active = self
                    .layout
                    .pane_ids()
                    .into_iter()
                    .next()
                    .unwrap_or(self.active);
            }
        }
        if self.panes.is_empty() {
            let new_id = self.new_pane();
            self.layout = PaneLayout::Leaf(new_id);
            self.active = new_id;
        }
    }

    // ── PTY processing ────────────────────────────────────────────────────────

    fn process_active_pane(&mut self) {
        let active = self.active;
        let idx = self.pane_idx(active);

        let mut received = Vec::new();
        while let Ok(bytes) = self.panes[idx].pty.rx.try_recv() {
            received.extend_from_slice(&bytes);
        }
        if received.is_empty() {
            return;
        }

        self.panes[idx].partial_line.extend_from_slice(&received);
        let buf = std::mem::take(&mut self.panes[idx].partial_line);

        let mut lines: Vec<Vec<u8>> = Vec::new();
        let mut start = 0;
        for (i, &b) in buf.iter().enumerate() {
            if b == b'\n' {
                lines.push(buf[start..i].to_vec());
                start = i + 1;
            }
        }
        self.panes[idx].partial_line = buf[start..].to_vec();

        for raw in lines {
            self.handle_raw_line(active, raw);
        }
    }

    fn handle_raw_line(&mut self, pane_id: PaneId, raw: Vec<u8>) {
        use myterm_pty::{META_MARKER, PROMPT_MARKER};

        let raw_str = String::from_utf8_lossy(&raw);
        let trimmed = raw_str.trim_end_matches('\r');
        let clean = strip_ansi(trimmed);

        if clean == PROMPT_MARKER {
            self.on_prompt_ready(pane_id);
            return;
        }
        if let Some(meta) = clean.strip_prefix(META_MARKER) {
            let meta_owned = meta.to_string();
            self.on_meta(pane_id, &meta_owned);
            return;
        }

        let idx = self.pane_idx(pane_id);
        if !self.panes[idx].startup_done {
            return;
        }
        let is_running = self.panes[idx]
            .blocks
            .last()
            .map(|b| b.is_running)
            .unwrap_or(false);
        if !is_running {
            return;
        }

        {
            let pane = &mut self.panes[idx];
            let (parser, proc) = (&mut pane.vte_parser, &mut pane.vte_proc);
            for &b in &raw {
                parser.advance(proc, b);
            }
            parser.advance(proc, b'\n');
        }

        let new_lines = self.panes[idx].vte_proc.take_completed();
        let is_fullscreen = self.panes[idx].vte_proc.alternate_screen;
        if let Some(block) = self.panes[idx].blocks.last_mut() {
            block.is_fullscreen = is_fullscreen;
            block.output.extend(new_lines);
        }
    }

    fn on_meta(&mut self, pane_id: PaneId, meta: &str) {
        let mut parts = meta.rsplitn(2, ':');
        if let (Some(code_s), Some(path)) = (parts.next(), parts.next()) {
            let exit_code: i32 = code_s.trim().parse().unwrap_or(0);
            let idx = self.pane_idx(pane_id);
            self.panes[idx].cwd = std::path::PathBuf::from(path.trim());
            self.panes[idx].refresh_title();
            if let Some(b) = self.panes[idx].blocks.last_mut() {
                if b.is_running {
                    b.exit_code = exit_code;
                }
            }
        }
    }

    fn on_prompt_ready(&mut self, pane_id: PaneId) {
        let idx = self.pane_idx(pane_id);
        if !self.panes[idx].startup_done {
            self.panes[idx].startup_done = true;
            return;
        }

        let last_is_running = self.panes[idx]
            .blocks
            .last()
            .map(|b| b.is_running)
            .unwrap_or(false);
        if last_is_running {
            let remaining = self.panes[idx].vte_proc.take_all();
            let duration_ms = self.panes[idx]
                .block_start
                .take()
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(0);
            if let Some(b) = self.panes[idx].blocks.last_mut() {
                b.output.extend(remaining);
                while b
                    .output
                    .last()
                    .map(|l| l.iter().all(|sc| sc.ch == ' ' || sc.ch == '\0'))
                    .unwrap_or(false)
                {
                    b.output.pop();
                }
                b.is_running = false;
                b.is_fullscreen = false;
                b.duration_ms = duration_ms;
            }
        }

        self.panes[idx].vte_parser = vte::Parser::new();
        self.panes[idx].vte_proc = myterm_vte::OutputProcessor::new();
        self.panes[idx].scroll_to_bottom = true;

        let cwd = self.panes[idx].cwd.clone();
        self.jump_db.add(&cwd);
        if self.git_timer.elapsed() > Duration::from_secs(5) {
            self.git_cache = feature_git::get_info(&cwd);
            self.git_timer = Instant::now();
        }
    }

    fn submit_command(&mut self, pane_id: PaneId) {
        let idx = self.pane_idx(pane_id);
        let cmd = self.panes[idx].input.trim().to_string();
        if cmd.is_empty() {
            return;
        }

        let actual = self
            .features
            .iter()
            .find_map(|f| f.on_command_submit(&cmd))
            .unwrap_or_else(|| {
                if let Some(q) = cmd.strip_prefix("z ").map(str::trim) {
                    self.jump_db
                        .query(q)
                        .map(|p| format!("cd '{}'", p.display()))
                        .unwrap_or_else(|| cmd.clone())
                } else {
                    cmd.clone()
                }
            });

        let idx = self.pane_idx(pane_id);
        let cwd = self.panes[idx].cwd.clone();
        self.panes[idx].blocks.push(myterm_core::CommandBlock {
            command: cmd.clone(),
            output: Vec::new(),
            exit_code: 0,
            cwd,
            duration_ms: 0,
            is_running: true,
            is_fullscreen: false,
        });
        self.panes[idx].block_start = Some(Instant::now());
        self.panes[idx].input.clear();
        self.panes[idx].suggestion = None;
        self.panes[idx].history_nav = None;
        self.panes[idx].scroll_to_bottom = true;
        let _ = self.panes[idx].pty.send_command(&actual);

        self.suggester.add(cmd);
        let _ = self.jump_db.save();
    }
}

impl eframe::App for TerminalApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_active_pane();
        ctx.request_repaint_after(Duration::from_millis(16));

        // Periodic git refresh (outside prompt_ready to handle long-lived sessions)
        if self.git_timer.elapsed() > Duration::from_secs(5) {
            let active = self.active;
            let idx = self.pane_idx(active);
            let cwd = self.panes[idx].cwd.clone();
            self.git_cache = feature_git::get_info(&cwd);
            self.git_timer = Instant::now();
        }

        // ── Tab bar ──────────────────────────────────────────────────────────
        let tab_data: Vec<(PaneId, String)> =
            self.panes.iter().map(|p| (p.id, p.title.clone())).collect();
        let tab_refs: Vec<(PaneId, &str)> =
            tab_data.iter().map(|(id, t)| (*id, t.as_str())).collect();

        let tab_action = render_tab_bar(&tab_refs, self.active, &mut self.drag, &self.palette, ctx);

        if let Some(action) = tab_action {
            self.handle_tab_action(action);
        }

        // ── Status bar ───────────────────────────────────────────────────────
        let active = self.active;
        let idx = self.pane_idx(active);
        let cwd = self.panes[idx].cwd.clone();
        render_status_bar(self.git_cache.as_ref(), &[], &cwd, &self.palette, ctx);

        // ── Command palette overlay ──────────────────────────────────────────
        let hist: Vec<String> = self.suggester.history().to_vec();
        let palette_ref = &self.palette;
        if let Some(cmd) = render_palette(&mut self.fzf, &hist, palette_ref, ctx) {
            let idx = self.pane_idx(active);
            self.panes[idx].input = cmd;
        }

        // ── Central pane area ────────────────────────────────────────────────
        let layout = self.layout.clone();
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.palette.bg))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                self.render_layout_node(&layout, rect, active, ctx, ui);
            });
    }
}

impl TerminalApp {
    fn handle_tab_action(&mut self, action: TabAction) {
        match action {
            TabAction::Activate(id) => self.active = id,
            TabAction::Close(id) => self.close_pane(id),
            TabAction::NewTab => {
                let new_id = self.new_pane();
                let target = self.active;
                self.layout = self
                    .layout
                    .clone()
                    .split(target, new_id, SplitDir::Horizontal, true);
                self.active = new_id;
            }
            TabAction::Split {
                target,
                new,
                dir,
                new_is_second,
            } => {
                self.layout = self.layout.clone().split(target, new, dir, new_is_second);
                self.active = new;
            }
            TabAction::Reorder(_) => {}
        }
    }

    fn render_layout_node(
        &mut self,
        node: &PaneLayout,
        rect: egui::Rect,
        active: PaneId,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
    ) {
        match node {
            PaneLayout::Leaf(id) => {
                let id = *id;
                self.render_single_pane(id, rect, active == id, ctx, ui);
            }
            PaneLayout::Split {
                dir,
                ratio,
                first,
                second,
            } => {
                let (r1, r2) = split_rect(rect, *dir, *ratio);
                let first = first.clone();
                let second = second.clone();
                self.render_layout_node(&first, r1, active, ctx, ui);
                self.render_layout_node(&second, r2, active, ctx, ui);
            }
        }
    }

    fn render_single_pane(
        &mut self,
        pane_id: PaneId,
        rect: egui::Rect,
        is_active: bool,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
    ) {
        let font_size = self.settings.font.size;
        let active = self.active;

        // Click to activate pane
        if ui.rect_contains_pointer(rect) && ctx.input(|i| i.pointer.primary_clicked()) {
            self.active = pane_id;
        }

        // Compute predict chips before the mutable pane borrow
        let predict_chips = if myterm_features::FeatureFlag::PredictBar.is_enabled() {
            let idx = self.pane_idx(pane_id);
            let input = self.panes[idx].input.clone();
            if !input.is_empty() {
                let ctx_p = feature_predict::parse(&input);
                let hist = self.suggester.history().to_vec();
                feature_predict::suggest(&ctx_p, &self.predict_index, &self.predict_sigs, &hist, 7)
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );

        // Render blocks
        let idx = self.pane_idx(pane_id);
        let scroll_to_bottom = self.panes[idx].scroll_to_bottom;
        self.panes[idx].scroll_to_bottom = false;

        // Extract what we need to avoid multiple borrows
        let palette = &self.palette as *const ResolvedPalette;
        // SAFETY: palette is not mutated during render calls
        let palette_ref = unsafe { &*palette };

        render_blocks(
            &self.panes[idx].blocks,
            &mut self.flash_states,
            font_size,
            scroll_to_bottom,
            palette_ref,
            ctx,
            &mut child_ui,
        );

        // Render predict bar
        let idx = self.pane_idx(pane_id);
        render_predict_bar(
            &predict_chips,
            &mut self.panes[idx].input,
            palette_ref,
            ctx,
            &mut child_ui,
        );

        // Render input panel (active pane only)
        let input_action = if is_active {
            let idx = self.pane_idx(pane_id);
            render_input_panel(
                &mut self.panes[idx],
                &self.suggester,
                palette_ref,
                font_size,
                ctx,
                &mut child_ui,
            )
        } else {
            None
        };

        if let Some(action) = input_action {
            self.handle_input_action(pane_id, action);
        }

        // Drop overlay while dragging
        let dragged_id = self.drag.dragging;
        if let Some(dragged_id) = dragged_id {
            if let Some(ptr) = ctx.pointer_latest_pos() {
                if rect.contains(ptr) && dragged_id != pane_id {
                    let zone = classify_drop(ptr, rect);
                    render_drop_overlay(ctx, rect, zone);

                    if ctx.input(|i| i.pointer.primary_released()) {
                        if let Some(action) = zone_to_split(zone, dragged_id, pane_id) {
                            let new_id = action.new;
                            self.layout = self
                                .layout
                                .clone()
                                .remove(dragged_id)
                                .unwrap_or_else(|| self.layout.clone());
                            self.layout = self.layout.clone().split(
                                action.target,
                                new_id,
                                action.dir,
                                action.new_is_second,
                            );
                            self.active = new_id;
                        }
                        self.drag.dragging = None;
                        self.drag.ghost_pos = None;
                    }
                }
            }
        }

        let _ = active; // suppress unused warning
    }

    fn handle_input_action(&mut self, pane_id: PaneId, action: InputAction) {
        match action {
            InputAction::Submit => self.submit_command(pane_id),
            InputAction::OpenPalette => {
                let hist = self.suggester.history().to_vec();
                self.fzf.open(&hist);
            }
            InputAction::NewTab => {
                let new_id = self.new_pane();
                let target = self.active;
                self.layout = self
                    .layout
                    .clone()
                    .split(target, new_id, SplitDir::Horizontal, true);
                self.active = new_id;
            }
            InputAction::CloseTab => self.close_pane(pane_id),
            InputAction::NextPane => {
                let ids = self.layout.pane_ids();
                if let Some(pos) = ids.iter().position(|&id| id == self.active) {
                    self.active = ids[(pos + 1) % ids.len()];
                }
            }
            InputAction::PrevPane => {
                let ids = self.layout.pane_ids();
                if let Some(pos) = ids.iter().position(|&id| id == self.active) {
                    self.active = ids[(pos + ids.len() - 1) % ids.len()];
                }
            }
            InputAction::SplitRight => {
                let new_id = self.new_pane();
                self.layout =
                    self.layout
                        .clone()
                        .split(pane_id, new_id, SplitDir::Horizontal, true);
                self.active = new_id;
            }
            InputAction::SplitBelow => {
                let new_id = self.new_pane();
                self.layout = self
                    .layout
                    .clone()
                    .split(pane_id, new_id, SplitDir::Vertical, true);
                self.active = new_id;
            }
        }
    }
}

fn render_drop_overlay(ctx: &egui::Context, pane_rect: egui::Rect, active_zone: DropZone) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("drop_overlay"),
    ));
    let zones = [
        (
            DropZone::Top,
            egui::Rect::from_min_max(
                pane_rect.left_top(),
                egui::pos2(
                    pane_rect.right(),
                    pane_rect.top() + pane_rect.height() * 0.25,
                ),
            ),
        ),
        (
            DropZone::Bottom,
            egui::Rect::from_min_max(
                egui::pos2(
                    pane_rect.left(),
                    pane_rect.bottom() - pane_rect.height() * 0.25,
                ),
                pane_rect.right_bottom(),
            ),
        ),
        (
            DropZone::Left,
            egui::Rect::from_min_max(
                pane_rect.left_top(),
                egui::pos2(
                    pane_rect.left() + pane_rect.width() * 0.25,
                    pane_rect.bottom(),
                ),
            ),
        ),
        (
            DropZone::Right,
            egui::Rect::from_min_max(
                egui::pos2(
                    pane_rect.right() - pane_rect.width() * 0.25,
                    pane_rect.top(),
                ),
                pane_rect.right_bottom(),
            ),
        ),
    ];
    for (zone, rect) in zones {
        let alpha = if zone == active_zone { 80u8 } else { 30u8 };
        painter.rect_filled(
            rect,
            4.0,
            egui::Color32::from_rgba_unmultiplied(86, 156, 214, alpha),
        );
    }
}

fn load_history() -> Vec<String> {
    directories::ProjectDirs::from("", "", "myterm")
        .and_then(|d| {
            let path = d.data_dir().join("history.json");
            std::fs::read_to_string(path).ok()
        })
        .and_then(|data| serde_json::from_str::<Vec<String>>(&data).ok())
        .unwrap_or_default()
}
