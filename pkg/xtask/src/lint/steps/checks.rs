use std::io::ErrorKind;
use std::path::Path;
use std::time::Instant;

use crate::error::{Result, XTaskError};

use crate::lint::i18n;
use crate::lint::steps::{StepResult, run_command};

pub fn run_taplo_check(repo_root: &Path) -> Result<StepResult> {
    let start = Instant::now();
    let status = match run_command(repo_root, "taplo", &["check"]) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "TOML validation",
                "taplo not installed; skipping validation step".to_string(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if status.success() {
        Ok(StepResult::success(
            "TOML validation",
            "Configuration files validated successfully".to_string(),
            start.elapsed(),
        ))
    } else {
        Ok(StepResult::failed(
            "TOML validation",
            "taplo check reported issues".to_string(),
            start.elapsed(),
        ))
    }
}

pub fn run_ast_grep(repo_root: &Path) -> Result<StepResult> {
    let start = Instant::now();
    let status = match run_command(repo_root, "ast-grep", &["scan"]) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "AST-grep",
                "ast-grep not installed; skipping scan".to_string(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if status.success() {
        Ok(StepResult::success(
            "AST-grep",
            "No violations found".to_string(),
            start.elapsed(),
        ))
    } else {
        Ok(StepResult::failed(
            "AST-grep",
            "ast-grep reported violations".to_string(),
            start.elapsed(),
        ))
    }
}

pub fn run_file_length(repo_root: &Path) -> Result<StepResult> {
    let start = Instant::now();
    let status = match run_command(repo_root, "scripts/check-file-length.sh", &[]) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "File length",
                "scripts/check-file-length.sh not found; skipping length check".to_string(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if status.success() {
        Ok(StepResult::success(
            "File length",
            "All files within length limits".to_string(),
            start.elapsed(),
        ))
    } else {
        Ok(StepResult::failed(
            "File length",
            "scripts/check-file-length.sh reported issues".to_string(),
            start.elapsed(),
        ))
    }
}

pub fn run_i18n_consistency(repo_root: &Path) -> Result<StepResult> {
    i18n::run(repo_root)
}

pub fn run_clippy(repo_root: &Path) -> Result<StepResult> {
    let start = Instant::now();
    let status = match run_command(
        repo_root,
        "cargo",
        &["clippy", "--all-targets", "--quiet", "--", "-D", "warnings"],
    ) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "Cargo clippy",
                "cargo not found; skipping clippy step".to_string(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if status.success() {
        Ok(StepResult::success(
            "Cargo clippy",
            "All checks passed".to_string(),
            start.elapsed(),
        ))
    } else {
        Ok(StepResult::failed(
            "Cargo clippy",
            "cargo clippy reported issues".to_string(),
            start.elapsed(),
        ))
    }
}
