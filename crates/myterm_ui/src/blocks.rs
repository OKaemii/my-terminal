use std::collections::HashMap;
use std::time::Instant;

use egui::{Frame, Margin, Rounding, Stroke};
use myterm_core::CommandBlock;
use myterm_vte::line_to_layout_job;

use crate::theme::ResolvedPalette;

/// Per-block flash state for the copy button feedback.
pub struct BlockFlash {
    /// Which button was last clicked: "cmd" or "out".
    pub kind: &'static str,
    pub at: Instant,
}

const FLASH_DURATION_MS: u128 = 300;

/// Render all command blocks in a scroll area.
pub fn render_blocks(
    blocks: &[CommandBlock],
    flash_states: &mut HashMap<usize, BlockFlash>,
    font_size: f32,
    scroll_to_bottom: bool,
    palette: &ResolvedPalette,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(scroll_to_bottom)
        .show(ui, |ui| {
            for (idx, block) in blocks.iter().enumerate() {
                render_block(idx, block, flash_states, font_size, palette, ctx, ui);
                ui.add_space(4.0);
            }
        });
}

fn render_block(
    idx: usize,
    block: &CommandBlock,
    flash_states: &mut HashMap<usize, BlockFlash>,
    font_size: f32,
    palette: &ResolvedPalette,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    let resp = Frame::none()
        .fill(palette.block_bg)
        .stroke(Stroke::new(1.0, palette.block_border))
        .rounding(Rounding::same(4.0))
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            // ── header row ────────────────────────────────────────────────
            ui.horizontal(|ui| {
                // Exit code dot
                let dot_color = if block.exit_code == 0 { palette.green } else { palette.red };
                ui.colored_label(dot_color, if block.exit_code == 0 { "●" } else { "●" });

                ui.colored_label(palette.text, &block.command);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if block.duration_ms > 0 {
                        ui.colored_label(palette.dim, format!("{}ms", block.duration_ms));
                    }
                    let cwd_str = block.cwd.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    if !cwd_str.is_empty() {
                        ui.colored_label(palette.dim, format!(" {cwd_str}"));
                    }
                });
            });

            if block.is_running {
                ui.colored_label(palette.dim, "  running…");
                return;
            }

            if block.is_fullscreen {
                ui.colored_label(palette.dim, "  (full-screen TUI — output suppressed)");
                return;
            }

            // ── output lines ──────────────────────────────────────────────
            for line in &block.output {
                let job = line_to_layout_job(line, font_size);
                ui.label(job);
            }
        });

    // ── hover copy buttons ────────────────────────────────────────────────
    if resp.response.hovered() || flash_states.contains_key(&idx) {
        let block_rect = resp.response.rect;
        let button_area = egui::Rect::from_min_size(
            egui::pos2(block_rect.right() - 144.0, block_rect.top() + 4.0),
            egui::vec2(140.0, 24.0),
        );
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(button_area), |ui| {
            ui.horizontal(|ui| {
                // Determine flash colour
                let flash = flash_states.get(&idx);
                let cmd_color = if flash.map(|f| f.kind == "cmd").unwrap_or(false)
                    && flash.map(|f| f.at.elapsed().as_millis() < FLASH_DURATION_MS).unwrap_or(false)
                {
                    palette.green
                } else {
                    palette.dim
                };
                let out_color = if flash.map(|f| f.kind == "out").unwrap_or(false)
                    && flash.map(|f| f.at.elapsed().as_millis() < FLASH_DURATION_MS).unwrap_or(false)
                {
                    palette.green
                } else {
                    palette.dim
                };

                if ui.add(egui::Button::new(
                    egui::RichText::new("⎘ cmd").color(cmd_color).small()
                ).frame(false)).clicked() {
                    ctx.output_mut(|o| o.copied_text = block.command.clone());
                    flash_states.insert(idx, BlockFlash { kind: "cmd", at: Instant::now() });
                }
                ui.add_space(4.0);
                if ui.add(egui::Button::new(
                    egui::RichText::new("⎘ out").color(out_color).small()
                ).frame(false)).clicked() {
                    let text = block.output.iter()
                        .map(|line| line.iter().map(|sc| sc.ch).collect::<String>())
                        .collect::<Vec<_>>()
                        .join("\n");
                    ctx.output_mut(|o| o.copied_text = text);
                    flash_states.insert(idx, BlockFlash { kind: "out", at: Instant::now() });
                }
            });
        });

        // Expire flash states
        flash_states.retain(|_, f| f.at.elapsed().as_millis() < FLASH_DURATION_MS + 50);
    }
}
