# PTY & VTE Processing

How myterm communicates with the shell and renders its output.

---

## PTY Session (`crates/myterm_pty`)

### What a PTY is
A pseudo-terminal (PTY) is a pair of file descriptors — master and slave — that look like a terminal to the process running on the slave side. The shell (bash/zsh) thinks it's talking to a real terminal. myterm holds the master end and can read everything the shell writes, and write anything the shell should "receive from the keyboard."

### Setup (from `PtySession::new`)

```
native_pty_system()
    └── openpty(rows=40, cols=200)
        ├── slave  ← shell spawns here (TERM=xterm-256color, MYTERM=1)
        └── master ← myterm holds this
            ├── reader thread → mpsc::Sender<Vec<u8>>
            └── writer (PtySession.writer) ← stdin to shell
```

The reader thread (`thread::spawn`) continuously reads from the master PTY and sends byte chunks via `mpsc::Sender<Vec<u8>>`. The UI receives chunks by draining `self.pty.rx` in `process_pty()` every frame.

### Marker protocol

Two magic strings are injected into the shell's RC (via `PROMPT_COMMAND` for bash, `precmd()` for zsh) so myterm can detect command boundaries without guessing:

| Marker | When emitted | Meaning |
|--------|-------------|---------|
| `__MYTERM_READY__` | After every prompt display | Shell is idle, ready for input |
| `__MYTERM_META__:/cwd:exit_code` | Just before READY | Cwd + exit code of last command |

**Init script for bash:**
```bash
stty -echo
PROMPT_COMMAND='printf "__MYTERM_META__:%s:%d\n" "$PWD" "$?"'
PS1='__MYTERM_READY__\n'
```

**Init script for zsh:**
```zsh
stty -echo
unsetopt PROMPT_CR
precmd() {
    printf '__MYTERM_META__:%s:%d\n' "$PWD" "$?"
    printf '__MYTERM_READY__\n'
}
PS1=''
RPROMPT=''
```

`stty -echo` prevents the shell from echoing typed characters back (myterm handles display of input itself). `PS1=''` removes the shell's own prompt since myterm draws its own.

### Writing to the shell

- Normal commands: `write!(self.writer, "{cmd}\n")`
- Ctrl+C / EOF: write raw bytes `\x03` / `\x04`
- Interactive mode (TUI apps): forward all key events as raw bytes

### Resize

When the egui window is resized, call:
```rust
self._master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
```
This sends `SIGWINCH` to the shell, which propagates to running programs. Currently not implemented — known limitation to fix.

---

## VTE Output Processing (`crates/myterm_vte`)

### What VTE is
VTE (Virtual Terminal Emulator) is a state machine that parses ANSI/VT100 escape sequences from raw byte streams. We use the `vte` crate (a Rust port of Paul Williams' VT100 state machine).

The `vte::Perform` trait has callbacks for each parsed event type. `OutputProcessor` implements `Perform`.

### OutputProcessor state

```
cur_line: Vec<StyledChar>   ← the line currently being built
col: usize                   ← cursor column position
fg: Color32                  ← current foreground colour
bold: bool                   ← current bold state
completed: Vec<Vec<StyledChar>> ← fully finished lines
alternate_screen: bool       ← true when TUI app is active
```

### Events handled

| VTE event | What it does |
|-----------|-------------|
| `print(char)` | `put_char()` — writes to `cur_line[col]`, advances col |
| `execute('\n')` | `newline()` — pushes cur_line to completed, resets col |
| `execute('\r')` | resets col to 0 (carriage return) |
| `execute('\x08')` | backspace — decrements col |
| `csi_dispatch 'm'` | SGR — calls `sgr(params)` to update fg/bold |
| `csi_dispatch 'G'` | CHA — moves col to specified column |
| `csi_dispatch 'C'` | CUF — advances col by N |
| `csi_dispatch 'D'` | CUB — retreats col by N |
| `csi_dispatch 'K'` | EL — erase to end of line |
| `csi_dispatch 'h'` | DEC private set: `?1049h`/`?47h` → `alternate_screen = true` |
| `csi_dispatch 'l'` | DEC private reset: `?1049l`/`?47l` → `alternate_screen = false` |
| everything else | silently ignored |

### SGR (Select Graphic Rendition) parsing

`ESC[<params>m` changes colour and text attributes. Params are a semicolon-separated list of codes:

| Code | Effect |
|------|--------|
| 0 | Reset all — fg → TEXT, bold → false |
| 1 | Bold on |
| 22 | Bold off |
| 30–37 | Standard foreground colours (black→white) |
| 38;5;N | 256-colour foreground |
| 38;2;R;G;B | True-colour foreground |
| 39 | Default foreground (TEXT) |
| 90–97 | Bright foreground colours |

256-colour and true-colour are supported. Warp maps codes to `Color32` directly; myterm does the same.

### Alternate screen detection

When a TUI program (vim, htop, man, less) starts, it sends:
```
ESC[?1049h   ← save cursor + switch to alternate screen
```
When it exits:
```
ESC[?1049l   ← restore cursor + return to normal screen
```

myterm uses this to switch to "interactive / full-passthrough" mode: no command block rendering, all keypresses forwarded raw to the PTY. The status bar shows "⌨ INTERACTIVE" as a hint.

### Line splitting vs VTE feeding

The current design splits on `\n` at the application level (in `handle_raw_line`) before feeding bytes to the VTE parser. This is slightly wrong for programs that write partial lines (progress bars, spinners). A correct implementation would feed raw bytes directly to the VTE parser and let it trigger newlines. This is a known limitation to address in Phase 3.

### `take_completed()` vs `take_all()`

- `take_completed()` — drains only fully-terminated lines (safe to call every frame)
- `take_all()` — flushes the current partial line too (called at command completion in `on_prompt_ready`)

### Rendering styled chars

`line_to_layout_job(line: &[StyledChar], font_size: f32) -> egui::text::LayoutJob` converts a `Vec<StyledChar>` into egui's layout format. Each run of chars with the same colour is a single `LayoutSection`.
