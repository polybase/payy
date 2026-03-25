// lint-long-file-override allow-max-lines=300
use std::path::Path;

use clap::Parser;
use rustyline::{
    Cmd, ConditionalEventHandler, Config, Editor, Event, EventContext, EventHandler, Helper,
    KeyCode, KeyEvent, Modifiers, RepeatCount,
    history::{DefaultHistory, History, SearchDirection, SearchResult},
};

use crate::cli::Cli;

pub(crate) struct ReplHistory {
    inner: DefaultHistory,
}

impl ReplHistory {
    pub(crate) fn new() -> Self {
        Self::with_config(&Config::default())
    }

    pub(crate) fn with_config(config: &Config) -> Self {
        Self {
            inner: DefaultHistory::with_config(config),
        }
    }

    pub(crate) fn iter(&self) -> impl DoubleEndedIterator<Item = &String> + '_ {
        self.inner.iter()
    }
}

impl Default for ReplHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl History for ReplHistory {
    fn get(
        &self,
        index: usize,
        dir: SearchDirection,
    ) -> rustyline::Result<Option<SearchResult<'_>>> {
        self.inner.get(index, dir)
    }

    fn add(&mut self, line: &str) -> rustyline::Result<bool> {
        self.inner.add(line)
    }

    fn add_owned(&mut self, line: String) -> rustyline::Result<bool> {
        self.inner.add_owned(line)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn set_max_len(&mut self, len: usize) -> rustyline::Result<()> {
        self.inner.set_max_len(len)
    }

    fn ignore_dups(&mut self, yes: bool) -> rustyline::Result<()> {
        self.inner.ignore_dups(yes)
    }

    fn ignore_space(&mut self, yes: bool) {
        self.inner.ignore_space(yes);
    }

    fn save(&mut self, path: &Path) -> rustyline::Result<()> {
        self.inner.save(path)
    }

    fn append(&mut self, path: &Path) -> rustyline::Result<()> {
        self.inner.append(path)
    }

    fn load(&mut self, path: &Path) -> rustyline::Result<()> {
        self.inner.load(path)
    }

    fn clear(&mut self) -> rustyline::Result<()> {
        self.inner.clear()
    }

    fn search(
        &self,
        term: &str,
        start: usize,
        dir: SearchDirection,
    ) -> rustyline::Result<Option<SearchResult<'_>>> {
        self.inner.search(term, start, dir)
    }

    fn starts_with(
        &self,
        term: &str,
        start: usize,
        dir: SearchDirection,
    ) -> rustyline::Result<Option<SearchResult<'_>>> {
        Ok(self.inner.starts_with(term, start, dir)?.map(|mut result| {
            // Accepted prefix-history matches should behave like completed input:
            // the cursor belongs at the end of the inserted command.
            result.pos = result.entry.len();
            result
        }))
    }
}

pub(crate) fn sanitize_history(history: &mut ReplHistory) -> rustyline::Result<bool> {
    let retained = history
        .iter()
        .filter(|entry| should_persist_history(entry))
        .cloned()
        .collect::<Vec<_>>();

    if retained.len() == history.len() {
        return Ok(false);
    }

    history.clear()?;
    history.ignore_dups(false)?;
    for entry in retained {
        history.add_owned(entry)?;
    }
    history.ignore_dups(true)?;
    Ok(true)
}

pub(crate) fn bind_matching_prefix_history_search<H, I>(editor: &mut Editor<H, I>)
where
    H: Helper,
    I: History,
{
    bind_history_search(
        editor,
        KeyEvent(KeyCode::Up, Modifiers::NONE),
        SearchDirection::Reverse,
    );
    bind_history_search(
        editor,
        KeyEvent(KeyCode::Down, Modifiers::NONE),
        SearchDirection::Forward,
    );
}

pub(crate) fn uses_matching_prefix_history_search(line: &str, pos: usize) -> bool {
    line.get(..pos).is_some_and(|prefix| {
        pos == line.len() && !line.contains('\n') && !prefix.trim().is_empty()
    })
}

pub(crate) fn history_navigation_command(
    line: &str,
    pos: usize,
    direction: SearchDirection,
    repeat_count: RepeatCount,
) -> Cmd {
    if uses_matching_prefix_history_search(line, pos) {
        match direction {
            SearchDirection::Reverse => Cmd::HistorySearchBackward,
            SearchDirection::Forward => Cmd::HistorySearchForward,
        }
    } else {
        match direction {
            SearchDirection::Reverse => Cmd::LineUpOrPreviousHistory(repeat_count),
            SearchDirection::Forward => Cmd::LineDownOrNextHistory(repeat_count),
        }
    }
}

pub(crate) fn should_persist_history(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return false;
    }

    if let Some(args) = shlex::split(line) {
        if let Ok(cli) =
            Cli::try_parse_from(std::iter::once("beam").chain(args.iter().map(String::as_str)))
        {
            return match cli.command {
                Some(command) => !command.is_sensitive(),
                None => true,
            };
        }

        return !looks_like_sensitive_wallet_command(&args);
    }

    let args = line
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    !looks_like_sensitive_wallet_command(&args)
}

fn looks_like_sensitive_wallet_command(args: &[String]) -> bool {
    let Some(command_index) = command_index(args) else {
        return false;
    };

    matches!(
        args.get(command_index)
            .map(String::as_str)
            .map(normalized_command),
        Some("wallet" | "wallets")
    ) && matches!(
        args.get(command_index + 1).map(String::as_str),
        Some("import" | "address")
    )
}

fn normalized_command(command: &str) -> &str {
    command.strip_prefix('/').unwrap_or(command)
}

fn command_index(args: &[String]) -> Option<usize> {
    let mut index = 0;
    if args.get(index).map(String::as_str) == Some("beam") {
        index += 1;
    }

    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "--no-update-check" {
            index += 1;
            continue;
        }

        let flag = arg.split_once('=').map_or(arg, |(flag, _)| flag);
        if matches!(
            flag,
            "--chain" | "--color" | "--from" | "--output" | "--rpc"
        ) {
            index += if arg.contains('=') { 1 } else { 2 };
            continue;
        }

        return Some(index);
    }

    None
}

fn bind_history_search<H, I>(editor: &mut Editor<H, I>, key: KeyEvent, direction: SearchDirection)
where
    H: Helper,
    I: History,
{
    editor.bind_sequence(
        key,
        EventHandler::Conditional(Box::new(PrefixHistorySearchHandler { direction })),
    );
}

struct PrefixHistorySearchHandler {
    direction: SearchDirection,
}

impl ConditionalEventHandler for PrefixHistorySearchHandler {
    fn handle(
        &self,
        event: &Event,
        n: RepeatCount,
        positive: bool,
        ctx: &EventContext,
    ) -> Option<Cmd> {
        let _ = (event, n, positive);

        Some(history_navigation_command(
            ctx.line(),
            ctx.pos(),
            self.direction,
            n,
        ))
    }
}
