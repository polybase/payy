#[path = "interactive_history_sensitive.rs"]
mod sensitive;

use std::path::Path;

use clap::Parser;
use rustyline::{
    Config,
    history::{DefaultHistory, History, SearchDirection, SearchResult},
};

use super::interactive_history_navigation::HistoryNavigationStateHandle;
use crate::cli::{Cli, normalize_cli_args};
use sensitive::looks_like_sensitive_command;

pub(crate) struct ReplHistory {
    inner: DefaultHistory,
    navigation_state: Option<HistoryNavigationStateHandle>,
}

impl ReplHistory {
    pub(crate) fn new() -> Self {
        Self::with_config(&Config::default())
    }

    pub(crate) fn with_config(config: &Config) -> Self {
        Self {
            inner: DefaultHistory::with_config(config),
            navigation_state: None,
        }
    }

    pub(crate) fn iter(&self) -> impl DoubleEndedIterator<Item = &String> + '_ {
        self.inner.iter()
    }

    pub(super) fn set_navigation_state(&mut self, state: HistoryNavigationStateHandle) {
        self.navigation_state = Some(state);
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
        let result = self.inner.get(index, dir)?;
        if let (Some(state), Some(result)) = (&self.navigation_state, &result) {
            let mut state = state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.set_cycled_entry(result.entry.as_ref());
        }
        Ok(result)
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
        if let Some(state) = &self.navigation_state {
            let mut state = state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(prefix) = state.take_prefix_search_term(term) {
                if let Some(mut result) = self.inner.starts_with(&prefix, start, dir)? {
                    result.pos = result.entry.len();
                    return Ok(Some(result));
                }
                return Ok(None);
            }
        }

        self.inner.starts_with(term, start, dir)
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

pub(crate) fn should_persist_history(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return false;
    }

    if let Some(args) = shlex::split(line) {
        if let Ok(cli) = Cli::try_parse_from(normalize_cli_args(
            std::iter::once("beam").chain(args.iter().map(String::as_str)),
        )) {
            return match cli.command {
                Some(command) => !command.is_sensitive(),
                None => true,
            };
        }

        return !looks_like_sensitive_command(&args);
    }

    let args = line
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    !looks_like_sensitive_command(&args)
}
