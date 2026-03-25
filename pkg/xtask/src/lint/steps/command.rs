use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use contextful::ResultContextExt;

use crate::error::Result;

pub fn run_command(repo_root: &Path, program: &'static str, args: &[&str]) -> Result<ExitStatus> {
    let mut command = Command::new(program);
    command.current_dir(repo_root);
    command.args(args);

    if program == "taplo" {
        // Taplo logs at info level by default; force a quieter level for lint output.
        command.env("TAPLO_LOG", "warn");
        command.env("RUST_LOG", "warn");
    }

    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    Ok(command
        .status()
        .with_context(|| format!("spawn {program} with args {args:?}"))?)
}
