use myterm_core::CommandBlock;

/// A short status bar segment emitted by a feature.
pub struct StatusSegment {
    pub label: String,
    pub accent: bool,
}

/// Implemented by every feature crate that participates in the terminal lifecycle.
/// Registered at startup in `app/src/main.rs`.
pub trait Feature: Send + Sync {
    fn name(&self) -> &'static str;

    /// Called after the user submits a command.
    /// May return a replacement command string (e.g. jump rewrites `z foo` → `cd /real/path`).
    fn on_command_submit(&self, cmd: &str) -> Option<String> {
        let _ = cmd;
        None
    }

    /// Called when a prompt is ready (command completed).
    /// May return a short segment for the status bar.
    fn status_segment(&self, cwd: &std::path::Path) -> Option<StatusSegment> {
        let _ = cwd;
        None
    }

    /// Called with every completed block for post-processing (e.g. history recording).
    fn on_block_complete(&self, block: &CommandBlock) {
        let _ = block;
    }
}
