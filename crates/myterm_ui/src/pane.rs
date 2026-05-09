use anyhow::Result;
use myterm_core::CommandBlock;
use myterm_pty::PtySession;
use myterm_vte::OutputProcessor;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;
use vte::Parser as VteParser;

static NEXT_PANE_ID: AtomicU32 = AtomicU32::new(1);

/// Unique identifier for a terminal pane.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PaneId(pub(crate) u32);

impl PaneId {
    pub fn next() -> Self {
        Self(NEXT_PANE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// One independent terminal session — owns its PTY, VTE state, and block history.
pub struct TerminalPane {
    pub id: PaneId,
    pub title: String,
    pub pty: PtySession,
    pub startup_done: bool,
    pub partial_line: Vec<u8>,
    pub block_start: Option<Instant>,
    pub vte_parser: VteParser,
    pub vte_proc: OutputProcessor,
    pub blocks: Vec<CommandBlock>,
    pub input: String,
    pub suggestion: Option<String>,
    pub cwd: std::path::PathBuf,
    pub scroll_to_bottom: bool,
    pub history_nav: Option<usize>,
    pub history_draft: String,
}

impl TerminalPane {
    pub fn spawn(shell: &str) -> Result<Self> {
        let pty = PtySession::spawn(40, 200)?;
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
        let title = cwd
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("~")
            .to_string();
        let _ = shell; // shell is already determined by PtySession via $SHELL
        Ok(Self {
            id: PaneId::next(),
            title,
            pty,
            startup_done: false,
            partial_line: Vec::new(),
            block_start: None,
            vte_parser: VteParser::new(),
            vte_proc: OutputProcessor::new(),
            blocks: Vec::new(),
            input: String::new(),
            suggestion: None,
            cwd,
            scroll_to_bottom: false,
            history_nav: None,
            history_draft: String::new(),
        })
    }

    /// Update the tab title to reflect the current working directory.
    pub fn refresh_title(&mut self) {
        self.title = self
            .cwd
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("~")
            .to_string();
    }
}
