pub(super) fn looks_like_sensitive_command(args: &[String]) -> bool {
    let Some(command_index) = command_index(args) else {
        return false;
    };

    match args
        .get(command_index)
        .map(String::as_str)
        .map(normalized_command)
    {
        Some("wallet" | "wallets") => matches!(
            args.get(command_index + 1).map(String::as_str),
            Some("import" | "address")
        ),
        Some("privacy") => {
            matches!(
                args.get(command_index + 1).map(String::as_str),
                Some("claim")
            ) || is_sensitive_privacy_send(args, command_index + 1)
        }
        Some("fetch") => args[command_index + 1..]
            .iter()
            .any(|arg| arg == "--private-payment"),
        _ => false,
    }
}

fn is_sensitive_privacy_send(args: &[String], action_index: usize) -> bool {
    matches!(args.get(action_index).map(String::as_str), Some("send"))
        && args[action_index + 1..].iter().any(|arg| {
            matches!(
                arg.as_str(),
                "--ephemeral" | "--claim-link-message" | "--memo"
            ) || arg.starts_with("--claim-link-message=")
                || arg.starts_with("--memo=")
        })
}

fn normalized_command(command: &str) -> &str {
    command.strip_prefix('/').unwrap_or(command)
}

fn command_index(args: &[String]) -> Option<usize> {
    let mut index = 0;
    if args.get(index).map(String::as_str) == Some("beam") {
        index += 1;
    }

    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "--no-update-check" {
            index += 1;
            continue;
        }

        let flag = arg.split_once('=').map_or(arg, |(flag, _)| flag);
        if matches!(
            flag,
            "--chain" | "--color" | "--format" | "--from" | "--output" | "--rpc"
        ) {
            index += if arg.contains('=') { 1 } else { 2 };
            continue;
        }

        return Some(index);
    }

    None
}
