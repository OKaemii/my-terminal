use super::*;

#[test]
fn parse_two_tokens_gives_correct_context() {
    let ctx = parse("git co");
    assert_eq!(ctx.command, "git");
    assert_eq!(ctx.partial, "co");
    assert_eq!(ctx.token_index, 1);
}

#[test]
fn parse_empty_gives_empty_context() {
    let ctx = parse("");
    assert_eq!(ctx.token_index, 0);
    assert!(ctx.partial.is_empty());
}

#[test]
fn parse_single_token() {
    let ctx = parse("git");
    assert_eq!(ctx.token_index, 0);
    assert_eq!(ctx.partial, "git");
    assert!(ctx.command.is_empty());
}

#[test]
fn sigs_git_commit_and_config() {
    let sigs = SignatureDb::load();
    let chips = sigs.subcommands("git", "co");
    let labels: Vec<_> = chips.iter().map(|c| c.label.as_str()).collect();
    assert!(
        labels.contains(&"commit"),
        "commit must be in git co completions"
    );
    assert!(
        labels.contains(&"config"),
        "config must be in git co completions"
    );
}

#[test]
fn sigs_git_commit_full_match() {
    let sigs = SignatureDb::load();
    let chips = sigs.subcommands("git", "commit");
    assert_eq!(chips.len(), 1);
    assert_eq!(chips[0].label, "commit");
}

#[test]
fn apply_completion_replaces_partial() {
    let mut input = "git co".to_string();
    apply_completion(&mut input, "commit ");
    assert_eq!(input, "git commit ");
}

#[test]
fn apply_completion_no_space() {
    let mut input = "gi".to_string();
    apply_completion(&mut input, "git ");
    assert_eq!(input, "git ");
}

#[test]
fn suggest_empty_input_returns_nothing() {
    let index = CommandIndex::build();
    let sigs = SignatureDb::load();
    let ctx = parse("");
    let chips = suggest(&ctx, &index, &sigs, &[], 7);
    assert!(chips.is_empty());
}
