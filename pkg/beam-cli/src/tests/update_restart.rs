use std::path::Path;

use crate::{
    commands::update::{restart_after_update_args, restart_executable},
    display::ColorMode,
    output::OutputMode,
    runtime::InvocationOverrides,
};

fn invocation_overrides(chain: Option<&str>, from: Option<&str>) -> InvocationOverrides {
    InvocationOverrides {
        chain: chain.map(ToOwned::to_owned),
        from: from.map(ToOwned::to_owned),
        rpc: None,
    }
}

#[test]
fn restart_after_update_preserves_matching_interactive_startup_args() {
    let interactive_args = restart_after_update_args(
        [
            "beam", "--chain", "base", "--from", "alice", "--color", "always",
        ],
        &invocation_overrides(Some("base"), Some("alice")),
        OutputMode::Default,
        ColorMode::Always,
    )
    .expect("compute restart args");

    assert_eq!(
        interactive_args,
        Some(vec![
            "--chain".into(),
            "base".into(),
            "--from".into(),
            "alice".into(),
            "--color".into(),
            "always".into(),
        ])
    );
}

#[test]
fn restart_after_update_falls_back_to_plain_beam_for_changed_interactive_sessions() {
    assert_eq!(
        restart_after_update_args(
            ["beam", "--chain", "base"],
            &invocation_overrides(Some("ethereum"), None),
            OutputMode::Default,
            ColorMode::Auto,
        )
        .expect("restart changed session"),
        Some(vec![])
    );
    assert_eq!(
        restart_after_update_args(
            ["beam", "--output", "json"],
            &InvocationOverrides::default(),
            OutputMode::Default,
            ColorMode::Auto,
        )
        .expect("restart output mismatch"),
        Some(vec![])
    );
    assert_eq!(
        restart_after_update_args(
            ["beam", "--color", "never"],
            &InvocationOverrides::default(),
            OutputMode::Default,
            ColorMode::Auto,
        )
        .expect("restart color mismatch"),
        Some(vec![])
    );
}

#[test]
fn restart_after_update_skips_non_interactive_invocations() {
    assert_eq!(
        restart_after_update_args(
            ["beam", "--chain", "base", "update"],
            &invocation_overrides(Some("base"), None),
            OutputMode::Default,
            ColorMode::Auto,
        )
        .expect("skip non-interactive restart"),
        None
    );
}

#[test]
fn restart_executable_forwards_args() {
    let status = restart_executable(
        Path::new("/bin/sh"),
        [
            "-c",
            "test \"$1\" = \"--chain\" && test \"$2\" = \"base\" && test \"$3\" = \"--from\" && test \"$4\" = \"alice\"",
            "beam",
            "--chain",
            "base",
            "--from",
            "alice",
        ],
    )
    .expect("restart child process");

    assert!(status.success());
}

#[test]
fn restart_executable_exposes_failure_status() {
    let status =
        restart_executable(Path::new("/bin/sh"), ["-c", "exit 7"]).expect("run failing child");

    assert_eq!((status.success(), status.code()), (false, Some(7)));
}
