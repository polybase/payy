use clap::{ArgMatches, CommandFactory, FromArgMatches, parser::ValueSource};
use contextful::ResultContextExt;

use crate::{
    cli::{Cli, normalize_cli_args},
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
    match Cli::command().try_get_matches_from(normalize_cli_args(
        std::iter::once("beam").chain(args.iter().map(String::as_str)),
    )) {
        Ok(matches) => {
            let cli = Cli::from_arg_matches(&matches).context("build beam repl cli from clap")?;
            Ok(ParsedLine::Cli {
                cli: Box::new(cli),
                global_flags: ParsedGlobalFlags {
                    color_explicit: is_command_line_value(&matches, "color"),
                    output_explicit: is_command_line_value(&matches, "output"),
                },
            })
        }
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ParsedGlobalFlags {
    pub(crate) color_explicit: bool,
    pub(crate) output_explicit: bool,
}

pub(crate) enum ParsedLine {
    ReplCommand(Vec<String>),
    Cli {
        cli: Box<Cli>,
        global_flags: ParsedGlobalFlags,
    },
    CliError(clap::Error),
}

pub(crate) fn resolved_color_mode(
    global_flags: ParsedGlobalFlags,
    cli: &Cli,
    app: &BeamApp,
) -> ColorMode {
    if global_flags.color_explicit {
        cli.color
    } else {
        app.color_mode
    }
}

pub(crate) fn resolved_output_mode(
    global_flags: ParsedGlobalFlags,
    cli: &Cli,
    app: &BeamApp,
) -> OutputMode {
    if global_flags.output_explicit {
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
                    | "export-private-key"
                    | "export-recovery-phrase"
                    | "import-recovery-phrase"
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

fn is_command_line_value(matches: &ArgMatches, arg_id: &str) -> bool {
    matches.value_source(arg_id) == Some(ValueSource::CommandLine)
}
