use std::path::Path;

use duct::cmd;

use crate::error::{Result, XTaskError};

use crate::setup::run_expression;

pub fn ensure_params(repo_root: &Path) -> Result<()> {
    eprintln!("Ensuring fixture params...");

    let script = repo_root.join("scripts/download-fixtures-params.sh");
    let script_path = script.to_str().ok_or_else(|| XTaskError::NonUtf8Path {
        path: script.clone(),
    })?;

    run_expression("download-fixtures-params", cmd!("bash", script_path))?;

    Ok(())
}
