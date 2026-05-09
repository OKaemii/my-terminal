# Tabs & Split View

How myterm manages multiple terminal sessions and arranges them in the window.

---

## Mental model

- A **tab** is a named `TerminalPane` — one independent shell process with its own PTY, VTE state, and command history.
- A **layout** is a binary tree describing how visible panes are arranged in the window.
- There is one tab bar at the top. All `TerminalPane` instances appear there. The layout tree controls which subset of them are displayed side-by-side vs stacked.
- When you drag a tab onto an active pane you create a split: the tab moves from "background tab" to "visible in a split node".

---

## Data structures

### `TerminalPane` (`crates/myterm_ui/src/pane.rs`)

One complete terminal session. Owns:
- `pty: PtySession` — shell subprocess
- `vte_parser`, `vte_proc` — VTE output processor state
- `blocks: Vec<CommandBlock>` — rendered command history
- `input`, `cwd`, `startup_done`, `scroll_to_bottom` — per-session UI state

### `PaneLayout` (`crates/myterm_ui/src/layout.rs`)

Recursive binary tree:

```rust
pub enum PaneLayout {
    Leaf(PaneId),
    Split { dir: SplitDir, ratio: f32, first: Box<PaneLayout>, second: Box<PaneLayout> },
}
```

- `SplitDir::Horizontal` → `first` is left, `second` is right
- `SplitDir::Vertical` → `first` is top, `second` is bottom
- `ratio` is the fraction of total space given to `first` (0.0–1.0, clamped to min pane size)

**Example: 3-pane layout**
```
Split(Horizontal, 0.5,
  Leaf(A),
  Split(Vertical, 0.6,
    Leaf(B),
    Leaf(C)
  )
)
```
Renders as:
```
┌────────┬────────────┐
│        │     B      │
│   A    ├────────────┤
│        │     C      │
└────────┴────────────┘
```

### `TerminalApp` owns

```rust
pub struct TerminalApp {
    panes:  Vec<TerminalPane>,   // all tabs (visible or not)
    layout: PaneLayout,          // arrangement of visible panes
    active: PaneId,              // which pane has keyboard focus
    next_id: u32,
    drag_state: Option<TabDragState>,
    // ... shared state (settings, features, jump_db, etc.)
}
```

---

## Tab bar (`crates/myterm_ui/src/tabs.rs`)

Rendered as a top panel. Shows one label per `TerminalPane` in `panes` order.

- **Left-click tab** → `app.active = pane_id`
- **Middle-click tab** → `app.close_pane(pane_id)` (splits collapse)
- **Ctrl+T** → `app.new_pane()` — spawns new PTY, appends to `panes`, sets as active
- **Ctrl+W** → close active pane
- **Ctrl+Tab / Ctrl+Shift+Tab** → cycle active pane
- **"+" button** at right edge → same as Ctrl+T

Tab title is the last component of `pane.cwd` (e.g. `~`, `src`, `myterm`), updated on each `on_prompt_ready`.

---

## Drag-to-split

### Lifecycle

1. **Drag starts** — `response.drag_started()` is true for a tab label. Store `TabDragState { pane_id, ghost_pos }` in `app.drag_state`.
2. **Drag ongoing** — render a floating ghost (translucent copy of the tab label) at `ctx.pointer_latest_pos()`.
3. **Drop zone hover** — if pointer is over a pane's content rect, call `classify_drop(pointer, rect)` and highlight the relevant zone.
4. **Drop released** — `response.drag_released()` or pointer-up detected. Look up which pane was hovered and which zone. Call `PaneLayout::split(target, dragged, dir, new_is_second)` to update the tree.

### Drop zone geometry

```
┌──────────────────────────────────┐
│           TOP  (25%)             │  SplitDir::Vertical, new=top
├───────┬──────────────┬───────────┤
│ LEFT  │   CENTER     │   RIGHT   │  LEFT/RIGHT → Horizontal split
│ 25%   │  (no split)  │    25%    │  CENTER → reorder tab (no split)
├───────┴──────────────┴───────────┤
│          BOTTOM (25%)            │  SplitDir::Vertical, new=bottom
└──────────────────────────────────┘
```

