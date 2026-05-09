use serde::{Deserialize, Serialize};

/// A full colour palette as RGB triples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub bg: [u8; 3],
    pub block_bg: [u8; 3],
    pub block_border: [u8; 3],
    pub input_bg: [u8; 3],
    pub status_bg: [u8; 3],
    pub text: [u8; 3],
    pub dim: [u8; 3],
    pub green: [u8; 3],
    pub red: [u8; 3],
    pub yellow: [u8; 3],
    pub cyan: [u8; 3],
    pub blue: [u8; 3],
    pub orange: [u8; 3],
    pub magenta: [u8; 3],
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg: [18, 18, 28],
            block_bg: [24, 24, 38],
            block_border: [55, 55, 85],
            input_bg: [22, 22, 36],
            status_bg: [14, 14, 24],
            text: [220, 220, 220],
            dim: [100, 100, 130],
            green: [78, 201, 148],
            red: [255, 107, 107],
            yellow: [220, 220, 170],
            cyan: [78, 201, 176],
            blue: [86, 156, 214],
            orange: [206, 145, 120],
            magenta: [197, 134, 192],
        }
    }

    pub fn light() -> Self {
        Self {
            bg: [248, 248, 248],
            block_bg: [235, 235, 240],
            block_border: [200, 200, 210],
            input_bg: [240, 240, 245],
            status_bg: [220, 220, 230],
            text: [30, 30, 30],
            dim: [120, 120, 140],
            green: [0, 128, 80],
            red: [180, 0, 0],
            yellow: [128, 100, 0],
            cyan: [0, 128, 160],
            blue: [0, 80, 180],
            orange: [160, 80, 0],
            magenta: [140, 0, 140],
        }
    }

    /// Solarized Dark — official palette from Ethan Schoonover.
    pub fn solarized() -> Self {
        Self {
            bg: [0, 43, 54],
            block_bg: [7, 54, 66],
            block_border: [88, 110, 117],
            input_bg: [0, 43, 54],
            status_bg: [0, 43, 54],
            text: [131, 148, 150],
            dim: [88, 110, 117],
            green: [133, 153, 0],
            red: [220, 50, 47],
            yellow: [181, 137, 0],
            cyan: [42, 161, 152],
            blue: [38, 139, 210],
            orange: [203, 75, 22],
            magenta: [108, 113, 196],
        }
    }

    /// Dracula — official palette.
    pub fn dracula() -> Self {
        Self {
            bg: [40, 42, 54],
            block_bg: [48, 50, 65],
            block_border: [68, 71, 90],
            input_bg: [40, 42, 54],
            status_bg: [33, 34, 44],
            text: [248, 248, 242],
            dim: [98, 114, 164],
            green: [80, 250, 123],
            red: [255, 85, 85],
            yellow: [241, 250, 140],
            cyan: [139, 233, 253],
            blue: [189, 147, 249],
            orange: [255, 184, 108],
            magenta: [255, 121, 198],
        }
    }

    /// Nord — official palette from Arctic Ice Studio.
    pub fn nord() -> Self {
        Self {
            bg: [46, 52, 64],
            block_bg: [59, 66, 82],
            block_border: [76, 86, 106],
            input_bg: [46, 52, 64],
            status_bg: [36, 41, 51],
            text: [216, 222, 233],
            dim: [76, 86, 106],
            green: [163, 190, 140],
            red: [191, 97, 106],
            yellow: [235, 203, 139],
            cyan: [136, 192, 208],
            blue: [129, 161, 193],
            orange: [208, 135, 112],
            magenta: [180, 142, 173],
        }
    }

    /// Atom One Dark — official palette.
    pub fn one_dark() -> Self {
        Self {
            bg: [40, 44, 52],
            block_bg: [50, 55, 65],
            block_border: [70, 75, 90],
            input_bg: [40, 44, 52],
            status_bg: [33, 37, 43],
            text: [171, 178, 191],
            dim: [92, 99, 112],
            green: [152, 195, 121],
            red: [224, 108, 117],
            yellow: [229, 192, 123],
            cyan: [86, 182, 194],
            blue: [97, 175, 239],
            orange: [209, 154, 102],
            magenta: [198, 120, 221],
        }
    }
}

/// Which named colour scheme to use. `Custom` reads the `[appearance.custom_theme]` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ColorScheme {
    #[default]
    Dark,
    Light,
    Solarized,
    Dracula,
    Nord,
    OneDark,
    Custom,
}

impl ColorScheme {
    pub fn resolve(&self, custom: &Theme) -> Theme {
        match self {
            ColorScheme::Dark => Theme::dark(),
            ColorScheme::Light => Theme::light(),
            ColorScheme::Solarized => Theme::solarized(),
            ColorScheme::Dracula => Theme::dracula(),
            ColorScheme::Nord => Theme::nord(),
            ColorScheme::OneDark => Theme::one_dark(),
            ColorScheme::Custom => custom.clone(),
        }
    }
}
