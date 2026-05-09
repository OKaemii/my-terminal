use egui::Color32;
use myterm_core::palette;
use myterm_settings::{Settings, Theme};

/// Flat `Color32` palette resolved from settings for a single frame.
pub struct ResolvedPalette {
    pub bg: Color32,
    pub block_bg: Color32,
    pub block_border: Color32,
    pub input_bg: Color32,
    pub status_bg: Color32,
    pub text: Color32,
    pub dim: Color32,
    pub green: Color32,
    pub red: Color32,
    pub yellow: Color32,
    pub cyan: Color32,
    pub blue: Color32,
    pub orange: Color32,
    pub magenta: Color32,
}

impl Default for ResolvedPalette {
    fn default() -> Self {
        Self::from_settings_theme(&Theme::dark(), 100)
    }
}

impl ResolvedPalette {
    pub fn from_settings(settings: &Settings) -> Self {
        let theme = settings.appearance.color_scheme.resolve(&settings.appearance.custom_theme);
        let opacity = settings.appearance.background_opacity;
        Self::from_settings_theme(&theme, opacity)
    }

    fn from_settings_theme(theme: &Theme, opacity: u8) -> Self {
        let bg_op = |rgb: [u8; 3]| bg_with_opacity(rgb, opacity);
        Self {
            bg: bg_op(theme.bg),
            block_bg: bg_op(theme.block_bg),
            block_border: bg_op(theme.block_border),
            input_bg: bg_op(theme.input_bg),
            status_bg: bg_op(theme.status_bg),
            text: rgb(theme.text),
            dim: rgb(theme.dim),
            green: rgb(theme.green),
            red: rgb(theme.red),
            yellow: rgb(theme.yellow),
            cyan: rgb(theme.cyan),
            blue: rgb(theme.blue),
            orange: rgb(theme.orange),
            magenta: rgb(theme.magenta),
        }
    }

    /// Fall back to the compile-time palette constants.
    pub fn fallback() -> Self {
        Self {
            bg: palette::BG,
            block_bg: palette::BLOCK_BG,
            block_border: palette::BLOCK_BORDER,
            input_bg: palette::INPUT_BG,
            status_bg: palette::STATUS_BG,
            text: palette::TEXT,
            dim: palette::DIM,
            green: palette::GREEN,
            red: palette::RED,
            yellow: palette::YELLOW,
            cyan: palette::CYAN,
            blue: palette::BLUE,
            orange: palette::ORANGE,
            magenta: palette::MAGENTA,
        }
    }
}

/// Apply background opacity to an RGB colour.
pub fn bg_with_opacity(rgb: [u8; 3], opacity_pct: u8) -> Color32 {
    let alpha = ((opacity_pct as u32 * 255) / 100) as u8;
    Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], alpha)
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Configure egui visuals from the resolved palette.
pub fn apply_visuals(palette: &ResolvedPalette, ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = palette.bg;
    visuals.window_fill = palette.bg;
    visuals.override_text_color = Some(palette.text);
    ctx.set_visuals(visuals);
}

/// Build the AppearanceSettings reference used by eframe for window transparency.
pub fn native_options_transparent() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    }
}
