use clap::Parser;

use crate::cli::{Cli, Command, Erc20GasAction, GasAction};

#[test]
fn parses_gas_estimation_commands() {
    let transfer = Cli::try_parse_from(["beam", "gas", "transfer", "0xrecipient", "0.01"])
        .expect("parse gas transfer");
    assert!(matches!(
        transfer.command,
        Some(Command::Gas {
            action: GasAction::Transfer(args)
        }) if args.to == "0xrecipient" && args.amount == "0.01"
    ));

    let send = Cli::try_parse_from([
        "beam",
        "estimate-gas",
        "send",
        "--value",
        "0.01",
        "0xcontract",
        "deposit(address)",
        "0xrecipient",
    ])
    .expect("parse gas send");
    assert!(matches!(
        send.command,
        Some(Command::Gas {
            action: GasAction::Send(args)
        }) if args.call.contract == "0xcontract"
            && args.call.function_sig == "deposit(address)"
            && args.call.args == vec!["0xrecipient".to_string()]
            && args.value.as_deref() == Some("0.01")
    ));

    let estimate = Cli::try_parse_from(["beam", "estimate", "transfer", "0xrecipient", "1"])
        .expect("parse estimate alias");
    assert!(matches!(
        estimate.command,
        Some(Command::Gas {
            action: GasAction::Transfer(_)
        })
    ));

    let erc20 = Cli::try_parse_from([
        "beam",
        "gas",
        "erc20",
        "approve",
        "USDC",
        "0xspender",
        "12.5",
    ])
    .expect("parse erc20 gas approve");
    assert!(matches!(
        erc20.command,
        Some(Command::Gas {
            action: GasAction::Erc20 {
                action: Erc20GasAction::Approve {
                    token,
                    spender,
                    amount,
                },
            }
        }) if token == "USDC" && spender == "0xspender" && amount == "12.5"
    ));
}
