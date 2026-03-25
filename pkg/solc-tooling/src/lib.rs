// lint-long-file-override allow-max-lines=240
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use contextful::{FromContextful, InternalError, ResultContextExt};
use sha2::{Digest, Sha256};

pub const SOLC_VERSION: &str = "0.8.29+commit.ab55807c";
const SOLC_BASE_URL: &str = "https://binaries.soliditylang.org";
const SOLC_SHA256_LINUX: &str = "18d418a40dc04d17656b1b5c8a7b35cfbab8942b51f38d005d5b59e8aa6637e0";
const SOLC_SHA256_MACOS: &str = "66fabdd17c8c0091311997ec7d17b4d92e1b7b2c2d213dc14e4ff28c3de864d1";
const MACOS_BAD_CPU_TYPE_IN_EXECUTABLE: i32 = 86;

#[derive(Debug, thiserror::Error, FromContextful)]
pub enum Error {
    #[error(
        "[solc-tooling] unsupported platform: {os}-{arch}; {guidance}",
        guidance = unsupported_platform_guidance(.os, .arch)
    )]
    UnsupportedPlatform {
        os: &'static str,
        arch: &'static str,
    },

    #[error("[solc-tooling] archive checksum mismatch: expected {expected}, got {actual}")]
    ArchiveChecksumMismatch {
        expected: &'static str,
        actual: String,
    },

    #[error(
        "[solc-tooling] failed to execute pinned solc at {path}: install Rosetta 2 to run the macOS amd64 binary on Apple Silicon"
    )]
    RosettaRequired { path: String },

    #[error("[solc-tooling] failed to execute pinned solc at {path}: {details}")]
    SolcExecutionFailed { path: String, details: String },

    #[error("[solc-tooling] internal error")]
    Internal(#[from] InternalError),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn ensure_solc() -> Result<PathBuf> {
    let solc_cache_dir = env::var("SOLC_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            home::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".polybase")
                .join("solc")
        });

    let target = SolcTarget::detect()?;
    let target_path = solc_cache_dir.join(target.target_name);
    if target_path.exists() {
        let bytes = fs::read(&target_path)
            .with_context(|| format!("read cached solc binary {}", target_path.display()))?;
        if sha256_hex(&bytes) == target.expected_sha256 {
            set_executable(&target_path)?;
            ensure_executable(&target, &target_path)?;
            return Ok(target_path);
        }
    }

    fs::create_dir_all(&solc_cache_dir)
        .with_context(|| format!("create solc cache directory {}", solc_cache_dir.display()))?;

    let url = format!("{SOLC_BASE_URL}/{}/{}", target.platform, target.filename);
    eprintln!("Downloading solc from {url}");
    let response = reqwest::blocking::get(url)
        .context("download solc binary")?
        .error_for_status()
        .context("download solc binary")?;
    let bytes = response.bytes().context("read solc download bytes")?;

    let actual_sha256 = sha256_hex(&bytes);
    if actual_sha256 != target.expected_sha256 {
        return Err(Error::ArchiveChecksumMismatch {
            expected: target.expected_sha256,
            actual: actual_sha256,
        });
    }

    fs::write(&target_path, &bytes)
        .with_context(|| format!("write downloaded solc binary {}", target_path.display()))?;
    set_executable(&target_path)?;
    ensure_executable(&target, &target_path)?;
    Ok(target_path)
}

struct SolcTarget {
    platform: &'static str,
    filename: String,
    target_name: &'static str,
    expected_sha256: &'static str,
    execution_check: ExecutionCheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExecutionCheck {
    None,
    Rosetta,
}

impl SolcTarget {
    fn detect() -> Result<Self> {
        Self::detect_from(env::consts::OS, env::consts::ARCH)
    }

    fn detect_from(os: &'static str, arch: &'static str) -> Result<Self> {
        // Solidity only publishes the pinned 0.8.29 binary for macOS/Linux amd64.
        match (os, arch) {
            ("macos", "x86_64") => Ok(Self::macos(ExecutionCheck::None)),
            ("macos", "aarch64") => Ok(Self::macos(ExecutionCheck::Rosetta)),
            ("linux", "x86_64") => Ok(Self::linux()),
            (os, arch) => Err(Error::UnsupportedPlatform { os, arch }),
        }
    }

    fn macos(execution_check: ExecutionCheck) -> Self {
        Self {
            platform: "macosx-amd64",
            filename: format!("solc-macosx-amd64-v{SOLC_VERSION}"),
            target_name: "solc-v0.8.29-macos",
            expected_sha256: SOLC_SHA256_MACOS,
            execution_check,
        }
    }

    fn linux() -> Self {
        Self {
            platform: "linux-amd64",
            filename: format!("solc-linux-amd64-v{SOLC_VERSION}"),
            target_name: "solc-v0.8.29-linux",
            expected_sha256: SOLC_SHA256_LINUX,
            execution_check: ExecutionCheck::None,
        }
    }
}

fn unsupported_platform_guidance(os: &str, arch: &str) -> String {
    match (os, arch) {
        ("linux", "aarch64") => format!(
            "solc {SOLC_VERSION} is only published for Linux amd64; run this under x86_64 emulation or in a linux/amd64 container"
        ),
        _ => format!(
            "solc {SOLC_VERSION} is only supported on Linux x86_64 and macOS amd64 (including Apple Silicon via Rosetta)"
        ),
    }
}

fn ensure_executable(target: &SolcTarget, path: &Path) -> Result<()> {
    match target.execution_check {
        ExecutionCheck::None => Ok(()),
        ExecutionCheck::Rosetta => probe_rosetta_solc(path),
    }
}

fn probe_rosetta_solc(path: &Path) -> Result<()> {
    let output = match Command::new(path).arg("--version").output() {
        Ok(output) => output,
        Err(error) if error.raw_os_error() == Some(MACOS_BAD_CPU_TYPE_IN_EXECUTABLE) => {
            return Err(Error::RosettaRequired {
                path: path.display().to_string(),
            });
        }
        Err(error) => {
            return Err(Error::SolcExecutionFailed {
                path: path.display().to_string(),
                details: error.to_string(),
            });
        }
    };

    if output.status.success() {
        return Ok(());
    }

    Err(Error::SolcExecutionFailed {
        path: path.display().to_string(),
        details: execution_failure_details(&output),
    })
}

fn execution_failure_details(output: &Output) -> String {
    let stderr = if output.stderr.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    } else {
        String::from_utf8_lossy(&output.stderr).trim().to_owned()
    };

    match (output.status.code(), stderr.is_empty()) {
        (Some(code), true) => format!("command exited with status {code}"),
        (Some(code), false) => format!("command exited with status {code}: {stderr}"),
        (None, true) => "command terminated by signal".to_owned(),
        (None, false) => format!("command terminated by signal: {stderr}"),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    let mut permissions = fs::metadata(path)
        .with_context(|| format!("read metadata for {}", path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("set executable permissions for {}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests;
