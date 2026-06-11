use std::ffi::OsString;

use super::apps::APP_HELP_ARG;

pub(crate) fn normalize_cli_args<I, S>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut normalized = Vec::new();
    let mut command = None::<String>;
    let mut consume_flag_value = false;

    for (index, arg) in args.into_iter().enumerate() {
        let arg = arg.into();
        let Some(value) = arg.to_str().map(ToString::to_string) else {
            normalized.push(arg);
            continue;
        };

        if index == 0 {
            normalized.push(arg);
            continue;
        }

        if consume_flag_value {
            normalized.push(arg);
            consume_flag_value = false;
            continue;
        }

        if command.is_none() {
            if let Some(rewritten) = rewrite_output_flag(&value, false) {
                normalized.push(rewritten.into());
                if !value.contains('=') {
                    consume_flag_value = true;
                }
                continue;
            }

            if expects_global_flag_value(&value) {
                normalized.push(arg);
                consume_flag_value = !value.contains('=');
                continue;
            }

            if is_command_token(&value) {
                command = Some(value);
                normalized.push(arg);
                continue;
            }

            normalized.push(arg);
            continue;
        }

        if let Some(rewritten) = rewrite_output_flag(&value, command.as_deref() == Some("fetch")) {
            normalized.push(rewritten.into());
            if !value.contains('=') {
                consume_flag_value = true;
            }
            continue;
        }

        normalized.push(arg);
    }

    rewrite_installed_app_help_args(normalized)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppRunParseState {
    Root,
    XAwaitApp,
    XAfterApp,
    AppsAwaitAction,
    AppsRunAwaitApp,
    AppsRunAfterApp,
    OtherCommand,
}

fn rewrite_installed_app_help_args(args: Vec<OsString>) -> Vec<OsString> {
    let mut rewritten = Vec::with_capacity(args.len());
    let mut state = AppRunParseState::Root;
    let mut consume_flag_value = false;

    for (index, arg) in args.into_iter().enumerate() {
        if index == 0 {
            rewritten.push(arg);
            continue;
        }

        let Some(value) = arg.to_str().map(ToString::to_string) else {
            rewritten.push(arg);
            continue;
        };

        if consume_flag_value {
            rewritten.push(arg);
            consume_flag_value = false;
            continue;
        }

        if expects_global_flag_value(&value) {
            rewritten.push(arg);
            consume_flag_value = !value.contains('=');
            continue;
        }

        let replacement = match state {
            AppRunParseState::Root => {
                state = match value.as_str() {
                    "x" => AppRunParseState::XAwaitApp,
                    "apps" => AppRunParseState::AppsAwaitAction,
                    _ if is_command_token(&value) => AppRunParseState::OtherCommand,
                    _ => AppRunParseState::Root,
                };
                None
            }
            AppRunParseState::XAwaitApp => {
                if !is_flag(&value) {
                    state = AppRunParseState::XAfterApp;
                }
                None
            }
            AppRunParseState::XAfterApp => rewrite_help_flag(&value),
            AppRunParseState::AppsAwaitAction => {
                state = if value == "run" {
                    AppRunParseState::AppsRunAwaitApp
                } else {
                    AppRunParseState::OtherCommand
                };
                None
            }
            AppRunParseState::AppsRunAwaitApp => {
                if !is_flag(&value) {
                    state = AppRunParseState::AppsRunAfterApp;
                }
                None
            }
            AppRunParseState::AppsRunAfterApp => rewrite_help_flag(&value),
            AppRunParseState::OtherCommand => None,
        };

        rewritten.push(replacement.map_or(arg, OsString::from));
    }

    rewritten
}

fn is_flag(value: &str) -> bool {
    value.starts_with('-')
}

fn rewrite_help_flag(value: &str) -> Option<&'static str> {
    matches!(value, "--help" | "-h").then_some(APP_HELP_ARG)
}

fn rewrite_output_flag(value: &str, is_fetch_command: bool) -> Option<String> {
    if is_fetch_command {
        return None;
    }

    if value == "--output" {
        return Some("--format".to_string());
    }

    value
        .strip_prefix("--output=")
        .map(|suffix| format!("--format={suffix}"))
}

fn expects_global_flag_value(value: &str) -> bool {
    matches!(
        value.split_once('=').map_or(value, |(flag, _)| flag),
        "--chain" | "--color" | "--format" | "--from" | "--rpc"
    )
}

fn is_command_token(value: &str) -> bool {
    matches!(
        value,
        "wallets"
            | "util"
            | "chains"
            | "rpc"
            | "tokens"
            | "balance"
            | "transfer"
            | "txn"
            | "tx"
            | "block"
            | "erc20"
            | "call"
            | "send"
            | "fetch"
            | "update"
            | "__refresh-update-status"
    )
}
