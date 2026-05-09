# Warp Patterns

Patterns extracted from `/home/okamii/Documents/projects/warp/` that myterm adopts, with the simplifications appropriate for a personal project.

Reference project: warp (open source terminal, AGPL-3.0 / MIT)
Reference path: `/home/okamii/Documents/projects/warp/`

---

## Feature Flags

### Warp's approach
- `FeatureFlag` is a `#[derive(Sequence)]` enum (from the `enum_iterator` crate)
- A static array of `AtomicBool` sized by `cardinality::<FeatureFlag>()` stores runtime state
- A second array of `AtomicTriState` stores per-user preference overrides
- Flags are tiered: `DOGFOOD_FLAGS` (team only), `PREVIEW_FLAGS` (beta users), `RELEASE_FLAGS` (everyone)
- `is_enabled()` checks user-preference override first, then global flag state

### myterm adaptation
myterm omits cloud sync and user preference tiers — a single `AtomicBool` per flag is sufficient.

```rust
// crates/myterm_features/src/lib.rs
use std::sync::atomic::{AtomicBool, Ordering};
use enum_iterator::Sequence;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Sequence)]
pub enum FeatureFlag {
    GitStatus,
    Autosuggestions,
    SyntaxHighlight,
    FrecencyJump,
    FzfFiles,
    FzfEnvVars,
    ExtractArchive,
    WebSearch,
    ColoredManPages,
    CommandNotFound,
}

static FLAG_STATES: [AtomicBool; enum_iterator::cardinality::<FeatureFlag>()] =
    [const { AtomicBool::new(false) }; enum_iterator::cardinality::<FeatureFlag>()];

impl FeatureFlag {
    pub fn is_enabled(self) -> bool {
        FLAG_STATES[self as usize].load(Ordering::Relaxed)
    }

    pub fn set(self, enabled: bool) {
        FLAG_STATES[self as usize].store(enabled, Ordering::Relaxed);
    }
}
```

Use in code: `if FeatureFlag::GitStatus.is_enabled() { ... }`

### Exhaustive matching on FeatureFlag
When you write a match on `FeatureFlag`, cover every variant — do not use `_`. Adding a new variant then causes a compile error at every match site, forcing you to decide consciously whether the new feature needs handling.

---

## Settings System

### Warp's approach
- `define_settings_group!` macro generates a singleton model with typed getters/setters
- Each setting has: `toml_path`, `default`, `supported_platforms`, `sync_to_cloud`, `private`
- `SettingsManager` holds `HashMap<storage_key, update_fn / clear_fn / load_fn>`
- JSON Schema generated from `inventory::submit!` calls for editor autocomplete
- Hot-reload via file watcher + `load_fn` (avoids write-back loops)

### myterm adaptation
The full macro + inventory + cloud-sync machinery is too heavy. myterm uses a plain struct:

```rust
// crates/myterm_settings/src/lib.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    pub font: FontSettings,
    pub features: FeatureToggles,
}
```

Loading: `toml::from_str(&fs::read_to_string(path)?)` with `unwrap_or_default()` on parse error.
Saving: `toml::to_string_pretty(&self)` written to `~/.config/myterm/settings.toml`.

TOML example:
```toml
[theme]
bg = [18, 18, 28]
text = [220, 220, 220]
green = [78, 201, 148]

[font]
size = 14.0
family = "monospace"

[features]
git_status = true
autosuggestions = true
jump = true
fzf_files = false
```

---

## Testing Convention

### Warp's approach
Test files are separate from source files, included via:
```rust
#[cfg(test)]
#[path = "filename_tests.rs"]
mod tests;
```

Tests for `foo.rs` go in `foo_tests.rs` in the same directory.
Tests for `mod.rs` go in `mod_test.rs`.

### myterm adoption
Same convention, applied to every crate. Tests live next to the code they test, not in a `tests/` subdirectory (those are reserved for integration tests).

Example for `crates/myterm_vte/src/lib.rs`:
```rust
// at the bottom of lib.rs:
#[cfg(test)]
#[path = "vte_tests.rs"]
mod tests;
```

```rust
// crates/myterm_vte/src/vte_tests.rs
use super::*;

#[test]
fn sgr_reset_clears_color() {
    let mut proc = OutputProcessor::new();
    // set red, then reset
    proc.perform_sgr(&[31, 0]);
    assert_eq!(proc.fg, myterm_core::palette::TEXT);
}
```

---

## Coding Style

Direct adoptions from Warp's guidelines, applied to myterm:

### Imports
Place all `use` statements at the top of the file. Exception: imports inside `#[cfg(...)]` blocks may be local.

```rust
// ✅ correct
use std::path::PathBuf;
use egui::Color32;

// ❌ wrong
fn foo() {
    use std::path::PathBuf;
}
```

### Format args
Use inline format args:
```rust
// ✅
eprintln!("{err}");
format!("{name}: {value}");

// ❌
eprintln!("{}", err);
format!("{}: {}", name, value);
```

### Exhaustive match
Never use `_` in a match on a project-internal enum:
```rust
// ✅
match flag {
    FeatureFlag::GitStatus => { ... }
    FeatureFlag::Autosuggestions => { ... }
    // compiler error here if a new variant is added ← desired
}

// ❌
match flag {
    FeatureFlag::GitStatus => { ... }
    _ => {}
}
```

### Context / App parameters
If a function takes application context, name it `ctx` and put it last (matching Warp's convention for `AppContext`, adapted to `&mut eframe::Frame` or similar):
```rust
// ✅
fn render_status(app: &TerminalApp, ctx: &egui::Context) { ... }

// ❌
fn render_status(ctx: &egui::Context, app: &TerminalApp) { ... }
```

### Unused parameters
Remove them completely. Do not prefix with `_`. Update all call sites.

### Type annotations
Avoid unnecessary annotations in closures:
```rust
// ✅
let v: Vec<_> = items.iter().map(|x| x.score()).collect();

// ❌
let v: Vec<i64> = items.iter().map(|x: &Item| -> i64 { x.score() }).collect();
```

---

## Presubmit Checks

Before committing anything, run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All three must pass. Add a `script/presubmit` shell script that runs these in order.

---

## Workspace Dependencies

Declare shared deps in the workspace `Cargo.toml` to ensure a single version:

```toml
[workspace.dependencies]
egui = "0.29"
eframe = "0.29"
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
directories = "5"
portable-pty = "0.8"
vte = "0.13"
enum-iterator = "2"
fuzzy-matcher = "0.3"
which = "7"
```

Each crate then references: `egui = { workspace = true }` — no version pinning needed per-crate.
