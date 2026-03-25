// lint-long-file-override allow-max-lines=300
use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Output;

use clap::Args;
use contextful::ResultContextExt;
use duct::Expression;
use home::cargo_home;
use which::which;

use crate::error::{Result, XTaskError, workspace_root};

mod bb;
mod checksum;
mod eth;
mod fixtures;
mod noir;
mod postgres;

const REQUIRED_COMMANDS: &[(&str, &str)] = &[
    (
        "docker",
        "Install Docker: https://docs.docker.com/get-docker/",
    ),
    ("git", "Install Git: https://git-scm.com/downloads"),
    ("cargo", "Install Rust via https://rustup.rs"),
    ("node", "Install Node.js: https://nodejs.org"),
    (
        "yarn",
        "Install Yarn (e.g. corepack enable or https://classic.yarnpkg.com)",
    ),
    ("curl", "Install curl with your system package manager"),
    ("tar", "Install tar with your system package manager"),
    (
        "make",
        "Install make with your system package manager (e.g., `apt install build-essential` on Debian/Ubuntu, `xcode-select --install` on macOS)",
    ),
    ("perl", "Install perl with your system package manager"),
    ("gzip", "Install gzip with your system package manager"),
];

#[derive(Debug, Clone, Args)]
pub struct SetupArgs {
    /// Port to expose Postgres on localhost
    #[arg(long, default_value_t = 5432)]
    pub postgres_port: u16,
    /// Postgres username used for the development database
    #[arg(long, default_value = "postgres")]
    pub postgres_user: String,
    /// Postgres password used for the development database
    #[arg(long, default_value = "postgres")]
    pub postgres_password: String,
    /// Postgres database name used for development
    #[arg(long, default_value = "guild")]
    pub postgres_database: String,
    /// Docker image tag used for the Postgres container
    #[arg(long, default_value = postgres::DEFAULT_IMAGE)]
    pub postgres_image: String,
    /// Name of the Postgres docker container to manage
    #[arg(long, default_value = postgres::DEFAULT_CONTAINER)]
    pub postgres_container: String,
    /// Skip installing Ethereum workspace dependencies
    #[arg(long)]
    pub skip_eth: bool,
    /// Output environment variables as GitHub Actions assignments instead of shell exports
    #[arg(long)]
    pub github_env: bool,
}

#[derive(Clone, Copy)]
pub enum Platform {
    MacArm64,
    LinuxX86_64,
}

pub fn run_setup(args: SetupArgs) -> Result<()> {
    eprintln!("Starting xtask setup...");
    ensure_prerequisites()?;

    let repo_root = workspace_root()?;
    let cargo_bin = ensure_cargo_bin()?;
    let platform = detect_platform()?;

    let mut installed_to_cargo_bin = false;

    let bb_result = bb::ensure_bb(&cargo_bin, platform)?;
    installed_to_cargo_bin |= bb_result.installed;

    let noir_result = noir::ensure_nargo(&cargo_bin, platform)?;
    installed_to_cargo_bin |= noir_result.installed;

    let pg_result = postgres::ensure_postgres(&repo_root, &cargo_bin, &args)?;
    installed_to_cargo_bin |= pg_result.installed_diesel;

    fixtures::ensure_params(&repo_root)?;

    let claude_md = repo_root.join("CLAUDE.md");
    if fs::symlink_metadata(&claude_md).is_err() {
        symlink("GENERATED_AI_GUIDANCE.md", &claude_md)
            .with_context(|| "create CLAUDE.md symlink".to_string())?;
        eprintln!("Created CLAUDE.md -> GENERATED_AI_GUIDANCE.md");
    }

    if args.skip_eth {
        eprintln!("Skipping eth dependency installation (requested)");
    } else {
        eth::ensure_eth(&repo_root)?;
    }

    let mut exports = Vec::new();

    if installed_to_cargo_bin && !path_contains(&cargo_bin) {
        if args.github_env {
            let mut path_value = cargo_bin.to_string_lossy().to_string();
            if let Ok(existing_path) = env::var("PATH")
                && !existing_path.is_empty()
            {
                path_value.push(':');
                path_value.push_str(&existing_path);
            }
            exports.push(format!("PATH={path_value}"));
        } else {
            let mut path_value = cargo_bin.to_string_lossy().to_string();
            path_value.push_str(":$PATH");
            exports.push(format!("export PATH=\"{}\"", escape_double(&path_value)));
        }
    }

    let database_url = postgres::build_database_url(&args);
    if args.github_env {
        exports.push(format!("DATABASE_URL={database_url}"));
    } else {
        exports.push(format!(
            "export DATABASE_URL={}",
            single_quote(&database_url)
        ));
    }

    for line in exports {
        println!("{line}");
    }

    if args.github_env {
        eprintln!("Setup complete. Append the printed lines to $GITHUB_ENV.");
    } else {
        eprintln!("Setup complete. Evaluate the printed exports to finish configuring your shell.");
    }
    Ok(())
}

pub fn run_expression(program: &'static str, expression: Expression) -> Result<Output> {
    run_with_capture(program, expression, true)
}

pub fn run_expression_unchecked(program: &'static str, expression: Expression) -> Result<Output> {
    run_with_capture(program, expression, false)
}

pub fn path_to_string(path: &Path) -> Result<String> {
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| XTaskError::NonUtf8Path {
            path: path.to_path_buf(),
        })
}

fn run_with_capture(
    program: &'static str,
    expression: Expression,
    enforce_success: bool,
) -> Result<Output> {
    let expression = expression.stderr_capture().stdout_capture();
    let output = expression
        .unchecked()
        .run()
        .with_context(|| format!("run {program} command"))?;

    if enforce_success && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(XTaskError::CommandFailure {
            program,
            status: output.status.code(),
            stderr,
        });
    }

    Ok(output)
}

fn ensure_prerequisites() -> Result<()> {
    let mut missing = Vec::new();
    for (command, hint) in REQUIRED_COMMANDS {
        if which(command).is_err() {
            missing.push((*command, *hint));
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(XTaskError::MissingCommands { commands: missing })
    }
}

fn ensure_cargo_bin() -> Result<PathBuf> {
    let mut cargo_path = cargo_home().context("resolve cargo home directory")?;
    cargo_path.push("bin");
    fs::create_dir_all(&cargo_path)
        .with_context(|| format!("create cargo bin directory at {}", cargo_path.display()))?;
    Ok(cargo_path)
}

fn detect_platform() -> Result<Platform> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok(Platform::MacArm64),
        ("linux", "x86_64") => Ok(Platform::LinuxX86_64),
        (os, arch) => Err(XTaskError::UnsupportedPlatform { os, arch }),
    }
}

pub fn path_contains(path: &Path) -> bool {
    if let Some(paths) = env::var_os("PATH") {
        env::split_paths(&paths).any(|existing| existing == path)
    } else {
        false
    }
}

fn single_quote(value: &str) -> String {
    let escaped = value.replace('\'', "'\\''");
    format!("'{escaped}'")
}

fn escape_double(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
