use egui::Color32;
use myterm_core::palette;

const KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac", "function",
    "in", "until", "select", "time", "return", "break", "continue", "exit", "export", "local",
    "readonly", "declare", "typeset", "unset", "shift", "source", ".",
];

const BUILTINS: &[&str] = &[
    "echo", "printf", "cd", "pwd", "ls", "cat", "grep", "sed", "awk", "find", "mkdir", "rm", "mv",
    "cp", "touch", "chmod", "chown", "kill", "jobs", "fg", "bg", "alias", "unalias", "history",
    "read", "test", "[", "true", "false", "eval", "exec",
];

/// Semantic colour category — replaces the ratatui `Color` dependency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HighlightColor {
    Keyword,
    Builtin,
    CommandOk,
    CommandErr,
    Flag,
    StringLit,
    Variable,
    PipeOp,
    Comment,
    Dim,
}

pub fn to_egui(c: HighlightColor) -> Color32 {
    match c {
        HighlightColor::Keyword => palette::YELLOW,
        HighlightColor::Builtin => palette::CYAN,
        HighlightColor::CommandOk => palette::GREEN,
        HighlightColor::CommandErr => palette::RED,
        HighlightColor::Flag => palette::BLUE,
        HighlightColor::StringLit => palette::GREEN,
        HighlightColor::Variable => palette::MAGENTA,
        HighlightColor::PipeOp => palette::ORANGE,
        HighlightColor::Comment => palette::DIM,
        HighlightColor::Dim => palette::DIM,
    }
}

/// A highlighted token: (text, colour category).
pub type Token = (String, HighlightColor);

/// Tokenise `input` and return coloured tokens for rendering.
pub fn highlight(input: &str) -> Vec<Token> {
    if input.is_empty() {
        return vec![];
    }
    tokenize(input)
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut is_first_word = true;
    let mut after_pipe = false;

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' => {
                let mut ws = String::new();
                while matches!(chars.peek(), Some(' ') | Some('\t')) {
                    ws.push(chars.next().unwrap());
                }
                tokens.push((ws, HighlightColor::Dim));
            }
            '#' => {
                let mut comment = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    comment.push(chars.next().unwrap());
                }
                tokens.push((comment, HighlightColor::Comment));
            }
            '"' | '\'' => {
                let quote = chars.next().unwrap();
                let mut s = String::new();
                s.push(quote);
                while let Some(&c) = chars.peek() {
                    s.push(chars.next().unwrap());
                    if c == quote {
                        break;
                    }
                }
                tokens.push((s, HighlightColor::StringLit));
                is_first_word = false;
            }
            '$' => {
                let mut var = String::new();
                var.push(chars.next().unwrap());
                if chars.peek() == Some(&'{') {
                    var.push(chars.next().unwrap());
                    while let Some(&c) = chars.peek() {
                        var.push(chars.next().unwrap());
                        if c == '}' {
                            break;
                        }
                    }
                } else if chars.peek() == Some(&'(') {
                    var.push(chars.next().unwrap());
                    let mut depth = 1usize;
                    while let Some(&c) = chars.peek() {
                        var.push(chars.next().unwrap());
                        if c == '(' {
                            depth += 1;
                        }
                        if c == ')' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                } else {
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            var.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                }
                tokens.push((var, HighlightColor::Variable));
                is_first_word = false;
            }
            '|' | '&' | ';' => {
                let mut op = String::new();
                op.push(chars.next().unwrap());
                if chars.peek() == Some(&c) || (c == '&' && chars.peek() == Some(&'&')) {
                    op.push(chars.next().unwrap());
                }
                is_first_word = true;
                after_pipe = true;
                tokens.push((op, HighlightColor::PipeOp));
            }
            '>' | '<' => {
                let mut op = String::new();
                op.push(chars.next().unwrap());
                if matches!(chars.peek(), Some('>') | Some('<') | Some('&')) {
                    op.push(chars.next().unwrap());
                }
                tokens.push((op, HighlightColor::PipeOp));
            }
            _ => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if " \t|&;<>\"'$#".contains(c) {
                        break;
                    }
                    word.push(chars.next().unwrap());
                }
                if word.is_empty() {
                    chars.next();
                    continue;
                }

                let color = if is_first_word || after_pipe {
                    after_pipe = false;
                    if KEYWORDS.contains(&word.as_str()) {
                        HighlightColor::Keyword
                    } else if BUILTINS.contains(&word.as_str()) {
                        HighlightColor::Builtin
                    } else if command_exists(&word) {
                        HighlightColor::CommandOk
                    } else {
                        HighlightColor::CommandErr
                    }
                } else if word.starts_with('-') {
                    HighlightColor::Flag
                } else {
                    HighlightColor::Dim
                };

                is_first_word = false;
                tokens.push((word, color));
            }
        }
    }
    tokens
}

fn command_exists(name: &str) -> bool {
    if name.contains('/') {
        return std::path::Path::new(name).exists();
    }
    which::which(name).is_ok()
}

#[cfg(test)]
#[path = "highlight_tests.rs"]
mod tests;
