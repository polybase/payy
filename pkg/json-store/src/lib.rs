// lint-long-file-override allow-max-lines=400
//! A simple JSON store that persists data to JSON files with atomic operations.
//!
//! This crate provides a generic JSON store that:
//! - Stores data as JSON files
//! - Ensures atomic writes using temporary files
//! - Loads existing state on initialization
//! - Creates default state if file doesn't exist
//! - Supports async operations
//!
//! # Example
//!
//! ```rust
//! use json_store::JsonStore;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize, Default)]
//! struct MyState {
//!     counter: u64,
//!     name: String,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let store = JsonStore::<MyState>::new("./test_fixtures", "state.json").await?;
//!
//!     // Read current state
//!     let state = store.get().await;
//!     println!("Current state: {:?}", state);
//!
//!     // Update a single property
//!     store.update(|state| {
//!         state.counter += 1;
//!         state.name = "Updated".to_string();
//!     }).await?;
//!
//!     Ok(())
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Error types for the JSON store
#[derive(thiserror::Error, Debug)]
pub enum JsonStoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("File path error: {0}")]
    PathError(String),
}

/// A generic JSON store that persists data to JSON files with atomic operations
pub struct JsonStore<T> {
    data: Arc<RwLock<T>>,
    file_path: PathBuf,
}

impl<T> JsonStore<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Default + Clone + Send + Sync + 'static,
{
    /// Creates a new JsonStore instance
    ///
    /// # Arguments
    /// * `dir` - Directory path where the JSON file will be stored
    /// * `filename` - Name of the JSON file
    ///
    /// # Returns
    /// A new JsonStore instance with loaded or default state
    pub async fn new<P: AsRef<Path>>(dir: P, filename: &str) -> Result<Self, JsonStoreError> {
        let dir_path = dir.as_ref();
        let file_path = dir_path.join(filename);

        // Ensure the directory exists
        if !dir_path.exists() {
            info!("Creating directory: {:?}", dir_path);
            fs::create_dir_all(dir_path).await?;
        }

        // Load existing data or create default
        let data = if file_path.exists() {
            debug!("Loading existing state from: {:?}", file_path);
            let content = fs::read_to_string(&file_path).await?;
            match serde_json::from_str::<T>(&content) {
                Ok(parsed_data) => {
                    info!("Successfully loaded state from: {:?}", file_path);
                    parsed_data
                }
                Err(e) => {
                    warn!(
                        "Failed to parse JSON from {:?}, using default: {}",
                        file_path, e
                    );
                    T::default()
                }
            }
        } else {
            info!(
                "File {:?} doesn't exist, creating with default state",
                file_path
            );
            T::default()
        };

        let store = Self {
            data: Arc::new(RwLock::new(data)),
            file_path,
        };

        // Write initial state to file if it didn't exist
        if !store.file_path.exists() {
            store.persist().await?;
        }

        Ok(store)
    }

    /// Gets a clone of the current state
    pub async fn get(&self) -> T {
        let data = self.data.read().await;
        data.clone()
    }

    /// Updates the state using a closure and persists the changes atomically
    ///
    /// # Arguments
    /// * `update_fn` - A closure that takes a mutable reference to the state
    ///
    /// # Returns
    /// Result indicating success or failure of the update operation
    pub async fn update<F>(&self, update_fn: F) -> Result<(), JsonStoreError>
    where
        F: FnOnce(&mut T) + Send,
    {
        {
            let mut data = self.data.write().await;
            update_fn(&mut *data);
        }

        self.persist().await
    }

    /// Replaces the entire state with a new value and persists it atomically
    ///
    /// # Arguments
    /// * `new_state` - The new state to replace the current one
    ///
    /// # Returns
    /// Result indicating success or failure of the set operation
    pub async fn set(&self, new_state: T) -> Result<(), JsonStoreError> {
        {
            let mut data = self.data.write().await;
            *data = new_state;
        }

        self.persist().await
    }

    /// Persists the current state to the JSON file atomically
    ///
    /// This method writes to a temporary file first, then atomically moves it
    /// to the target location to ensure consistency.
    async fn persist(&self) -> Result<(), JsonStoreError> {
        let data = self.data.read().await;

        // Serialize the data
        let json_content = serde_json::to_string_pretty(&*data)?;

        // Create a temporary file in the same directory as the target file
        let temp_path = self.file_path.with_extension("tmp");

        // Write to temporary file
        fs::write(&temp_path, &json_content).await?;

        // Atomically move the temporary file to the target location
        fs::rename(&temp_path, &self.file_path).await?;

        debug!("Successfully persisted state to: {:?}", self.file_path);
        Ok(())
    }

    /// Gets the file path where the data is stored
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }
}

impl<T> Clone for JsonStore<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
            file_path: self.file_path.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        // Update the state
        store
            .update(|state| {
                state.counter = 42;
                state.name = "test".to_string();
                state.active = true;
            })
            .await
            .unwrap();

        // Verify the state was updated
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

        // Create first instance and update state
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

        // Create second instance and verify state was loaded
        {
            let store = JsonStore::<TestState>::new(temp_dir.path(), "persistent.json")
                .await
                .unwrap();

            let state = store.get().await;
            assert_eq!(state.counter, 999);
            assert_eq!(state.name, "persistent");
        }

        // Verify the file actually exists
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_atomic_writes() {
        let temp_dir = TempDir::new("json_kv_store_test").unwrap();
        let store = JsonStore::<TestState>::new(temp_dir.path(), "atomic.json")
            .await
            .unwrap();

        // Perform multiple rapid updates
        for i in 0..10 {
            store
                .update(|state| {
                    state.counter = i;
                })
                .await
                .unwrap();
        }

        // Verify final state
        let state = store.get().await;
        assert_eq!(state.counter, 9);

        // Verify no temporary files are left behind
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

        // Update through first store
        store1
            .update(|state| {
                state.counter = 123;
            })
            .await
            .unwrap();

        // Verify change is visible through second store
        let state = store2.get().await;
        assert_eq!(state.counter, 123);
    }
}
