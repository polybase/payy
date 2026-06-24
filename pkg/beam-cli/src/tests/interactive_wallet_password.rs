use crate::commands::{interactive::repl_command_args, interactive_helper::completion_candidates};

#[test]
fn change_password_wallet_command_is_parsed_as_cli_subcommand_in_repl() {
    assert_eq!(
        repl_command_args("wallets change-password alice").expect("parse password change"),
        None
    );
}

#[test]
fn change_password_wallet_command_is_completion_candidate() {
    let wallet = completion_candidates("wallets ", "wallets ".len());

    assert!(
        wallet
            .iter()
            .any(|candidate| candidate == "change-password")
    );
}
