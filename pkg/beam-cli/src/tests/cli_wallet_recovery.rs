use clap::Parser;

use crate::cli::{Cli, Command, WalletAction};

#[test]
fn parses_recovery_phrase_wallet_sources() {
    let export = Cli::try_parse_from(["beam", "wallets", "export-recovery-phrase", "alice"])
        .expect("parse recovery phrase export");
    assert!(matches!(
        export.command,
        Some(Command::Wallet {
            action: WalletAction::ExportRecoveryPhrase { wallet }
        }) if wallet.as_deref() == Some("alice")
    ));

    let import = Cli::try_parse_from([
        "beam",
        "wallets",
        "import-recovery-phrase",
        "--phrase-stdin",
        "--expected-address",
        "0x1111111111111111111111111111111111111111",
        "--name",
        "alice",
    ])
    .expect("parse recovery phrase import");
    assert!(matches!(
        import.command,
        Some(Command::Wallet {
            action: WalletAction::ImportRecoveryPhrase {
                expected_address,
                phrase_source,
                name,
            }
        }) if name.as_deref() == Some("alice")
            && expected_address.as_deref() == Some("0x1111111111111111111111111111111111111111")
            && phrase_source.phrase_stdin
            && phrase_source.phrase_fd.is_none()
    ));

    let fd_import = Cli::try_parse_from([
        "beam",
        "wallets",
        "import-recovery-phrase",
        "--phrase-fd",
        "3",
    ])
    .expect("parse fd-backed recovery phrase import");
    assert!(matches!(
        fd_import.command,
        Some(Command::Wallet {
            action: WalletAction::ImportRecoveryPhrase { phrase_source, .. }
        }) if !phrase_source.phrase_stdin && phrase_source.phrase_fd == Some(3)
    ));

    Cli::try_parse_from([
        "beam",
        "wallets",
        "import-recovery-phrase",
        "--phrase-stdin",
        "--phrase-fd",
        "3",
    ])
    .expect_err("reject multiple recovery phrase sources");

    Cli::try_parse_from([
        "beam",
        "wallets",
        "import-recovery-phrase",
        "abandon abandon abandon abandon",
    ])
    .expect_err("reject positional recovery phrase");
}