Zones are rendered as semi-transparent blue overlays (alpha=80 for active zone, alpha=30 for others).

### `classify_drop` logic

```rust
pub fn classify_drop(pointer: egui::Pos2, pane_rect: egui::Rect) -> DropZone {
    let rx = (pointer.x - pane_rect.left()) / pane_rect.width();
    let ry = (pointer.y - pane_rect.top()) / pane_rect.height();
    if ry < 0.25 { return DropZone::Top; }
    if ry > 0.75 { return DropZone::Bottom; }
    if rx < 0.25 { return DropZone::Left; }
    if rx > 0.75 { return DropZone::Right; }
    DropZone::Center
}
```

---

## Divider resize

Each `Split` node renders a thin divider between its two children. The divider is a narrow `egui::Rect` that:
- Shows a resize cursor on hover (`CursorIcon::ResizeHorizontal` or `ResizeVertical`)
- On drag, adjusts `ratio` clamped to `[min_frac, 1.0 - min_frac]` where `min_frac = 100.0 / total_size`

`TerminalApp` stores `divider_drag: Option<DividerDragState>` which identifies the layout path to the Split node and the initial pointer position, enabling sub-pixel accurate ratio updates each frame.

---

## Closing a pane / collapsing splits

When `app.close_pane(id)` is called:
1. `PaneLayout::remove(id)` is called — if a `Split` loses one child, it collapses to the remaining child.
2. The `TerminalPane` is removed from `app.panes`.
3. `active` is updated to the most recently active remaining pane.
4. The closed pane's PTY receives an EOF signal before being dropped.

---

## Keyboard shortcuts

| Keys | Action |
|------|--------|
| Ctrl+T | New tab |
| Ctrl+W | Close active pane |
| Ctrl+Tab | Focus next pane (cycles through layout order) |
| Ctrl+Shift+Tab | Focus previous pane |
| Ctrl+\ | Split active pane: add new pane to the right |
| Ctrl+- | Split active pane: add new pane below |
| Ctrl+Shift+W | Close active split pane (not its tab) |

---

## Rendering the layout tree

`render_layout(layout, rect, ui, app)` is a recursive function:

```rust
fn render_layout(layout: &PaneLayout, rect: egui::Rect, app: &mut TerminalApp, ui: &mut egui::Ui) {
    match layout {
        PaneLayout::Leaf(id) => {
            render_pane(*id, rect, app, ui);
        }
        PaneLayout::Split { dir, ratio, first, second } => {
            let (r1, divider, r2) = split_rect(rect, *dir, *ratio);
            render_layout(first,  r1, app, ui);
            render_divider(divider, *dir, ui);  // draggable divider
            render_layout(second, r2, app, ui);
        }
    }
}

fn split_rect(rect: Rect, dir: SplitDir, ratio: f32) -> (Rect, Rect, Rect) {
    const DIV: f32 = 4.0;  // divider thickness in pixels
    match dir {
        SplitDir::Horizontal => {
            let x_split = rect.left() + rect.width() * ratio;
            (
                Rect::from_min_max(rect.left_top(),  pos2(x_split - DIV/2.0, rect.bottom())),
                Rect::from_min_max(pos2(x_split - DIV/2.0, rect.top()), pos2(x_split + DIV/2.0, rect.bottom())),
                Rect::from_min_max(pos2(x_split + DIV/2.0, rect.top()), rect.right_bottom()),
            )
        }
        SplitDir::Vertical => {
            let y_split = rect.top() + rect.height() * ratio;
            (
                Rect::from_min_max(rect.left_top(),  pos2(rect.right(), y_split - DIV/2.0)),
                Rect::from_min_max(pos2(rect.left(), y_split - DIV/2.0), pos2(rect.right(), y_split + DIV/2.0)),
                Rect::from_min_max(pos2(rect.left(), y_split + DIV/2.0), rect.right_bottom()),
            )
        }
    }
}
```
