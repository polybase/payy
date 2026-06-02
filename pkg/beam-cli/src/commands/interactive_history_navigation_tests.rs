// lint-long-file-override allow-max-lines=300
use super::*;
use crate::commands::interactive_history::ReplHistory;
use rustyline::history::History;
use std::sync::{Arc, Mutex};

fn nav(line: &str, pos: usize, direction: SearchDirection, repeat_count: RepeatCount) -> Cmd {
    let mut state = HistoryNavigationState::default();
    history_navigation_command(&mut state, line, pos, direction, repeat_count)
}

fn assert_nav(
    state: &mut HistoryNavigationState,
    line: &str,
    pos: usize,
    direction: SearchDirection,
    expected: Cmd,
) {
    assert_eq!(
        history_navigation_command(state, line, pos, direction, 1),
        expected
    );
}

fn reverse(line: &str, pos: usize) -> Cmd {
    nav(line, pos, SearchDirection::Reverse, 1)
}

fn forward(line: &str, pos: usize) -> Cmd {
    nav(line, pos, SearchDirection::Forward, 1)
}

#[test]
fn prefix_history_navigation_only_runs_for_real_prefixes_at_line_end() {
    assert!(uses_matching_prefix_history_search(
        "transfer",
        "transfer".len()
    ));
    assert!(uses_matching_prefix_history_search(
        "transfer ",
        "transfer ".len()
    ));

    assert!(!uses_matching_prefix_history_search("", 0));
    assert!(!uses_matching_prefix_history_search("   ", 3));
    assert!(!uses_matching_prefix_history_search("transfer", 3));
    assert!(!uses_matching_prefix_history_search(
        "transfer\n0xabc",
        "transfer\n0xabc".len()
    ));
}

#[test]
fn up_and_down_fall_back_to_history_cycling_without_a_prefix() {
    assert_eq!(reverse("", 0), Cmd::LineUpOrPreviousHistory(1));
    assert_eq!(forward("", 0), Cmd::LineDownOrNextHistory(1));
    assert_eq!(
        nav("transfer", 3, SearchDirection::Reverse, 4),
        Cmd::LineUpOrPreviousHistory(4)
    );
}

#[test]
fn up_and_down_keep_prefix_history_search_when_typing_at_line_end() {
    assert_eq!(
        reverse("transfer", "transfer".len()),
        Cmd::HistorySearchBackward
    );
    assert_eq!(
        forward("transfer", "transfer".len()),
        Cmd::HistorySearchForward
    );
}

#[test]
fn multiline_entries_keep_line_navigation_fallback_and_latch_history() {
    let entry = "transfer alice.eth\n--amount 1";
    let mut state = HistoryNavigationState::default();

    assert_nav(&mut state, "", 0, SearchDirection::Reverse, reverse("", 0));
    state.set_cycled_entry(entry);
    assert_nav(
        &mut state,
        entry,
        entry.len(),
        SearchDirection::Reverse,
        Cmd::LineUpOrPreviousHistory(1),
    );
    assert_eq!(
        history_navigation_command(&mut state, entry, entry.len(), SearchDirection::Forward, 3),
        Cmd::LineDownOrNextHistory(3)
    );

    let mut state = HistoryNavigationState::default();
    assert_eq!(
        history_navigation_command(&mut state, entry, entry.len(), SearchDirection::Reverse, 2),
        Cmd::LineUpOrPreviousHistory(2)
    );
    state.set_cycled_entry("balance");
    assert_nav(
        &mut state,
        "balance",
        "balance".len(),
        SearchDirection::Reverse,
        Cmd::PreviousHistory,
    );
}

#[test]
fn repeated_history_cycling_does_not_switch_to_prefix_search() {
    let mut state = HistoryNavigationState::default();

    assert_nav(
        &mut state,
        "",
        0,
        SearchDirection::Reverse,
        Cmd::LineUpOrPreviousHistory(1),
    );
    state.set_cycled_entry("transfer alice.eth");
    assert_nav(
        &mut state,
        "transfer alice.eth",
        "transfer alice.eth".len(),
        SearchDirection::Reverse,
        Cmd::PreviousHistory,
    );
    assert_nav(
        &mut state,
        "transfer alice.eth",
        "transfer alice.eth".len(),
        SearchDirection::Forward,
        Cmd::NextHistory,
    );
}

