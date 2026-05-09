use super::{highlight, HighlightColor};

fn first_color(input: &str) -> HighlightColor {
    highlight(input)
        .into_iter()
        .find(|(s, _)| !s.trim().is_empty())
        .map(|(_, c)| c)
        .expect("at least one non-whitespace token")
}

#[test]
fn keyword_if() {
    assert_eq!(first_color("if"), HighlightColor::Keyword);
}

#[test]
fn keyword_for() {
    assert_eq!(first_color("for"), HighlightColor::Keyword);
}

#[test]
fn builtin_echo() {
    assert_eq!(first_color("echo"), HighlightColor::Builtin);
}

#[test]
fn builtin_cd() {
    assert_eq!(first_color("cd"), HighlightColor::Builtin);
}

#[test]
fn flag_verbose() {
    let tokens = highlight("echo --verbose");
    let flag = tokens.iter().find(|(s, _)| s == "--verbose").expect("flag token");
    assert_eq!(flag.1, HighlightColor::Flag);
}

#[test]
fn variable_home() {
    let tokens = highlight("$HOME");
    let var = tokens.iter().find(|(s, _)| s == "$HOME").expect("var token");
    assert_eq!(var.1, HighlightColor::Variable);
}

#[test]
fn comment_token() {
    let tokens = highlight("# comment here");
    let comment = tokens.first().expect("comment token");
    assert_eq!(comment.1, HighlightColor::Comment);
}

#[test]
fn empty_input_returns_empty() {
    assert!(highlight("").is_empty());
}
