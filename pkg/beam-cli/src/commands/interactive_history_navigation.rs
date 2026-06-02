// lint-long-file-override allow-max-lines=300
use std::sync::{Arc, Mutex};

use rustyline::{
    Cmd, ConditionalEventHandler, Editor, Event, EventContext, EventHandler, Helper, KeyCode,
    KeyEvent, Modifiers, RepeatCount,
    history::{History, SearchDirection},
};

use super::interactive_history::ReplHistory;

pub(super) type HistoryNavigationStateHandle = Arc<Mutex<HistoryNavigationState>>;

pub(crate) struct HistoryNavigation {
    state: HistoryNavigationStateHandle,
}

impl HistoryNavigation {
    pub(crate) fn attach_to_history(&self, history: &mut ReplHistory) {
        history.set_navigation_state(Arc::clone(&self.state));
    }

    pub(crate) fn reset(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.mode = None;
    }
}

pub(crate) fn bind_matching_prefix_history_search<H, I>(
    editor: &mut Editor<H, I>,
) -> HistoryNavigation
where
    H: Helper,
    I: History,
{
    let history_navigation = HistoryNavigation {
        state: Arc::new(Mutex::new(HistoryNavigationState::default())),
    };
    bind_history_search(
        editor,
        KeyEvent(KeyCode::Up, Modifiers::NONE),
        SearchDirection::Reverse,
        Arc::clone(&history_navigation.state),
    );
    bind_history_search(
        editor,
        KeyEvent(KeyCode::Down, Modifiers::NONE),
        SearchDirection::Forward,
        Arc::clone(&history_navigation.state),
    );
    history_navigation
}

fn uses_matching_prefix_history_search(line: &str, pos: usize) -> bool {
    line.get(..pos).is_some_and(|prefix| {
        pos == line.len() && !line.contains('\n') && !prefix.trim().is_empty()
    })
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct HistoryNavigationState {
    mode: Option<HistoryNavigationMode>,
}

impl HistoryNavigationState {
    fn command(
        &mut self,
        line: &str,
        pos: usize,
        direction: SearchDirection,
        repeat_count: RepeatCount,
    ) -> Cmd {
        if line.contains('\n') {
            self.mode = Some(HistoryNavigationMode::Cycling { entry: None });
            return line_history_command(direction, repeat_count);
        }

        match &mut self.mode {
            Some(HistoryNavigationMode::Cycling { entry }) if entry.as_deref() == Some(line) => {
                return cycling_history_command(direction);
            }
            Some(HistoryNavigationMode::Prefix {
                prefix,
                pending_term,
            }) if line
                .get(..pos)
                .is_some_and(|term| term.starts_with(prefix.as_str())) =>
            {
                *pending_term = line.get(..pos).map(str::to_string);
                return prefix_history_command(direction);
            }
            _ => {}
        }

        if uses_matching_prefix_history_search(line, pos) {
            self.mode = Some(HistoryNavigationMode::Prefix {
                prefix: line[..pos].to_string(),
                pending_term: Some(line[..pos].to_string()),
            });
            prefix_history_command(direction)
        } else {
            self.mode = Some(HistoryNavigationMode::Cycling { entry: None });
            line_history_command(direction, repeat_count)
        }
    }

    pub(super) fn set_cycled_entry(&mut self, entry: &str) {
        if matches!(self.mode, Some(HistoryNavigationMode::Cycling { .. })) {
            self.mode = Some(HistoryNavigationMode::Cycling {
                entry: Some(entry.to_string()),
            });
        }
    }

    pub(super) fn take_prefix_search_term(&mut self, term: &str) -> Option<String> {
        match &mut self.mode {
            Some(HistoryNavigationMode::Prefix {
                prefix,
                pending_term,
                ..
            }) => {
                let expected = pending_term.take();
                expected
                    .as_deref()
                    .is_some_and(|expected| expected == term)
                    .then(|| prefix.clone())
            }
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum HistoryNavigationMode {
    Cycling {
        entry: Option<String>,
    },
    Prefix {
        prefix: String,
        pending_term: Option<String>,
    },
}

fn history_navigation_command(
    state: &mut HistoryNavigationState,
    line: &str,
    pos: usize,
    direction: SearchDirection,
    repeat_count: RepeatCount,
) -> Cmd {
    state.command(line, pos, direction, repeat_count)
}

#[cfg(test)]
#[path = "interactive_history_navigation_tests.rs"]
mod tests;

fn prefix_history_command(direction: SearchDirection) -> Cmd {
    match direction {
        SearchDirection::Reverse => Cmd::HistorySearchBackward,
        SearchDirection::Forward => Cmd::HistorySearchForward,
    }
}

fn cycling_history_command(direction: SearchDirection) -> Cmd {
    match direction {
        SearchDirection::Reverse => Cmd::PreviousHistory,
        SearchDirection::Forward => Cmd::NextHistory,
    }
}

fn line_history_command(direction: SearchDirection, repeat_count: RepeatCount) -> Cmd {
    match direction {
        SearchDirection::Reverse => Cmd::LineUpOrPreviousHistory(repeat_count),
        SearchDirection::Forward => Cmd::LineDownOrNextHistory(repeat_count),
    }
}

fn bind_history_search<H, I>(
    editor: &mut Editor<H, I>,
    key: KeyEvent,
    direction: SearchDirection,
    state: HistoryNavigationStateHandle,
) where
    H: Helper,
    I: History,
{
    editor.bind_sequence(
        key,
        EventHandler::Conditional(Box::new(PrefixHistorySearchHandler { direction, state })),
    );
}

struct PrefixHistorySearchHandler {
    direction: SearchDirection,
    state: HistoryNavigationStateHandle,
}

impl ConditionalEventHandler for PrefixHistorySearchHandler {
    fn handle(
        &self,
        _event: &Event,
        n: RepeatCount,
        _positive: bool,
        ctx: &EventContext,
    ) -> Option<Cmd> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        Some(history_navigation_command(
            &mut state,
            ctx.line(),
            ctx.pos(),
            self.direction,
            n,
        ))
    }
}
