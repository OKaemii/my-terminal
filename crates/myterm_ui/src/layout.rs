use egui::{Pos2, Rect};

use crate::pane::PaneId;

/// Direction to split two children.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SplitDir {
    Horizontal, // children side by side: [left | right]
    Vertical,   // children stacked: [top] / [bottom]
}

/// Recursive layout tree.
#[derive(Clone, Debug)]
pub enum PaneLayout {
    Leaf(PaneId),
    Split {
        dir: SplitDir,
        /// Fraction of total space given to `first` (0.0..1.0).
        ratio: f32,
        first: Box<PaneLayout>,
        second: Box<PaneLayout>,
    },
}

impl PaneLayout {
    /// Insert a new pane by splitting an existing leaf.
    pub fn split(self, target: PaneId, new: PaneId, dir: SplitDir, new_is_second: bool) -> Self {
        match self {
            PaneLayout::Leaf(id) if id == target => {
                let (first, second) = if new_is_second {
                    (
                        Box::new(PaneLayout::Leaf(id)),
                        Box::new(PaneLayout::Leaf(new)),
                    )
                } else {
                    (
                        Box::new(PaneLayout::Leaf(new)),
                        Box::new(PaneLayout::Leaf(id)),
                    )
                };
                PaneLayout::Split {
                    dir,
                    ratio: 0.5,
                    first,
                    second,
                }
            }
            PaneLayout::Split {
                dir: d,
                ratio,
                first,
                second,
            } => PaneLayout::Split {
                dir: d,
                ratio,
                first: Box::new(first.split(target, new, dir, new_is_second)),
                second: Box::new(second.split(target, new, dir, new_is_second)),
            },
            other => other,
        }
    }

    /// Remove a pane from the tree; sibling collapses when parent becomes childless.
    pub fn remove(self, target: PaneId) -> Option<Self> {
        match self {
            PaneLayout::Leaf(id) => {
                if id == target {
                    None
                } else {
                    Some(PaneLayout::Leaf(id))
                }
            }
            PaneLayout::Split {
                dir,
                ratio,
                first,
                second,
            } => match (first.remove(target), second.remove(target)) {
                (None, Some(s)) => Some(s),
                (Some(f), None) => Some(f),
                (Some(f), Some(s)) => Some(PaneLayout::Split {
                    dir,
                    ratio,
                    first: Box::new(f),
                    second: Box::new(s),
                }),
                (None, None) => None,
            },
        }
    }

    /// Collect all leaf PaneIds in tree order.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        match self {
            PaneLayout::Leaf(id) => vec![*id],
            PaneLayout::Split { first, second, .. } => {
                let mut v = first.pane_ids();
                v.extend(second.pane_ids());
                v
            }
        }
    }

    /// Update the ratio of a split node identified by path from root.
    /// `path` is a sequence of booleans: false = go to first, true = go to second.
    pub fn set_ratio(&mut self, path: &[bool], new_ratio: f32) {
        match self {
            PaneLayout::Split {
                ratio,
                first,
                second,
                ..
            } => {
                if path.is_empty() {
                    *ratio = new_ratio.clamp(0.1, 0.9);
                } else if !path[0] {
                    first.set_ratio(&path[1..], new_ratio);
                } else {
                    second.set_ratio(&path[1..], new_ratio);
                }
            }
            PaneLayout::Leaf(_) => {}
        }
    }
}

/// Which edge zone a drag was released in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DropZone {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

/// Classify pointer position relative to `pane_rect` into a drop zone.
pub fn classify_drop(pointer: Pos2, pane_rect: Rect) -> DropZone {
    let rx = (pointer.x - pane_rect.left()) / pane_rect.width();
    let ry = (pointer.y - pane_rect.top()) / pane_rect.height();

    if ry < 0.25 {
        return DropZone::Top;
    }
    if ry > 0.75 {
        return DropZone::Bottom;
    }
    if rx < 0.25 {
        return DropZone::Left;
    }
    if rx > 0.75 {
        return DropZone::Right;
    }
    DropZone::Center
}

pub struct SplitAction {
    pub target: PaneId,
    pub new: PaneId,
    pub dir: SplitDir,
    pub new_is_second: bool,
}

pub fn zone_to_split(zone: DropZone, dragged: PaneId, target: PaneId) -> Option<SplitAction> {
    match zone {
        DropZone::Left => Some(SplitAction {
            target,
            new: dragged,
            dir: SplitDir::Horizontal,
            new_is_second: false,
        }),
        DropZone::Right => Some(SplitAction {
            target,
            new: dragged,
            dir: SplitDir::Horizontal,
            new_is_second: true,
        }),
        DropZone::Top => Some(SplitAction {
            target,
            new: dragged,
            dir: SplitDir::Vertical,
            new_is_second: false,
        }),
        DropZone::Bottom => Some(SplitAction {
            target,
            new: dragged,
            dir: SplitDir::Vertical,
            new_is_second: true,
        }),
        DropZone::Center => None,
    }
}

/// Divide `rect` into two sub-rects with a 4px divider gap.
pub fn split_rect(rect: Rect, dir: SplitDir, ratio: f32) -> (Rect, Rect) {
    const DIVIDER: f32 = 4.0;
    match dir {
        SplitDir::Horizontal => {
            let split_x = rect.left() + rect.width() * ratio;
            let first =
                Rect::from_min_max(rect.min, egui::pos2(split_x - DIVIDER / 2.0, rect.max.y));
            let second =
                Rect::from_min_max(egui::pos2(split_x + DIVIDER / 2.0, rect.min.y), rect.max);
            (first, second)
        }
        SplitDir::Vertical => {
            let split_y = rect.top() + rect.height() * ratio;
            let first =
                Rect::from_min_max(rect.min, egui::pos2(rect.max.x, split_y - DIVIDER / 2.0));
            let second =
                Rect::from_min_max(egui::pos2(rect.min.x, split_y + DIVIDER / 2.0), rect.max);
            (first, second)
        }
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
