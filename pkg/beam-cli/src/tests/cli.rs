// lint-long-file-override allow-max-lines=300
use clap::{CommandFactory, Parser};

use crate::{
    cli::{
        BlockArgs, ChainAction, Cli, Command, Erc20Action, RpcAction, TokenAction, TxnArgs,
        WalletAction, util::UtilAction,
    },
    display::ColorMode,
    output::OutputMode,
};

#[test]
fn parses_interactive_defaults() {
    let cli = Cli::try_parse_from(["beam"]).expect("parse interactive cli");

    assert!(cli.is_interactive());
    assert_eq!(cli.color, ColorMode::Auto);
    assert_eq!(cli.output, OutputMode::Default);
    assert!(cli.command.is_none());
}

#[test]
fn parses_hidden_background_update_command() {
    let cli = Cli::try_parse_from(["beam", "--no-update-check", "__refresh-update-status"])
        .expect("parse hidden update refresh command");

    assert!(cli.no_update_check);
    assert!(matches!(cli.command, Some(Command::RefreshUpdateStatus)));
}

#[test]
fn parses_global_overrides_and_balance_command() {
    let cli = Cli::try_parse_from([
        "beam",
        "--chain",
        "base",
        "--from",
        "alice",
        "--rpc",
        "https://beam.example/rpc",
        "--color",
        "never",
        "balance",
        "USDC",
    ])
    .expect("parse balance cli");

    let overrides = cli.overrides();
    assert_eq!(overrides.chain.as_deref(), Some("base"));
    assert_eq!(cli.color, ColorMode::Never);
    assert_eq!(overrides.from.as_deref(), Some("alice"));
    assert_eq!(overrides.rpc.as_deref(), Some("https://beam.example/rpc"));
    assert!(matches!(
        cli.command,
        Some(Command::Balance(args)) if args.token.as_deref() == Some("USDC")
    ));
}

#[test]
fn parses_wallet_and_erc20_subcommands() {
    let wallet = Cli::try_parse_from([
        "beam",
        "wallets",
        "import",
        "--private-key-stdin",
        "--name",
        "alice",
    ])
    .expect("parse wallet import");
    assert!(matches!(
        wallet.command,
        Some(Command::Wallet {
            action: WalletAction::Import {
                private_key_source,
                name,
            }
        }) if name.as_deref() == Some("alice")
            && private_key_source.private_key_stdin
            && private_key_source.private_key_fd.is_none()
    ));

    let rename = Cli::try_parse_from(["beam", "wallets", "rename", "alice", "primary"])
        .expect("parse wallet rename");
    assert!(matches!(
        rename.command,
        Some(Command::Wallet {
            action: WalletAction::Rename { name, new_name }
        }) if name == "alice" && new_name == "primary"
    ));

    let erc20 = Cli::try_parse_from(["beam", "erc20", "approve", "USDC", "0xspender", "12.5"])
        .expect("parse erc20 approve");
    assert!(matches!(
        erc20.command,
        Some(Command::Erc20 {
            action: Erc20Action::Approve { token, spender, amount }
        }) if token == "USDC" && spender == "0xspender" && amount == "12.5"
    ));

    let chain = Cli::try_parse_from(["beam", "chains", "use", "base"]).expect("parse chain use");
    assert!(matches!(
        chain.command,
        Some(Command::Chain {
            action: ChainAction::Use { chain }
        }) if chain == "base"
    ));
    Cli::try_parse_from(["beam", "wallet", "list"]).expect_err("reject singular wallets command");
    Cli::try_parse_from(["beam", "chain", "list"]).expect_err("reject singular chains command");

    let rpc = Cli::try_parse_from([
        "beam",
        "--chain",
        "base",
        "rpc",
        "add",
        "https://beam.example/base",
    ])
    .expect("parse rpc add");
    assert!(matches!(
        rpc.command,
        Some(Command::Rpc {
            action: RpcAction::Add(args)
        }) if args.rpc.as_deref() == Some("https://beam.example/base")
    ));

    let tokens = Cli::try_parse_from([
        "beam",
        "tokens",
        "add",
        "0xabc",
        "BEAMUSD",
        "--decimals",
        "6",
    ])
    .expect("parse tokens add");
    assert!(matches!(
        tokens.command,
        Some(Command::Tokens {
            action: Some(TokenAction::Add(args))
        }) if args.token.as_deref() == Some("0xabc")
            && args.label.as_deref() == Some("BEAMUSD")
            && args.decimals == Some(6)
    ));

    let tokens_list = Cli::try_parse_from(["beam", "tokens"]).expect("parse bare tokens command");
    assert!(matches!(
        tokens_list.command,
        Some(Command::Tokens { action: None })
    ));
}