#[test]
fn editing_after_history_cycling_reenables_prefix_search() {
    let mut state = HistoryNavigationState::default();

    assert_nav(
        &mut state,
        "",
        0,
        SearchDirection::Reverse,
        Cmd::LineUpOrPreviousHistory(1),
    );
    state.set_cycled_entry("balance");
    assert_nav(
        &mut state,
        "t",
        "t".len(),
        SearchDirection::Reverse,
        Cmd::HistorySearchBackward,
    );
}

#[test]
fn repeated_prefix_history_search_keeps_the_original_prefix() {
    let mut state = HistoryNavigationState::default();

    assert_nav(
        &mut state,
        "trans",
        "trans".len(),
        SearchDirection::Reverse,
        Cmd::HistorySearchBackward,
    );
    assert_nav(
        &mut state,
        "transfer alice.eth",
        "trans".len(),
        SearchDirection::Reverse,
        Cmd::HistorySearchBackward,
    );
    assert_nav(
        &mut state,
        "transfer alice.eth",
        "trans".len(),
        SearchDirection::Forward,
        Cmd::HistorySearchForward,
    );
}

#[test]
fn prefix_history_search_keeps_cursor_at_end_while_reusing_original_prefix() {
    let mut history = ReplHistory::new();
    history
        .add("transfer bob.eth")
        .expect("add first transfer history");
    history
        .add("transfer alice.eth")
        .expect("add second transfer history");

    let state: HistoryNavigationStateHandle =
        Arc::new(Mutex::new(HistoryNavigationState::default()));
    history.set_navigation_state(Arc::clone(&state));

    {
        let mut state = state.lock().expect("lock navigation state");
        assert_nav(
            &mut state,
            "trans",
            "trans".len(),
            SearchDirection::Reverse,
            Cmd::HistorySearchBackward,
        );
    }

    let first = history
        .starts_with("trans", history.len() - 1, SearchDirection::Reverse)
        .expect("search reverse history")
        .expect("find reverse history entry");
    assert_eq!(first.entry.as_ref(), "transfer alice.eth");
    assert_eq!(first.pos, first.entry.len());

    assert!(
        history
            .starts_with(
                "transfer alice.ethx",
                history.len() - 1,
                SearchDirection::Reverse,
            )
            .expect("normal history hint search")
            .is_none()
    );

    {
        let mut state = state.lock().expect("lock navigation state");
        assert_nav(
            &mut state,
            first.entry.as_ref(),
            first.entry.len(),
            SearchDirection::Reverse,
            Cmd::HistorySearchBackward,
        );
    }

    let repeated = history
        .starts_with(
            first.entry.as_ref(),
            first.idx - 1,
            SearchDirection::Reverse,
        )
        .expect("repeat reverse history search")
        .expect("find previous matching history entry");
    assert_eq!(repeated.entry.as_ref(), "transfer bob.eth");
    assert_eq!(repeated.pos, repeated.entry.len());
}

#[test]
fn prefix_history_search_stops_when_cursor_moves_before_original_prefix() {
    let mut state = HistoryNavigationState::default();

    assert_nav(
        &mut state,
        "trans",
        "trans".len(),
        SearchDirection::Reverse,
        Cmd::HistorySearchBackward,
    );
    assert_eq!(state.take_prefix_search_term("balance"), None);

    assert_nav(
        &mut state,
        "trans",
        "trans".len(),
        SearchDirection::Reverse,
        Cmd::HistorySearchBackward,
    );
    assert_eq!(state.take_prefix_search_term("transfer bob.ethx"), None);

    assert_nav(
        &mut state,
        "trans",
        "trans".len(),
        SearchDirection::Reverse,
        Cmd::HistorySearchBackward,
    );
    assert_eq!(
        state.take_prefix_search_term("trans"),
        Some("trans".to_string())
    );

    assert_nav(
        &mut state,
        "transfer alice.eth",
        2,
        SearchDirection::Reverse,
        Cmd::LineUpOrPreviousHistory(1),
    );
}
