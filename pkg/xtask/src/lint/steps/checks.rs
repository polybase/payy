use std::io::ErrorKind;
use std::path::Path;
use std::time::Instant;

use crate::error::{Result, XTaskError};

use crate::lint::file_length;
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
    file_length::run(repo_root)
}

pub fn run_i18n_consistency(repo_root: &Path) -> Result<StepResult> {
    i18n::run(repo_root)
}
