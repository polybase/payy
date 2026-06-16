mod selection;

use std::io::ErrorKind;
use std::path::Path;
use std::time::Instant;

use crate::error::{Result, XTaskError};
use crate::lint::LintMode;
use crate::lint::steps::clippy::selection::{
    ClippySelection, build_clippy_args, print_selection, select_clippy_packages,
    selection_failure_detail, selection_success_detail,
};
use crate::lint::steps::{StepResult, run_command_with_env};

pub fn run_clippy(repo_root: &Path, mode: LintMode) -> Result<StepResult> {
    let start = Instant::now();
    let target_dir = repo_root.join("target/clippy");
    let target_dir = target_dir.to_string_lossy();
    let selection = select_clippy_packages(repo_root, mode)?;
    if let ClippySelection::Skip { detail } = &selection {
        return Ok(StepResult::skipped(
            "Cargo clippy",
            detail.clone(),
            start.elapsed(),
        ));
    }

    let Some(args) = build_clippy_args(mode, &selection) else {
        return Ok(StepResult::skipped(
            "Cargo clippy",
            "No clippy command selected".to_owned(),
            start.elapsed(),
        ));
    };

    print_selection(&selection);
    let args_ref = args.iter().map(String::as_str).collect::<Vec<_>>();
    let status = match run_command_with_env(
        repo_root,
        "cargo",
        &args_ref,
        &[("CARGO_TARGET_DIR", target_dir.as_ref())],
    ) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::skipped(
                "Cargo clippy",
                "cargo not found; skipping clippy step".to_owned(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if status.success() {
        Ok(StepResult::success(
            "Cargo clippy",
            selection_success_detail(&selection),
            start.elapsed(),
        ))
    } else {
        Ok(StepResult::failed(
            "Cargo clippy",
            selection_failure_detail(&selection),
            start.elapsed(),
        ))
    }
}
