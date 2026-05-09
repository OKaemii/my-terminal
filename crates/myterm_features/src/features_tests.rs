use super::*;

#[test]
fn set_true_then_is_enabled() {
    FeatureFlag::GitStatus.set(true);
    assert!(FeatureFlag::GitStatus.is_enabled());
}

#[test]
fn set_false_then_not_enabled() {
    FeatureFlag::GitStatus.set(false);
    assert!(!FeatureFlag::GitStatus.is_enabled());
}

#[test]
fn init_from_settings_all_true() {
    let toggles = FeatureToggles {
        git_status: true,
        autosuggestions: true,
        syntax_highlight: true,
        jump: true,
        predict_bar: true,
        fzf_files: true,
        command_not_found: true,
    };
    init_from_settings(&toggles);
    assert!(FeatureFlag::GitStatus.is_enabled());
    assert!(FeatureFlag::Autosuggestions.is_enabled());
    assert!(FeatureFlag::SyntaxHighlight.is_enabled());
    assert!(FeatureFlag::FrecencyJump.is_enabled());
    assert!(FeatureFlag::PredictBar.is_enabled());
}

#[test]
fn init_from_settings_all_false() {
    let toggles = FeatureToggles {
        git_status: false,
        autosuggestions: false,
        syntax_highlight: false,
        jump: false,
        predict_bar: false,
        fzf_files: false,
        command_not_found: false,
    };
    init_from_settings(&toggles);
    assert!(!FeatureFlag::GitStatus.is_enabled());
    assert!(!FeatureFlag::Autosuggestions.is_enabled());
}
