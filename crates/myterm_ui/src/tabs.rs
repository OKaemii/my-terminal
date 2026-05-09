use egui::{Frame, Pos2};

use crate::pane::PaneId;

/// Drag state while a tab is being dragged.
#[derive(Default)]
pub struct TabDragState {
    pub dragging: Option<PaneId>,
    pub ghost_pos: Option<Pos2>,
}

/// Actions the tab bar can emit to the caller.
pub enum TabAction {
    Activate(PaneId),
    Close(PaneId),
    NewTab,
    Split {
        target: PaneId,
        new: PaneId,
        dir: crate::layout::SplitDir,
        new_is_second: bool,
    },
    Reorder(PaneId),
}

/// Render the tab bar and return any action the user triggered.
pub fn render_tab_bar(
    panes: &[(PaneId, &str)], // (id, title)
    active: PaneId,
    drag: &mut TabDragState,
    palette: &crate::theme::ResolvedPalette,
    ctx: &egui::Context,
) -> Option<TabAction> {
    let mut action = None;

    egui::TopBottomPanel::top("tab_bar")
        .exact_height(28.0)
        .frame(Frame::none().fill(palette.status_bg))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(4.0);

                for &(pane_id, title) in panes {
                    let is_active = pane_id == active;
                    let label = egui::RichText::new(title)
                        .color(if is_active { palette.text } else { palette.dim })
                        .monospace()
                        .size(13.0);

                    let resp = ui.selectable_label(is_active, label);

                    if resp.clicked() {
                        action = Some(TabAction::Activate(pane_id));
                    }
                    if resp.drag_started() {
                        drag.dragging = Some(pane_id);
                    }
                    if resp.dragged() {
                        drag.ghost_pos = ctx.pointer_latest_pos();
                    }
                    if resp.drag_stopped() {
                        drag.dragging = None;
                        drag.ghost_pos = None;
                    }
                    if resp.middle_clicked() {
                        action = Some(TabAction::Close(pane_id));
                    }

                    ui.add_space(2.0);
                }

                if ui.small_button("+").clicked() {
                    action = Some(TabAction::NewTab);
                }

                // Ghost label while dragging
                if let (Some(_), Some(pos)) = (drag.dragging, drag.ghost_pos) {
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Tooltip,
                        egui::Id::new("tab_drag_ghost"),
                    ));
                    painter.text(
                        pos,
                        egui::Align2::LEFT_TOP,
                        "⠿",
                        egui::FontId::monospace(13.0),
                        palette.dim,
                    );
                }
            });
        });

    action
}
