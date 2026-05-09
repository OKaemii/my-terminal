use super::*;
use myterm_core::palette;

fn feed(proc: &mut OutputProcessor, input: &[u8]) {
    let mut parser = vte::Parser::new();
    for &b in input {
        parser.advance(proc, b);
    }
}

#[test]
fn bold_sgr_sets_bold() {
    let mut p = OutputProcessor::new();
    feed(&mut p, b"\x1b[1mhello\x1b[0m\n");
    let lines = p.take_completed();
    assert!(!lines.is_empty());
    let line = &lines[0];
    assert!(!line.is_empty(), "parsed");
    // The 'h','e','l','l','o' chars should be bold
    let bold_chars: Vec<_> = line.iter().filter(|sc| sc.bold).collect();
    assert!(
        !bold_chars.is_empty(),
        "bold chars should exist after SGR 1"
    );
}

#[test]
fn green_sgr_sets_green_color() {
    let mut p = OutputProcessor::new();
    feed(&mut p, b"\x1b[32mok\x1b[0m\n");
    let lines = p.take_completed();
    assert!(!lines.is_empty());
    let line = &lines[0];
    let green_chars: Vec<_> = line
        .iter()
        .filter(|sc| sc.color == palette::GREEN)
        .collect();
    assert!(
        !green_chars.is_empty(),
        "green chars should exist after SGR 32"
    );
}

#[test]
fn bare_ascii_has_default_color_not_bold() {
    let mut p = OutputProcessor::new();
    feed(&mut p, b"hello\n");
    let lines = p.take_completed();
    assert!(!lines.is_empty());
    let line = &lines[0];
    for sc in line {
        assert_eq!(
            sc.color,
            palette::TEXT,
            "bare ascii must have default TEXT color"
        );
        assert!(!sc.bold, "bare ascii must not be bold");
    }
}

#[test]
fn strip_ansi_removes_sgr() {
    let result = strip_ansi("\x1b[32mhello\x1b[0m");
    assert_eq!(result, "hello");
}

#[test]
fn strip_ansi_removes_osc() {
    let result = strip_ansi("\x1b]0;title\x07plain");
    assert_eq!(result, "plain");
}
