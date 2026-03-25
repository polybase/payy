// lint-long-file-override allow-max-lines=400
use rustyline::highlight::Highlighter;

use super::fixtures::test_app;
use crate::{
    cli::{ChainAction, Command, Erc20Action, RpcAction, WalletAction},
    commands::interactive::{
        ParsedLine, is_exit_command, merge_overrides, parse_line, repl_command_args,
        set_repl_chain_override, should_persist_history,
    },
    commands::interactive_helper::{BeamHelper, completion_candidates, help_text},
    display::{render_colored_shell_prefix, render_shell_prefix},
    error::Error,
    runtime::InvocationOverrides,
};

#[test]
fn excludes_sensitive_wallet_commands_from_repl_history() {
    assert!(!should_persist_history("wallets import"));
    assert!(!should_persist_history(
        "wallets import --private-key-stdin --name alice"
    ));
    assert!(!should_persist_history(
        "/wallets import --private-key-stdin --name alice"
    ));
    assert!(!should_persist_history("wallets address"));
    assert!(!should_persist_history("/wallets address 0x1234"));
    assert!(!should_persist_history(
        "--chain base wallets import 0x1234"
    ));
    assert!(!should_persist_history(
        "--chain base /wallets import 0x1234"
    ));
    assert!(!should_persist_history(
        "--color never wallets import --private-key-stdin --name alice"
    ));
    assert!(!should_persist_history(
        "--output=json wallets address 0x1234"
    ));
    assert!(!should_persist_history(r#"wallets import "0x1234"#));

    assert!(should_persist_history("wallets list"));
    assert!(should_persist_history("wallets alice"));
    assert!(should_persist_history("balance"));
}

#[test]
fn recognizes_bare_repl_shortcuts_without_breaking_cli_subcommands() {
    assert_eq!(
        repl_command_args("wallets alice").expect("parse wallet shortcut"),
        Some(vec!["wallets".to_string(), "alice".to_string()])
    );
    assert_eq!(
        repl_command_args("/wallets alice").expect("ignore slash wallet shortcut"),
        None
    );
    assert_eq!(
        repl_command_args("chains base").expect("parse chain shortcut"),
        Some(vec!["chains".to_string(), "base".to_string()])
    );
    assert_eq!(
        repl_command_args("chains use base").expect("parse chain subcommand"),
        None
    );
    assert_eq!(
        repl_command_args("rpc https://beam.example/rpc").expect("parse rpc shortcut"),
        Some(vec![
            "rpc".to_string(),
            "https://beam.example/rpc".to_string(),
        ])
    );
    assert_eq!(
        repl_command_args("rpc list").expect("parse rpc subcommand"),
        None
    );
    assert_eq!(
        repl_command_args("wallets list").expect("parse wallet list"),
        None
    );
    assert_eq!(
        repl_command_args("wallets rename alice primary").expect("parse wallet rename"),
        None
    );
    assert_eq!(
        repl_command_args("balance 0xabc").expect("parse balance with address"),
        None
    );
}

#[test]
fn slash_prefixed_commands_fall_back_to_clap_errors() {
    let parsed = parse_line("/wallets alice").expect("parse slash wallet command");
    assert!(matches!(parsed, ParsedLine::CliError(_)));

    let parsed = parse_line("/exit").expect("parse slash exit command");
    assert!(matches!(parsed, ParsedLine::CliError(_)));
}

#[test]
fn interactive_parser_preserves_clap_help_for_wallet_commands() {
    let parsed = parse_line("wallets --help").expect("parse wallet help");
    let ParsedLine::CliError(err) = parsed else {
        panic!("expected clap help output");
    };

    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    assert!(!err.use_stderr());
    assert!(err.render().to_string().contains("Usage: beam wallets"));
}

#[test]
fn interactive_parser_accepts_regular_cli_commands() {
    let parsed = parse_line("wallets create alice").expect("parse wallet create");
    let ParsedLine::Cli { cli, .. } = parsed else {
        panic!("expected clap command");
    };
    assert!(matches!(
        &cli.command,
        Some(Command::Wallet {
            action: WalletAction::Create { name },
        }) if name.as_deref() == Some("alice")
    ));

    let parsed = parse_line("transfer 0xabc 1").expect("parse transfer");
    let ParsedLine::Cli { cli, .. } = parsed else {
        panic!("expected clap command");
    };
    assert!(matches!(
        &cli.command,
        Some(Command::Transfer(args)) if args.to == "0xabc" && args.amount == "1"
    ));

    let parsed = parse_line("txn 0xabc").expect("parse txn");
    let ParsedLine::Cli { cli, .. } = parsed else {
        panic!("expected clap command");
    };
    assert!(matches!(
        &cli.command,
        Some(Command::Txn(args)) if args.tx_hash == "0xabc"
    ));

    let parsed = parse_line("block latest").expect("parse block");
    let ParsedLine::Cli { cli, .. } = parsed else {
        panic!("expected clap command");
    };
    assert!(matches!(
        &cli.command,
        Some(Command::Block(args)) if args.block.as_deref() == Some("latest")
    ));

    let parsed = parse_line("erc20 approve USDC 0xspender 12.5").expect("parse erc20 approve");
    let ParsedLine::Cli { cli, .. } = parsed else {
        panic!("expected clap command");
    };
    assert!(matches!(
        &cli.command,
        Some(Command::Erc20 {
            action: Erc20Action::Approve {
                token,
                spender,
                amount,
            },
        }) if token == "USDC" && spender == "0xspender" && amount == "12.5"
    ));

    let parsed = parse_line("chains use base").expect("parse chain use");
    let ParsedLine::Cli { cli, .. } = parsed else {
        panic!("expected clap command");
    };
    assert!(matches!(
        &cli.command,
        Some(Command::Chain {
            action: ChainAction::Use { chain },
        }) if chain == "base"
    ));

    let parsed =
        parse_line("--chain base rpc use https://beam.example/base").expect("parse rpc use");
    let ParsedLine::Cli { cli, .. } = parsed else {
        panic!("expected clap command");
    };
    assert!(matches!(
        &cli.command,
        Some(Command::Rpc {
            action: RpcAction::Use { rpc },
        }) if rpc == "https://beam.example/base"
    ));
}

#[test]
fn interactive_parser_accepts_optional_beam_prefix() {
    let parsed = parse_line("beam wallets create alice").expect("parse prefixed wallet create");
    let ParsedLine::Cli { cli, .. } = parsed else {
        panic!("expected clap command");
    };
    assert!(matches!(
        &cli.command,
        Some(Command::Wallet {
            action: WalletAction::Create { name },
        }) if name.as_deref() == Some("alice")
    ));

    let parsed = parse_line("beam --help").expect("parse prefixed help");
    let ParsedLine::CliError(err) = parsed else {
        panic!("expected clap help output");
    };

    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    assert!(err.render().to_string().contains("Usage: beam"));
}

#[test]
fn interactive_help_and_completion_surface_full_cli() {
    let help = help_text();
    assert!(help.contains("Usage: beam"));
    for expected in [
        "transfer", "txn", "block", "erc20", "wallets", "chains", "rpc", "exit",
    ] {
        assert!(help.contains(expected));
    }
    assert!(!help.contains("Session shortcuts:"));
    assert!(!help.contains("with or without a leading `beam`"));

    let top_level = completion_candidates("", 0);
    for expected in [
        "transfer", "txn", "block", "erc20", "wallets", "chains", "rpc", "exit", "--chain",
    ] {
        assert!(top_level.iter().any(|candidate| candidate == expected));
    }
    assert!(!top_level.iter().any(|candidate| candidate.starts_with('/')));

    let wallet = completion_candidates("wallets ", "wallets ".len());
    for expected in ["create", "import", "--help"] {
        assert!(wallet.iter().any(|candidate| candidate == expected));
    }

    let wallet_import = completion_candidates("wallets import --", "wallets import --".len());
    for expected in [
        "--name",
        "--private-key-stdin",
        "--private-key-fd",
        "--chain",
    ] {
        assert!(wallet_import.iter().any(|candidate| candidate == expected));
    }
}

#[test]
fn repl_helper_only_colorizes_the_active_shell_prompt() {
    let plain = render_shell_prefix(
        "wallet-1 0x740747e7...e3a1e112",
        "ethereum",
        "https://et...node.com",
    );
    let colored = render_colored_shell_prefix(
        "wallet-1 0x740747e7...e3a1e112",
        "ethereum",
        "https://et...node.com",
    );
    let mut helper = BeamHelper::new();
    helper.set_shell_prompt(plain.clone(), Some(colored.clone()));

    assert_eq!(helper.highlight_prompt(&plain, false).as_ref(), colored);
    assert_eq!(
        helper
            .highlight_prompt("(reverse-i-search)`wallets': ", false)
            .as_ref(),
        "(reverse-i-search)`wallets': "
    );
}

#[test]
fn recognizes_bare_exit_commands_only() {
    assert!(is_exit_command("exit"));
    assert!(is_exit_command("beam exit"));
    assert!(!is_exit_command("/exit"));
    assert!(!is_exit_command("quit"));
    assert!(!is_exit_command("/quit"));
    assert!(!is_exit_command("quit now"));
}

#[tokio::test]
async fn repl_chain_command_rejects_unknown_chain_without_mutating_state() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    let mut overrides = InvocationOverrides {
        chain: Some("base".to_string()),
        rpc: Some("https://beam.example/base".to_string()),
        ..InvocationOverrides::default()
    };

    let err = set_repl_chain_override(&app, &mut overrides, Some("nope"))
        .await
        .expect_err("reject unknown chain");

    assert!(matches!(err, Error::UnknownChain { chain } if chain == "nope"));
    assert_eq!(overrides.chain.as_deref(), Some("base"));
    assert_eq!(overrides.rpc.as_deref(), Some("https://beam.example/base"));
}

#[tokio::test]
async fn repl_chain_command_stores_canonical_chain_key() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    let mut overrides = InvocationOverrides {
        rpc: Some("https://beam.example/base".to_string()),
        ..InvocationOverrides::default()
    };

    set_repl_chain_override(&app, &mut overrides, Some("8453"))
        .await
        .expect("set base chain");

    assert_eq!(overrides.chain.as_deref(), Some("base"));
    assert_eq!(overrides.rpc, None);
}

