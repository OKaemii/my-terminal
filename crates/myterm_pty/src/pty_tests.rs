use super::{PtySession, META_MARKER, PROMPT_MARKER};

#[test]
fn bash_init_script_contains_prompt_marker() {
    let script = PtySession::init_script("bash");
    assert!(
        script.contains(PROMPT_MARKER),
        "bash init script must contain PROMPT_MARKER"
    );
}

#[test]
fn zsh_init_script_contains_meta_marker() {
    let script = PtySession::init_script("zsh");
    assert!(
        script.contains(META_MARKER),
        "zsh init script must contain META_MARKER"
    );
}

#[test]
fn zsh_init_script_has_precmd_hook() {
    let script = PtySession::init_script("zsh");
    assert!(
        script.contains("precmd"),
        "zsh init script must define precmd hook"
    );
}

#[test]
fn bash_init_script_has_prompt_command() {
    let script = PtySession::init_script("bash");
    assert!(
        script.contains("PROMPT_COMMAND"),
        "bash init script must set PROMPT_COMMAND"
    );
}
