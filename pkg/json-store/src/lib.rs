// lint-long-file-override allow-max-lines=430
//! A simple JSON store that persists data to JSON files with atomic operations.
//!
//! This crate provides a generic JSON store that:
//! - Stores data as JSON files
//! - Ensures atomic writes using temporary files
//! - Loads existing state on initialization
//! - Creates default state if file doesn't exist
//! - Recovers with default state by default, or can fail closed on invalid JSON
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

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::fs;
#[cfg(unix)]
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Error types for the JSON store
#[derive(thiserror::Error, Debug)]
pub enum JsonStoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("JSON parse error in {}: {source}", path.display())]
    Deserialization {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("File path error: {0}")]
    PathError(String),
}

/// A generic JSON store that persists data to JSON files with atomic operations
pub struct JsonStore<T> {
    data: Arc<RwLock<T>>,
    file_path: PathBuf,
    file_access: FileAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidJsonBehavior {
    UseDefault,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileAccess {
    #[default]
    Shared,
    OwnerOnly,
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
    /// A new JsonStore instance with loaded state, or default state when the file is missing
    /// or contains invalid JSON.
    pub async fn new<P: AsRef<Path>>(dir: P, filename: &str) -> Result<Self, JsonStoreError> {
        Self::new_with_invalid_json_behavior_and_access(
            dir,
            filename,
            InvalidJsonBehavior::UseDefault,
            FileAccess::Shared,
        )
        .await
    }

    pub async fn new_with_invalid_json_behavior<P: AsRef<Path>>(
        dir: P,
        filename: &str,
        invalid_json_behavior: InvalidJsonBehavior,
    ) -> Result<Self, JsonStoreError> {
        Self::new_with_invalid_json_behavior_and_access(
            dir,
            filename,
            invalid_json_behavior,
            FileAccess::Shared,
        )
        .await
    }

    pub async fn new_with_invalid_json_behavior_and_access<P: AsRef<Path>>(
        dir: P,
        filename: &str,
        invalid_json_behavior: InvalidJsonBehavior,
        file_access: FileAccess,
    ) -> Result<Self, JsonStoreError> {
        let dir_path = dir.as_ref();
        let file_path = dir_path.join(filename);
        let file_exists = file_path.exists();

        // Ensure the directory exists
        if !dir_path.exists() {
            info!("Creating directory: {:?}", dir_path);
            fs::create_dir_all(dir_path).await?;
        }

        // Load existing data, or create default state when the file is missing.
        let data = if file_exists {
            debug!("Loading existing state from: {:?}", file_path);
            let content = fs::read_to_string(&file_path).await?;
            match serde_json::from_str::<T>(&content) {
                Ok(parsed_data) => {
                    info!("Successfully loaded state from: {:?}", file_path);
                    parsed_data
                }
                Err(source) => match invalid_json_behavior {
                    InvalidJsonBehavior::UseDefault => {
                        warn!(
                            "Failed to parse JSON from {:?}, using default state: {}",
                            file_path, source
                        );
                        T::default()
                    }
                    InvalidJsonBehavior::Error => {
                        return Err(JsonStoreError::Deserialization {
                            path: file_path.clone(),
                            source,
                        });
                    }
                },
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
            file_access,
        };

        if !file_exists {
            store.persist().await?;
        } else {
            ensure_file_access(&store.file_path, store.file_access).await?;
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
        write_json_file(&temp_path, &json_content, self.file_access).await?;

        // Atomically move the temporary file to the target location
        fs::rename(&temp_path, &self.file_path).await?;
        ensure_file_access(&self.file_path, self.file_access).await?;

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
            file_access: self.file_access,
        }
    }
}

async fn write_json_file(
    path: &Path,
    content: &str,
    file_access: FileAccess,
) -> Result<(), std::io::Error> {
    if matches!(file_access, FileAccess::OwnerOnly) {
        return write_owner_only_json_file(path, content).await;
    }

    fs::write(path, content).await
}

#[cfg(unix)]
async fn write_owner_only_json_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(true).write(true).mode(0o600);

    let mut file = options.open(path).await?;
    file.write_all(content.as_bytes()).await?;
    file.flush().await?;
    drop(file);
    ensure_file_access(path, FileAccess::OwnerOnly).await
}

#[cfg(not(unix))]
async fn write_owner_only_json_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    fs::write(path, content).await
}

async fn ensure_file_access(path: &Path, file_access: FileAccess) -> Result<(), std::io::Error> {
    #[cfg(unix)]
    {
        if matches!(file_access, FileAccess::OwnerOnly) {
            fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).await?;
        }
    }

    let _ = (path, file_access);
    Ok(())
}

#[cfg(test)]
mod tests;
