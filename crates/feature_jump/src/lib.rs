use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone)]
struct Entry {
    rank: f64,
    last_visit: u64,
}

pub struct JumpDb {
    entries: HashMap<String, Entry>,
    path: PathBuf,
}

impl Default for JumpDb {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            path: PathBuf::new(),
        }
    }
}

impl JumpDb {
    pub fn load() -> Result<Self> {
        let path = db_path();
        let entries: HashMap<String, Entry> = if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Ok(Self { entries, path })
    }

    pub fn add(&mut self, dir: &Path) {
        let key = dir.to_string_lossy().to_string();
        let now = now_secs();
        let entry = self.entries.entry(key).or_insert(Entry { rank: 0.0, last_visit: now });
        entry.rank += 1.0;
        entry.last_visit = now;
        self.age_entries();
    }

    fn age_entries(&mut self) {
        let now = now_secs();
        for entry in self.entries.values_mut() {
            let hours_inactive = (now.saturating_sub(entry.last_visit)) as f64 / 3600.0;
            let decay = 0.5f64.powf(hours_inactive / 24.0);
            entry.rank *= decay;
        }
        self.entries.retain(|_, e| e.rank > 0.01);
    }

    pub fn query(&self, query: &str) -> Option<PathBuf> {
        let query_lower = query.to_lowercase();
        let mut best: Option<(&str, f64)> = None;
        for (path, entry) in &self.entries {
            let path_lower = path.to_lowercase();
            let parts: Vec<&str> = query_lower.split_whitespace().collect();
            if parts.iter().all(|p| path_lower.contains(p)) {
                let basename = Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let end_bonus =
                    if parts.last().is_some_and(|p| basename.contains(p)) { 2.0 } else { 1.0 };
                let score = entry.rank * end_bonus;
                if best.map(|(_, s)| score > s).unwrap_or(true) {
                    best = Some((path, score));
                }
            }
        }
        best.map(|(p, _)| PathBuf::from(p))
    }

    pub fn save(&self) -> Result<()> {
        if self.path == PathBuf::new() {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }
}

fn db_path() -> PathBuf {
    ProjectDirs::from("", "", "myterm")
        .map(|d| d.data_dir().join("jump.json"))
        .unwrap_or_else(|| PathBuf::from("~/.local/share/myterm/jump.json"))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "jump_tests.rs"]
mod tests;
