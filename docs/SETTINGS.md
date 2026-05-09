# Settings

Configuration system for myterm.

## File location

`~/.config/myterm/settings.toml`  
(resolved via `directories::ProjectDirs::from("", "", "myterm").config_dir()`)

## Loading strategy

1. On startup, attempt to read and deserialise `settings.toml`
2. If the file does not exist: use `Settings::default()` (all hardcoded defaults)
3. If the file exists but fails to parse: log a warning, use `Settings::default()` — **never panic**
4. Pass `Arc<RwLock<Settings>>` to `TerminalApp` and all feature crates that need it

There is no hot-reload in Phase 1–2. A future Phase could use the `notify` crate to watch the file for changes and re-load without restarting.

## Schema

```toml
# ~/.config/myterm/settings.toml
# All fields are optional — missing fields use their defaults.

[appearance]
# Named colour preset. Options: dark | light | solarized | dracula | nord | one_dark | custom
# When set to anything except "custom", the [theme] section below is ignored.
color_scheme = "dark"

# Background transparency, 0 (invisible) to 100 (fully opaque). Default: 100.
# Requires a compositor that supports window transparency (Wayland/KWin, X11+picom, macOS, Windows).
background_opacity = 100

[theme]
# RGB colour values as [R, G, B] arrays (0-255 each)
bg           = [18, 18, 28]     # main background
block_bg     = [24, 24, 38]     # command block background
block_border = [55, 55, 85]     # command block border
input_bg     = [22, 22, 36]     # input panel background
status_bg    = [14, 14, 24]     # status bar background
text         = [220, 220, 220]  # default text
dim          = [100, 100, 130]  # secondary/dimmed text
green        = [78, 201, 148]   # success, git clean
red          = [255, 107, 107]  # error, git dirty
yellow       = [220, 220, 170]  # warning, keywords
cyan         = [78, 201, 176]   # paths, builtins
blue         = [86, 156, 214]   # flags, info
orange       = [206, 145, 120]  # strings
magenta      = [197, 134, 192]  # variables

[font]
size   = 14.0        # font size in points
family = "monospace" # font family name (must be installed)

[features]
git_status       = true   # show git branch/status in status bar
autosuggestions  = true   # show inline command suggestions
jump             = true   # enable 'z <query>' frecency jump
syntax_highlight = true   # highlight input syntax (always recommended)
fzf_files        = false  # Tab to fuzzy-complete file paths (Phase 4)
fzf_env_vars     = false  # $-triggered env var completion (Phase 4)
extract_archive  = false  # 'extract <file>' auto-detect (Phase 4)
web_search       = false  # 'google <query>' browser shortcut (Phase 4)
colored_man      = false  # render man pages inline with colour (Phase 4)
command_not_found = false # suggest apt/pacman install for 127 exits (Phase 4)
```

## Preset colour schemes

| Name | Description |
|------|-------------|
| `dark` | Default — deep blue-black bg, muted purple accents |
| `light` | White background, dark text for bright environments |
| `solarized` | Ethan Schoonover's Solarized Dark |
| `dracula` | Dracula theme (purple bg, pink/cyan accents) |
| `nord` | Arctic, north-bluish colour palette |
| `one_dark` | Atom One Dark (slate bg, warm accents) |
| `custom` | Use the `[theme]` section below for full control |

## Rust types

```rust
// crates/myterm_settings/src/lib.rs

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub appearance: AppearanceSettings,
    pub font: FontSettings,
    pub features: FeatureToggles,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    pub color_scheme:       ColorScheme,   // default: Dark
    pub background_opacity: u8,            // 0–100, default: 100
    pub custom_theme:       Theme,         // only used when color_scheme == Custom
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ColorScheme {
    #[default] Dark,
    Light, Solarized, Dracula, Nord, OneDark, Custom,
}

impl ColorScheme {
    pub fn theme(&self, custom: &Theme) -> Theme {
        match self {
            ColorScheme::Dark      => Theme::default(),
            ColorScheme::Light     => Theme::light(),
            ColorScheme::Solarized => Theme::solarized(),
            ColorScheme::Dracula   => Theme::dracula(),
            ColorScheme::Nord      => Theme::nord(),
            ColorScheme::OneDark   => Theme::one_dark(),
            ColorScheme::Custom    => custom.clone(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Theme {
    pub bg:           [u8; 3],
    pub block_bg:     [u8; 3],
    pub block_border: [u8; 3],
    pub input_bg:     [u8; 3],
    pub status_bg:    [u8; 3],
    pub text:         [u8; 3],
    pub dim:          [u8; 3],
    pub green:        [u8; 3],
    pub red:          [u8; 3],
    pub yellow:       [u8; 3],
    pub cyan:         [u8; 3],
    pub blue:         [u8; 3],
    pub orange:       [u8; 3],
    pub magenta:      [u8; 3],
}

impl Theme {
    pub fn to_egui(&self, rgb: [u8; 3]) -> egui::Color32 {
        egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FontSettings {
    pub size:   f32,
    pub family: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FeatureToggles {
    pub git_status:        bool,
    pub autosuggestions:   bool,
    pub jump:              bool,
    pub syntax_highlight:  bool,
    pub fzf_files:         bool,
    pub fzf_env_vars:      bool,
    pub extract_archive:   bool,
    pub web_search:        bool,
    pub colored_man:       bool,
    pub command_not_found: bool,
}
```

## Default values

All defaults match the current hardcoded values in `terminal_app.rs` and `vte_proc.rs`, so upgrading from the legacy codebase changes nothing for users who don't create a settings file.

```rust
impl Default for Theme {
    fn default() -> Self {
        Self {
            bg:           [18, 18, 28],
            block_bg:     [24, 24, 38],
            block_border: [55, 55, 85],
            input_bg:     [22, 22, 36],
            status_bg:    [14, 14, 24],
            text:         [220, 220, 220],
            dim:          [100, 100, 130],
            green:        [78, 201, 148],
            red:          [255, 107, 107],
            yellow:       [220, 220, 170],
            cyan:         [78, 201, 176],
            blue:         [86, 156, 214],
            orange:       [206, 145, 120],
            magenta:      [197, 134, 192],
        }
    }
}

impl Default for FontSettings {
    fn default() -> Self {
        Self { size: 14.0, family: "monospace".into() }
    }
}

impl Default for FeatureToggles {
    fn default() -> Self {
        Self {
            git_status:        true,
            autosuggestions:   true,
            jump:              true,
            syntax_highlight:  true,
            fzf_files:         false,
            fzf_env_vars:      false,
            extract_archive:   false,
            web_search:        false,
            colored_man:       false,
            command_not_found: false,
        }
    }
}
```

## Usage in crates

Crates that need settings receive either:
- `&Settings` (read-only, no Arc overhead for simple single-threaded use)
- `Arc<RwLock<Settings>>` (for crates that need to re-read after hot-reload)

In Phase 1–2, `&Settings` is sufficient everywhere. Promote to `Arc<RwLock>` only when hot-reload is implemented.
