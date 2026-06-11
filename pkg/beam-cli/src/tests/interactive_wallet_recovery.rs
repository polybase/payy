use crate::{
    cli::{Command, WalletAction},
    commands::interactive::{ParsedLine, parse_line, repl_command_args},
};

#[test]
fn recovery_phrase_wallet_commands_are_parsed_as_cli_subcommands_in_repl() {
    assert_eq!(
        repl_command_args("wallets export-recovery-phrase").expect("parse export command"),
        None
    );
    assert_eq!(
        repl_command_args("wallets import-recovery-phrase --phrase-stdin")
            .expect("parse import command"),
        None
    );

    match parse_line("wallets export-recovery-phrase alice").expect("parse export line") {
        ParsedLine::Cli { cli, .. } => assert!(matches!(
            cli.command,
            Some(Command::Wallet {
                action: WalletAction::ExportRecoveryPhrase { wallet }
            }) if wallet.as_deref() == Some("alice")
        )),
        ParsedLine::ReplCommand(_) | ParsedLine::CliError(_) => {
            panic!("expected recovery phrase export to parse as cli")
        }
    }

    match parse_line(
        "wallets import-recovery-phrase --phrase-fd 3 --expected-address 0x1111111111111111111111111111111111111111",
    )
    .expect("parse import line")
    {
        ParsedLine::Cli { cli, .. } => assert!(matches!(
            cli.command,
            Some(Command::Wallet {
                action: WalletAction::ImportRecoveryPhrase {
                    expected_address,
                    phrase_source,
                    ..
                }
            }) if phrase_source.phrase_fd == Some(3)
                && expected_address.as_deref() == Some("0x1111111111111111111111111111111111111111")
        )),
        ParsedLine::ReplCommand(_) | ParsedLine::CliError(_) => {
            panic!("expected recovery phrase import to parse as cli")
        }
    }
}
