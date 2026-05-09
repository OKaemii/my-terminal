use egui::Color32;
use myterm_core::{palette, StyledChar};
use vte::Perform;

pub struct OutputProcessor {
    pub completed: Vec<Vec<StyledChar>>,
    cur_line: Vec<StyledChar>,
    col: usize,
    fg: Color32,
    bold: bool,
    pub alternate_screen: bool,
}

impl OutputProcessor {
    pub fn new() -> Self {
        Self {
            completed: Vec::new(),
            cur_line: Vec::new(),
            col: 0,
            fg: palette::TEXT,
            bold: false,
            alternate_screen: false,
        }
    }

    pub fn take_completed(&mut self) -> Vec<Vec<StyledChar>> {
        std::mem::take(&mut self.completed)
    }

    pub fn take_all(&mut self) -> Vec<Vec<StyledChar>> {
        if !self.cur_line.is_empty() {
            self.completed.push(std::mem::take(&mut self.cur_line));
            self.col = 0;
        }
        std::mem::take(&mut self.completed)
    }

    fn put_char(&mut self, c: char) {
        let sc = StyledChar {
            ch: c,
            color: self.fg,
            bold: self.bold,
        };
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
                0 => {
                    self.fg = palette::TEXT;
                    self.bold = false;
                }
                1 => self.bold = true,
                2 | 22 => self.bold = false,
                30 => self.fg = Color32::from_rgb(0, 0, 0),
                31 => self.fg = palette::RED,
                32 => self.fg = palette::GREEN,
                33 => self.fg = palette::YELLOW,
                34 => self.fg = palette::BLUE,
                35 => self.fg = palette::MAGENTA,
                36 => self.fg = palette::CYAN,
                37 | 97 => self.fg = palette::TEXT,
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
                39 => self.fg = palette::TEXT,
                90 => self.fg = palette::DIM,
                91 => self.fg = Color32::from_rgb(255, 120, 120),
                92 => self.fg = Color32::from_rgb(120, 220, 120),
                93 => self.fg = Color32::from_rgb(220, 220, 100),
                94 => self.fg = Color32::from_rgb(100, 150, 240),
                95 => self.fg = Color32::from_rgb(200, 100, 200),
                96 => self.fg = Color32::from_rgb(100, 210, 210),
                _ => {}
            }
            i += 1;
        }
    }
}

impl Default for OutputProcessor {
    fn default() -> Self {
        Self::new()
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
            b'\x08' if self.col > 0 => {
                self.col -= 1;
            }
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
        let codes: Vec<u16> = params.iter().flat_map(|g| g.iter().copied()).collect();
        let first = codes.first().copied().unwrap_or(0);

        if intermediates.contains(&b'?') {
            match action {
                'h' if codes.contains(&1049) || codes.contains(&47) => {
                    self.alternate_screen = true;
                    self.completed.clear();
                    self.cur_line.clear();
                    self.col = 0;
                }
                'l' if codes.contains(&1049) || codes.contains(&47) => {
                    self.alternate_screen = false;
                }
                _ => {}
            }
            return;
        }

        match action {
            'm' => self.sgr(&codes),
            'G' => self.col = (first as usize).saturating_sub(1),
            'C' => self.col += first.max(1) as usize,
            'D' => self.col = self.col.saturating_sub(first.max(1) as usize),
            'K' => match first {
                0 => self.cur_line.truncate(self.col),
                1 => {
                    for i in 0..self.col.min(self.cur_line.len()) {
                        self.cur_line[i] = StyledChar::default();
                    }
                }
                2 => {
                    self.cur_line.clear();
                    self.col = 0;
                }
                _ => {}
            },
            'J' if first == 2 || first == 3 => {
                self.completed.clear();
                self.cur_line.clear();
                self.col = 0;
            }
            'A' => {
                let n = first.max(1) as usize;
                for _ in 0..n {
                    if !self.completed.is_empty() {
                        let prev = self.completed.pop().unwrap_or_default();
                        if !self.cur_line.is_empty() {
                            self.completed.push(std::mem::take(&mut self.cur_line));
                        }
                        self.cur_line = prev;
                        self.col = self.cur_line.len();
                    }
                }
            }
            'H' | 'f' => {
                let col = codes.get(1).copied().unwrap_or(1);
                self.col = (col as usize).saturating_sub(1);
            }
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
    }
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

/// Convert a rendered line into an egui `LayoutJob` for display.
pub fn line_to_layout_job(line: &[StyledChar], font_size: f32) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};
    use egui::FontId;

    let mut job = LayoutJob::default();
    if line.is_empty() {
        job.append(
            " ",
            0.0,
            TextFormat {
                font_id: FontId::monospace(font_size),
                color: palette::TEXT,
                ..Default::default()
            },
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

/// Convert an xterm-256 colour index to `Color32`.
pub fn xterm256(idx: u8) -> Color32 {
    match idx {
        0 => Color32::from_rgb(0, 0, 0),
        1 => palette::RED,
        2 => palette::GREEN,
        3 => palette::YELLOW,
        4 => palette::BLUE,
        5 => palette::MAGENTA,
        6 => palette::CYAN,
        7 => Color32::from_rgb(192, 192, 192),
        8 => palette::DIM,
        9 => Color32::from_rgb(255, 120, 120),
        10 => Color32::from_rgb(120, 220, 120),
        11 => Color32::from_rgb(220, 220, 100),
        12 => Color32::from_rgb(100, 150, 240),
        13 => Color32::from_rgb(200, 100, 200),
        14 => Color32::from_rgb(100, 210, 210),
        15 => palette::TEXT,
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
                chars.next();
                for fc in chars.by_ref() {
                    if ('@'..='~').contains(&fc) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                while let Some(fc) = chars.next() {
                    if fc == '\x07' {
                        break;
                    }
                    if fc == '\x1b' {
                        if chars.peek() == Some(&'\\') {
                            chars.next();
                        }
                        break;
                    }
                }
            }
            Some(_) => {
                chars.next();
            }
            None => {}
        }
    }
    out
}

#[cfg(test)]
#[path = "vte_tests.rs"]
mod tests;
