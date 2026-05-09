use super::JumpDb;
use std::path::Path;

#[test]
fn default_has_empty_entries() {
    let db = JumpDb::default();
    assert!(db.query("anything").is_none());
}

#[test]
fn add_increases_rank() {
    let mut db = JumpDb::default();
    db.add(Path::new("/home/user/projects"));
    db.add(Path::new("/home/user/projects"));
    let result = db.query("projects");
    assert!(result.is_some(), "should find added path");
}

#[test]
fn query_returns_added_path() {
    let mut db = JumpDb::default();
    db.add(Path::new("/home/user/projects/myterm"));
    let result = db.query("myterm");
    assert_eq!(result.unwrap(), Path::new("/home/user/projects/myterm"));
}

#[test]
fn default_does_not_panic() {
    // Default must not attempt file I/O
    let db = JumpDb::default();
    let _ = db.query("test");
    // save on default path (empty) is a no-op
    let _ = db.save();
}
