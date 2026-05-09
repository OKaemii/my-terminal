# Plugin API

The `Feature` trait is the extension point for adding new capabilities to myterm. Each oh-my-zsh plugin equivalent is implemented as a crate that provides a type implementing `Feature`.

## Stability note

The `Feature` trait is **internal** — it is not published as a crate and breaking changes are expected until Phase 5 is complete. Do not treat it as a stable public API.

## The trait

```rust
// crates/myterm_ui/src/plugin.rs

use myterm_core::CommandBlock;
use std::path::Path;

/// A segment shown in the status bar by a feature.
pub struct StatusSegment {
    pub text: String,
    pub color: egui::Color32,
}

/// The extension point for terminal features.
/// Implement this trait in a `feature_*` crate and register an instance in `app/main.rs`.
pub trait Feature: Send + Sync {
    /// Short identifier, used in logging and debug output.
    fn name(&self) -> &'static str;

    /// Called before a command is sent to the shell.
    ///
    /// Return `Some(replacement)` to substitute the command (e.g. jump translates
    /// `z foo` to `cd '/real/path'`). Return `None` to leave the command unchanged.
    fn on_command_submit(&self, cmd: &str, cwd: &Path) -> Option<String> {
        let _ = (cmd, cwd);
        None
    }

    /// Called immediately after a command completes (prompt is ready).
    ///
    /// Return one or more `StatusSegment`s to display in the status bar.
    /// Return an empty vec to contribute nothing.
    fn status_segments(&self, cwd: &Path) -> Vec<StatusSegment> {
        let _ = cwd;
        vec![]
    }

    /// Called when a command block is fully complete.
    ///
    /// Use for post-processing: appending suggestions, detecting errors, etc.
    /// Return `Some(text)` to append an annotation line to the block's output.
    fn on_block_complete(&self, block: &CommandBlock) -> Option<String> {
        let _ = block;
        None
    }
}
```

## Default implementations

All methods have default no-op implementations. A feature only needs to override the hooks it actually uses.

## Registration

Features are registered in `app/src/main.rs`:

```rust
// app/src/main.rs

fn build_features(settings: &Settings) -> Vec<Box<dyn Feature>> {
    let mut features: Vec<Box<dyn Feature>> = Vec::new();

    if settings.features.git_status {
        features.push(Box::new(feature_git::GitFeature::new()));
    }
    if settings.features.autosuggestions {
        features.push(Box::new(feature_suggest::SuggestFeature::new()));
    }
    if settings.features.jump {
        features.push(Box::new(feature_jump::JumpFeature::new()));
    }
    // ... etc

    features
}
```

The `Vec<Box<dyn Feature>>` is owned by `TerminalApp`. Hooks are called in registration order.

## Lifecycle call points

| Hook | Where called in UI | Typical use |
|------|--------------------|-------------|
| `on_command_submit` | `TerminalApp::submit()` before writing to PTY | command rewriting (jump, web-search) |
| `status_segments` | `TerminalApp::on_prompt_ready()` + render cycle | git branch, session info |
| `on_block_complete` | `TerminalApp::on_prompt_ready()` after block finalized | command-not-found hints, timing annotations |

## Example feature crate

```rust
// crates/feature_jump/src/lib.rs

use myterm_core::CommandBlock;
use myterm_ui::plugin::{Feature, StatusSegment};
use myterm_features::FeatureFlag;
use std::path::Path;

pub struct JumpFeature {
    db: std::sync::Mutex<JumpDb>,
}

impl JumpFeature {
    pub fn new() -> Self {
        Self {
            db: std::sync::Mutex::new(JumpDb::load().unwrap_or_default()),
        }
    }
}

impl Feature for JumpFeature {
    fn name(&self) -> &'static str { "jump" }

    fn on_command_submit(&self, cmd: &str, cwd: &Path) -> Option<String> {
        if !FeatureFlag::FrecencyJump.is_enabled() { return None; }

        let query = cmd.strip_prefix("z ")?.trim();
        let mut db = self.db.lock().ok()?;
        let target = db.query(query)?;
        Some(format!("cd '{}'", target.display()))
    }

    fn on_block_complete(&self, block: &CommandBlock) -> Option<String> {
        if !FeatureFlag::FrecencyJump.is_enabled() { return None; }
        let mut db = self.db.lock().ok()?;
        db.add(&block.cwd);
        None
    }
}
```

## Thread safety

`Feature` requires `Send + Sync`. Use `Mutex<T>` for interior mutability (as above). `TerminalApp` runs on a single egui thread, so lock contention is not a concern — but the bounds are required for `Box<dyn Feature>` to be usable.

## Future: user-loadable plugins

Phase 5 could support dynamically loaded plugins via `libloading` — a `.so` / `.dylib` per plugin with a known exported symbol `fn create_feature() -> Box<dyn Feature>`. This would allow community plugins without recompiling myterm. This is out of scope for the current plan but the `Feature` trait is designed to be ABI-stable if this direction is taken.