#[tokio::test]
async fn repl_chain_command_clears_rpc_when_resetting_to_default_chain() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    let mut overrides = InvocationOverrides {
        chain: Some("base".to_string()),
        rpc: Some("https://beam.example/base".to_string()),
        ..InvocationOverrides::default()
    };

    set_repl_chain_override(&app, &mut overrides, None)
        .await
        .expect("clear chain override");

    assert_eq!(overrides.chain, None);
    assert_eq!(overrides.rpc, None);
}

#[test]
fn repl_cli_chain_override_drops_session_rpc_without_explicit_rpc() {
    let merged = merge_overrides(
        &InvocationOverrides {
            chain: Some("base".to_string()),
            rpc: Some("https://beam.example/base".to_string()),
            ..InvocationOverrides::default()
        },
        &InvocationOverrides {
            chain: Some("ethereum".to_string()),
            ..InvocationOverrides::default()
        },
    );

    assert_eq!(merged.chain.as_deref(), Some("ethereum"));
    assert_eq!(merged.rpc, None);
}

#[test]
fn repl_cli_chain_override_keeps_explicit_rpc() {
    let merged = merge_overrides(
        &InvocationOverrides {
            chain: Some("base".to_string()),
            rpc: Some("https://beam.example/base".to_string()),
            ..InvocationOverrides::default()
        },
        &InvocationOverrides {
            chain: Some("ethereum".to_string()),
            rpc: Some("https://beam.example/ethereum".to_string()),
            ..InvocationOverrides::default()
        },
    );

    assert_eq!(merged.chain.as_deref(), Some("ethereum"));
    assert_eq!(merged.rpc.as_deref(), Some("https://beam.example/ethereum"));
}
