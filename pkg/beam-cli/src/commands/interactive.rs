// lint-long-file-override allow-max-lines=300
use std::path::Path;

use contextful::{ErrorContextExt, ResultContextExt};
use rustyline::{Config, Editor, error::ReadlineError, history::History};
use serde_json::json;

pub(crate) use super::interactive_history::should_persist_history;
#[cfg(test)]
pub(crate) use super::interactive_history::uses_matching_prefix_history_search;
#[cfg(test)]
pub(crate) use super::interactive_parse::repl_command_args;
pub(crate) use super::interactive_parse::{
    ParsedLine, is_exit_command, merge_overrides, normalized_repl_command, parse_line, repl_err,
};
use super::{
    interactive_helper::{BeamHelper, help_text},
    interactive_history::{ReplHistory, bind_matching_prefix_history_search, sanitize_history},
    interactive_interrupt::run_with_interrupt_owner,
    interactive_parse::{resolved_color_mode, resolved_output_mode},
    interactive_state::{capture_repl_state, reconcile_repl_state, repl_state_mutation},
};
use crate::{
    chains::{ensure_rpc_matches_chain_id, find_chain},
    cli::BalanceArgs,
    commands,
    display::{error_message, render_colored_shell_prefix, render_shell_prefix, shrink},
    error::{Error, Result},
    output::{CommandOutput, with_loading},
    runtime::{BeamApp, InvocationOverrides},
};
pub async fn run(app: &BeamApp) -> Result<()> {
    let config = Config::default();
    let mut editor = Editor::<BeamHelper, ReplHistory>::with_history(config, ReplHistory::new())
        .context("create beam repl editor")?;
    editor.set_helper(Some(BeamHelper::new()));
    bind_matching_prefix_history_search(&mut editor);
    load_sanitized_history(editor.history_mut(), &app.paths.history)
        .context("sanitize beam repl history")?;
    let mut overrides = app.overrides.clone();
    canonicalize_startup_wallet_override(app, &mut overrides).await?;

    loop {
        let session = session(app, &overrides);
        let prompt = prompt(&session).await?;
        if let Some(helper) = editor.helper_mut() {
            helper.set_shell_prompt(prompt.plain.clone(), prompt.colored.clone());
        }

        match editor.readline(&prompt.plain) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if should_persist_history(line) {
                    let _ = editor.add_history_entry(line);
                    let _ = editor.save_history(&app.paths.history);
                }
                if is_exit_command(line) {
                    break;
                }
                match handle_line(app, &mut overrides, line).await {
                    Ok(()) | Err(Error::Interrupted) => {}
                    Err(err) => {
                        eprintln!(
                            "{}",
                            error_message(&err.to_string(), app.color_mode.colors_stderr()),
                        );
                    }
                }
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                return Err(std::io::Error::other(err.to_string())
                    .context("read beam repl line")
                    .into());
            }
        }
    }

    let _ = editor.save_history(&app.paths.history);
    Ok(())
}

pub(crate) fn load_sanitized_history(
    history: &mut ReplHistory,
    path: &Path,
) -> rustyline::Result<()> {
    let _ = history.load(path);
    if sanitize_history(history)? {
        let _ = history.save(path);
    }
    Ok(())
}

pub(crate) async fn canonicalize_startup_wallet_override(
    app: &BeamApp,
    overrides: &mut InvocationOverrides,
) -> Result<()> {
    if overrides.from.is_some() {
        overrides.from = app
            .canonical_wallet_selector(overrides.from.as_deref())
            .await?;
    }

    Ok(())
}

async fn handle_line(app: &BeamApp, overrides: &mut InvocationOverrides, line: &str) -> Result<()> {
    let parsed = parse_line(line)?;
    let interrupt_owner = parsed.interrupt_owner();

    run_with_interrupt_owner(
        interrupt_owner,
        handle_parsed_line(app, overrides, parsed),
        tokio::signal::ctrl_c(),
    )
    .await
}

