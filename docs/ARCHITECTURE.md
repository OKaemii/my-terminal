# Architecture

myterm is a Cargo workspace. Each crate has a single responsibility and a defined place in the dependency graph. No crate may introduce a circular dependency.

## Workspace layout

```
myterm/
├── Cargo.toml              workspace manifest
├── CLAUDE.md               dev commands + coding conventions
├── legacy/                 original ~/myterm/ (reference, not compiled)
├── docs/                   this directory
├── agent-docs/plans/       implementation plans
├── app/                    [[bin]] — entry point only
└── crates/
    ├── myterm_core/        shared primitives (no internal deps)
    ├── myterm_pty/         PTY session
    ├── myterm_vte/         VTE output processor
    ├── myterm_settings/    TOML config
    ├── myterm_features/    feature flag enum
    ├── myterm_ui/          egui rendering + orchestration
    ├── feature_git/        git status
    ├── feature_suggest/    autosuggestions
    ├── feature_jump/       frecency directory jump
    ├── feature_highlight/  input syntax highlighting
    ├── feature_fzf/        enhanced fuzzy finder
    ├── feature_extract/    archive extraction (Phase 4)
    ├── feature_websearch/  open URLs (Phase 4)
    ├── feature_man/        coloured man pages (Phase 4)
    └── feature_cnf/        command-not-found hints (Phase 4)
```

## Dependency graph

```
app
 ├── myterm_core      ← no deps
 ├── myterm_pty       ← myterm_core
 ├── myterm_vte       ← myterm_core
 ├── myterm_settings  ← myterm_core
 ├── myterm_features  ← no deps
 ├── myterm_ui        ← myterm_core, myterm_settings, myterm_features
 │                       myterm_pty, myterm_vte
 ├── feature_git      ← myterm_core
 ├── feature_suggest  ← myterm_core
 ├── feature_jump     ← myterm_core
 ├── feature_highlight← myterm_core
 └── feature_fzf      ← myterm_core, feature_suggest
```

Rule: `feature_*` crates must not depend on `myterm_ui`. UI pulls features in; features do not pull UI.

## Data flow

```
Shell process
    │  bytes via PTY fd
    ▼
myterm_pty::PtySession
    │  Vec<u8> chunks via mpsc::Receiver
    ▼
myterm_ui::TerminalApp::process_pty()
    │  line-split + marker detection
    ▼
myterm_vte::OutputProcessor (vte::Perform impl)
    │  Vec<StyledChar> lines
    ▼
CommandBlock.output  (in myterm_core)
    │
    ▼
myterm_ui::blocks::render_block()  →  egui frame
```

## Key types

**`myterm_core`:**

| Type | Description |
|------|-------------|
| `StyledChar` | A single char with `Color32` + bold flag |
| `CommandBlock` | One complete command: input, output, exit code, cwd, duration |
| `palette::*` | Canonical colour constants — single definition for all crates |

**`myterm_ui` (tabs/layout):**

| Type | File | Description |
|------|------|-------------|
| `PaneId` | `pane.rs` | Unique ID for a terminal session |
| `TerminalPane` | `pane.rs` | One independent PTY session + block history |
| `PaneLayout` | `layout.rs` | Recursive tree: `Leaf(PaneId)` or `Split{dir, ratio, first, second}` |
| `SplitDir` | `layout.rs` | `Horizontal` (side-by-side) or `Vertical` (stacked) |
| `DropZone` | `layout.rs` | `Left`, `Right`, `Top`, `Bottom`, `Center` — drag-drop target areas |
| `TabDragState` | `tabs.rs` | Active drag in progress: which pane, current pointer position |

See `docs/TABS_SPLITS.md` for full design.

## Startup sequence

1. `app/src/main.rs` — load `Settings` (TOML, falls back to default)
2. Resolve active `Theme` from `settings.appearance.color_scheme` (preset or custom)
3. Initialise `myterm_features` flags from `settings.features`
4. Construct feature instances, collect as `Vec<Box<dyn Feature>>`
5. Construct `myterm_ui::TerminalApp` with features + settings + resolved theme
6. Hand off to `eframe::run_native` (with `with_transparent(true)` when `background_opacity < 100`)

## PTY marker protocol

Two out-of-band markers are injected into the shell RC via `PROMPT_COMMAND` / `precmd`:

- `__MYTERM_READY__` — shell prompt is ready; a new command may be entered
- `__MYTERM_META__:/path/to/cwd:exit_code` — cwd + exit code of the just-completed command

These markers let myterm know exactly when commands start and finish without polling.

## Alternate-screen detection

When a TUI program (vim, htop) writes `ESC[?1049h` or `ESC[?47h`, `myterm_vte` sets `alternate_screen = true`. The UI switches to a passthrough mode that forwards all keypresses directly to the PTY instead of routing them through the input field.

## Adding a new feature crate

1. `cargo new --lib crates/feature_mything`
2. Add `feature_mything = { path = "crates/feature_mything" }` to workspace `Cargo.toml`
3. Implement `myterm_ui::plugin::Feature` for your type
4. Register in `app/src/main.rs`: `features.push(Box::new(feature_mything::MyThing::new()))`
5. Gate with `myterm_features::FeatureFlag::MyThing.is_enabled()` inside the crate
