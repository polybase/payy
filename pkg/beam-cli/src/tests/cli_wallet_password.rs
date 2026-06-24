use clap::Parser;

use crate::cli::{Cli, Command, WalletAction};

#[test]
fn parses_wallet_change_password_subcommand() {
    let cli = Cli::try_parse_from(["beam", "wallets", "change-password", "alice"])
        .expect("parse wallet password change");

    assert!(matches!(
        cli.command,
        Some(Command::Wallet {
            action: WalletAction::ChangePassword { wallet }
        }) if wallet.as_deref() == Some("alice")
    ));
}
