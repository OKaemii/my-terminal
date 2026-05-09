pub struct Suggester {
    history: Vec<String>,
}

impl Suggester {
    pub fn new(history: Vec<String>) -> Self {
        Self { history }
    }

    /// Return the suffix needed to complete `input` from the most-recent history match.
    /// Returns `None` if no match or `input` is empty.
    pub fn suggest(&self, input: &str) -> Option<String> {
        if input.is_empty() {
            return None;
        }
        self.history
            .iter()
            .rev()
            .find(|h| h.starts_with(input) && h.as_str() != input)
            .map(|h| h[input.len()..].to_string())
    }

    pub fn add(&mut self, cmd: String) {
        if cmd.is_empty() {
            return;
        }
        self.history.retain(|h| h != &cmd);
        self.history.push(cmd);
        if self.history.len() > 10_000 {
            self.history.drain(..1);
        }
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }
}

impl Default for Suggester {
    fn default() -> Self {
        Self { history: Vec::new() }
    }
}

#[cfg(test)]
#[path = "suggest_tests.rs"]
mod tests;
