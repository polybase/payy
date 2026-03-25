use rustyline::{
    Cmd, CompletionType, Context,
    highlight::Highlighter,
    hint::Hinter,
    history::{DefaultHistory, History, SearchDirection},
};

use crate::commands::{
    interactive::uses_matching_prefix_history_search,
    interactive_helper::BeamHelper,
    interactive_history::{ReplHistory, history_navigation_command},
};

#[test]
fn inline_hint_prefers_matching_history_entries() {
    let mut history = DefaultHistory::new();
    history
        .add("transfer calummoore.eth")
        .expect("add transfer history");
    history
        .add("wallets create alice")
        .expect("add wallet history");

    let helper = BeamHelper::new();
    let ctx = Context::new(&history);

    assert_eq!(
        helper.hint("transfer", "transfer".len(), &ctx),
        Some(" calummoore.eth".to_string())
    );
    assert_eq!(
        helper.hint("wallets ", "wallets ".len(), &ctx),
        Some("create alice".to_string())
    );
}

#[test]
fn inline_hint_falls_back_to_completion_prefixes() {
    let history = DefaultHistory::new();
    let helper = BeamHelper::new();
    let ctx = Context::new(&history);

    assert_eq!(
        helper.hint("wallets imp", "wallets imp".len(), &ctx),
        Some("ort".to_string())
    );
    assert_eq!(
        helper.hint("wallets import --pri", "wallets import --pri".len(), &ctx),
        Some("vate-key-".to_string())
    );
}

#[test]
fn inline_hint_skips_ambiguous_static_suggestions() {
    let history = DefaultHistory::new();
    let helper = BeamHelper::new();
    let ctx = Context::new(&history);

    assert_eq!(helper.hint("t", 1, &ctx), None);
}

#[test]
fn interactive_suggestions_are_dimmed() {
    let helper = BeamHelper::new();

    assert_eq!(
        helper.highlight_hint("wallets").as_ref(),
        "\u{1b}[2mwallets\u{1b}[0m"
    );
    assert_eq!(
        helper
            .highlight_candidate("wallets", CompletionType::List)
            .as_ref(),
        "\u{1b}[2mwallets\u{1b}[0m"
    );
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
    assert_eq!(
        history_navigation_command("", 0, SearchDirection::Reverse, 1),
        Cmd::LineUpOrPreviousHistory(1)
    );
    assert_eq!(
        history_navigation_command("", 0, SearchDirection::Forward, 1),
        Cmd::LineDownOrNextHistory(1)
    );
    assert_eq!(
        history_navigation_command("transfer", 3, SearchDirection::Reverse, 4),
        Cmd::LineUpOrPreviousHistory(4)
    );
}

#[test]
fn up_and_down_keep_prefix_history_search_when_typing_at_line_end() {
    assert_eq!(
        history_navigation_command("transfer", "transfer".len(), SearchDirection::Reverse, 1),
        Cmd::HistorySearchBackward
    );
    assert_eq!(
        history_navigation_command("transfer", "transfer".len(), SearchDirection::Forward, 1),
        Cmd::HistorySearchForward
    );
}

#[test]
fn prefix_history_search_places_cursor_at_end_of_selected_entry() {
    let mut history = ReplHistory::new();
    history
        .add("transfer calummoore.eth")
        .expect("add first transfer history");
    history
        .add("transfer alice.eth")
        .expect("add second transfer history");

    let term = "trans";
    let reverse = history
        .starts_with(term, history.len() - 1, SearchDirection::Reverse)
        .expect("search reverse history")
        .expect("find reverse history entry");
    assert_eq!(reverse.pos, reverse.entry.len());
    assert!(reverse.pos > term.len());

    let forward = history
        .starts_with(term, 0, SearchDirection::Forward)
        .expect("search forward history")
        .expect("find forward history entry");
    assert_eq!(forward.pos, forward.entry.len());
    assert!(forward.pos > term.len());
}
