use super::*;
use crate::pane::PaneId;

fn id(n: u32) -> PaneId {
    PaneId(n)
}

// Make PaneId constructible in tests
impl PaneId {
    pub(crate) fn raw(n: u32) -> Self { Self(n) }
}

#[test]
fn split_leaf_produces_split_node() {
    let a = id(1);
    let b = id(2);
    let layout = PaneLayout::Leaf(a).split(a, b, SplitDir::Horizontal, true);
    match layout {
        PaneLayout::Split { first, second, dir, .. } => {
            assert_eq!(dir, SplitDir::Horizontal);
            assert!(matches!(*first, PaneLayout::Leaf(id) if id == a));
            assert!(matches!(*second, PaneLayout::Leaf(id) if id == b));
        }
        _ => panic!("expected Split"),
    }
}

#[test]
fn remove_second_leaf_collapses_to_first() {
    let a = id(1);
    let b = id(2);
    let layout = PaneLayout::Leaf(a).split(a, b, SplitDir::Horizontal, true);
    let after = layout.remove(b).expect("should not be empty");
    assert!(matches!(after, PaneLayout::Leaf(id) if id == a));
}

#[test]
fn remove_first_leaf_collapses_to_second() {
    let a = id(1);
    let b = id(2);
    let layout = PaneLayout::Leaf(a).split(a, b, SplitDir::Horizontal, true);
    let after = layout.remove(a).expect("should not be empty");
    assert!(matches!(after, PaneLayout::Leaf(id) if id == b));
}

#[test]
fn classify_drop_far_left_is_left() {
    let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 200.0));
    let zone = classify_drop(egui::pos2(10.0, 100.0), rect);
    assert_eq!(zone, DropZone::Left);
}

#[test]
fn classify_drop_center_is_center() {
    let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 200.0));
    let zone = classify_drop(egui::pos2(100.0, 100.0), rect);
    assert_eq!(zone, DropZone::Center);
}

#[test]
fn classify_drop_top_edge() {
    let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 200.0));
    let zone = classify_drop(egui::pos2(100.0, 10.0), rect);
    assert_eq!(zone, DropZone::Top);
}

#[test]
fn pane_ids_collects_all_leaves() {
    let a = id(1);
    let b = id(2);
    let c = id(3);
    let layout = PaneLayout::Leaf(a)
        .split(a, b, SplitDir::Horizontal, true)
        .split(b, c, SplitDir::Vertical, true);
    let ids = layout.pane_ids();
    assert!(ids.contains(&a));
    assert!(ids.contains(&b));
    assert!(ids.contains(&c));
}
