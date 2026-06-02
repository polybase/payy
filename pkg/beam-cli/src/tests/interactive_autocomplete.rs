use rustyline::{
    CompletionType, Config, Context, Editor,
    highlight::Highlighter,
    hint::Hinter,
    history::{DefaultHistory, History},
};

use crate::commands::{
    interactive_helper::{BeamHelper, completion_candidates},
    interactive_history::ReplHistory,
    interactive_history_navigation::bind_matching_prefix_history_search,
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
fn inline_hint_uses_repl_history_with_navigation_state() {
    let mut editor =
        Editor::<BeamHelper, ReplHistory>::with_history(Config::default(), ReplHistory::new())
            .expect("create beam repl editor");
    editor.set_helper(Some(BeamHelper::new()));
    let history_navigation = bind_matching_prefix_history_search(&mut editor);
    history_navigation.attach_to_history(editor.history_mut());
    editor
        .add_history_entry("transfer calummoore.eth")
        .expect("add transfer history");

    let helper = editor.helper().expect("beam helper");
    let ctx = Context::new(editor.history());

    assert_eq!(
        helper.hint("transfer", "transfer".len(), &ctx),
        Some(" calummoore.eth".to_string())
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
fn privacy_commands_are_completion_candidates() {
    assert!(
        completion_candidates("pri", 3)
            .iter()
            .any(|candidate| candidate == "privacy")
    );
    assert!(
        completion_candidates("privacy ", "privacy ".len())
            .iter()
            .any(|candidate| candidate == "incoming")
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
