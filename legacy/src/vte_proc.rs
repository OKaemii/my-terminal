/// A minimal VTE performer that renders PTY output into styled character lines.
///
/// Handles:
///   - SGR colour / bold codes
///   - CR, LF, backspace, cursor-column movement (ESC[G, ESC[C/D)
///   - Erase-in-line (ESC[K)
///   - Alternate-screen detection (ESC[?1049h/l, ESC[?47h/l)
///   - All other escape sequences are silently swallowed
use egui::Color32;
use vte::Perform;

// ── shared colours (keep in sync with terminal_app.rs) ─────────────────────
pub const TEXT: Color32 = Color32::from_rgb(220, 220, 220);
pub const DIM: Color32 = Color32::from_rgb(100, 100, 130);
const RED: Color32 = Color32::from_rgb(255, 107, 107);
const GREEN: Color32 = Color32::from_rgb(78, 201, 148);
const YELLOW: Color32 = Color32::from_rgb(220, 220, 170);
const BLUE: Color32 = Color32::from_rgb(86, 156, 214);
const MAGENTA: Color32 = Color32::from_rgb(197, 134, 192);
const CYAN: Color32 = Color32::from_rgb(78, 201, 176);

// ── data types ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct StyledChar {
    pub ch: char,
    pub color: Color32,
    pub bold: bool,
}

impl Default for StyledChar {
    fn default() -> Self { Self { ch: ' ', color: TEXT, bold: false } }
}

// ── processor ─────────────────────────────────────────────────────────────────

pub struct OutputProcessor {
    /// Fully completed lines available for rendering.
    pub completed: Vec<Vec<StyledChar>>,
    /// The line currently being written.
    cur_line: Vec<StyledChar>,
    /// Cursor column within cur_line.
    col: usize,
    /// Current foreground colour.
    fg: Color32,
    /// Current bold state.
    bold: bool,
    /// Whether the program has entered an alternate screen.
    pub alternate_screen: bool,
}

impl OutputProcessor {
    pub fn new() -> Self {
        Self {
            completed: Vec::new(),
            cur_line: Vec::new(),
            col: 0,
            fg: TEXT,
            bold: false,
            alternate_screen: false,
        }
    }

    /// Drain and return all completed lines, keeping colour state.
    pub fn take_completed(&mut self) -> Vec<Vec<StyledChar>> {
        std::mem::take(&mut self.completed)
    }

    /// Flush partial current line, then drain everything.
    pub fn take_all(&mut self) -> Vec<Vec<StyledChar>> {
        if !self.cur_line.is_empty() {
            self.completed.push(std::mem::take(&mut self.cur_line));
            self.col = 0;
        }
        std::mem::take(&mut self.completed)
    }

    // ── internal helpers ──────────────────────────────────────────────────────

    fn put_char(&mut self, c: char) {
        let sc = StyledChar { ch: c, color: self.fg, bold: self.bold };
        if self.col < self.cur_line.len() {
            self.cur_line[self.col] = sc;
        } else {
            while self.cur_line.len() < self.col {
                self.cur_line.push(StyledChar::default());
            }
            self.cur_line.push(sc);
        }
        self.col += 1;
    }

    fn newline(&mut self) {
        self.completed.push(std::mem::take(&mut self.cur_line));
        self.col = 0;
    }

    fn sgr(&mut self, codes: &[u16]) {
        let mut i = 0;
        while i < codes.len() {
            match codes[i] {
                0  => { self.fg = TEXT; self.bold = false; }
                1  => self.bold = true,
                2  => self.bold = false,
                22 => self.bold = false,
                30 => self.fg = Color32::from_rgb(0, 0, 0),
                31 => self.fg = RED,
                32 => self.fg = GREEN,
                33 => self.fg = YELLOW,
                34 => self.fg = BLUE,
                35 => self.fg = MAGENTA,
                36 => self.fg = CYAN,
                37 | 97 => self.fg = TEXT,
                38 if i + 2 < codes.len() && codes[i + 1] == 5 => {
                    self.fg = xterm256(codes[i + 2] as u8);
                    i += 2;
                }
                38 if i + 4 < codes.len() && codes[i + 1] == 2 => {
                    self.fg = Color32::from_rgb(
                        codes[i + 2] as u8,
                        codes[i + 3] as u8,
                        codes[i + 4] as u8,
                    );
                    i += 4;
                }
                39 => self.fg = TEXT,
                90 => self.fg = DIM,
                91 => self.fg = Color32::from_rgb(255, 120, 120),
                92 => self.fg = Color32::from_rgb(120, 220, 120),
                93 => self.fg = Color32::from_rgb(220, 220, 100),
                94 => self.fg = Color32::from_rgb(100, 150, 240),
                95 => self.fg = Color32::from_rgb(200, 100, 200),
                96 => self.fg = Color32::from_rgb(100, 210, 210),
                // background colours - ignore for now
                _ => {}
            }
            i += 1;
        }
    }
}

impl Perform for OutputProcessor {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.newline(),
            b'\r' => self.col = 0,
            b'\x08' => { if self.col > 0 { self.col -= 1; } }
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        // Flatten parameters into a plain Vec<u16> for easy matching.
        let codes: Vec<u16> = params.iter().flat_map(|g| g.iter().copied()).collect();
        let first = codes.first().copied().unwrap_or(0);

        // DEC private sequences (intermediates contains b'?')
        if intermediates.contains(&b'?') {
            match action {
                'h' if codes.contains(&1049) || codes.contains(&47) => {
                    self.alternate_screen = true;
                    // Clear buffer when entering alternate screen
                    self.completed.clear();
                    self.cur_line.clear();
                    self.col = 0;
                }
                'l' if codes.contains(&1049) || codes.contains(&47) => {
                    self.alternate_screen = false;
                }
                _ => {} // all other DEC modes silently ignored
            }
            return;
        }

