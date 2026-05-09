use egui::Frame;
use feature_git::GitInfo;

use crate::plugin::StatusSegment;
use crate::theme::ResolvedPalette;

/// Render the status bar at the bottom of the window.
pub fn render_status_bar(
    git_info: Option<&GitInfo>,
    segments: &[StatusSegment],
    cwd: &std::path::Path,
    palette: &ResolvedPalette,
    ctx: &egui::Context,
) {
    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(22.0)
        .frame(Frame::none().fill(palette.status_bg))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(8.0);

                // CWD
                let cwd_str = cwd.to_string_lossy();
                ui.colored_label(palette.dim, cwd_str.as_ref());

                // Git info
                if let Some(git) = git_info {
                    ui.add_space(8.0);
                    ui.colored_label(palette.blue, format!(" {}", git.branch));
                    if git.is_dirty {
                        ui.colored_label(palette.yellow, "✎");
                    }
                    if git.ahead > 0 {
                        ui.colored_label(palette.green, format!("↑{}", git.ahead));
                    }
                    if git.behind > 0 {
                        ui.colored_label(palette.red, format!("↓{}", git.behind));
                    }
                }

                // Feature segments
                for seg in segments {
                    ui.add_space(8.0);
                    let color = if seg.accent { palette.cyan } else { palette.dim };
                    ui.colored_label(color, &seg.label);
                }

                // Right-aligned: feature flag indicators
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    ui.colored_label(palette.dim, "myterm");
                });
            });
        });
}
