use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};
use std::time::Duration;

// ── TOML schema for signatures.toml ──────────────────────────────────────────

#[derive(serde::Deserialize)]
struct SigFile {
    command: Vec<CommandSigRaw>,
}

#[derive(serde::Deserialize)]
struct CommandSigRaw {
    name: String,
    description: Option<String>,
    usage: Option<String>,
    subcommands: Option<Vec<SubcommandRaw>>,
}

#[derive(serde::Deserialize, Clone)]
struct SubcommandRaw {
    name: String,
    description: Option<String>,
    usage: Option<String>,
}

// ── public types ──────────────────────────────────────────────────────────────

/// A single chip shown in the predict bar.
#[derive(Clone, Debug)]
pub struct PredictChip {
    pub label: String,
    pub completion: String,
    pub description: Option<String>,
    pub usage: Option<String>,
    pub kind: ChipKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChipKind {
    Command,
    Builtin,
    Keyword,
    Subcommand,
    Flag,
    History,
}

/// Parsed view of the current input line, with cursor assumed at end.
#[derive(Debug, PartialEq)]
pub struct PredictContext {
    pub command: String,
    pub partial: String,
    pub token_index: usize,
}

// ── context parser ────────────────────────────────────────────────────────────

/// Tokenise `input` (whitespace-split, naive quote awareness) and build context.
pub fn parse(input: &str) -> PredictContext {
    let tokens = shell_tokenize(input);
    match tokens.as_slice() {
        [] => PredictContext { command: String::new(), partial: String::new(), token_index: 0 },
        [partial] => PredictContext { command: String::new(), partial: partial.to_string(), token_index: 0 },
        [cmd, .., partial] => PredictContext {
            command: cmd.to_string(),
            partial: partial.to_string(),
            token_index: tokens.len() - 1,
        },
    }
}

fn shell_tokenize(input: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = None;
    let mut in_quote: Option<char> = None;
    let bytes = input.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        let c = b as char;
        match in_quote {
            Some(q) if c == q => {
                in_quote = None;
            }
            Some(_) => {}
            None => match c {
                '\'' | '"' => {
                    in_quote = Some(c);
                    if start.is_none() {
                        start = Some(i);
                    }
                }
                ' ' | '\t' => {
                    if let Some(s) = start.take() {
                        tokens.push(&input[s..i]);
                    }
                }
                _ => {
                    if start.is_none() {
                        start = Some(i);
                    }
                }
            },
        }
    }
    if let Some(s) = start {
        tokens.push(&input[s..]);
    }
    tokens
}

// ── signature database ────────────────────────────────────────────────────────

pub struct CommandSig {
    pub description: Option<String>,
    pub usage: Option<String>,
    pub(crate) subcommands: Vec<SubcommandRaw>,
}

pub struct SignatureDb {
    commands: HashMap<String, CommandSig>,
}

impl SignatureDb {
    pub fn load() -> Self {
        let raw = include_str!("../data/signatures.toml");
        let file: SigFile = toml::from_str(raw).expect("signatures.toml must be valid TOML");
        let commands = file
            .command
            .into_iter()
            .map(|c| {
                (
                    c.name,
                    CommandSig {
                        description: c.description,
                        usage: c.usage,
                        subcommands: c.subcommands.unwrap_or_default(),
                    },
                )
            })
            .collect();
        Self { commands }
    }

    pub fn subcommands(&self, command: &str, partial: &str) -> Vec<PredictChip> {
        let Some(sig) = self.commands.get(command) else { return vec![] };
        sig.subcommands
            .iter()
            .filter(|s| s.name.starts_with(partial))
            .map(|s| PredictChip {
                label: s.name.clone(),
                completion: format!("{} ", s.name),
                description: s.description.clone(),
                usage: s.usage.clone(),
                kind: ChipKind::Subcommand,
            })
            .collect()
    }

    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }
}

// ── command index ─────────────────────────────────────────────────────────────

const BUILTINS: &[&str] = &[
    "echo", "printf", "cd", "pwd", "ls", "cat", "grep", "sed", "awk", "find", "mkdir", "rm",
    "mv", "cp", "touch", "chmod", "chown", "kill", "jobs", "fg", "bg", "alias", "unalias",
    "history", "read", "test", "[", "true", "false", "eval", "exec", "source", "export",
];

const KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac",
    "function", "in", "until", "select", "time", "return", "break", "continue",
];

