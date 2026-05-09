use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

const KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac",
    "function", "in", "until", "select", "time", "return", "break", "continue", "exit",
    "export", "local", "readonly", "declare", "typeset", "unset", "shift", "source", ".",
];

const BUILTINS: &[&str] = &[
    "echo", "printf", "cd", "pwd", "ls", "cat", "grep", "sed", "awk", "find", "mkdir",
    "rm", "mv", "cp", "touch", "chmod", "chown", "kill", "jobs", "fg", "bg", "alias",
    "unalias", "history", "read", "test", "[", "true", "false", "eval", "exec",
];

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Command(String),
    Keyword(String),
    Builtin(String),
    Arg(String),
    Flag(String),
    StringLit(String),
    Variable(String),
    Pipe(String),
    Redirect(String),
    Whitespace(String),
    Unknown(String),
}

pub fn highlight(input: &str) -> Vec<Span<'static>> {
    if input.is_empty() {
        return vec![];
    }
    let tokens = tokenize(input);
    tokens.into_iter().map(token_to_span).collect()
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
                while chars.peek() == Some(&' ') || chars.peek() == Some(&'\t') {
                    ws.push(chars.next().unwrap());
                }
                tokens.push(Token::Whitespace(ws));
            }
            '#' => {
                let mut comment = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '\n' { break; }
                    comment.push(chars.next().unwrap());
                }
                tokens.push(Token::Unknown(comment));
            }
            '"' | '\'' => {
                let quote = chars.next().unwrap();
                let mut s = String::new();
                s.push(quote);
                while let Some(&c) = chars.peek() {
                    s.push(chars.next().unwrap());
                    if c == quote { break; }
                }
                tokens.push(Token::StringLit(s));
                is_first_word = false;
            }
            '$' => {
                let mut var = String::new();
                var.push(chars.next().unwrap());
                if chars.peek() == Some(&'{') {
                    var.push(chars.next().unwrap());
                    while let Some(&c) = chars.peek() {
                        var.push(chars.next().unwrap());
                        if c == '}' { break; }
                    }
                } else if chars.peek() == Some(&'(') {
                    var.push(chars.next().unwrap());
                    let mut depth = 1;
                    while let Some(&c) = chars.peek() {
                        var.push(chars.next().unwrap());
                        if c == '(' { depth += 1; }
                        if c == ')' { depth -= 1; if depth == 0 { break; } }
                    }
                } else {
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            var.push(chars.next().unwrap());
                        } else { break; }
                    }
                }
                tokens.push(Token::Variable(var));
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
                tokens.push(Token::Pipe(op));
            }
            '>' | '<' => {
                let mut op = String::new();
                op.push(chars.next().unwrap());
                if chars.peek() == Some(&'>') || chars.peek() == Some(&'<') || chars.peek() == Some(&'&') {
                    op.push(chars.next().unwrap());
                }
                tokens.push(Token::Redirect(op));
            }
            _ => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if " \t|&;<>\"'$#".contains(c) { break; }
                    word.push(chars.next().unwrap());
                }
                if word.is_empty() {
                    chars.next();
                    continue;
                }

                let token = if is_first_word || after_pipe {
                    after_pipe = false;
                    if KEYWORDS.contains(&word.as_str()) {
                        Token::Keyword(word)
                    } else if BUILTINS.contains(&word.as_str()) {
                        Token::Builtin(word)
                    } else {
                        let exists = command_exists(&word);
                        Token::Command(if exists { format!("\x01{}", word) } else { format!("\x00{}", word) })
                    }
                } else if word.starts_with('-') {
                    Token::Flag(word)
                } else {
                    Token::Arg(word)
                };

                is_first_word = false;
                tokens.push(token);
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

fn token_to_span(token: Token) -> Span<'static> {
    match token {
        Token::Keyword(s) => Span::styled(s, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Token::Builtin(s) => Span::styled(s, Style::default().fg(Color::Cyan)),
        Token::Command(s) => {
            if s.starts_with('\x01') {
                Span::styled(s[1..].to_string(), Style::default().fg(Color::Green))
            } else {
                Span::styled(s[1..].to_string(), Style::default().fg(Color::Red))
            }
        }
        Token::Flag(s) => Span::styled(s, Style::default().fg(Color::Blue)),
        Token::StringLit(s) => Span::styled(s, Style::default().fg(Color::LightGreen)),
        Token::Variable(s) => Span::styled(s, Style::default().fg(Color::Magenta)),
        Token::Pipe(s) => {
            Span::styled(s, Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
        }
        Token::Redirect(s) => Span::styled(s, Style::default().fg(Color::LightYellow)),
        Token::Arg(s) => Span::styled(s, Style::default().fg(Color::White)),
        Token::Whitespace(s) | Token::Unknown(s) => Span::raw(s),
    }
}
