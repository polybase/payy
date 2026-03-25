use clap::Parser;

use crate::{
    cli::Cli,
    display::ColorMode,
    error::{Error, Result},
    output::OutputMode,
    runtime::{BeamApp, InvocationOverrides},
};

pub(crate) fn parse_line(line: &str) -> Result<ParsedLine> {
    if let Some(args) = repl_command_args(line)? {
        return Ok(ParsedLine::ReplCommand(args));
    }

    let args = parse_shell_words(line)?;
    match Cli::try_parse_from(std::iter::once("beam").chain(args.iter().map(String::as_str))) {
        Ok(cli) => Ok(ParsedLine::Cli { args, cli }),
        Err(err) => Ok(ParsedLine::CliError(err)),
    }
}

pub(crate) fn repl_command_args(line: &str) -> Result<Option<Vec<String>>> {
    let args = parse_shell_words(line)?;

    let Some(command) = normalized_repl_command(args.first().map(String::as_str)) else {
        return Ok(None);
    };

    if is_cli_subcommand_invocation(command, &args) {
        return Ok(None);
    }

    if matches!(command, "balance") && args.len() > 1 {
        return Ok(None);
    }

    Ok(Some(args))
}

pub(crate) fn is_exit_command(line: &str) -> bool {
    matches!(
        parse_shell_words(line).ok().as_deref(),
        Some([command]) if command == "exit"
    )
}

pub(crate) fn merge_overrides(
    base: &InvocationOverrides,
    new: &InvocationOverrides,
) -> InvocationOverrides {
    let rpc = match new.rpc.clone() {
        Some(rpc) => Some(rpc),
        None if new.chain.is_some() => None,
        None => base.rpc.clone(),
    };

    InvocationOverrides {
        chain: new.chain.clone().or(base.chain.clone()),
        from: new.from.clone().or(base.from.clone()),
        rpc,
    }
}

pub(crate) enum ParsedLine {
    ReplCommand(Vec<String>),
    Cli { args: Vec<String>, cli: Cli },
    CliError(clap::Error),
}

pub(crate) fn resolved_color_mode(args: &[String], cli: &Cli, app: &BeamApp) -> ColorMode {
    if has_long_flag(args, "--color") {
        cli.color
    } else {
        app.color_mode
    }
}

pub(crate) fn resolved_output_mode(args: &[String], cli: &Cli, app: &BeamApp) -> OutputMode {
    if has_long_flag(args, "--output") {
        cli.output
    } else {
        app.output_mode
    }
}

fn parse_shell_words(line: &str) -> Result<Vec<String>> {
    let mut args = shlex::split(line).ok_or_else(|| repl_err(line))?;

    if matches!(args.first().map(String::as_str), Some("beam")) {
        args.remove(0);
    }

    Ok(args)
}

pub(crate) fn repl_err(cmd: impl Into<String>) -> Error {
    Error::UnknownReplCommand {
        command: cmd.into(),
    }
}

pub(crate) fn normalized_repl_command(command: Option<&str>) -> Option<&str> {
    let command = command?;
    matches!(
        command,
        "wallets" | "chains" | "rpc" | "balance" | "tokens" | "help"
    )
    .then_some(command)
}

fn is_cli_subcommand_invocation(command: &str, args: &[String]) -> bool {
    matches!(
        (command, args.get(1).map(String::as_str)),
        (
            "wallets",
            Some(
                "create"
                    | "import"
                    | "list"
                    | "rename"
                    | "address"
                    | "use"
                    | "help"
                    | "-h"
                    | "--help"
            )
        ) | (
            "chains",
            Some("list" | "add" | "remove" | "use" | "help" | "-h" | "--help")
        ) | (
            "rpc",
            Some("list" | "add" | "remove" | "use" | "help" | "-h" | "--help")
        ) | (
            "tokens",
            Some("list" | "add" | "remove" | "help" | "-h" | "--help")
        )
    )
}

fn has_long_flag(args: &[String], long_flag: &str) -> bool {
    args.iter().any(|arg| {
        arg == long_flag
            || arg
                .strip_prefix(long_flag)
                .is_some_and(|suffix| suffix.starts_with('='))
    })
}
