use std::fs;
use std::io;
use std::path::Path;

use contextful::{ErrorContextExt, ResultContextExt};
use duct::cmd;
use sha2::{Digest, Sha256};

use crate::error::{Result, XTaskError};
use crate::setup::run_expression;

const SPEC_LINT_DIR: &str = "docs/tools/spec-lint";
const PACKAGE_LOCK: &str = "package-lock.json";
const LOCK_HASH_STAMP: &str = ".xtask-package-lock.sha256";

pub fn ensure_spec_lint(repo_root: &Path) -> Result<()> {
    let spec_lint_dir = repo_root.join(SPEC_LINT_DIR);
    ensure_directory(&spec_lint_dir, "spec-lint workspace directory")?;

    let package_lock = spec_lint_dir.join(PACKAGE_LOCK);
    ensure_file(&package_lock, "spec-lint package-lock.json")?;

    let node_modules = spec_lint_dir.join("node_modules");
    let expected_hash = file_sha256(&package_lock)?;
    let stamp = node_modules.join(LOCK_HASH_STAMP);

    if node_modules.is_dir() && stamp_matches(&stamp, &expected_hash)? {
        eprintln!("spec-lint dependencies already installed");
        return Ok(());
    }

    eprintln!("Installing spec-lint dependencies with npm ci...");
    run_expression("npm", cmd("npm", ["ci"]).dir(&spec_lint_dir))?;
    fs::write(&stamp, expected_hash)
        .with_context(|| format!("write spec-lint lock hash stamp at {}", stamp.display()))?;
    eprintln!("spec-lint dependencies installed");
    Ok(())
}

fn ensure_directory(path: &Path, label: &'static str) -> Result<()> {
    if path.is_dir() {
        return Ok(());
    }

    let source = io::Error::new(io::ErrorKind::NotFound, label)
        .wrap_err_with(|| format!("{label} missing at {}", path.display()));
    Err(XTaskError::Io(source))
}

fn ensure_file(path: &Path, label: &'static str) -> Result<()> {
    if path.is_file() {
        return Ok(());
    }

    let source = io::Error::new(io::ErrorKind::NotFound, label)
        .wrap_err_with(|| format!("{label} missing at {}", path.display()));
    Err(XTaskError::Io(source))
}

fn file_sha256(path: &Path) -> Result<String> {
    let contents = fs::read(path)
        .with_context(|| format!("read spec-lint lock file at {}", path.display()))?;
    let digest = Sha256::digest(contents);
    Ok(hex::encode(digest))
}

fn stamp_matches(path: &Path, expected_hash: &str) -> Result<bool> {
    if !path.is_file() {
        return Ok(false);
    }

    let actual_hash = fs::read_to_string(path)
        .with_context(|| format!("read spec-lint lock hash stamp at {}", path.display()))?;
    Ok(actual_hash.trim() == expected_hash)
}
