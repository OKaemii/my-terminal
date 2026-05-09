use enum_iterator::Sequence;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Sequence)]
pub enum FeatureFlag {
    GitStatus,
    Autosuggestions,
    SyntaxHighlight,
    FrecencyJump,
    PredictBar,
    FzfFiles,
    FzfEnvVars,
    ExtractArchive,
    WebSearch,
    ColoredManPages,
    CommandNotFound,
}

const N: usize = enum_iterator::cardinality::<FeatureFlag>();

static FLAG_STATES: [AtomicBool; N] = {
    // const initialiser — one false per flag
    let mut arr: [AtomicBool; N] = unsafe { std::mem::zeroed() };
    let mut i = 0;
    while i < N {
        arr[i] = AtomicBool::new(false);
        i += 1;
    }
    arr
};

impl FeatureFlag {
    pub fn is_enabled(self) -> bool {
        FLAG_STATES[self as usize].load(Ordering::Relaxed)
    }

    pub fn set(self, enabled: bool) {
        FLAG_STATES[self as usize].store(enabled, Ordering::Relaxed);
    }
}

pub struct FeatureToggles {
    pub git_status: bool,
    pub autosuggestions: bool,
    pub syntax_highlight: bool,
    pub jump: bool,
    pub predict_bar: bool,
    pub fzf_files: bool,
    pub command_not_found: bool,
}

pub fn init_from_settings(toggles: &FeatureToggles) {
    FeatureFlag::GitStatus.set(toggles.git_status);
    FeatureFlag::Autosuggestions.set(toggles.autosuggestions);
    FeatureFlag::SyntaxHighlight.set(toggles.syntax_highlight);
    FeatureFlag::FrecencyJump.set(toggles.jump);
    FeatureFlag::PredictBar.set(toggles.predict_bar);
    FeatureFlag::FzfFiles.set(toggles.fzf_files);
    FeatureFlag::CommandNotFound.set(toggles.command_not_found);
    // These Phase 4 flags are always off until implemented
    FeatureFlag::FzfEnvVars.set(false);
    FeatureFlag::ExtractArchive.set(false);
    FeatureFlag::WebSearch.set(false);
    FeatureFlag::ColoredManPages.set(false);
}

#[cfg(test)]
#[path = "features_tests.rs"]
mod tests;
