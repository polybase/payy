use super::fixtures::test_app_with_output;
use crate::{
    cli::{Command, FetchArgs},
    commands::interactive::{ParsedLine, parse_line},
    commands::interactive_parse::resolved_output_mode,
    output::OutputMode,
    runtime::InvocationOverrides,
};

#[test]
fn interactive_parser_marks_explicit_global_format_flags() {
    for line in [
        "--format json fetch https://api.example.com/raw",
        "--output json fetch https://api.example.com/raw",
    ] {
        let parsed = parse_line(line).expect("parse fetch with explicit output override");
        let ParsedLine::Cli { cli, global_flags } = parsed else {
            panic!("expected clap command");
        };

        assert_eq!(cli.output, OutputMode::Json);
        assert!(global_flags.output_explicit);
    }
}

#[tokio::test]
async fn interactive_fetch_output_path_inherits_session_output_mode() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Json, InvocationOverrides::default()).await;
    let parsed = parse_line("fetch --output response.bin https://api.example.com/raw")
        .expect("parse fetch output path");
    let ParsedLine::Cli { cli, global_flags } = parsed else {
        panic!("expected clap command");
    };

    assert!(matches!(
        &cli.command,
        Some(Command::Fetch(FetchArgs {
            url,
            output_path,
            ..
        })) if url == "https://api.example.com/raw"
            && output_path.as_deref() == Some("response.bin")
    ));
    assert!(!global_flags.output_explicit);
    assert_eq!(
        resolved_output_mode(global_flags, &cli, &app),
        OutputMode::Json
    );
}
