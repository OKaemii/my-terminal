use egui::{Frame, Key, Margin, Modifiers};
use feature_suggest::Suggester;

use crate::pane::TerminalPane;
use crate::theme::ResolvedPalette;

/// Actions the input panel can emit to the parent app.
pub enum InputAction {
    Submit,
    OpenPalette,
    NewTab,
    CloseTab,
    NextPane,
    PrevPane,
    SplitRight,
    SplitBelow,
}

/// Render the input panel (bottom of a pane) and handle key events.
/// Returns any action that should be processed by the caller.
pub fn render_input_panel(
    pane: &mut TerminalPane,
    suggester: &Suggester,
    palette: &ResolvedPalette,
    font_size: f32,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) -> Option<InputAction> {
    let mut action = None;

    // Update suggestion from history
    pane.suggestion = suggester.suggest(&pane.input);

    egui::TopBottomPanel::bottom(egui::Id::new(("input_panel", pane.id)))
        .exact_height(36.0)
        .frame(
            Frame::none()
                .fill(palette.input_bg)
                .inner_margin(Margin::symmetric(8.0, 6.0)),
        )
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                // Prompt symbol
                ui.colored_label(palette.green, "❯");
                ui.add_space(4.0);

                let text_edit_resp = ui.add_sized(
                    [ui.available_width(), 22.0],
                    egui::TextEdit::singleline(&mut pane.input)
                        .font(egui::FontId::monospace(font_size))
                        .text_color(palette.text)
                        .frame(false)
                        .desired_width(f32::INFINITY),
                );

                // Ghost autosuggestion overlay (dim text after cursor)
                if let Some(ref sug) = pane.suggestion {
                    if !sug.is_empty() {
                        let ghost = format!("{}{}", pane.input, sug);
                        // We can't easily overlay egui TextEdit; paint ghost as a label beside it
                        // A proper overlay would require custom painter; this approximation suffices.
                        let _ = ghost; // rendered via the suggest bar instead
                    }
                }

                if text_edit_resp.lost_focus() && ctx.input(|i| i.key_pressed(Key::Enter)) {
                    action = Some(InputAction::Submit);
                }
            });
        });

    // Key handling (checked every frame via egui input state)
    ctx.input_mut(|i| {
        // Tab → accept autosuggestion
        if i.consume_key(Modifiers::NONE, Key::Tab) {
            if let Some(sug) = pane.suggestion.take() {
                pane.input.push_str(&sug);
            }
        }
        // Enter → submit
        if i.consume_key(Modifiers::NONE, Key::Enter) && action.is_none() {
            action = Some(InputAction::Submit);
        }
        // History navigation
        if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
            history_up(pane, suggester);
        }
        if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
            history_down(pane, suggester);
        }
        // Ctrl+R → open palette
        if i.consume_key(Modifiers::CTRL, Key::R) {
            action = Some(InputAction::OpenPalette);
        }
        // Ctrl+T → new tab
        if i.consume_key(Modifiers::CTRL, Key::T) {
            action = Some(InputAction::NewTab);
        }
        // Ctrl+W → close tab
        if i.consume_key(Modifiers::CTRL, Key::W) {
            action = Some(InputAction::CloseTab);
        }
        // Ctrl+Tab → next pane
        if i.consume_key(Modifiers::CTRL, Key::Tab) {
            action = Some(InputAction::NextPane);
        }
        // Ctrl+\ → split right
        if i.consume_key(Modifiers::CTRL, Key::Backslash) {
            action = Some(InputAction::SplitRight);
        }
        // Ctrl+- → split below
        if i.consume_key(Modifiers::CTRL, Key::Minus) {
            action = Some(InputAction::SplitBelow);
        }
    });

    action
}

fn history_up(pane: &mut TerminalPane, suggester: &Suggester) {
    let h = suggester.history();
    if h.is_empty() {
        return;
    }
    match pane.history_nav {
        None => {
            pane.history_draft = pane.input.clone();
            pane.history_nav = Some(h.len() - 1);
        }
        Some(0) => {}
        Some(ref mut i) => {
            *i -= 1;
        }
    }
    if let Some(i) = pane.history_nav {
        pane.input = suggester.history()[i].clone();
    }
}

fn history_down(pane: &mut TerminalPane, suggester: &Suggester) {
    let h = suggester.history();
    match pane.history_nav {
        None => {}
        Some(i) if i + 1 >= h.len() => {
            pane.history_nav = None;
            pane.input = pane.history_draft.clone();
        }
        Some(ref mut i) => {
            *i += 1;
            let idx = *i;
            pane.input = suggester.history()[idx].clone();
        }
    }
}
