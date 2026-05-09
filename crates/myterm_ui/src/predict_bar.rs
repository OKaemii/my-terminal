use egui::{Frame, Margin};
use feature_predict::{apply_completion, ChipKind, PredictChip};

use crate::theme::ResolvedPalette;

/// Render the predict bar chip strip above the input panel.
/// Hidden when `chips` is empty or the input is empty.
pub fn render_predict_bar(
    chips: &[PredictChip],
    input: &mut String,
    palette: &ResolvedPalette,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    if chips.is_empty() || input.is_empty() {
        return;
    }

    egui::TopBottomPanel::bottom(egui::Id::new("predict_bar"))
        .exact_height(28.0)
        .frame(
            Frame::none()
                .fill(palette.status_bg)
                .inner_margin(Margin::symmetric(8.0, 3.0)),
        )
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                for chip in chips.iter().take(7) {
                    let color = chip_color(chip.kind, palette);
                    let btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new(&chip.label)
                                .monospace()
                                .color(color)
                                .size(13.0),
                        )
                        .frame(false)
                        .min_size(egui::vec2(0.0, 22.0)),
                    );

                    if btn.clicked() {
                        apply_completion(input, &chip.completion);
                    }

                    // Tooltip on hover (no delay; egui handles pointer position)
                    if btn.hovered() {
                        let layer = egui::LayerId::new(
                            egui::Order::Tooltip,
                            egui::Id::new("predict_tip_layer"),
                        );
                        egui::show_tooltip_at_pointer(
                            ctx,
                            layer,
                            egui::Id::new("predict_tip"),
                            |ui| {
                                if let Some(desc) = &chip.description {
                                    ui.colored_label(palette.text, desc);
                                }
                                if let Some(usage) = &chip.usage {
                                    ui.label(
                                        egui::RichText::new(usage)
                                            .monospace()
                                            .color(palette.cyan)
                                            .small(),
                                    );
                                }
                            },
                        );
                    }

                    ui.add(egui::Separator::default().vertical().spacing(4.0));
                }
            });
        });
}

fn chip_color(kind: ChipKind, palette: &ResolvedPalette) -> egui::Color32 {
    match kind {
        ChipKind::Command => palette.green,
        ChipKind::Builtin => palette.cyan,
        ChipKind::Keyword => palette.yellow,
        ChipKind::Subcommand => palette.blue,
        ChipKind::Flag => palette.dim,
        ChipKind::History => palette.orange,
    }
}
