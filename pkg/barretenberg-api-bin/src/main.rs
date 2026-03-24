use std::{
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use barretenberg_api_server::build_app;
use barretenberg_cli::CliBackend;
use barretenberg_interface::BbBackend;
use clap::Parser;
use rpc::tracing::{LogFormat, LogLevel, setup_tracing};
use tracing::info;

const TRACING_MODULES: &[&str] = &[
    "barretenberg_api_bin",
    "barretenberg_api_server",
    "barretenberg_api_client",
    "barretenberg_cli",
    "barretenberg_interface",
    "barretenberg_rs",
    "element",
    "notes",
];

#[derive(Parser, Debug)]
#[command(
    name = "barretenberg-api-server",
    about = "HTTP API server for the Barretenberg backend"
)]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    #[arg(long, default_value_t = 9444)]
    port: u16,
    /// Log level
    #[arg(value_enum, long, env = "LOG_LEVEL", default_value = "INFO")]
    log_level: LogLevel,
    /// Log format
    #[arg(value_enum, long, env = "LOG_FORMAT", default_value = "PRETTY")]
    log_format: LogFormat,
    /// Deployment environment name for tracing / sentry tagging
    #[arg(long, env = "ENV_NAME", default_value = "dev")]
    env: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    let _guard = setup_tracing(
        TRACING_MODULES,
        &args.log_level,
        &args.log_format,
        std::env::var("SENTRY_DSN").ok(),
        args.env.clone(),
    )
    .map_err(|err| io::Error::other(err.to_string()))?;

    let bind_addr = parse_bind_addr(&args.host, args.port)?;

    let backend: Arc<dyn BbBackend> = Arc::new(CliBackend);

    info!(
        target: "barretenberg_api_server",
        %bind_addr,
        "starting barretenberg api binary"
    );

    let app = build_app(Arc::clone(&backend));
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await
}

fn parse_bind_addr(host: &str, port: u16) -> io::Result<SocketAddr> {
    let ip = host.parse::<IpAddr>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid host {host}: {err}"),
        )
    })?;
    Ok(SocketAddr::new(ip, port))
}
