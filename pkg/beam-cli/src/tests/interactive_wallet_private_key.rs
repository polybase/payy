use crate::{
    cli::{Command, WalletAction},
    commands::interactive::{ParsedLine, parse_line},
};

#[test]
fn interactive_parser_routes_export_private_key_to_cli_command() {
    let parsed =
        parse_line("wallets export-private-key alice").expect("parse wallet private key export");
    let ParsedLine::Cli { cli, .. } = parsed else {
        panic!("expected clap command");
    };

    assert!(matches!(
        &cli.command,
        Some(Command::Wallet {
            action: WalletAction::ExportPrivateKey { wallet },
        }) if wallet.as_deref() == Some("alice")
    ));
}
