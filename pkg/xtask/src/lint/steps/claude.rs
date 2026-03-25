use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use contextful::{ErrorContextExt, ResultContextExt};

use crate::error::Result;

use crate::lint::LintMode;
use crate::lint::steps::StepResult;

const CANONICAL_FILE: &str = "GENERATED_AI_GUIDANCE.md";

pub fn run_claude_doc(repo_root: &Path, mode: LintMode) -> Result<StepResult> {
    let start = Instant::now();
    let script_path = repo_root.join("GENERATED_AI_GUIDANCE.sh");
    let target_path = repo_root.join(CANONICAL_FILE);

    if !script_path.is_file() {
        return Ok(StepResult::failed(
            CANONICAL_FILE,
            "GENERATED_AI_GUIDANCE.sh was not found in the repository root".to_string(),
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
            CANONICAL_FILE,
            "GENERATED_AI_GUIDANCE.sh exited with a non-zero status code".to_string(),
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
                    CANONICAL_FILE,
                    format!(
                        "Regenerated {} from GENERATED_AI_GUIDANCE.sh",
                        CANONICAL_FILE
                    ),
                    vec![CANONICAL_FILE.to_owned()],
                    start.elapsed(),
                ))
            } else {
                Ok(StepResult::success(
                    CANONICAL_FILE,
                    format!(
                        "{} already matches GENERATED_AI_GUIDANCE.sh",
                        CANONICAL_FILE
                    ),
                    start.elapsed(),
                ))
            }
        }
        LintMode::CheckOnly => match existing {
            Some(bytes) if bytes == generated => Ok(StepResult::success(
                CANONICAL_FILE,
                format!(
                    "{} already matches GENERATED_AI_GUIDANCE.sh",
                    CANONICAL_FILE
                ),
                start.elapsed(),
            )),
            Some(_) => Ok(StepResult::failed(
                CANONICAL_FILE,
                format!(
                    "{} differs from GENERATED_AI_GUIDANCE.sh output. Re-run cargo xtask lint --fix.",
                    CANONICAL_FILE
                ),
                start.elapsed(),
            )),
            None => Ok(StepResult::failed(
                CANONICAL_FILE,
                format!(
                    "{} is missing; run cargo xtask lint --fix to regenerate it.",
                    CANONICAL_FILE
                ),
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
