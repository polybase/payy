use crate::{
    cli::{Command, TokenAction},
    commands::interactive::{ParsedLine, parse_line, repl_command_args},
};

#[test]
fn tokens_subcommands_route_through_clap_in_interactive_mode() {
    assert_eq!(
        repl_command_args("tokens list").expect("parse tokens list"),
        None
    );

    let parsed = parse_line("tokens remove USDC").expect("parse tokens remove");
    let ParsedLine::Cli { cli, .. } = parsed else {
        panic!("expected clap command");
    };
    assert!(matches!(
        &cli.command,
        Some(Command::Tokens {
            action: Some(TokenAction::Remove { token }),
        }) if token == "USDC"
    ));
}
