use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod themes;

pub use themes::{ColorScheme, Theme};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub appearance: AppearanceSettings,
    pub font: FontSettings,
    pub features: FeatureToggles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    pub color_scheme: ColorScheme,
    /// Background opacity, 0 (transparent) to 100 (opaque).
    pub background_opacity: u8,
    pub custom_theme: Theme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontSettings {
    pub size: f32,
    pub family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FeatureToggles {
    pub git_status: bool,
    pub autosuggestions: bool,
    pub syntax_highlight: bool,
    pub jump: bool,
    pub predict_bar: bool,
    pub fzf_files: bool,
    pub command_not_found: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance: AppearanceSettings::default(),
            font: FontSettings::default(),
            features: FeatureToggles::default(),
        }
    }
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::default(),
            background_opacity: 100,
            custom_theme: Theme::dark(),
        }
    }
}

impl Default for FontSettings {
    fn default() -> Self {
        Self { size: 14.0, family: "monospace".to_string() }
    }
}

impl Default for FeatureToggles {
    fn default() -> Self {
        Self {
            git_status: true,
            autosuggestions: true,
            syntax_highlight: true,
            jump: true,
            predict_bar: true,
            fzf_files: false,
            command_not_found: false,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("myterm: settings parse error ({e}), using defaults");
                Self::default()
            }
        }
    }

    fn try_load() -> Result<Self> {
        let path = settings_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)?;
        let mut s: Self = toml::from_str(&text)?;
        s.appearance.background_opacity = s.appearance.background_opacity.min(100);
        Ok(s)
    }

    pub fn save(&self) -> Result<()> {
        let path = settings_path()?;
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }
}

fn settings_path() -> Result<PathBuf> {
    directories::ProjectDirs::from("", "", "myterm")
        .map(|d| d.config_dir().join("settings.toml"))
        .ok_or_else(|| anyhow::anyhow!("could not determine config directory"))
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