#[derive(Clone)]
pub struct CommandEntry {
    pub kind: EntryKind,
    pub description: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum EntryKind {
    Builtin,
    Keyword,
    Executable,
}

pub struct CommandIndex {
    commands: Arc<RwLock<BTreeMap<String, CommandEntry>>>,
}

impl CommandIndex {
    pub fn build() -> Self {
        let map = Arc::new(RwLock::new(BTreeMap::new()));
        let map2 = Arc::clone(&map);

        std::thread::spawn(move || loop {
            let fresh = scan_path();
            if let Ok(mut w) = map2.write() {
                *w = fresh;
            }
            std::thread::sleep(Duration::from_secs(30));
        });

        Self { commands: map }
    }

    /// Return names with `prefix`, up to `limit`, sorted by priority.
    pub fn prefix_match(&self, prefix: &str, limit: usize) -> Vec<(String, CommandEntry)> {
        let Ok(map) = self.commands.read() else { return vec![] };
        map.range(prefix.to_string()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .take(limit)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

fn scan_path() -> BTreeMap<String, CommandEntry> {
    let mut map = BTreeMap::new();

    for k in KEYWORDS {
        map.insert(
            k.to_string(),
            CommandEntry { kind: EntryKind::Keyword, description: None },
        );
    }
    for b in BUILTINS {
        map.insert(
            b.to_string(),
            CommandEntry { kind: EntryKind::Builtin, description: None },
        );
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let is_exec = entry
                        .metadata()
                        .map(|m| {
                            use std::os::unix::fs::PermissionsExt;
                            m.permissions().mode() & 0o111 != 0
                        })
                        .unwrap_or(false);
                    if is_exec {
                        map.entry(name).or_insert(CommandEntry {
                            kind: EntryKind::Executable,
                            description: None,
                        });
                    }
                }
            }
        }
    }

    map
}

// ── suggest ───────────────────────────────────────────────────────────────────

pub fn suggest(
    ctx: &PredictContext,
    index: &CommandIndex,
    sigs: &SignatureDb,
    history: &[String],
    limit: usize,
) -> Vec<PredictChip> {
    if ctx.token_index == 0 {
        suggest_commands(&ctx.partial, index, history, limit)
    } else {
        suggest_subcommand_or_flag(&ctx.command, &ctx.partial, sigs, limit)
    }
}

fn suggest_commands(
    partial: &str,
    index: &CommandIndex,
    history: &[String],
    limit: usize,
) -> Vec<PredictChip> {
    if partial.is_empty() {
        return vec![];
    }

    // Score: (priority, history_boost, name_len)
    // priority: Keyword=3, Builtin=2, Executable=1
    struct Candidate {
        name: String,
        entry: CommandEntry,
        score: i32,
    }

    let mut seen: HashMap<String, i32> = HashMap::new();

    // History boost map
    let history_set: std::collections::HashSet<&str> =
        history.iter().map(|h| h.split_whitespace().next().unwrap_or("")).collect();

    let matches = index.prefix_match(partial, limit * 4);
    let mut candidates: Vec<Candidate> = matches
        .into_iter()
        .map(|(name, entry)| {
            let base = match entry.kind {
                EntryKind::Keyword => 30,
                EntryKind::Builtin => 20,
                EntryKind::Executable => 10,
            };
            let hist = if history_set.contains(name.as_str()) { 20 } else { 0 };
            let len_penalty = -(name.len() as i32).min(10);
            let score = base + hist + len_penalty;
            Candidate { name, entry, score }
        })
        .collect();

    candidates.sort_by(|a, b| b.score.cmp(&a.score));
    candidates.truncate(limit);

    candidates
        .into_iter()
        .filter_map(|c| {
            if seen.contains_key(&c.name) {
                return None;
            }
            seen.insert(c.name.clone(), c.score);
            let kind = match c.entry.kind {
                EntryKind::Keyword => ChipKind::Keyword,
                EntryKind::Builtin => ChipKind::Builtin,
                EntryKind::Executable => ChipKind::Command,
            };
            Some(PredictChip {
                completion: format!("{} ", c.name),
                label: c.name,
                description: c.entry.description,
                usage: None,
                kind,
            })
        })
        .collect()
}

fn suggest_subcommand_or_flag(
    command: &str,
    partial: &str,
    sigs: &SignatureDb,
    limit: usize,
) -> Vec<PredictChip> {
    let mut chips = sigs.subcommands(command, partial);
    chips.truncate(limit);
    chips
}

/// Replace the last whitespace-delimited token in `input` with `completion`.
pub fn apply_completion(input: &mut String, completion: &str) {
    if let Some(last_space) = input.rfind(' ') {
        input.truncate(last_space + 1);
        input.push_str(completion);
    } else {
        *input = completion.to_string();
    }
}

#[cfg(test)]
#[path = "predict_tests.rs"]
mod tests;