        match action {
            // SGR – colours / styles
            'm' => self.sgr(&codes),

            // Cursor column absolute  ESC [ Pn G
            'G' => {
                self.col = (first as usize).saturating_sub(1);
            }

            // Cursor right  ESC [ Pn C
            'C' => {
                let n = first.max(1) as usize;
                self.col += n;
            }

            // Cursor left  ESC [ Pn D
            'D' => {
                let n = first.max(1) as usize;
                self.col = self.col.saturating_sub(n);
            }

            // Erase in line  ESC [ Ps K
            'K' => match first {
                0 => self.cur_line.truncate(self.col),
                1 => {
                    for i in 0..self.col.min(self.cur_line.len()) {
                        self.cur_line[i] = StyledChar::default();
                    }
                }
                2 => { self.cur_line.clear(); self.col = 0; }
                _ => {}
            },

            // Erase in display  ESC [ Ps J
            'J' => {
                if first == 2 || first == 3 {
                    self.completed.clear();
                    self.cur_line.clear();
                    self.col = 0;
                }
            }

            // Cursor up  ESC [ Pn A  – best-effort for alternate screen
            'A' => {
                let n = first.max(1) as usize;
                for _ in 0..n {
                    if !self.completed.is_empty() {
                        let prev = self.completed.pop().unwrap_or_default();
                        // push current partial line first
                        if !self.cur_line.is_empty() {
                            self.completed.push(std::mem::take(&mut self.cur_line));
                        }
                        self.cur_line = prev;
                        self.col = self.cur_line.len();
                    }
                }
            }

            // Cursor position  ESC [ row ; col H
            'H' | 'f' => {
                let col = codes.get(1).copied().unwrap_or(1);
                self.col = (col as usize).saturating_sub(1);
            }

            // Everything else silently ignored
            _ => {}
        }
    }

    // All other sequences are silently swallowed.
    fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {}
    fn hook(&mut self, _: &vte::Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
    fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}
}

// ── rendering helper ──────────────────────────────────────────────────────────

/// Convert a rendered `Vec<StyledChar>` into an egui `LayoutJob`.
pub fn line_to_layout_job(
    line: &[StyledChar],
    font_size: f32,
) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};
    use egui::FontId;

    let mut job = LayoutJob::default();
    if line.is_empty() {
        // Preserve blank lines so block height is correct.
        job.append(
            " ",
            0.0,
            TextFormat { font_id: FontId::monospace(font_size), color: TEXT, ..Default::default() },
        );
        return job;
    }

    let mut cur_text = String::new();
    let mut cur_color = line[0].color;
    let mut cur_bold = line[0].bold;

    for sc in line {
        if sc.color != cur_color || sc.bold != cur_bold {
            if !cur_text.is_empty() {
                job.append(
                    &cur_text,
                    0.0,
                    TextFormat {
                        font_id: FontId::monospace(font_size),
                        color: cur_color,
                        ..Default::default()
                    },
                );
                cur_text.clear();
            }
            cur_color = sc.color;
            cur_bold = sc.bold;
        }
        cur_text.push(sc.ch);
    }
    if !cur_text.is_empty() {
        job.append(
            &cur_text,
            0.0,
            TextFormat {
                font_id: FontId::monospace(font_size),
                color: cur_color,
                ..Default::default()
            },
        );
    }
    job
}

// ── xterm-256 colour table ────────────────────────────────────────────────────

pub fn xterm256(idx: u8) -> Color32 {
    match idx {
        0  => Color32::from_rgb(0, 0, 0),
        1  => RED,
        2  => GREEN,
        3  => YELLOW,
        4  => BLUE,
        5  => MAGENTA,
        6  => CYAN,
        7  => Color32::from_rgb(192, 192, 192),
        8  => DIM,
        9  => Color32::from_rgb(255, 120, 120),
        10 => Color32::from_rgb(120, 220, 120),
        11 => Color32::from_rgb(220, 220, 100),
        12 => Color32::from_rgb(100, 150, 240),
        13 => Color32::from_rgb(200, 100, 200),
        14 => Color32::from_rgb(100, 210, 210),
        15 => TEXT,
        16..=231 => {
            let i = (idx - 16) as u32;
            let r = (i / 36) * 51;
            let g = ((i / 6) % 6) * 51;
            let b = (i % 6) * 51;
            Color32::from_rgb(r as u8, g as u8, b as u8)
        }
        232..=255 => {
            let v = 8 + (idx - 232) as u32 * 10;
            Color32::from_gray(v as u8)
        }
    }
}

// ── pure-Rust strip_ansi (no vte needed) ─────────────────────────────────────

/// Remove all ANSI / VT100 escape sequences from `s`, returning plain text.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('[') => {
                chars.next(); // consume '['
                // Consume all bytes up to and including the final byte (0x40-0x7E).
                // This correctly handles '?' and other private-mode prefixes.
                for fc in chars.by_ref() {
                    if ('@'..='~').contains(&fc) { break; }
                }
            }
            Some(']') => {
                chars.next(); // OSC – consume until BEL or ESC \
                while let Some(fc) = chars.next() {
                    if fc == '\x07' { break; }
                    if fc == '\x1b' {
                        if chars.peek() == Some(&'\\') { chars.next(); }
                        break;
                    }
                }
            }
            Some(_) => { chars.next(); } // 2-char escape, skip both
            None => {}
        }
    }
    out
}
