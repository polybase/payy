use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

use contextful::ResultContextExt;

use crate::error::Result;
use crate::lint::steps::StepResult;

mod report;

use report::{Violation, render_failure};

const DEFAULT_LIMIT: usize = 200;
const MAX_OVERRIDE_SCAN_LINES: usize = 20;
const TARGET_DIR: &str = "pkg";

pub fn run(repo_root: &Path) -> Result<StepResult> {
    let start = Instant::now();
    let target_dir = repo_root.join(TARGET_DIR);

    if !target_dir.is_dir() {
        return Ok(StepResult::failed(
            "File length",
            format!("pkg directory not found at {}", target_dir.display()),
            start.elapsed(),
        ));
    }

    let files = collect_rust_files(&target_dir)?;
    if files.is_empty() {
        return Ok(StepResult::success(
            "File length",
            "No Rust files found under pkg/.".to_owned(),
            start.elapsed(),
        ));
    }

    let mut checked_files = 0usize;
    let mut violations = Vec::new();

    for path in files {
        match inspect_file(&path)? {
            FileInspection::Generated => {}
            FileInspection::Checked { line_count, limit } => {
                checked_files += 1;
                if line_count > limit {
                    violations.push(Violation {
                        path,
                        line_count,
                        limit,
                    });
                }
            }
        }
    }

    if violations.is_empty() {
        return Ok(StepResult::success(
            "File length",
            format!("Checked {checked_files} Rust file(s); all within length limits"),
            start.elapsed(),
        ));
    }

    let summary = format!("{} file(s) exceed configured limits", violations.len());

    Ok(StepResult::failed("File length", summary, start.elapsed())
        .with_extra_output(render_failure(repo_root, &violations)))
}

fn collect_rust_files(target_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_rust_files_in(target_dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_rust_files_in(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).with_context(|| format!("list {}", dir.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("iterate {}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("read file type for {}", path.display()))?;

        if file_type.is_dir() {
            collect_rust_files_in(&path, files)?;
        } else if file_type.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs")
        {
            files.push(path);
        }
    }

    Ok(())
}

enum FileInspection {
    Generated,
    Checked { line_count: usize, limit: usize },
}

fn inspect_file(path: &Path) -> Result<FileInspection> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    let mut logical_line = 0usize;
    let mut line_count = 0usize;
    let mut limit = DEFAULT_LIMIT;
    let mut override_seen = false;

    loop {
        buffer.clear();
        let bytes_read = reader
            .read_until(b'\n', &mut buffer)
            .with_context(|| format!("read {}", path.display()))?;
        if bytes_read == 0 {
            break;
        }

        logical_line += 1;
        if buffer.ends_with(b"\n") {
            line_count += 1;
        }

        if logical_line == 1 && is_generated_line(&buffer) {
            return Ok(FileInspection::Generated);
        }

        if logical_line <= MAX_OVERRIDE_SCAN_LINES
            && !override_seen
            && let Some(override_limit) = parse_override_limit(&buffer)
        {
            limit = override_limit;
            override_seen = true;
        }
    }

    Ok(FileInspection::Checked { line_count, limit })
}

fn is_generated_line(line: &[u8]) -> bool {
    let line = line_without_ending(line);
    let Some(rest) = line.strip_prefix(b"//") else {
        return false;
    };

    trim_ascii_start(rest).starts_with(b"@generated")
}

fn parse_override_limit(line: &[u8]) -> Option<usize> {
    let mut rest = trim_ascii_start(line_without_ending(line)).strip_prefix(b"//")?;
    rest = trim_ascii_start(rest);
    rest = rest.strip_prefix(b"lint-long-file-override")?;
    rest = strip_required_ascii_whitespace(rest)?;
    rest = rest.strip_prefix(b"allow-max-lines")?;
    rest = trim_ascii_start(rest).strip_prefix(b"=")?;
    rest = trim_ascii_start(rest);

    let digit_count = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
    if digit_count == 0 {
        return None;
    }

    std::str::from_utf8(&rest[..digit_count])
        .ok()?
        .parse::<usize>()
        .ok()
}

fn strip_required_ascii_whitespace(bytes: &[u8]) -> Option<&[u8]> {
    bytes
        .first()
        .filter(|byte| byte.is_ascii_whitespace())
        .map(|_| trim_ascii_start(bytes))
}

fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let first_non_whitespace = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[first_non_whitespace..]
}

fn line_without_ending(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

#[cfg(test)]
mod tests;
