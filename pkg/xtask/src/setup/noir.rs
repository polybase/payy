// lint-long-file-override allow-max-lines=300
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use contextful::ResultContextExt;
use duct::cmd;
use tempfile::tempdir;
use which::which;

use crate::error::{Result, XTaskError};
use crate::setup::checksum::verify_sha256;

use crate::setup::{Platform, path_to_string, run_expression};

const NOIR_VERSION: &str = "1.0.0-beta.14";
const RELEASE_TAG: &str = "v1.0.0-beta.14";
const RELEASE_BASE: &str = "https://github.com/noir-lang/noir/releases/download";

pub struct NoirOutcome {
    pub installed: bool,
}

pub fn ensure_nargo(cargo_bin: &Path, platform: Platform) -> Result<NoirOutcome> {
    let current = current_version(cargo_bin)?;
    if let Some(existing) = current.as_deref() {
        if existing == NOIR_VERSION {
            eprintln!("nargo already installed (version {NOIR_VERSION})");
            return Ok(NoirOutcome { installed: false });
        }
        eprintln!(
            "nargo version mismatch (found {existing}, expected {NOIR_VERSION}). Reinstalling..."
        );
    } else {
        eprintln!("Installing nargo {NOIR_VERSION}...");
    }

    install_nargo(cargo_bin, platform)?;
    eprintln!("nargo installed successfully");

    Ok(NoirOutcome { installed: true })
}

fn current_version(cargo_bin: &Path) -> Result<Option<String>> {
    for candidate in candidate_binaries(cargo_bin) {
        if let Some(version) = version_from_binary(&candidate)? {
            return Ok(Some(version));
        }
    }
    Ok(None)
}

fn candidate_binaries(cargo_bin: &Path) -> Vec<PathBuf> {
    let mut binaries = Vec::new();
    let cargo_binary = cargo_bin.join("nargo");
    if cargo_binary.exists() {
        binaries.push(cargo_binary);
    }
    if let Ok(path) = which("nargo")
        && !binaries.iter().any(|existing| existing == &path)
    {
        binaries.push(path);
    }
    binaries
}

fn version_from_binary(path: &Path) -> Result<Option<String>> {
    let path_str = path_to_string(path)?;
    match run_expression("nargo", cmd(path_str.as_str(), ["--version"])) {
        Ok(output) => {
            let stdout =
                String::from_utf8(output.stdout).context("nargo version output as UTF-8")?;
            parse_version(stdout)
        }
        Err(err @ XTaskError::Io(_)) | Err(err @ XTaskError::CommandFailure { .. }) => {
            eprintln!(
                "Existing nargo binary at {} is unusable ({err}); ignoring it",
                path.display()
            );
            Ok(None)
        }
        Err(err) => Err(err),
    }
}

fn parse_version(stdout: String) -> Result<Option<String>> {
    for line in stdout.lines() {
        if let Some(rest) = line.trim().strip_prefix("nargo version") {
            let value = rest.trim().trim_start_matches('=').trim();
            if !value.is_empty() {
                return Ok(Some(value.to_string()));
            }
        }
    }
    Ok(None)
}

fn install_nargo(cargo_bin: &Path, platform: Platform) -> Result<()> {
    let asset = asset_for(platform);
    let url = format!("{RELEASE_BASE}/{RELEASE_TAG}/{}", asset.filename);

    let temp_dir = tempdir().context("create temporary directory for nargo install")?;
    let archive_path = temp_dir.path().join(asset.filename);
    let archive_str = path_to_string(&archive_path)?;
    let temp_dir_str = path_to_string(temp_dir.path())?;

    run_expression(
        "curl",
        cmd("curl", ["-fsSL", url.as_str(), "-o", archive_str.as_str()]),
    )?;

    verify_sha256(&archive_path, asset.sha256)?;

    run_expression(
        "tar",
        cmd(
            "tar",
            ["-xzf", archive_str.as_str(), "-C", temp_dir_str.as_str()],
        ),
    )?;

    let source = temp_dir.path().join("nargo");
    if !source.exists() {
        return Err(XTaskError::ArchiveMissingBinary {
            path: source.clone(),
        });
    }

    let target_tmp = cargo_bin.join("nargo.tmp");
    let target = cargo_bin.join("nargo");

    fs::copy(&source, &target_tmp).with_context(|| {
        format!(
            "copy nargo binary from {} to {}",
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
            "move nargo binary from {} to {}",
            target_tmp.display(),
            target.display()
        )
    })?;

    Ok(())
}
struct AssetInfo {
    filename: &'static str,
    sha256: &'static str,
}

fn asset_for(platform: Platform) -> AssetInfo {
    match platform {
        Platform::MacArm64 => AssetInfo {
            filename: "nargo-aarch64-apple-darwin.tar.gz",
            sha256: "2bb856a86e9e07ae94e052699ebd391426534d30fe43783bd6873f628a3a699b",
        },
        Platform::LinuxX86_64 => AssetInfo {
            filename: "nargo-x86_64-unknown-linux-gnu.tar.gz",
            sha256: "7854a340b5ce39f471036031aa94087a7cc328d2029d1e3976eeade2fe4a9bb1",
        },
    }
}
