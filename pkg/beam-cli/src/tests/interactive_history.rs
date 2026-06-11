use std::fs;

use rustyline::history::History;

use super::fixtures::test_app;
use crate::{
    commands::{
        interactive::load_sanitized_history,
        interactive_history::{ReplHistory, should_persist_history},
    },
    runtime::InvocationOverrides,
};

#[tokio::test]
async fn startup_history_scrub_rewrites_history_file_before_next_save() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    fs::write(
        &app.paths.history,
        "wallets import 0x1234\nbalance\nwallets export-private-key\n/wallets address 0x1234\n/wallets export-private-key alice\nwallets import-recovery-phrase --phrase-stdin\n",
    )
    .expect("write beam history");

    let mut history = ReplHistory::new();
    load_sanitized_history(&mut history, &app.paths.history).expect("load sanitized history");

    assert_eq!(
        history.iter().cloned().collect::<Vec<_>>(),
        vec!["balance".to_string()]
    );

    let persisted = fs::read_to_string(&app.paths.history).expect("read beam history");
    assert!(persisted.contains("balance"));
    assert!(!persisted.contains("wallets import"));
    assert!(!persisted.contains("wallets import-recovery-phrase"));
    assert!(!persisted.contains("/wallets address"));
    assert!(!persisted.contains("export-private-key"));

    let mut reloaded = ReplHistory::new();
    reloaded
        .load(&app.paths.history)
        .expect("reload beam history");
    assert_eq!(
        reloaded.iter().cloned().collect::<Vec<_>>(),
        vec!["balance".to_string()]
    );
}

#[test]
fn privacy_claim_artifacts_are_not_persisted_to_history() {
    assert!(!should_persist_history("wallets export-private-key"));
    assert!(!should_persist_history(
        "--chain base /wallets export-private-key alice"
    ));
    assert!(!should_persist_history(
        "wallets import-recovery-phrase --phrase-stdin"
    ));
    assert!(should_persist_history("wallets export-recovery-phrase"));
    assert!(!should_persist_history(
        "privacy claim payy:secret-artifact"
    ));
    assert!(!should_persist_history(
        "privacy send --ephemeral USDC 1 --claim-link-message invoice"
    ));
    assert!(!should_persist_history(
        "fetch --private-payment https://api.example.com/paid"
    ));
    assert!(should_persist_history("privacy balance USDC"));
}
