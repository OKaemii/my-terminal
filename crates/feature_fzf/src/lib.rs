use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

pub struct FzfState {
    pub open: bool,
    pub query: String,
    pub results: Vec<String>,
    pub selected: usize,
    matcher: SkimMatcherV2,
}

impl FzfState {
    pub fn new() -> Self {
        Self {
            open: false,
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn open(&mut self, history: &[String]) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.results = history.iter().rev().cloned().collect();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.results.clear();
        self.selected = 0;
    }

    pub fn update_query(&mut self, history: &[String], query: &str) {
        self.query = query.to_string();
        self.results = filter(history, query, &self.matcher);
        self.selected = self.selected.min(self.results.len().saturating_sub(1));
    }

    pub fn selected_entry(&self) -> Option<&str> {
        self.results.get(self.selected).map(|s| s.as_str())
    }

    pub fn move_up(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
        }
    }

    pub fn move_down(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
}

impl Default for FzfState {
    fn default() -> Self {
        Self::new()
    }
}

/// Fuzzy-filter `history` by `query`, returning matches sorted by score (best first).
/// If `query` is empty, returns all entries in reverse order (most recent first).
pub fn filter(history: &[String], query: &str, matcher: &SkimMatcherV2) -> Vec<String> {
    if query.is_empty() {
        return history.iter().rev().cloned().collect();
    }
    let mut scored: Vec<(i64, &String)> = history
        .iter()
        .filter_map(|h| matcher.fuzzy_match(h, query).map(|s| (s, h)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, h)| h.clone()).collect()
}