pub(crate) async fn handle_parsed_line(
    app: &BeamApp,
    overrides: &mut InvocationOverrides,
    parsed: ParsedLine,
) -> Result<()> {
    match parsed {
        ParsedLine::ReplCommand(args) => handle_repl_command(app, overrides, &args).await,
        ParsedLine::Cli { args, cli } => {
            let command_app = BeamApp {
                overrides: merge_overrides(overrides, &cli.overrides()),
                color_mode: resolved_color_mode(&args, &cli, app),
                output_mode: resolved_output_mode(&args, &cli, app),
                ..app.clone()
            };

            match cli.command {
                Some(command) => {
                    let mutation = repl_state_mutation(&command);
                    let snapshot = match mutation.as_ref() {
                        Some(mutation) => {
                            Some(capture_repl_state(app, &command_app, overrides, mutation).await?)
                        }
                        None => None,
                    };
                    commands::run(&command_app, command).await?;
                    if let (Some(mutation), Some(snapshot)) = (mutation.as_ref(), snapshot) {
                        reconcile_repl_state(app, overrides, mutation, snapshot).await?;
                    }
                    Ok(())
                }
                None => Ok(()),
            }
        }
        ParsedLine::CliError(err) => {
            err.print().context("print beam repl clap error")?;
            Ok(())
        }
    }
}

pub(crate) async fn handle_repl_command(
    app: &BeamApp,
    overrides: &mut InvocationOverrides,
    args: &[String],
) -> Result<()> {
    let command = normalized_repl_command(args.first().map(String::as_str))
        .ok_or_else(|| repl_err(args.first().cloned().unwrap_or_default()))?;

    match command {
        "wallets" => {
            overrides.from = app
                .canonical_wallet_selector(args.get(1).map(String::as_str))
                .await?
        }
        "chains" => {
            set_repl_chain_override(app, overrides, args.get(1).map(String::as_str)).await?
        }
        "rpc" => set_repl_rpc_override(app, overrides, args.get(1).map(String::as_str)).await?,
        "balance" => {
            commands::balance::run(&session(app, overrides), BalanceArgs { token: None }).await?
        }
        "tokens" => commands::tokens::list_tokens(&session(app, overrides)).await?,
        "help" => {
            let help = help_text();
            CommandOutput::new(
                help.clone(),
                json!({ "cli_prefix_optional": true, "help": help }),
            )
            .print(app.output_mode)?
        }
        _ => unreachable!("validated repl command"),
    }

    Ok(())
}

pub(crate) async fn set_repl_chain_override(
    app: &BeamApp,
    overrides: &mut InvocationOverrides,
    selection: Option<&str>,
) -> Result<()> {
    let next_chain = match selection {
        Some(selection) => {
            let available = app.chain_store.get().await;
            let chain = find_chain(selection, &available)?;
            Some(chain.key)
        }
        None => None,
    };

    // REPL chain switches reset any inherited or previously-selected RPC override so the
    // session falls back to the new chain's configured default unless the user selects a new
    // RPC explicitly.
    overrides.chain = next_chain;
    overrides.rpc = None;

    Ok(())
}

pub(crate) async fn set_repl_rpc_override(
    app: &BeamApp,
    overrides: &mut InvocationOverrides,
    rpc_url: Option<&str>,
) -> Result<()> {
    match rpc_url {
        Some(rpc_url) => {
            let chain = session(app, overrides).active_chain().await?;
            with_loading(
                app.output_mode,
                format!("Validating RPC {rpc_url}..."),
                async {
                    ensure_rpc_matches_chain_id(&chain.entry.key, chain.entry.chain_id, rpc_url)
                        .await
                },
            )
            .await?;
            overrides.rpc = Some(rpc_url.to_string());
        }
        None => overrides.rpc = None,
    }

    Ok(())
}

pub(crate) struct ReplPrompt {
    plain: String,
    colored: Option<String>,
}

pub(crate) async fn prompt(app: &BeamApp) -> Result<ReplPrompt> {
    let selected_address = app.active_optional_address().await?;
    let wallet = app.active_wallet().await.ok();
    let chain = app.active_chain().await?;
    let wallet_display = match (wallet.as_ref(), selected_address) {
        (Some(wallet), _) => format!("{} {}", wallet.name, shrink(&wallet.address)),
        (None, Some(address)) => shrink(&format!("{address:#x}")),
        (None, None) => "no-wallet".to_string(),
    };
    let rpc_url = shrink(&chain.rpc_url);

    Ok(ReplPrompt {
        plain: render_shell_prefix(&wallet_display, &chain.entry.key, &rpc_url),
        colored: app
            .color_mode
            .colors_stdout()
            .then(|| render_colored_shell_prefix(&wallet_display, &chain.entry.key, &rpc_url)),
    })
}

fn session(app: &BeamApp, ov: &InvocationOverrides) -> BeamApp {
    BeamApp {
        overrides: ov.clone(),
        ..app.clone()
    }
}
