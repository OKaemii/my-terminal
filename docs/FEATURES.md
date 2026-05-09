# Features

Every feature in myterm, its oh-my-zsh equivalent, current status, crate location, and roadmap.

## Feature status key

| Symbol | Meaning |
|--------|---------|
| ✅ | Complete and well-implemented |
| 🔧 | Partially done — present but needs improvement |
| ⬜ | Planned, not yet started |

---

## Autosuggestions

**oh-my-zsh equivalent:** `zsh-autosuggestions`  
**Crate:** `crates/feature_suggest`  
**Status:** 🔧 Partial

### How it works now
`Suggester::suggest()` searches history in reverse order for an entry that **starts with** the current input. The matched suffix is shown as a ghost text after the cursor. Tab accepts the suggestion.

### What needs improvement
- Only matches from the start of line; `fish` / zsh-autosuggestions also score by recency and frequency
- No fuzzy matching within suggestions
- No multi-word partial matching (e.g. typing `git co` should suggest `git commit` if that's recent)

### Roadmap
1. Weight suggestions by recency (most-recently-used first — already done via reverse iteration)
2. Add prefix-substring match: `gc` should match `git commit` if the user has typed it before
3. Cap suggestion length to avoid visual noise on long commands

---

## Syntax Highlighting

**oh-my-zsh equivalent:** `zsh-syntax-highlighting`  
**Crate:** `crates/feature_highlight`  
**Status:** 🔧 Partial (works, but uses ratatui Color types)

### How it works now
The input field has a custom `layouter` that calls `highlight::highlight(input)`. This tokenises the shell input into:
- Keywords (`if`, `for`, `while`, …) — yellow bold
- Builtins (`echo`, `cd`, `ls`, …) — cyan
- Commands — green if found on PATH, red if not
- Flags (`-v`, `--verbose`) — blue
- String literals — light green
- Variables (`$VAR`, `${VAR}`) — magenta
- Pipes and redirects — yellow bold
- Arguments — white

### What needs improvement
- Uses `ratatui::style::Color` — remove this dep, use `egui::Color32` or a local enum
- Command existence check (`which::which`) is synchronous, runs on the UI thread on every keystroke — should be cached
- No highlighting of `~`, globs (`*`, `?`), or process substitutions (`$(...)`)
- Comments (`# ...`) shown as plain `Unknown` token

### Roadmap
1. Remove ratatui dependency — define `HighlightColor` enum local to the crate
2. Cache `command_exists` results in a `HashMap<String, bool>` per session
3. Add glob and tilde token types
4. Add comment styling (dimmed grey)

---

## Directory Jump (z)

**oh-my-zsh equivalent:** `z` plugin (also `autojump`, `zoxide`)  
**Crate:** `crates/feature_jump`  
**Status:** 🔧 Partial (works, has a bug)

### How it works now
`JumpDb` stores directory entries as `HashMap<String, Entry { rank: f64, last_visit: u64 }`. The `add()` method increments rank and applies an exponential decay based on time since last visit (half-life 24 hours). `query()` finds the highest-scoring path whose components contain all query words.

The `z <query>` command in the input is intercepted in `submit()` and replaced with `cd '<path>'` before sending to the shell.

### Bug to fix
```rust
// Current (wrong):
JumpDb::load().unwrap_or_else(|_| JumpDb::load().unwrap())

// Fixed:
JumpDb::load().unwrap_or_default()
// + impl Default for JumpDb { ... }
```

### Roadmap
1. Fix the retry-on-failure bug
2. Surface top 5 jump targets in the fuzzy palette (Ctrl+R)
3. Add `z -l` (list) command support
4. Store per-session path for the terminal to display in status bar

---

## Git Status

**oh-my-zsh equivalent:** `git` plugin (prompt theme integration)  
**Crate:** `crates/feature_git`  
**Status:** 🔧 Partial (branch/dirty/ahead/behind; no stash/conflicts)

### How it works now
`get_info()` runs three git subprocesses:
1. `git rev-parse --show-toplevel` — detect if inside a repo
2. `git symbolic-ref --short HEAD` — current branch name (or short hash if detached)
3. `git status --porcelain` — dirty check
4. `git rev-list --left-right --count HEAD...@{upstream}` — ahead/behind

Status bar shows: `branch *` (dirty), `↑N` (ahead), `↓N` (behind).

### What needs improvement
- Subprocess calls are synchronous and run on the UI thread every 5 seconds (periodic timer in `update()`) — no real-time accuracy
- No stash count display
- No merge-conflict detection
- No untracked file count distinct from dirty

### Roadmap
1. Move git subprocess calls to a background thread with `std::thread::spawn` + mpsc
2. Add stash count: `git stash list --format="%gd" | wc -l`
3. Show `!M` (modified), `+N` (staged), `?N` (untracked) counts like Powerlevel10k
4. Detect merge/rebase in progress (presence of `.git/MERGE_HEAD`, `.git/rebase-merge/`)

---

## Fuzzy Command Palette

**oh-my-zsh equivalent:** `fzf` plugin  
**Crate:** `crates/feature_fzf`  
**Status:** 🔧 Partial (history-only fuzzy search via Ctrl+R)

### How it works now
Ctrl+R opens a floating window with a fuzzy search over command history using `SkimMatcherV2`.

### Roadmap
1. **Phase 2:** Add tab-triggered file path completion (list CWD files, fuzzy filter)
2. **Phase 3:** Add environment variable completion (`$` trigger)
3. **Phase 4:** Add git branch switching via palette
4. **Phase 5:** Make the palette a plugin insertion point so any feature can add items

---

## Tabs

**oh-my-zsh equivalent:** N/A — multi-session management  
**Crate:** `crates/myterm_ui` (tabs.rs, pane.rs)  
**Status:** ⬜ Not started (Phase 3)

### Design
Each tab is a `TerminalPane` — a fully independent shell process with its own PTY, VTE state, and block history. The tab bar lives in a thin top panel above the main content area.

| Keys | Action |
|------|--------|
| Ctrl+T | New tab |
| Ctrl+W | Close active pane |
| Ctrl+Tab | Next pane |
| Ctrl+Shift+Tab | Previous pane |

Tab title shows the last component of the current working directory (e.g. `~`, `src`, `myterm`).

---

## Split View

**oh-my-zsh equivalent:** N/A — terminal multiplexer (like tmux splits)  
**Crate:** `crates/myterm_ui` (layout.rs, tabs.rs)  
**Status:** ⬜ Not started (Phase 3)

### Design
Drag a tab from the tab bar and drop it onto the active terminal area. The drop zone determines the split:

| Drop zone | Result |
|-----------|--------|
| Left 25% | Horizontal split — dragged tab goes left |
| Right 25% | Horizontal split — dragged tab goes right |
| Top 25% | Vertical split — dragged tab goes above |
| Bottom 25% | Vertical split — dragged tab goes below |
| Center | Reorder in tab bar (no split) |

While dragging, the 4 zones are shown as semi-transparent blue overlays. A thin draggable divider between panes lets you resize the ratio.

Splits are represented as a binary tree (`PaneLayout`). A 3-pane layout looks like:
```
Split(Horizontal, [Leaf(A), Split(Vertical, [Leaf(B), Leaf(C)])])
```

See `docs/TABS_SPLITS.md` for the full data model and rendering design.

---

## Predict Bar

**oh-my-zsh equivalent:** Closest to `zsh-autocomplete` / fzf-tab, but mobile-style  
**Crate:** `crates/feature_predict`  
**UI:** `crates/myterm_ui/src/predict_bar.rs`  
**Status:** ⬜ Not started (Phase 3)

### Design
A horizontal strip of chip buttons sitting just above the input, like the autocomplete row on a phone keyboard. Updates on every keystroke.

| Context | Chips shown |
|---------|-------------|
| `"gi"` | `git · gimp · gist · ...` — PATH executables matching prefix |
| `"git "` | `add · commit · push · pull · status · log · ...` — subcommands from signature DB |
| `"git co"` | `commit · config` — filtered subcommands |
| `"cargo "` | `build · run · test · check · clippy · fmt · ...` |
| `"git commit -"` | `--message · --amend · --no-edit · ...` — flags |

**Hover tooltip** — hovering a chip for 300ms shows:
```
Record changes to the repository
git commit [-m <msg>] [--amend] [--no-edit]
```

**Click** — appends/replaces the partial token in the input and adds a trailing space.

**Signature DB** — `crates/feature_predict/data/signatures.toml` covers git, cargo, docker, npm, make, ssh, curl, tar, systemctl. New commands are added by editing the TOML — no code change needed.

**Fallback** — for unknown commands, runs `<cmd> --help` in a background thread and parses the output.

See `docs/PREDICT_BAR.md` for the full data pipeline and internals.

---

## Block Copy Actions

**oh-my-zsh equivalent:** N/A — this is a Warp-style UX feature  
**Crate:** `crates/myterm_ui` (blocks.rs)  
**Status:** ⬜ Not started (Phase 2)

### Design
When the mouse hovers over a command block, two small buttons appear in the top-right corner of the block frame:

- **⎘ cmd** — copies the command string to the clipboard
- **⎘ out** — copies ANSI-stripped plain text of all output lines joined with `\n`

Implementation uses egui's built-in `ctx.output_mut(|o| o.copied_text = ...)` which calls `arboard` (already a transitive dep via eframe) under the hood — no new dependency.

A brief visual flash on the button (colour inversion for ~300ms via `Instant`) confirms the copy.

### Keyboard shortcuts (Phase 3)
| Keys | Action |
|------|--------|
| Ctrl+Shift+C | Copy output of last block |
| Ctrl+Shift+X | Copy command of last block |

---

## Colour Schemes

**oh-my-zsh equivalent:** N/A — terminal theme selection  
**Crate:** `crates/myterm_settings` (themes.rs)  
**Status:** ⬜ Not started (Phase 2)

### Preset palette values

| Scheme | bg | text | accent |
|--------|-----|------|--------|
| `dark` (default) | `#12121c` | `#dcdcdc` | `#4ec994` green |
| `light` | `#f5f5f5` | `#1e1e1e` | `#0a7a55` green |
| `solarized` | `#002b36` | `#839496` | `#268bd2` blue |
| `dracula` | `#282a36` | `#f8f8f2` | `#bd93f9` purple |
| `nord` | `#2e3440` | `#d8dee9` | `#88c0d0` frost |
| `one_dark` | `#282c34` | `#abb2bf` | `#98c379` green |
| `custom` | from `[theme]` | from `[theme]` | from `[theme]` |

### Theme switcher UI (Phase 3)
A settings panel (Ctrl+,) shows a theme picker grid. Selecting a scheme saves `color_scheme = "..."` to `settings.toml` and hot-reloads the colours without restart. Preview on hover.

---

## Background Opacity

**oh-my-zsh equivalent:** N/A — window transparency  
**Crate:** `app/` + `crates/myterm_ui` (theme.rs)  
**Status:** ⬜ Not started (Phase 2)

### How it works
`eframe::NativeOptions` sets `viewport.with_transparent(true)`. All background fill colours have their alpha channel set based on `appearance.background_opacity`:

```rust
fn bg_with_opacity(rgb: [u8; 3], opacity_pct: u8) -> Color32 {
    let alpha = ((opacity_pct as u32 * 255) / 100) as u8;
    Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], alpha)
}
```

### Platform notes
| Platform | Transparency support |
|----------|---------------------|
| Wayland (KWin, Mutter) | ✅ works natively |
| X11 + picom/compton | ✅ compositor required |
| X11 no compositor | ❌ falls back to opaque |
| macOS | ✅ works natively |
| Windows 10+ | ✅ DWM |

Minimum sensible opacity: ~20% (below that, the terminal becomes unreadable). The UI should clamp the slider to 20–100.

---

## Archive Extraction

**oh-my-zsh equivalent:** `extract` plugin  
**Crate:** `crates/feature_extract`  
**Status:** ⬜ Not started (Phase 4)

### Design
Intercept commands that match `extract <file>` or detect common archive invocations. Map to the correct extraction tool: `tar xf`, `unzip`, `7z x`, etc. based on file extension. Show a progress block during extraction.

---

## Web Search

**oh-my-zsh equivalent:** `web-search` plugin  
**Crate:** `crates/feature_websearch`  
**Status:** ⬜ Not started (Phase 4)

### Design
Intercept commands like `google <query>`, `gh <query>`, `ddg <query>` and open the browser via `xdg-open`. Config maps shorthand → URL template.

---

## Coloured Man Pages

**oh-my-zsh equivalent:** `colored-man-pages` plugin  
**Crate:** `crates/feature_man`  
**Status:** ⬜ Not started (Phase 4)

### Design
Intercept `man <topic>` commands. Instead of passing to the shell, capture `man --no-hyphenation -P cat <topic>` output, parse the groff/troff formatting codes, and render inline as a scrollable `CommandBlock` with syntax-style colouring.

---

## Command Not Found

**oh-my-zsh equivalent:** `command-not-found` plugin  
**Crate:** `crates/feature_cnf`  
**Status:** ⬜ Not started (Phase 4)

### Design
Hook into `on_block_complete`. If `exit_code == 127` (command not found), query the system package database (`apt-cache search`, `pacman -Ss`, etc.) for the command name. Append a styled suggestion line: `→ Install with: sudo apt install <package>`.
