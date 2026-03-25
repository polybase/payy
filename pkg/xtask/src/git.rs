use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;
use std::process::Command;

use contextful::ResultContextExt;

use crate::error::{Result, XTaskError};

pub fn collect_changed_files(repo_root: &Path) -> Result<BTreeSet<String>> {
    let mut files = BTreeSet::new();

    let status_output = git_output(repo_root, &["status", "--porcelain"])?;
    for line in status_output.lines() {
        if line.len() < 4 {
            continue;
        }
        if !include_unstaged_status(line) {
            continue;
        }
        let path = line[3..].trim();
        let path = if let Some(idx) = path.find(" -> ") {
            path[idx + 4..].trim()
        } else {
            path
        };
        if !path.is_empty() {
            files.insert(path.to_string());
        }
    }

    let unstaged_output = git_output(repo_root, &["diff", "--name-only"])?;
    for line in unstaged_output.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            files.insert(trimmed.to_string());
        }
    }

    Ok(files)
}

pub fn capture_file_contents(
    repo_root: &Path,
    paths: &BTreeSet<String>,
) -> Result<HashMap<String, Vec<u8>>> {
    let mut map = HashMap::new();
    for path in paths {
        let full_path = repo_root.join(path);
        if full_path.is_file() {
            let bytes = fs::read(&full_path)
                .with_context(|| format!("read {} for lint capture", full_path.display()))?;
            map.insert(path.clone(), bytes);
        }
    }
    Ok(map)
}

pub fn summarize_file_updates(
    repo_root: &Path,
    before_set: &BTreeSet<String>,
    before_contents: &HashMap<String, Vec<u8>>,
    after_set: &BTreeSet<String>,
) -> Result<Vec<String>> {
    let mut changed = BTreeSet::new();

    for path in before_set {
        if !after_set.contains(path) {
            changed.insert(path.clone());
            continue;
        }
        if let Some(before_bytes) = before_contents.get(path) {
            let full_path = repo_root.join(path);
            let current_bytes = fs::read(&full_path)
                .with_context(|| format!("read {} for lint comparison", full_path.display()))?;
            if &current_bytes != before_bytes {
                changed.insert(path.clone());
            }
        }
    }

    for path in after_set {
        if !before_set.contains(path) {
            changed.insert(path.clone());
        }
    }

    Ok(changed.into_iter().collect())
}

fn git_output(repo_root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .with_context(|| format!("spawn git command with args {args:?}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(XTaskError::GitCommand {
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            stderr,
        });
    }

    Ok(String::from_utf8(output.stdout).context("parse git stdout as utf-8")?)
}

fn include_unstaged_status(line: &str) -> bool {
    if line.len() < 2 {
        return false;
    }
    let mut chars = line.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let Some(second) = chars.next() else {
        return false;
    };

    match (first, second) {
        ('?', '?') => true,
        ('!', '!') => false,
        (_, status) => status != ' ',
    }
}
