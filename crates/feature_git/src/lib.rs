use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct GitInfo {
    pub branch: String,
    pub is_dirty: bool,
    pub ahead: u32,
    pub behind: u32,
}

pub fn get_info(dir: &Path) -> Option<GitInfo> {
    let root = Command::new("git")
        .args(["-C", dir.to_str()?, "rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !root.status.success() {
        return None;
    }

    let branch_out = Command::new("git")
        .args(["-C", dir.to_str()?, "symbolic-ref", "--short", "HEAD"])
        .output()
        .ok()?;
    let branch = if branch_out.status.success() {
        String::from_utf8_lossy(&branch_out.stdout)
            .trim()
            .to_string()
    } else {
        let hash = Command::new("git")
            .args(["-C", dir.to_str()?, "rev-parse", "--short", "HEAD"])
            .output()
            .ok()?;
        format!(":{}", String::from_utf8_lossy(&hash.stdout).trim())
    };

    let status = Command::new("git")
        .args(["-C", dir.to_str()?, "status", "--porcelain"])
        .output()
        .ok()?;
    let is_dirty = !status.stdout.is_empty();

    let (ahead, behind) = get_ahead_behind(dir).unwrap_or((0, 0));

    Some(GitInfo {
        branch,
        is_dirty,
        ahead,
        behind,
    })
}

fn get_ahead_behind(dir: &Path) -> Option<(u32, u32)> {
    let out = Command::new("git")
        .args([
            "-C",
            dir.to_str()?,
            "rev-list",
            "--left-right",
            "--count",
            "HEAD...@{upstream}",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let mut parts = s.split_whitespace();
    let ahead: u32 = parts.next()?.parse().ok()?;
    let behind: u32 = parts.next()?.parse().ok()?;
    Some((ahead, behind))
}
