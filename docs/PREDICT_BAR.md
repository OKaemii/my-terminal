# Predict Bar

The predict bar is a horizontal strip of suggestion chips displayed between the command blocks and the input field — like the autocomplete bar on a mobile keyboard. It updates with every keystroke and adapts to context.

**Feature crate:** `crates/feature_predict/`  
**UI rendering:** `crates/myterm_ui/src/predict_bar.rs`  
**Toggle:** `settings.toml` → `[features] predict_bar = true`

---

## Visual layout

```
┌──────────────────────────────────────────────────────────┐
│  [block output area]                                     │
│                                                          │
├──────────────────────────────────────────────────────────┤
│  make │ man │ mapfile │ maven │ ...                      │  ← predict bar
├──────────────────────────────────────────────────────────┤
│  $ ma_                                                   │  ← input
└──────────────────────────────────────────────────────────┘
```

The bar is 28px tall. Each chip is a small button in monospace font. A thin separator `│` sits between chips. The bar is hidden when the input is empty.

---

## Context modes

### Mode 1: First-word (command mode)

Triggered when the cursor is on the first token.

- Source: `CommandIndex` (PATH executables + builtins + keywords)
- Matching: prefix match first, then substring
- Ranking: exact > prefix > history boost > alphabetical
- Chip colour: green (Executable), cyan (Builtin), yellow (Keyword), orange (History)
- Max chips: 7

Example:
```
input: "gi"
chips: git │ gist │ gimp │ ...
```

### Mode 2: Subcommand mode

Triggered when the cursor is on the second token and the first token is in the signature DB.

- Source: `SignatureDb` for the parent command
- Matching: prefix match on partial subcommand, fallback: `--help` heuristic parser
- Chip colour: blue (Subcommand)

Example:
```
input: "git "         → add │ commit │ push │ pull │ status │ log │ diff
input: "git co"       → commit │ config
input: "cargo "       → build │ run │ test │ check │ clippy │ fmt │ add
```

### Mode 3: Flag mode

Triggered when the cursor starts with `-` after a known command/subcommand.

- Source: `SignatureDb` options list for that command
- Chip colour: dim (Flag)

Example:
```
input: "git commit -"    → -m │ --amend │ --no-edit │ --allow-empty
input: "cargo build --"  → --release │ --features │ --package │ --target
```

---

## Hover tooltip

Hovering a chip (no click needed) shows a floating tooltip:

```
┌─────────────────────────────────────────────────┐
│  Record changes to the repository               │  ← description
│  git commit [-m <msg>] [--amend] [--no-edit]    │  ← usage (cyan monospace)
└─────────────────────────────────────────────────┘
```

The tooltip is rendered via `egui::show_tooltip_at_pointer`. It appears after a 300ms hover delay to avoid flicker while quickly scanning chips.

---

## Click behaviour

Clicking a chip calls `apply_completion(input, chip.completion)`:

- Replaces the **current partial token** with the chip's `completion` value
- Completions for commands/subcommands add a trailing space: `"commit "` → cursor ready for next arg
- Completions for flags that take a value add `=`: `"--message="` → cursor positioned for value

Example:
```
input = "git co"
click "commit"
→ input = "git commit "
```

```
input = "git commit -"
click "--message"
→ input = "git commit --message="
```

---

## Data pipeline

```
user keystroke
  │
  ▼
parse(input) → PredictContext { command, partial, token_index }
  │
  ▼
suggest(ctx, index, sigs, history, limit=7) → Vec<PredictChip>
  │
  ├── token_index == 0 → index.prefix_match(partial) + history_boost
  ├── token_index == 1 → sigs.subcommands(command, partial)
  └── partial.starts_with('-') → sigs.flags(command, subcommand, partial)
  │
  ▼
render_predict_bar(chips, &mut input, ctx)
```

---

## Signature database

Stored in `crates/feature_predict/data/signatures.toml`, embedded at compile time via `include_str!`.

Currently covers: `git`, `cargo`, `docker`, `npm`, `make`, `ssh`, `curl`, `tar`, `systemctl` (with full subcommand lists).

### Adding a new command

Add a `[[command]]` block to `signatures.toml`:

```toml
[[command]]
name = "kubectl"
description = "Kubernetes cluster manager"
usage = "kubectl <command> [flags]"

  [[command.subcommands]]
  name = "get"
  description = "Display one or many resources"
  usage = "kubectl get <resource> [-n <namespace>] [-o <format>]"

  [[command.subcommands]]
  name = "apply"
  description = "Apply a configuration to a resource from a file"
  usage = "kubectl apply -f <file>"
```

No code change needed — the file is parsed at startup.

---

## Fallback: `--help` parser

For commands **not** in `signatures.toml`, a background task runs:

```bash
<cmd> --help 2>&1 | head -60
```

The output is scanned for lines matching the heuristic:
```
  <word>    <description>
```
(two or more spaces, a word, two or more spaces, text)

This catches most `git`-style and `cargo`-style help formats. Results are cached in memory for the session.

### Minimum quality threshold

If fewer than 2 subcommands are parsed from `--help`, the result is discarded — shows no chips rather than misleading ones.

---

## CommandIndex internals

At startup, a background thread scans every directory in `$PATH` and collects executable file names. The result is a `BTreeMap<String, CommandEntry>` (sorted for efficient prefix queries) stored behind an `Arc<RwLock<...>>`.

The scan is repeated every 30 seconds so newly installed commands appear without restarting myterm.

### Builtin list (hard-coded)

```
echo printf cd pwd ls cat grep sed awk find mkdir rm mv cp touch chmod
chown kill jobs fg bg alias unalias history read test [ true false eval exec
```

### Keyword list (hard-coded)

```
if then else elif fi for while do done case esac function in until select
time return break continue exit export local readonly declare typeset unset
shift source .
```

---

## Performance notes

- `prefix_match` on a `BTreeMap` is O(log n + k) where k is the result count — fast.
- Signature DB lookup is O(1) hash map.
- `--help` subprocess runs once per command per session, cached after first run.
- The bar rerenders every frame while the input changes (egui is retained-mode so this is free).
- Suggestion computation runs on the UI thread but is synchronous and sub-millisecond for the index path. `--help` parsing runs off-thread.
