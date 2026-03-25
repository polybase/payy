mod abi;
mod chains;
mod cli;
mod commands;
mod config;
mod display;
mod ens;
mod error;
mod evm;
mod human_output;
mod keystore;
mod known_tokens;
mod output;
mod prompts;
mod runtime;
mod signer;
mod table;
mod transaction;
mod units;
mod update_cache;
mod update_client;
mod util;

#[cfg(test)]
mod tests;

use clap::Parser;
use runtime::{BeamApp, BeamPaths, ensure_root_dir};

use crate::{
    cli::{Cli, Command},
    commands::{interactive, run},
    display::error_message,
    error::{Error, Result},
    output::OutputMode,
};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let color_mode = cli.color;

    if let Err(err) = run_cli(cli).await {
        if matches!(err, Error::Interrupted) {
            std::process::exit(130);
        }
        eprintln!(
            "{}",
            error_message(&err.to_string(), color_mode.colors_stderr()),
        );
        std::process::exit(1);
    }
}

async fn run_cli(cli: Cli) -> Result<()> {
    run_cli_with_paths(cli, None).await
}

async fn run_cli_with_paths(cli: Cli, paths: Option<BeamPaths>) -> Result<()> {
    let Cli {
        command,
        rpc,
        from,
        chain,
        output,
        color,
        no_update_check,
    } = cli;
    let overrides = runtime::InvocationOverrides { chain, from, rpc };
    let command = match command {
        Some(Command::Util { action }) => return commands::util::run(output, action),
        // Self-update must remain available even when local Beam state is corrupted.
        Some(Command::Update) => {
            return commands::update::run_update(&overrides, output, color).await;
        }
        other => other,
    };
    let paths = match paths {
        Some(paths) => paths,
        None => BeamPaths::from_env_or_home()?,
    };
    ensure_root_dir(&paths.root)?;
    let skip_update_checks = update_cache::skip_update_checks(no_update_check);

    if matches!(command, Some(Command::RefreshUpdateStatus)) {
        update_cache::refresh_cached_update_status(&paths.root).await?;
        return Ok(());
    }

    let app = BeamApp::for_root(paths, color, output, overrides).await?;

    if !skip_update_checks {
        if command.is_none() {
            let _ =
                update_cache::maybe_warn_for_interactive_startup(&app.paths.root, app.color_mode)
                    .await;
        } else if output == OutputMode::Default {
            let _ = update_cache::maybe_print_cached_update_notice(&app.paths.root, app.color_mode)
                .await;
        }

        let _ = update_cache::spawn_background_refresh_if_stale(&app.paths.root).await;
    }

    if command.is_none() {
        interactive::run(&app).await?;
        return Ok(());
    }

    match command {
        Some(command) => run(&app, command).await,
        None => Ok(()),
    }
}
