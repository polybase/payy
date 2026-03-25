// lint-long-file-override allow-max-lines=300
use std::{
    borrow::Cow::{self, Borrowed, Owned},
    collections::HashSet,
};

use clap::{Arg, Command, CommandFactory};
use rustyline::{
    CompletionType, Context, Helper,
    completion::{Completer, Pair},
    highlight::Highlighter,
    hint::{Hinter, HistoryHinter},
    validate::{ValidationContext, ValidationResult, Validator},
};

use super::interactive_suggestion::completion_hint;
use crate::cli::Cli;

const REPL_OPTIONS: &[&str] = &[
    "wallets", "chains", "rpc", "balance", "tokens", "help", "exit",
];
const SUGGESTION_STYLE_PREFIX: &str = "\x1b[2m";
const SUGGESTION_STYLE_SUFFIX: &str = "\x1b[0m";

pub(crate) fn help_text() -> String {
    let mut cli = Cli::command().subcommand(Command::new("exit").about("Exit interactive mode"));
    cli.render_long_help().to_string()
}

pub(crate) fn completion_candidates(line: &str, pos: usize) -> Vec<String> {
    let head = &line[..pos];
    let start = head
        .rfind(|ch: char| ch.is_whitespace())
        .map_or(0, |index| index + 1);
    let needle = &head[start..];

    let tokens = completion_tokens(&head[..start]);
    let (root, current, expects_value) = completion_command(&tokens);
    let mut candidates = Vec::new();

    if tokens.is_empty() {
        candidates.extend(
            REPL_OPTIONS
                .iter()
                .map(|candidate| (*candidate).to_string()),
        );
    }

    if !expects_value {
        candidates.extend(current_visible_subcommands(&current));
        candidates.extend(current_visible_args(&current, false));

        if current.get_name() != root.get_name() {
            candidates.extend(current_visible_args(&root, true));
        }
    }

    candidates.extend(["-h".to_string(), "--help".to_string()]);
    filter_candidates(candidates, needle)
}

fn completion_tokens(head: &str) -> Vec<String> {
    let mut tokens = shlex::split(head).unwrap_or_else(|| {
        head.split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>()
    });

    if matches!(tokens.first().map(String::as_str), Some("beam")) {
        tokens.remove(0);
    }

    tokens
}

fn completion_command(tokens: &[String]) -> (Command, Command, bool) {
    let root = Cli::command();
    let mut current = root.clone();
    let mut expects_value = false;

    for token in tokens {
        if expects_value {
            expects_value = false;
            continue;
        }

        if token.starts_with('-') {
            expects_value = arg_for_token(&current, &root, token)
                .is_some_and(|arg| arg_takes_value(arg) && !token.contains('='));
            continue;
        }

        if let Some(subcommand) = current.find_subcommand(token) {
            current = subcommand.clone();
        }
    }

    (root, current, expects_value)
}

fn current_visible_subcommands(command: &Command) -> Vec<String> {
    command
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
        .flat_map(|subcommand| {
            std::iter::once(subcommand.get_name().to_string())
                .chain(subcommand.get_all_aliases().map(str::to_string))
        })
        .collect()
}

fn current_visible_args(command: &Command, globals_only: bool) -> Vec<String> {
    command
        .get_arguments()
        .filter(|arg| !arg.is_hide_set())
        .filter(|arg| !globals_only || arg.is_global_set())
        .flat_map(arg_spellings)
        .collect()
}

fn arg_spellings(arg: &Arg) -> Vec<String> {
    let mut values = Vec::new();

    if let Some(short) = arg.get_short() {
        values.push(format!("-{short}"));
    }
    if let Some(aliases) = arg.get_short_and_visible_aliases() {
        values.extend(aliases.into_iter().map(|short| format!("-{short}")));
    }
    if let Some(long) = arg.get_long() {
        values.push(format!("--{long}"));
    }
    if let Some(aliases) = arg.get_long_and_visible_aliases() {
        values.extend(aliases.into_iter().map(|long| format!("--{long}")));
    }

    values
}

fn arg_for_token<'a>(current: &'a Command, root: &'a Command, token: &str) -> Option<&'a Arg> {
    find_arg(current, token).or_else(|| find_arg(root, token).filter(|arg| arg.is_global_set()))
}

fn find_arg<'a>(command: &'a Command, token: &str) -> Option<&'a Arg> {
    if let Some(long) = token.strip_prefix("--") {
        let long = long.split('=').next().unwrap_or(long);
        return command.get_arguments().find(|arg| {
            arg.get_long() == Some(long)
                || arg
                    .get_long_and_visible_aliases()
                    .is_some_and(|aliases| aliases.into_iter().any(|alias| alias == long))
        });
    }

    if let Some(short) = token.strip_prefix('-') {
        let short = short.chars().next()?;
        return command.get_arguments().find(|arg| {
            arg.get_short() == Some(short)
                || arg
                    .get_short_and_visible_aliases()
                    .is_some_and(|aliases| aliases.into_iter().any(|alias| alias == short))
        });
    }

    None
}

fn arg_takes_value(arg: &Arg) -> bool {
    arg.get_action().takes_values()
}

fn filter_candidates(candidates: impl IntoIterator<Item = String>, needle: &str) -> Vec<String> {
    let mut seen = HashSet::new();

    candidates
        .into_iter()
        .filter(|candidate| candidate.starts_with(needle))
        .filter(|candidate| seen.insert(candidate.clone()))
        .collect()
}

#[derive(Default)]
struct ShellPrompt {
    plain: String,
    colored: String,
}

#[derive(Default)]
pub(crate) struct BeamHelper {
    shell_prompt: Option<ShellPrompt>,
}

impl BeamHelper {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn set_shell_prompt(&mut self, plain: String, colored: Option<String>) {
        self.shell_prompt = colored.map(|colored| ShellPrompt { plain, colored });
    }
}

impl Helper for BeamHelper {}

impl Highlighter for BeamHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        let _ = default;

        match &self.shell_prompt {
            Some(shell_prompt) if prompt == shell_prompt.plain => {
                Borrowed(shell_prompt.colored.as_str())
            }
            _ => Borrowed(prompt),
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned(format!(
            "{SUGGESTION_STYLE_PREFIX}{hint}{SUGGESTION_STYLE_SUFFIX}"
        ))
    }

    fn highlight_candidate<'c>(
        &self,
        candidate: &'c str,
        completion: CompletionType,
    ) -> Cow<'c, str> {
        let _ = completion;

        Owned(format!(
            "{SUGGESTION_STYLE_PREFIX}{candidate}{SUGGESTION_STYLE_SUFFIX}"
        ))
    }
}

impl Hinter for BeamHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<Self::Hint> {
        HistoryHinter::default()
            .hint(line, pos, ctx)
            .or_else(|| completion_hint(line, pos))
    }
}

impl Validator for BeamHelper {
    fn validate(&self, _ctx: &mut ValidationContext<'_>) -> rustyline::Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }
}

impl Completer for BeamHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let head = &line[..pos];
        let start = head
            .rfind(|ch: char| ch.is_whitespace())
            .map_or(0, |index| index + 1);

        Ok((
            start,
            completion_candidates(line, pos)
                .into_iter()
                .map(|candidate| Pair {
                    display: candidate.clone(),
                    replacement: candidate,
                })
                .collect(),
        ))
    }
}
