use std::io;
use std::path::Path;

use contextful::ErrorContextExt;
use duct::cmd;

use crate::error::{Result, XTaskError};

use crate::setup::run_expression;

pub fn ensure_eth(repo_root: &Path) -> Result<()> {
    let eth_dir = repo_root.join("eth");
    if !eth_dir.is_dir() {
        let source = io::Error::new(io::ErrorKind::NotFound, "eth directory")
            .wrap_err_with(|| format!("eth workspace directory missing at {}", eth_dir.display()));
        return Err(XTaskError::Io(source));
    }

    let node_modules = eth_dir.join("node_modules");
    if node_modules.is_dir() {
        eprintln!("eth dependencies already installed");
        return Ok(());
    }

    eprintln!("Installing eth workspace dependencies with yarn...");
    run_expression(
        "yarn",
        cmd("yarn", ["install", "--frozen-lockfile"]).dir(&eth_dir),
    )?;
    eprintln!("eth dependencies installed");
    Ok(())
}
