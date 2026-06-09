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

use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::fs;
#[cfg(unix)]
use tokio::fs::OpenOptions;
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

    /// Updates the state using a closure and persists the changes atomically.
    ///
    /// The updated value is written and renamed into place before the
    /// in-memory state is swapped. If persistence fails, callers receive the
    /// error and subsequent reads from the same store instance continue to see
    /// the last committed value.
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
        let mut data = self.data.write().await;
        let mut new_state = data.clone();
        update_fn(&mut new_state);
        self.persist_state(&new_state).await?;
        *data = new_state;
        Ok(())
    }

    /// Replaces the entire state with a new value and persists it atomically.
    ///
    /// The replacement becomes visible through [`Self::get`] only after the
    /// JSON file write and atomic rename succeed. This gives callers
    /// transaction-like rollback semantics for a single store instance when the
    /// filesystem rejects the write.
    ///
    /// # Arguments
    /// * `new_state` - The new state to replace the current one
    ///
    /// # Returns
    /// Result indicating success or failure of the set operation
    pub async fn set(&self, new_state: T) -> Result<(), JsonStoreError> {
        let mut data = self.data.write().await;
        self.persist_state(&new_state).await?;
        *data = new_state;
        Ok(())
    }

    /// Replace state, or recover to a caller-supplied state if persistence fails.
    ///
    /// Higher-level stores use this when a failed final commit must abandon a
    /// prepared in-memory attempt rather than exposing stale pending state. The
    /// recovery state is also persisted when the filesystem permits it. If both
    /// writes fail, the same store instance still swaps to `recovery_state` so
    /// subsequent in-process reads observe the caller's rollback decision.
    pub async fn set_with_recovery_on_error(
        &self,
        new_state: T,
        recovery_state: T,
    ) -> Result<(), JsonStoreError> {
        let mut data = self.data.write().await;
        match self.persist_state(&new_state).await {
            Ok(()) => {
                *data = new_state;
                Ok(())
            }
            Err(error) => {
                let _ = self.persist_state(&recovery_state).await;
                *data = recovery_state;
                Err(error)
            }
        }
    }

    /// Persists the current state to the JSON file atomically
    ///
    /// This method writes to a temporary file first, then atomically moves it
    /// to the target location to ensure consistency.
    async fn persist(&self) -> Result<(), JsonStoreError> {
        let data = self.data.read().await.clone();
        self.persist_state(&data).await
    }

    async fn persist_state(&self, data: &T) -> Result<(), JsonStoreError> {
        // Serialize the data
        let json_content = serde_json::to_string_pretty(data)?;

        let temp_path =
            write_json_temp_file(&self.file_path, &json_content, self.file_access).await?;

        if let Err(error) = fs::rename(&temp_path, &self.file_path).await {
            let _ = fs::remove_file(&temp_path).await;
            return Err(error.into());
        }
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

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn write_json_temp_file(
    target_path: &Path,
    content: &str,
    file_access: FileAccess,
) -> Result<PathBuf, std::io::Error> {
    for _ in 0..16 {
        let temp_path = unique_temp_path(target_path)?;
        let result = if matches!(file_access, FileAccess::OwnerOnly) {
            write_owner_only_json_file(&temp_path, content).await
        } else {
            write_shared_json_file(&temp_path, content).await
        };
        match result {
            Ok(()) => return Ok(temp_path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                let _ = fs::remove_file(&temp_path).await;
                return Err(error);
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not create a unique json-store temp file",
    ))
}

fn unique_temp_path(target_path: &Path) -> Result<PathBuf, std::io::Error> {
    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target_path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "missing json-store file name",
        )
    })?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(".tmp-{}-{nanos}-{counter}", process::id()));

    Ok(parent.join(temp_name))
}

#[cfg(unix)]
async fn write_owner_only_json_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true).mode(0o600);

    {
        let mut file = options.open(path).await?;
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;
    }
    ensure_file_access(path, FileAccess::OwnerOnly).await
}

#[cfg(not(unix))]
async fn write_owner_only_json_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    write_shared_json_file(path, content).await
}

async fn write_shared_json_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .await?;
    file.write_all(content.as_bytes()).await?;
    file.flush().await
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
