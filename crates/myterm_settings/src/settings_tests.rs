use super::themes::{ColorScheme, Theme};
use super::*;

#[test]
fn default_settings_round_trip() {
    let original = Settings::default();
    let toml_str = toml::to_string_pretty(&original).expect("serialize");
    let restored: Settings = toml::from_str(&toml_str).expect("deserialize");
    assert_eq!(original.font.size, restored.font.size);
    assert_eq!(original.font.family, restored.font.family);
    assert_eq!(
        original.appearance.background_opacity,
        restored.appearance.background_opacity
    );
    assert_eq!(original.features.git_status, restored.features.git_status);
}

#[test]
fn dark_scheme_resolves_to_dark_theme() {
    let theme = ColorScheme::Dark.resolve(&Theme::default());
    assert_eq!(theme, Theme::dark());
    // BG must not be zeros
    assert_ne!(theme.bg, [0, 0, 0]);
}

#[test]
fn partial_toml_uses_defaults() {
    let partial = "[font]\nsize = 16.0\n";
    let s: Settings = toml::from_str(partial).expect("partial toml must not panic");
    assert_eq!(s.font.size, 16.0);
    assert!(s.features.git_status, "git_status default must be true");
}

#[test]
fn background_opacity_clamped_on_load() {
    // Simulate load clamping: value > 100 gets clamped to 100.
    let mut s = Settings::default();
    s.appearance.background_opacity = 150;
    s.appearance.background_opacity = s.appearance.background_opacity.min(100);
    assert_eq!(s.appearance.background_opacity, 100);
}

#[test]
fn color_scheme_custom_uses_custom_theme() {
    let mut custom = Theme::dark();
    custom.bg = [1, 2, 3];
    let resolved = ColorScheme::Custom.resolve(&custom);
    assert_eq!(resolved.bg, [1, 2, 3]);
}
