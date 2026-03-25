use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use contextful::ResultContextExt;
use duct::cmd;
use which::which;

use crate::error::{Result, XTaskError};

use crate::setup::{SetupArgs, path_to_string, run_expression, run_expression_unchecked};

pub const DEFAULT_IMAGE: &str = "postgres:18";
pub const DEFAULT_CONTAINER: &str = "polybase-pg";

pub struct PostgresOutcome {
    pub installed_diesel: bool,
}

struct DieselCli {
    installed: bool,
    path: PathBuf,
}

pub fn ensure_postgres(
    repo_root: &Path,
    cargo_bin: &Path,
    args: &SetupArgs,
) -> Result<PostgresOutcome> {
    let container = args.postgres_container.as_str();

    if is_container_running(container)? {
        eprintln!("Postgres container `{container}` is already running");
    } else if container_exists(container)? {
        eprintln!("Starting existing Postgres container `{container}`");
        run_expression("docker", cmd("docker", ["start", container]))?;
    } else {
        eprintln!(
            "Creating Postgres container `{container}` (image {})",
            args.postgres_image
        );
        start_new_container(args)?;
    }

    wait_for_postgres(container, &args.postgres_user, &args.postgres_database)?;

    let diesel_cli = ensure_diesel_cli(cargo_bin)?;
    run_migrations(repo_root, args, &diesel_cli.path)?;

    Ok(PostgresOutcome {
        installed_diesel: diesel_cli.installed,
    })
}

pub fn build_database_url(args: &SetupArgs) -> String {
    format!(
        "postgres://{}:{}@localhost:{}/{}",
        args.postgres_user, args.postgres_password, args.postgres_port, args.postgres_database
    )
}

fn is_container_running(name: &str) -> Result<bool> {
    let filter = format!("name={name}");
    let output = run_expression("docker", cmd("docker", ["ps", "-q", "-f", filter.as_str()]))?;
    let stdout = String::from_utf8(output.stdout).context("docker ps output as UTF-8")?;
    Ok(!stdout.trim().is_empty())
}

fn container_exists(name: &str) -> Result<bool> {
    let filter = format!("name={name}");
    let output = run_expression(
        "docker",
        cmd("docker", ["ps", "-aq", "-f", filter.as_str()]),
    )?;
    let stdout = String::from_utf8(output.stdout).context("docker ps output as UTF-8")?;
    Ok(!stdout.trim().is_empty())
}

fn start_new_container(args: &SetupArgs) -> Result<()> {
    let port_binding = format!("{}:5432", args.postgres_port);
    let env_db = format!("POSTGRES_DB={}", args.postgres_database);
    let env_user = format!("POSTGRES_USER={}", args.postgres_user);
    let env_password = format!("POSTGRES_PASSWORD={}", args.postgres_password);

    run_expression(
        "docker",
        cmd(
            "docker",
            [
                "run",
                "-d",
                "--name",
                args.postgres_container.as_str(),
                "-p",
                port_binding.as_str(),
                "-e",
                env_db.as_str(),
                "-e",
                env_user.as_str(),
                "-e",
                env_password.as_str(),
                args.postgres_image.as_str(),
            ],
        ),
    )?;

    Ok(())
}

fn wait_for_postgres(container: &str, user: &str, database: &str) -> Result<()> {
    eprintln!("Waiting for Postgres to become ready...");
    for attempt in 0..60 {
        let result = run_expression_unchecked(
            "docker",
            cmd(
                "docker",
                ["exec", container, "pg_isready", "-U", user, "-d", database],
            ),
        );

        match result {
            Ok(output) if output.status.success() => {
                eprintln!("Postgres is ready (after {attempt} checks)");
                return Ok(());
            }
            Ok(_) | Err(_) => {
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    Err(XTaskError::PostgresReadyTimeout)
}

fn ensure_diesel_cli(cargo_bin: &Path) -> Result<DieselCli> {
    if let Ok(existing) = which("diesel") {
        return Ok(DieselCli {
            installed: false,
            path: existing,
        });
    }

    let cargo_bin_candidate = cargo_bin.join("diesel");
    if cargo_bin_candidate.exists() {
        return Ok(DieselCli {
            installed: false,
            path: cargo_bin_candidate,
        });
    }

    eprintln!("Installing diesel_cli...");
    run_expression(
        "cargo",
        cmd(
            "cargo",
            [
                "install",
                "diesel_cli",
                "--no-default-features",
                "--features",
                "postgres",
                "--locked",
            ],
        ),
    )?;

    let resolved_path = which("diesel").unwrap_or(cargo_bin_candidate);

    Ok(DieselCli {
        installed: true,
        path: resolved_path,
    })
}

fn run_migrations(repo_root: &Path, args: &SetupArgs, diesel_path: &Path) -> Result<()> {
    let database_url = build_database_url(args);
    let database_dir = repo_root.join("pkg").join("database");

    let diesel_bin = path_to_string(diesel_path)?;
    let mut expression = cmd(diesel_bin.as_str(), ["migration", "run"]);
    expression = expression.env("DATABASE_URL", database_url.as_str());
    expression = expression.dir(&database_dir);

    run_expression("diesel", expression)?;
    eprintln!("Database migrations applied");
    Ok(())
}
