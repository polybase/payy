use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use contextful::ResultContextExt;
use duct::cmd;
use tempfile::tempdir;
use which::which;

use crate::error::{Result, XTaskError};
use crate::setup::checksum::verify_sha256;

use crate::setup::{Platform, path_to_string, run_expression};

// TODO: bb --version gives the version 00000000.00000000.00000000 instead of 3.0.0-nightly.20251016
const VERSION: &str = "3.0.0-manual.20251030";
const RELEASE_TAG: &str = "v3.0.0-manual.20251030";
const RELEASE_BASE: &str = "https://storage.googleapis.com/payy-public-fixtures/bb";

pub struct BbOutcome {
    pub installed: bool,
}

pub fn ensure_bb(cargo_bin: &Path, platform: Platform) -> Result<BbOutcome> {
    if let Some(current) = current_version()? {
        if current.trim().trim_start_matches('v') == VERSION.trim_start_matches('v') {
            eprintln!("bb already installed (version {VERSION})");
            return Ok(BbOutcome { installed: false });
        }
        eprintln!("bb version {current} found but {VERSION} is required. Reinstalling...");
    } else {
        eprintln!("Installing bb {VERSION}...");
    }

    install_bb(cargo_bin, platform)?;
    eprintln!("bb installed successfully");
    Ok(BbOutcome { installed: true })
}

fn current_version() -> Result<Option<String>> {
    if which("bb").is_err() {
        return Ok(None);
    }
    let output = run_expression("bb", cmd("bb", ["--version"]))?;
    let version = String::from_utf8(output.stdout).context("bb version output as UTF-8")?;
    Ok(Some(version))
}

fn install_bb(cargo_bin: &Path, platform: Platform) -> Result<()> {
    let asset = asset_for(platform);
    let url = format!("{RELEASE_BASE}/{RELEASE_TAG}/{}", asset.filename);

    let temp_dir = tempdir().context("create temporary directory for bb install")?;
    let archive_path = temp_dir.path().join(asset.filename);
    let archive_str = path_to_string(&archive_path)?;
    let temp_dir_str = path_to_string(temp_dir.path())?;

    run_expression(
        "curl",
        cmd("curl", ["-fsSL", url.as_str(), "-o", archive_str.as_str()]),
    )?;

    verify_sha256(&archive_path, asset.archive_sha256)?;

    run_expression(
        "tar",
        cmd(
            "tar",
            ["-xzf", archive_str.as_str(), "-C", temp_dir_str.as_str()],
        ),
    )?;

    let source = temp_dir.path().join("bb");
    if !source.exists() {
        return Err(XTaskError::ArchiveMissingBinary {
            path: source.clone(),
        });
    }

    let target_tmp = cargo_bin.join("bb.tmp");
    let target = cargo_bin.join("bb");

    fs::copy(&source, &target_tmp).with_context(|| {
        format!(
            "copy bb binary from {} to {}",
            source.display(),
            target_tmp.display()
        )
    })?;

    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&target_tmp)
            .with_context(|| format!("read metadata for {}", target_tmp.display()))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_tmp, perms)
            .with_context(|| format!("set permissions for {}", target_tmp.display()))?;
    }

    fs::rename(&target_tmp, &target).with_context(|| {
        format!(
            "move bb binary from {} to {}",
            target_tmp.display(),
            target.display()
        )
    })?;

    Ok(())
}

struct AssetInfo {
    filename: &'static str,
    archive_sha256: &'static str,
}

fn asset_for(platform: Platform) -> AssetInfo {
    match platform {
        Platform::MacArm64 => AssetInfo {
            filename: "barretenberg-arm64-darwin.tar.gz",
            archive_sha256: "7f72eaf42bec065fd3e3fd1d1989a0c7333b236447b66f0954eda13125a01ab2",
        },
        Platform::LinuxX86_64 => AssetInfo {
            filename: "barretenberg-amd64-linux.tar.gz",
            archive_sha256: "88586691621fdbf6105e064aca1b6e4f1f5345f2e75560d1d385693019480697",
        },
    }
}
