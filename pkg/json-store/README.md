# JSON Store

A simple, thread-safe JSON store that persists data to JSON files with atomic operations.

## Features

- **Atomic Operations**: All writes use temporary files and atomic moves to ensure data consistency
- **Generic Storage**: Store any type that implements `Serialize`, `Deserialize`, `Default`, `Clone`, `Send`, and `Sync`
- **Async Support**: Built with `tokio` for async operations
- **Auto-loading**: Automatically loads existing state on initialization
- **Missing-file Initialization**: Creates default state when the target file does not exist
- **Recovering Defaults**: `JsonStore::new` falls back to `T::default()` when an existing file contains invalid JSON
- **Optional Fail-closed Loading**: Callers can opt into an error when an existing file contains invalid JSON
- **Thread-safe**: Uses `Arc<RwLock<T>>` for safe concurrent access
- **Configurable Path**: Specify custom directory and filename

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
json-store = { workspace = true }
```

## Example

```rust
use json_store::JsonStore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppState {
    counter: u64,
    user_name: String,
    settings: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create store with custom directory and filename
    let store = JsonStore::<AppState>::new("./test_fixtures", "app_state.json").await?;

    // Read current state
    let current_state = store.get().await;
    println!("Current state: {:?}", current_state);

    // Update specific properties
    store.update(|state| {
        state.counter += 1;
        state.user_name = "Alice".to_string();
        state.settings.push("dark_mode".to_string());
    }).await?;

    // Replace entire state
    let new_state = AppState {
        counter: 100,
        user_name: "Bob".to_string(),
        settings: vec!["light_mode".to_string(), "notifications".to_string()],
    };
    store.set(new_state).await?;

    // Clone the store (shares the same underlying data)
    let store_clone = store.clone();
    let state_from_clone = store_clone.get().await;
    println!("State from clone: {:?}", state_from_clone);

    Ok(())
}
```

## API

### `JsonStore::new(dir, filename)`

Creates a new store instance. The directory will be created if it doesn't exist. If the file
already exists but contains invalid JSON, `new` loads `T::default()` and leaves the invalid file
untouched so callers can recover without failing startup.

**Parameters:**
- `dir`: Directory path where the JSON file will be stored
- `filename`: Name of the JSON file

**Returns:** `Result<JsonStore<T>, JsonStoreError>`

### `JsonStore::new_with_invalid_json_behavior(dir, filename, behavior)`

Creates a new store instance with explicit invalid-JSON handling. Use
`InvalidJsonBehavior::UseDefault` to recover with `T::default()`, or
`InvalidJsonBehavior::Error` to return `JsonStoreError::Deserialization`.

**Parameters:**
- `dir`: Directory path where the JSON file will be stored
- `filename`: Name of the JSON file
- `behavior`: How to handle existing files that contain invalid JSON

**Returns:** `Result<JsonStore<T>, JsonStoreError>`

### `get()`

Returns a clone of the current state.

**Returns:** `T`

### `update(update_fn)`

Updates the state using a closure and persists changes atomically.

**Parameters:**
- `update_fn`: Closure that takes `&mut T` and modifies the state

**Returns:** `Result<(), JsonStoreError>`

### `set(new_state)`

Replaces the entire state with a new value and persists it atomically.

**Parameters:**
- `new_state`: The new state to replace the current one

**Returns:** `Result<(), JsonStoreError>`

### `file_path()`

Returns the path to the JSON file.

**Returns:** `&Path`

## Error Handling

The crate defines `JsonStoreError` enum with the following variants:

- `Io(std::io::Error)`: File system operations errors
- `Serialization(serde_json::Error)`: JSON serialization errors while persisting data
- `Deserialization { path, source }`: Invalid JSON in an existing file when the store is opened in error mode
- `PathError(String)`: Path-related errors

## Atomic Operations

The store ensures atomicity by:

1. Writing data to a temporary file (`.tmp` extension)
2. Using `serde_json::to_string_pretty` for human-readable JSON
3. Atomically moving the temporary file to the target location using `fs::rename`

This approach prevents data corruption even if the process is interrupted during writes.

## Thread Safety

The store uses `Arc<RwLock<T>>` internally, allowing:

- Multiple concurrent readers
- Exclusive writer access
- Safe sharing across threads via `Clone`

## Requirements

Your data type must implement:

- `Serialize + Deserialize`: For JSON serialization
- `Default`: For creating initial state when file doesn't exist
- `Clone`: For returning state copies
- `Send + Sync`: For thread safety
- `'static`: For async operations

## Testing

Run tests with:

```bash
cargo test -p json-store
```

The test suite includes:

- Basic CRUD operations
- Persistence across instances
- Atomic write verification
- Concurrent access through cloning
- Error handling scenarios
