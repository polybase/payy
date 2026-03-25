use std::path::PathBuf;

use contextful::Contextful;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, XTaskError>;

#[derive(Debug, Error)]
pub enum XTaskError {
    #[error("[xtask] failed to determine repository root directory")]
    RepoRoot,
    #[error("[xtask] failed to resolve home directory")]
    HomeDir,
    #[error("[xtask] cargo metadata failed: {stderr}")]
    CargoMetadataFailed { stderr: String },
    #[error("[xtask] git command {args:?} failed: {stderr}")]
    GitCommand { args: Vec<String>, stderr: String },
    #[error("[xtask] lint failures: {count}")]
    LintFailures { count: usize },
    #[error("[xtask] tests failed for crates: {failed_crates:?}")]
    TestsFailed { failed_crates: Vec<String> },
    #[error("[xtask] command `{program}` exited with status {status:?}: {stderr}")]
    CommandFailure {
        program: &'static str,
        status: Option<i32>,
        stderr: String,
    },
    #[error("[xtask/error] missing required commands:\n{}", format_missing_commands(.commands))]
    MissingCommands {
        commands: Vec<(&'static str, &'static str)>,
    },
    #[error("[xtask] unsupported platform: {os}-{arch}")]
    UnsupportedPlatform {
        os: &'static str,
        arch: &'static str,
    },
    #[error("[xtask] timed out waiting for postgres to become ready")]
    PostgresReadyTimeout,
    #[error("[xtask] path is not valid UTF-8: {path:?}")]
    NonUtf8Path { path: PathBuf },
    #[error("[xtask] archive missing expected binary at {path:?}")]
    ArchiveMissingBinary { path: PathBuf },
    #[error("[xtask] checksum mismatch for {path:?}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        path: PathBuf,
        expected: &'static str,
        actual: String,
    },
    #[error("[xtask/noir-fixtures] invalid manifest metadata at {path:?}: {reason}")]
    NoirManifest { path: PathBuf, reason: String },
    #[error("[xtask/noir-fixtures] failed to update `{name}` in {path:?}")]
    NoirHashUpdateNotFound { path: PathBuf, name: String },
    #[error("[xtask/noir-fixtures] invalid vk_hash output for circuit `{circuit}`")]
    NoirVkHashOutput { circuit: String },
    #[error("[xtask/noir-fixtures] missing source key in solc output")]
    NoirSolcMissingSourceKey,
    #[error("[xtask/noir-fixtures] missing bytecode for contract `{contract}` in solc output")]
    NoirSolcMissingContractBytecode { contract: &'static str },
    #[error("[xtask] crate manifest is outside workspace root: {path:?}")]
    InvalidCrateManifest { path: PathBuf },
    #[error("[xtask] failed to parse TOML input")]
    TomlParse(#[from] Contextful<toml::de::Error>),
    #[error("[xtask] failed to parse utf-8 output")]
    Utf8(#[from] Contextful<std::string::FromUtf8Error>),
    #[error("[xtask] failed to parse JSON payload")]
    Json(#[from] Contextful<serde_json::Error>),
    #[error("[xtask] configuration environment error")]
    ConfigEnv(#[from] Contextful<std::env::VarError>),
    #[error("[xtask] configuration parse error")]
    ConfigParseInt(#[from] Contextful<std::num::ParseIntError>),
    #[error("[xtask] http error")]
    Http(#[from] Contextful<reqwest::Error>),
    #[error("[xtask] solc tooling error: {0}")]
    SolcTooling(#[from] Contextful<solc_tooling::Error>),
    #[error("[xtask] io error")]
    Io(#[from] Contextful<std::io::Error>),
}

pub fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|pkg_dir| pkg_dir.parent())
        .map(PathBuf::from)
        .ok_or(XTaskError::RepoRoot)
}

fn format_missing_commands(commands: &[(&'static str, &'static str)]) -> String {
    commands
        .iter()
        .map(|(command, hint)| format!("  - {command}: {hint}"))
        .collect::<Vec<_>>()
        .join("\n")
}
