use clap::CommandFactory;

use crate::cli::Cli;

#[test]
fn visible_commands_have_descriptions() {
    let cli = Cli::command();

    assert_visible_commands_have_descriptions(&cli);
}

pub(super) fn assert_visible_commands_have_descriptions(command: &clap::Command) {
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
