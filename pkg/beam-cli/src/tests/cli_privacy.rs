use clap::Parser;

use crate::cli::{Cli, Command, PrivacyAction};

#[test]
fn parses_privacy_subcommands() {
    let address =
        Cli::try_parse_from(["beam", "privacy", "address"]).expect("parse privacy address");
    assert!(matches!(
        address.command,
        Some(Command::Privacy {
            action: PrivacyAction::Address
        })
    ));

    let send = Cli::try_parse_from([
        "beam",
        "privacy",
        "send",
        "--ephemeral",
        "USDC",
        "1.25",
        "--claim-link-message",
        "invoice",
    ])
    .expect("parse privacy send");
    assert!(matches!(
        send.command,
        Some(Command::Privacy {
            action: PrivacyAction::Send(args)
        }) if args.ephemeral
            && args.args == vec!["USDC".to_string(), "1.25".to_string()]
            && args.claim_link_message.as_deref() == Some("invoice")
    ));
}
