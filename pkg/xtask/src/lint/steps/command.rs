use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use contextful::ResultContextExt;

use crate::error::Result;

const CARGO_PACKAGE_ENV_VARS: &[&str] = &[
    "CARGO_BIN_NAME",
    "CARGO_CRATE_NAME",
    "CARGO_MANIFEST_DIR",
    "CARGO_MANIFEST_LINKS",
    "CARGO_MANIFEST_PATH",
    "CARGO_PRIMARY_PACKAGE",
    "OUT_DIR",
];

pub fn run_command(repo_root: &Path, program: &'static str, args: &[&str]) -> Result<ExitStatus> {
    run_command_with_env(repo_root, program, args, &[])
}

pub fn run_command_with_env(
    repo_root: &Path,
    program: &'static str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<ExitStatus> {
    let mut command = Command::new(program);
    command.current_dir(repo_root);
    command.args(args);

    // Nested tools should not observe xtask's own Cargo package metadata.
    for (key, _) in std::env::vars() {
        if key.starts_with("CARGO_PKG_") {
            command.env_remove(key);
        }
    }
    for key in CARGO_PACKAGE_ENV_VARS {
        command.env_remove(key);
    }
    for (key, value) in envs {
        command.env(key, value);
    }

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
