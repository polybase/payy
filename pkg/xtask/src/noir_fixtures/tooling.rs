use std::process::{Command, Output};

use contextful::ResultContextExt;

use crate::error::{Result, XTaskError};

pub(super) fn run_checked(program: &'static str, command: &mut Command) -> Result<Output> {
    let output = command
        .output()
        .with_context(|| format!("run {program} command"))?;

    if output.status.success() {
        return Ok(output);
    }

    let stderr = if output.stderr.is_empty() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).to_string()
    };

    Err(XTaskError::CommandFailure {
        program,
        status: output.status.code(),
        stderr,
    })
}
