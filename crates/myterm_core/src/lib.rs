use egui::Color32;

/// A single styled character from VTE output.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledChar {
    pub ch: char,
    pub color: Color32,
    pub bold: bool,
}

impl Default for StyledChar {
    fn default() -> Self {
        Self {
            ch: ' ',
            color: palette::TEXT,
            bold: false,
        }
    }
}

/// A completed or in-progress command block.
#[derive(Debug)]
pub struct CommandBlock {
    pub command: String,
    pub output: Vec<Vec<StyledChar>>,
    pub exit_code: i32,
    pub cwd: std::path::PathBuf,
    pub duration_ms: u64,
    pub is_running: bool,
    pub is_fullscreen: bool,
}

/// Canonical colour palette — single source of truth for all crates.
/// In Phase 2 this becomes a fallback default; the settings system
/// overlays theme values at runtime.
pub mod palette {
    use egui::Color32;

    pub const BG: Color32 = Color32::from_rgb(18, 18, 28);
    pub const BLOCK_BG: Color32 = Color32::from_rgb(24, 24, 38);
    pub const BLOCK_BORDER: Color32 = Color32::from_rgb(55, 55, 85);
    pub const INPUT_BG: Color32 = Color32::from_rgb(22, 22, 36);
    pub const STATUS_BG: Color32 = Color32::from_rgb(14, 14, 24);
    pub const TEXT: Color32 = Color32::from_rgb(220, 220, 220);
    pub const DIM: Color32 = Color32::from_rgb(100, 100, 130);
    pub const GREEN: Color32 = Color32::from_rgb(78, 201, 148);
    pub const RED: Color32 = Color32::from_rgb(255, 107, 107);
    pub const YELLOW: Color32 = Color32::from_rgb(220, 220, 170);
    pub const CYAN: Color32 = Color32::from_rgb(78, 201, 176);
    pub const BLUE: Color32 = Color32::from_rgb(86, 156, 214);
    pub const ORANGE: Color32 = Color32::from_rgb(206, 145, 120);
    pub const MAGENTA: Color32 = Color32::from_rgb(197, 134, 192);
}

#[cfg(test)]
#[path = "core_tests.rs"]
mod tests;
