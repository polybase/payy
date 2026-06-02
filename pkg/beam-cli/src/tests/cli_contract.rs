use clap::Parser;

use crate::cli::{Cli, Command, ContractAction};

#[test]
fn parses_contract_inspection_commands_without_code_alias() {
    let info = Cli::try_parse_from([
        "beam",
        "contract",
        "info",
        "0x1111111111111111111111111111111111111111",
    ])
    .expect("parse contract info");
    assert!(matches!(
        info.command,
        Some(Command::Contract {
            action: ContractAction::Info(args)
        }) if args.address == "0x1111111111111111111111111111111111111111"
    ));

    let bytecode = Cli::try_parse_from([
        "beam",
        "contract",
        "bytecode",
        "0x1111111111111111111111111111111111111111",
        "--block",
        "safe",
    ])
    .expect("parse contract bytecode");
    assert!(matches!(
        bytecode.command,
        Some(Command::Contract {
            action: ContractAction::Bytecode(args)
        }) if args.block.as_deref() == Some("safe")
    ));

    let source = Cli::try_parse_from([
        "beam",
        "contract",
        "source",
        "0x1111111111111111111111111111111111111111",
        "Foo.sol",
    ])
    .expect("parse contract source");
    assert!(matches!(
        source.command,
        Some(Command::Contract {
            action: ContractAction::Source(args)
        }) if args.source_path.as_deref() == Some("Foo.sol")
    ));

    Cli::try_parse_from([
        "beam",
        "contract",
        "code",
        "0x1111111111111111111111111111111111111111",
    ])
    .expect_err("contract code alias is not supported");
}
