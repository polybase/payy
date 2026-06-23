use crate::cli::APP_HELP_ARG;

pub(super) fn filtered_app_args(args: &[String]) -> Vec<String> {
    let mut filtered = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--prepare" || arg == "--no-prompt" {
            index += 1;
            continue;
        }
        if arg == "--max-network-fee-wei" {
            index += 2;
            continue;
        }
        if arg.starts_with("--max-network-fee-wei=") {
            index += 1;
            continue;
        }
        filtered.push(if arg == APP_HELP_ARG {
            "--help".to_string()
        } else {
            arg.clone()
        });
        index += 1;
    }
    filtered
}

pub(super) fn is_help_requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}
