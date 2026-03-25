// lint-long-file-override allow-max-lines=300
use super::*;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use serde::{Deserialize, Serialize};
use tempdir::TempDir;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct TestState {
    counter: u64,
    name: String,
    active: bool,
}

#[tokio::test]
async fn test_new_store_creates_default_state() {
    let temp_dir = TempDir::new("json_kv_store_test").unwrap();
    let store = JsonStore::<TestState>::new(temp_dir.path(), "test.json")
        .await
        .unwrap();

    let state = store.get().await;
    assert_eq!(state, TestState::default());
}

#[tokio::test]
async fn test_update_and_persist() {
    let temp_dir = TempDir::new("json_kv_store_test").unwrap();
    let store = JsonStore::<TestState>::new(temp_dir.path(), "test.json")
        .await
        .unwrap();

    store
        .update(|state| {
            state.counter = 42;
            state.name = "test".to_string();
            state.active = true;
        })
        .await
        .unwrap();

    let state = store.get().await;
    assert_eq!(state.counter, 42);
    assert_eq!(state.name, "test");
    assert!(state.active);
}

#[tokio::test]
async fn test_set_and_persist() {
    let temp_dir = TempDir::new("json_kv_store_test").unwrap();
    let store = JsonStore::<TestState>::new(temp_dir.path(), "test.json")
        .await
        .unwrap();

    let new_state = TestState {
        counter: 100,
        name: "new_test".to_string(),
        active: false,
    };

    store.set(new_state.clone()).await.unwrap();

    let retrieved_state = store.get().await;
    assert_eq!(retrieved_state, new_state);
}

#[tokio::test]
async fn test_persistence_across_instances() {
    let temp_dir = TempDir::new("json_kv_store_test").unwrap();
    let file_path = temp_dir.path().join("persistent.json");

    {
        let store = JsonStore::<TestState>::new(temp_dir.path(), "persistent.json")
            .await
            .unwrap();

        store
            .update(|state| {
                state.counter = 999;
                state.name = "persistent".to_string();
            })
            .await
            .unwrap();
    }

    {
        let store = JsonStore::<TestState>::new(temp_dir.path(), "persistent.json")
            .await
            .unwrap();

        let state = store.get().await;
        assert_eq!(state.counter, 999);
        assert_eq!(state.name, "persistent");
    }

    assert!(file_path.exists());
}

#[tokio::test]
async fn test_atomic_writes() {
    let temp_dir = TempDir::new("json_kv_store_test").unwrap();
    let store = JsonStore::<TestState>::new(temp_dir.path(), "atomic.json")
        .await
        .unwrap();

    for i in 0..10 {
        store
            .update(|state| {
                state.counter = i;
            })
            .await
            .unwrap();
    }

    let state = store.get().await;
    assert_eq!(state.counter, 9);

    let temp_file = store.file_path.with_extension("tmp");
    assert!(!temp_file.exists());
}

#[tokio::test]
async fn test_clone_shares_state() {
    let temp_dir = TempDir::new("json_kv_store_test").unwrap();
    let store1 = JsonStore::<TestState>::new(temp_dir.path(), "shared.json")
        .await
        .unwrap();

    let store2 = store1.clone();

    store1
        .update(|state| {
            state.counter = 123;
        })
        .await
        .unwrap();

    let state = store2.get().await;
    assert_eq!(state.counter, 123);
}

#[tokio::test]
async fn test_invalid_json_behavior_is_configurable() {
    let temp_dir = TempDir::new("json_kv_store_test").unwrap();
    let file_path = temp_dir.path().join("broken.json");
    fs::write(&file_path, "{ invalid json").await.unwrap();

    let store = JsonStore::<TestState>::new(temp_dir.path(), "broken.json")
        .await
        .unwrap();
    assert_eq!(store.get().await, TestState::default());

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(content, "{ invalid json");

    let err = match JsonStore::<TestState>::new_with_invalid_json_behavior(
        temp_dir.path(),
        "broken.json",
        InvalidJsonBehavior::Error,
    )
    .await
    {
        Ok(_) => panic!("expected invalid persisted state to fail"),
        Err(err) => err,
    };

    match err {
        JsonStoreError::Deserialization { path, .. } => assert_eq!(path, file_path),
        other => panic!("unexpected error: {other:?}"),
    }

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(content, "{ invalid json");
}

#[cfg(unix)]
#[tokio::test]
async fn test_owner_only_access_restricts_existing_and_persisted_files() {
    let temp_dir = TempDir::new("json_kv_store_test").unwrap();
    let file_path = temp_dir.path().join("secure.json");
    fs::write(
        &file_path,
        serde_json::to_string_pretty(&TestState::default()).unwrap(),
    )
    .await
    .unwrap();
    std::fs::set_permissions(&file_path, std::fs::Permissions::from_mode(0o644)).unwrap();

    let store = JsonStore::<TestState>::new_with_invalid_json_behavior_and_access(
        temp_dir.path(),
        "secure.json",
        InvalidJsonBehavior::Error,
        FileAccess::OwnerOnly,
    )
    .await
    .unwrap();

    assert_eq!(
        std::fs::metadata(&file_path).unwrap().permissions().mode() & 0o777,
        0o600
    );

    store
        .update(|state| {
            state.counter = 1;
        })
        .await
        .unwrap();

    assert_eq!(
        std::fs::metadata(&file_path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
