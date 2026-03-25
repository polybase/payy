use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::Path;
use std::time::Instant;

use crate::error::{Result, XTaskError};
use crate::git::{capture_file_contents, collect_changed_files, summarize_file_updates};

use crate::lint::LintMode;
use crate::lint::steps::{StepResult, run_command};

fn ensure_hakari_installed(repo_root: &Path, start: Instant) -> Result<Option<StepResult>> {
    let version_status = match run_command(repo_root, "cargo", &["hakari", "--version"]) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(Some(StepResult::skipped(
                "Cargo Hakari",
                "cargo not found; skipping hakari step".to_string(),
                start.elapsed(),
            )));
        }
        Err(error) => return Err(error),
    };

    if version_status.success() {
        return Ok(None);
    }

    if version_status.code() == Some(101) {
        return Ok(Some(StepResult::skipped(
            "Cargo Hakari",
            "cargo hakari not installed; install via `cargo install cargo-hakari --locked`."
                .to_string(),
            start.elapsed(),
        )));
    }

    Ok(Some(StepResult::failed(
        "Cargo Hakari",
        "cargo hakari --version failed; see output above.".to_string(),
        start.elapsed(),
    )))
}

pub fn run_hakari(repo_root: &Path, mode: LintMode) -> Result<StepResult> {
    let start = Instant::now();

    if let Some(step) = ensure_hakari_installed(repo_root, start)? {
        return Ok(step);
    }

    if matches!(mode, LintMode::CheckOnly) {
        return run_hakari_check_only(repo_root, start);
    }

    run_hakari_manage_deps(repo_root, start)
}

fn run_hakari_check_only(repo_root: &Path, start: Instant) -> Result<StepResult> {
    let generate_status = match run_command(repo_root, "cargo", &["hakari", "generate", "--diff"]) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "Cargo Hakari",
                "cargo not found; skipping hakari step".to_string(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if !generate_status.success() {
        return Ok(StepResult::failed(
            "Cargo Hakari",
            "workspace-hack crate is out of date; run `cargo hakari generate`.".to_string(),
            start.elapsed(),
        ));
    }

    let manage_status =
        match run_command(repo_root, "cargo", &["hakari", "manage-deps", "--dry-run"]) {
            Ok(status) => status,
            Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
                return Ok(StepResult::skipped(
                    "Cargo Hakari",
                    "cargo not found; skipping hakari step".to_string(),
                    start.elapsed(),
                ));
            }
            Err(error) => return Err(error),
        };

    if !manage_status.success() {
        return Ok(StepResult::failed(
            "Cargo Hakari",
            "workspace-hack dependencies need updates; run `cargo hakari manage-deps --yes`."
                .to_string(),
            start.elapsed(),
        ));
    }

    Ok(StepResult::success(
        "Cargo Hakari",
        "Workspace hack crate is up to date".to_string(),
        start.elapsed(),
    ))
}

fn run_hakari_manage_deps(repo_root: &Path, start: Instant) -> Result<StepResult> {
    let before_set = collect_changed_files(repo_root)?
        .into_iter()
        .filter(|path| is_relevant_path(path))
        .collect::<BTreeSet<_>>();
    let before_contents = capture_file_contents(repo_root, &before_set)?;

    let generate_status = match run_command(repo_root, "cargo", &["hakari", "generate"]) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "Cargo Hakari",
                "cargo not found; skipping hakari step".to_string(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if !generate_status.success() {
        return Ok(StepResult::failed(
            "Cargo Hakari",
            "cargo hakari generate failed; see output above.".to_string(),
            start.elapsed(),
        ));
    }

    let manage_status = match run_command(repo_root, "cargo", &["hakari", "manage-deps", "--yes"]) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "Cargo Hakari",
                "cargo not found; skipping hakari step".to_string(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if !manage_status.success() {
        return Ok(StepResult::failed(
            "Cargo Hakari",
            "cargo hakari manage-deps failed; see output above.".to_string(),
            start.elapsed(),
        ));
    }

    let after_set = collect_changed_files(repo_root)?
        .into_iter()
        .filter(|path| is_relevant_path(path))
        .collect::<BTreeSet<_>>();

    let updated = summarize_file_updates(repo_root, &before_set, &before_contents, &after_set)?;

    if updated.is_empty() {
        Ok(StepResult::success(
            "Cargo Hakari",
            "Workspace hack crate already up to date".to_string(),
            start.elapsed(),
        ))
    } else {
        let detail = format!("Updated {} file(s) via cargo hakari", updated.len());
        Ok(StepResult::fixed(
            "Cargo Hakari",
            detail,
            updated,
            start.elapsed(),
        ))
    }
}

fn is_relevant_path(path: &str) -> bool {
    path.ends_with(".toml") || path == "Cargo.lock" || path.contains("workspace-hack")
}
