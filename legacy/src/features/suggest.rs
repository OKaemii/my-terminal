pub struct Suggester {
    history: Vec<String>,
}

impl Suggester {
    pub fn new(history: Vec<String>) -> Self {
        Self { history }
    }

    pub fn suggest(&self, input: &str) -> Option<String> {
        if input.is_empty() {
            return None;
        }
        // Search history in reverse (most recent first)
        self.history
            .iter()
            .rev()
            .find(|h| h.starts_with(input) && h.as_str() != input)
            .map(|h| h[input.len()..].to_string())
    }

    pub fn add(&mut self, cmd: String) {
        if cmd.is_empty() { return; }
        // Deduplicate: remove older duplicate entry
        self.history.retain(|h| h != &cmd);
        self.history.push(cmd);
        // Cap history at 10000 entries
        if self.history.len() > 10000 {
            self.history.drain(..1);
        }
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }
}
