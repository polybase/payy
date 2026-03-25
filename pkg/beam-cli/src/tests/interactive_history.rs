use std::fs;

use rustyline::history::History;

use super::fixtures::test_app;
use crate::{
    commands::interactive::load_sanitized_history, commands::interactive_history::ReplHistory,
    runtime::InvocationOverrides,
};

#[tokio::test]
async fn startup_history_scrub_rewrites_history_file_before_next_save() {
    let (_temp_dir, app) = test_app(InvocationOverrides::default()).await;
    fs::write(
        &app.paths.history,
        "wallets import 0x1234\nbalance\n/wallets address 0x1234\n",
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
    assert!(!persisted.contains("/wallets address"));

    let mut reloaded = ReplHistory::new();
    reloaded
        .load(&app.paths.history)
        .expect("reload beam history");
    assert_eq!(
        reloaded.iter().cloned().collect::<Vec<_>>(),
        vec!["balance".to_string()]
    );
}
