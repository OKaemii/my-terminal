use egui::{Frame, Key, Margin, Modifiers};
use feature_fzf::FzfState;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use crate::theme::ResolvedPalette;

/// Render the floating fuzzy-search palette window.
/// Returns `Some(command)` when the user confirms a selection, `None` otherwise.
pub fn render_palette(
    fzf: &mut FzfState,
    history: &[String],
    palette: &ResolvedPalette,
    ctx: &egui::Context,
) -> Option<String> {
    if !fzf.open {
        return None;
    }

    let mut chosen = None;

    egui::Window::new("command palette")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 60.0))
        .fixed_size([560.0, 320.0])
        .frame(
            Frame::none()
                .fill(palette.block_bg)
                .stroke(egui::Stroke::new(1.0, palette.block_border))
                .inner_margin(Margin::same(8.0)),
        )
        .show(ctx, |ui| {
            // Query input
            let query_resp = ui.add(
                egui::TextEdit::singleline(&mut fzf.query)
                    .font(egui::FontId::monospace(14.0))
                    .text_color(palette.text)
                    .hint_text("search history…")
                    .frame(true)
                    .desired_width(f32::INFINITY),
            );
            query_resp.request_focus();

            if query_resp.changed() {
                fzf.update_query(history, &fzf.query.clone());
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Results list
            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                let results = fzf.results.clone();
                for (i, entry) in results.iter().enumerate() {
                    let is_sel = i == fzf.selected;
                    let label = egui::RichText::new(entry)
                        .monospace()
                        .size(13.0)
                        .color(if is_sel { palette.text } else { palette.dim });
                    let resp = ui.selectable_label(is_sel, label);
                    if resp.clicked() {
                        chosen = Some(entry.clone());
                        fzf.close();
                    }
                    if is_sel {
                        resp.scroll_to_me(None);
                    }
                }
            });

            // Keyboard navigation
            ctx.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::Escape) {
                    fzf.close();
                }
                if i.consume_key(Modifiers::NONE, Key::Enter) {
                    if let Some(entry) = fzf.selected_entry() {
                        chosen = Some(entry.to_string());
                    }
                    fzf.close();
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    fzf.move_up();
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    fzf.move_down();
                }
            });
        });

    chosen
}

/// Score-based fuzzy filter, exported for use in FzfState::update_query.
pub fn fuzzy_filter(items: &[String], query: &str) -> Vec<String> {
    if query.is_empty() {
        return items.iter().rev().cloned().collect();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &String)> = items
        .iter()
        .filter_map(|h| matcher.fuzzy_match(h, query).map(|s| (s, h)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, h)| h.clone()).collect()
}
