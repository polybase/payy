use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use contextful::{ErrorContextExt, ResultContextExt};

use crate::error::Result;

use crate::lint::LintMode;
use crate::lint::steps::StepResult;

pub fn run_claude_doc(repo_root: &Path, mode: LintMode) -> Result<StepResult> {
    let start = Instant::now();
    let script_path = repo_root.join("CLAUDE.md.sh");
    let target_path = repo_root.join("CLAUDE.md");

    if !script_path.is_file() {
        return Ok(StepResult::failed(
            "CLAUDE.md",
            "CLAUDE.md.sh was not found in the repository root".to_string(),
            start.elapsed(),
        ));
    }

    let output = Command::new("bash")
        .arg(&script_path)
        .current_dir(repo_root)
        .output()
        .with_context(|| format!("spawn bash to run {}", script_path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let step = StepResult::failed(
            "CLAUDE.md",
            "CLAUDE.md.sh exited with a non-zero status code".to_string(),
            start.elapsed(),
        );
        return if stderr.is_empty() {
            Ok(step)
        } else {
            Ok(step.with_extra_output(vec![stderr]))
        };
    }

    let generated = output.stdout;
    let existing = read_optional_file(&target_path)?;

    match mode {
        LintMode::AutoFix => {
            let needs_update = existing
                .as_ref()
                .map(|bytes| bytes != &generated)
                .unwrap_or(true);

            if needs_update {
                fs::write(&target_path, &generated)
                    .with_context(|| format!("write {}", target_path.display()))?;

                Ok(StepResult::fixed(
                    "CLAUDE.md",
                    "Regenerated CLAUDE.md from CLAUDE.md.sh".to_string(),
                    vec!["CLAUDE.md".to_owned()],
                    start.elapsed(),
                ))
            } else {
                Ok(StepResult::success(
                    "CLAUDE.md",
                    "CLAUDE.md already matches CLAUDE.md.sh".to_string(),
                    start.elapsed(),
                ))
            }
        }
        LintMode::CheckOnly => match existing {
            Some(bytes) if bytes == generated => Ok(StepResult::success(
                "CLAUDE.md",
                "CLAUDE.md already matches CLAUDE.md.sh".to_string(),
                start.elapsed(),
            )),
            Some(_) => Ok(StepResult::failed(
                "CLAUDE.md",
                "CLAUDE.md differs from CLAUDE.md.sh output. Re-run cargo xtask lint --fix."
                    .to_string(),
                start.elapsed(),
            )),
            None => Ok(StepResult::failed(
                "CLAUDE.md",
                "CLAUDE.md is missing; run cargo xtask lint --fix to regenerate it.".to_string(),
                start.elapsed(),
            )),
        },
    }
}

fn read_optional_file(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.wrap_err_with(|| format!("read {}", path.display())))?,
    }
}
