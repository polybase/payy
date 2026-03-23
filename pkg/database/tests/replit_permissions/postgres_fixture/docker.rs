use std::{io, num::ParseIntError, process::Command, string::FromUtf8Error};

use contextful::{Contextful, ResultContextExt};
use thiserror::Error;
use tokio::time::{sleep, Duration};
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;

use super::util::last_non_empty_trimmed_line;

#[derive(Debug, Error)]
pub enum FixtureError {
    #[error("[replit-permissions-test] io error")]
    Io(#[from] Contextful<io::Error>),
    #[error("[replit-permissions-test] utf8 error")]
    Utf8(#[from] Contextful<FromUtf8Error>),
    #[error("[replit-permissions-test] postgres error")]
    Postgres(#[from] Contextful<tokio_postgres::Error>),
    #[error("[replit-permissions-test] parse int error")]
    ParseInt(#[from] Contextful<ParseIntError>),
    #[error("[replit-permissions-test] docker command failed for {command}: {stderr}")]
    CommandFailure { command: &'static str, stderr: String },
    #[error("[replit-permissions-test] docker port output missing mapping: {output}")]
    UnexpectedDockerPortOutput { output: String },
    #[error("[replit-permissions-test] docker run output missing container id: {output}")]
    MissingDockerRunContainerId { output: String },
    #[error("[replit-permissions-test] postgres did not become ready in time")]
    NotReady,
}

impl FixtureError {
    pub fn is_missing_docker(&self) -> bool {
        matches!(self, FixtureError::Io(err) if err.source_ref().kind() == io::ErrorKind::NotFound)
    }
}

pub type Result<T> = std::result::Result<T, FixtureError>;

/// Minimal Postgres container wrapper tailored for the replit permissions test.
pub struct DockerPostgres {
    container_id: String,
    port: u16,
}

impl DockerPostgres {
    pub async fn start() -> Result<Self> {
        let name = format!("replit-perm-{}", Uuid::new_v4().simple());
        let run_output = Command::new("docker")
            .args([
                "run",
                "-d",
                "--rm",
                "--name",
                name.as_str(),
                "-e",
                "POSTGRES_PASSWORD=postgres",
                "-e",
                "POSTGRES_USER=postgres",
                "-e",
                "POSTGRES_DB=postgres",
                "-P",
                "postgres:18",
            ])
            .output()
            .context("launch postgres container for replit permissions test")?;
        if !run_output.status.success() {
            return Err(FixtureError::CommandFailure {
                command: "docker run",
                stderr: String::from_utf8_lossy(&run_output.stderr).into_owned(),
            });
        }

        let run_stdout = String::from_utf8(run_output.stdout)
            .context("decode docker container id for replit permissions test")?;
        let container_id = last_non_empty_trimmed_line(&run_stdout)
            .ok_or_else(|| FixtureError::MissingDockerRunContainerId {
                output: run_stdout.clone(),
            })?
            .to_owned();
        let port_output = Command::new("docker")
            .args(["port", container_id.as_str(), "5432/tcp"])
            .output()
            .context("fetch mapped postgres port for replit permissions test")?;
        if !port_output.status.success() {
            return Err(FixtureError::CommandFailure {
                command: "docker port",
                stderr: String::from_utf8_lossy(&port_output.stderr).into_owned(),
            });
        }

        let port_lines = String::from_utf8(port_output.stdout)
            .context("decode docker port output for replit permissions test")?;
        let host_port = parse_mapped_port(&port_lines)?;
        let instance = Self { container_id, port: host_port };
        instance.wait_for_ready().await?;
        Ok(instance)
    }

    async fn wait_for_ready(&self) -> Result<()> {
        for _ in 0..30 {
            if let Ok((client, connection)) = tokio_postgres::connect(
                &format!(
                    "host=127.0.0.1 port={} user=postgres password=postgres dbname=postgres",
                    self.port
                ),
                NoTls,
            )
            .await
            {
                tokio::spawn(async move {
                    if let Err(err) = connection.await {
                        eprintln!("postgres connection error: {err}");
                    }
                });

                if client.simple_query("SELECT 1").await.is_ok() {
                    return Ok(());
                }
            }

            sleep(Duration::from_secs(1)).await;
        }

        Err(FixtureError::NotReady)
    }

    pub async fn connect(&self, db: &str, user: &str, password: &str) -> Result<Client> {
        let (client, connection) = tokio_postgres::connect(
            &format!(
                "host=127.0.0.1 port={} user={} password={} dbname={}",
                self.port, user, password, db
            ),
            NoTls,
        )
        .await
        .context("connect to postgres container for replit permissions test")?;
        tokio::spawn(async move {
            if let Err(err) = connection.await {
                eprintln!("postgres connection error: {err}");
            }
        });
        Ok(client)
    }

    pub async fn connect_superuser(&self, db: &str) -> Result<Client> {
        self.connect(db, "postgres", "postgres").await
    }

    pub async fn recreate_database(&self, name: &str) -> Result<()> {
        let superuser = self
            .connect_superuser("postgres")
            .await
            .context("connect as postgres superuser to recreate database")?;
        superuser
            .simple_query(&format!("DROP DATABASE IF EXISTS {name}"))
            .await
            .context("drop existing replit permissions database")?;
        superuser
            .simple_query(&format!("CREATE DATABASE {name}"))
            .await
            .context("create fresh replit permissions database")?;
        Ok(())
    }

    pub async fn apply_sql(&self, db: &str, sql: &str) -> Result<()> {
        let superuser = self
            .connect_superuser(db)
            .await
            .context("connect as postgres superuser to apply sql")?;
        superuser
            .batch_execute(sql)
            .await
            .context("apply SQL snippet for replit permissions test")?;
        Ok(())
    }
}

impl Drop for DockerPostgres {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", self.container_id.as_str()])
            .status();
    }
}

fn parse_mapped_port(output: &str) -> Result<u16> {
    let mapping = output
        .lines()
        .next()
        .and_then(|line| line.split(':').nth(1))
        .ok_or_else(|| FixtureError::UnexpectedDockerPortOutput {
            output: output.to_string(),
        })?;
    Ok(mapping
        .parse::<u16>()
        .context("parse docker mapped postgres port value")?)
}