#[test]
fn parses_send_value_for_payable_contract_calls() {
    let cli = Cli::try_parse_from([
        "beam",
        "send",
        "--value",
        "0.01",
        "0xcontract",
        "deposit(address)",
        "0xrecipient",
    ])
    .expect("parse payable send");

    let Some(Command::Send(args)) = cli.command else {
        panic!("expected send command");
    };

    assert_eq!(args.call.contract, "0xcontract");
    assert_eq!(args.call.function_sig, "deposit(address)");
    assert_eq!(args.call.args, vec!["0xrecipient".to_string()]);
    assert_eq!(args.value.as_deref(), Some("0.01"));
}

#[test]
fn parses_transaction_and_block_inspection_commands() {
    let txn = Cli::try_parse_from([
        "beam",
        "txn",
        "0x00000000000000000000000000000000000000000000000000000000000000aa",
    ])
    .expect("parse txn command");
    assert!(matches!(
        txn.command,
        Some(Command::Txn(TxnArgs { tx_hash })) if tx_hash == "0x00000000000000000000000000000000000000000000000000000000000000aa"
    ));

    let tx_alias = Cli::try_parse_from([
        "beam",
        "tx",
        "0x00000000000000000000000000000000000000000000000000000000000000bb",
    ])
    .expect("parse tx alias");
    assert!(matches!(
        tx_alias.command,
        Some(Command::Txn(TxnArgs { tx_hash })) if tx_hash == "0x00000000000000000000000000000000000000000000000000000000000000bb"
    ));

    let block = Cli::try_parse_from(["beam", "block", "latest"]).expect("parse block command");
    assert!(matches!(
        block.command,
        Some(Command::Block(BlockArgs { block })) if block.as_deref() == Some("latest")
    ));
}

#[test]
fn parses_explicit_color_modes() {
    let cli = Cli::try_parse_from(["beam", "--color", "always", "wallets", "list"])
        .expect("parse explicit color mode");

    assert_eq!(cli.color, ColorMode::Always);
}

#[test]
fn rejects_positional_private_keys_and_parses_secure_wallet_sources() {
    Cli::try_parse_from(["beam", "wallets", "import", "0x1234"])
        .expect_err("reject positional private key");

    let import = Cli::try_parse_from(["beam", "wallets", "import", "--name", "alice"])
        .expect("parse prompt-backed wallet import");
    assert!(matches!(
        import.command,
        Some(Command::Wallet {
            action: WalletAction::Import {
                private_key_source,
                name,
            }
        }) if name.as_deref() == Some("alice")
            && !private_key_source.private_key_stdin
            && private_key_source.private_key_fd.is_none()
    ));

    let address = Cli::try_parse_from(["beam", "wallets", "address", "--private-key-fd", "3"])
        .expect("parse fd-backed wallet address");
    assert!(matches!(
        address.command,
        Some(Command::Wallet {
            action: WalletAction::Address { private_key_source }
        }) if !private_key_source.private_key_stdin
            && private_key_source.private_key_fd == Some(3)
    ));

    Cli::try_parse_from([
        "beam",
        "wallets",
        "address",
        "--private-key-stdin",
        "--private-key-fd",
        "3",
    ])
    .expect_err("reject multiple private key sources");
}

#[test]
fn parses_util_subcommands() {
    let sig = Cli::try_parse_from(["beam", "util", "sig", "transfer(address,uint256)"])
        .expect("parse util sig");
    assert!(matches!(
        sig.command,
        Some(Command::Util {
            action: UtilAction::Sig(args)
        }) if args.value.as_deref() == Some("transfer(address,uint256)")
    ));

    let fixed = Cli::try_parse_from(["beam", "util", "from-fixed-point", "3", "1.23"])
        .expect("parse util from-fixed-point");
    assert!(matches!(
        fixed.command,
        Some(Command::Util {
            action: UtilAction::FromFixedPoint(args)
        }) if args.decimals.as_deref() == Some("3") && args.value.as_deref() == Some("1.23")
    ));
}

#[test]
fn visible_commands_have_descriptions() {
    let cli = Cli::command();

    assert_visible_commands_have_descriptions(&cli);
}

fn assert_visible_commands_have_descriptions(command: &clap::Command) {
    for subcommand in command.get_subcommands() {
        if subcommand.is_hide_set() {
            continue;
        }

        assert!(
            subcommand.get_about().is_some() || subcommand.get_long_about().is_some(),
            "subcommand `{}` under `{}` is missing a description",
            subcommand.get_name(),
            command.get_name(),
        );

        assert_visible_commands_have_descriptions(subcommand);
    }
}
