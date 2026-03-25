use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::Path;
use std::time::Instant;

use crate::error::{Result, XTaskError};
use crate::git::{capture_file_contents, collect_changed_files, summarize_file_updates};

use crate::lint::LintMode;
use crate::lint::steps::{StepResult, run_command};

pub fn run_rustfmt(repo_root: &Path, mode: LintMode) -> Result<StepResult> {
    let start = Instant::now();
    let check_status = match run_command(repo_root, "cargo", &["fmt", "--check"]) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "Rust formatting",
                "cargo not found; skipping rustfmt step".to_string(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if check_status.success() {
        return Ok(StepResult::success(
            "Rust formatting",
            "All files formatted correctly".to_string(),
            start.elapsed(),
        ));
    }

    if matches!(mode, LintMode::CheckOnly) {
        return Ok(StepResult::failed(
            "Rust formatting",
            "Formatting required. Re-run with --fix to apply changes.".to_string(),
            start.elapsed(),
        ));
    }

    let before_set = collect_changed_files(repo_root)?
        .into_iter()
        .filter(|path| path.ends_with(".rs"))
        .collect::<BTreeSet<_>>();
    let before_contents = capture_file_contents(repo_root, &before_set)?;

    let fmt_status = match run_command(repo_root, "cargo", &["fmt"]) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "Rust formatting",
                "cargo not found; skipping rustfmt step".to_string(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if !fmt_status.success() {
        return Ok(StepResult::failed(
            "Rust formatting",
            "Failed to apply rustfmt".to_string(),
            start.elapsed(),
        ));
    }

    let after_set = collect_changed_files(repo_root)?
        .into_iter()
        .filter(|path| path.ends_with(".rs"))
        .collect::<BTreeSet<_>>();

    let mut formatted =
        summarize_file_updates(repo_root, &before_set, &before_contents, &after_set)?;
    if formatted.is_empty() && !before_set.is_empty() {
        formatted = before_set.iter().cloned().collect();
    }

    let detail = if formatted.is_empty() {
        "Applied rustfmt".to_string()
    } else {
        format!("Applied rustfmt to {} file(s)", formatted.len())
    };

    Ok(StepResult::fixed(
        "Rust formatting",
        detail,
        formatted,
        start.elapsed(),
    ))
}

pub fn run_taplo_fmt(repo_root: &Path, mode: LintMode) -> Result<StepResult> {
    let start = Instant::now();
    let check_status = match run_command(repo_root, "taplo", &["fmt", "--check"]) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "TOML formatting",
                "taplo not installed; skipping format step".to_string(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if check_status.success() {
        return Ok(StepResult::success(
            "TOML formatting",
            "All TOML files formatted correctly".to_string(),
            start.elapsed(),
        ));
    }

    if matches!(mode, LintMode::CheckOnly) {
        return Ok(StepResult::failed(
            "TOML formatting",
            "Formatting required. Re-run with --fix to apply changes.".to_string(),
            start.elapsed(),
        ));
    }

    let before_set = collect_changed_files(repo_root)?
        .into_iter()
        .filter(|path| path.ends_with(".toml"))
        .collect::<BTreeSet<_>>();
    let before_contents = capture_file_contents(repo_root, &before_set)?;

    let fmt_status = match run_command(repo_root, "taplo", &["fmt"]) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "TOML formatting",
                "taplo not installed; skipping format step".to_string(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if !fmt_status.success() {
        return Ok(StepResult::failed(
            "TOML formatting",
            "Failed to apply taplo formatting".to_string(),
            start.elapsed(),
        ));
    }

    let after_set = collect_changed_files(repo_root)?
        .into_iter()
        .filter(|path| path.ends_with(".toml"))
        .collect::<BTreeSet<_>>();

    let mut formatted =
        summarize_file_updates(repo_root, &before_set, &before_contents, &after_set)?;
    if formatted.is_empty() && !before_set.is_empty() {
        formatted = before_set.iter().cloned().collect();
    }

    let detail = if formatted.is_empty() {
        "Applied taplo formatting".to_string()
    } else {
        format!("Applied taplo formatting to {} file(s)", formatted.len())
    };

    Ok(StepResult::fixed(
        "TOML formatting",
        detail,
        formatted,
        start.elapsed(),
    ))
}
