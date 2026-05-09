use super::Suggester;

#[test]
fn empty_history_returns_none() {
    let s = Suggester::default();
    assert_eq!(s.suggest("git"), None);
}

#[test]
fn prefix_match_returns_suffix() {
    let s = Suggester::new(vec!["git commit".to_string(), "git push".to_string()]);
    // "git c" matches "git commit", suffix = "ommit"
    assert_eq!(s.suggest("git c"), Some("ommit".to_string()));
}

#[test]
fn most_recent_entry_wins() {
    let s = Suggester::new(vec!["git commit".to_string(), "git config".to_string()]);
    // "git co" matches both; most recent (config) wins
    assert_eq!(s.suggest("git co"), Some("nfig".to_string()));
}

#[test]
fn exact_match_returns_none() {
    let s = Suggester::new(vec!["git commit".to_string()]);
    assert_eq!(s.suggest("git commit"), None);
}

#[test]
fn add_deduplicates() {
    let mut s = Suggester::default();
    s.add("git commit".to_string());
    s.add("git push".to_string());
    s.add("git commit".to_string()); // duplicate — moves to end
    assert_eq!(s.history().len(), 2);
    assert_eq!(s.history().last().unwrap(), "git commit");
}
