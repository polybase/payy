# test-spy

A testing utility crate that provides spy-based mock implementations for Rust traits.

## Features

- **Spy Types**: `Spy` and `AsyncSpy` for tracking function calls and controlling return values
- **Procedural Macro**: `#[spy_mock]` attribute macro for auto-generating mock implementations
- **Call Tracking**: Record all function calls with their parameters
- **Return Value Control**: Queue specific return values or use default functions

## Usage

### Manual Mock Creation

```rust
use test_spy::{AsyncSpy, Spy};
use uuid::Uuid;

pub struct ServiceMock {
    pub sync_method: Spy<String, Result<u32, Error>>,
    pub async_method: AsyncSpy<Uuid, Result<String, Error>>,
}

impl ServiceMock {
    pub fn new() -> Self {
        Self {
            sync_method: Spy::new(|_| Ok(42)),
            async_method: AsyncSpy::new(|_| Ok("default".to_string())),
        }
    }
}
```

### Using the Macro

The `#[spy_mock]` macro can auto-generate mock implementations for traits:

```rust
use test_spy::spy_mock;

#[spy_mock]
trait SimpleService {
    fn get_value(&self, id: u32) -> String;
    fn set_value(&self, id: u32, value: String) -> bool;
}

// Generates SimpleServiceMock with spy fields
```

For async traits:

```rust
use test_spy::spy_mock;

#[spy_mock]
#[async_trait::async_trait]
trait AsyncService {
    async fn fetch(&self, url: &str) -> Result<String, Error>;
    async fn process(&self) -> u64;
}

// Generates AsyncServiceMock with AsyncSpy fields
```

## Working with Spies

### Recording Calls

```rust
let mock = ServiceMock::new();
mock.sync_method.register_call("test".to_string());

// Check recorded calls
let calls = mock.sync_method.calls();
assert_eq!(calls.len(), 1);
assert_eq!(calls[0].params, "test".to_string());
```

### Controlling Return Values

```rust
// Queue a specific return value
mock.sync_method.return_next(Ok(100));

// Next call will return the queued value
let result = mock.sync_method.register_call("input".to_string());
assert_eq!(result, Ok(100));

// Subsequent calls use the default function
let result = mock.sync_method.register_call("input2".to_string());
assert_eq!(result, Ok(42)); // default
```

### Resetting State

```rust
// Clear all recorded calls and queued returns
mock.sync_method.reset();
```

## Type Conversions

The macro automatically handles common type conversions:
- `&str` parameters are stored as `String`
- `&[T]` parameters are stored as `Vec<T>`
- References are cloned for storage

## Return Type Defaults

- For `Result<T, E>` types, the default is `Ok(T::default())`
- For other types, `Default::default()` is used
- Custom defaults can be provided when creating spies manually

## Migration from testutil

If migrating from `testutil::spy`:
1. Replace `use testutil::spy::{Spy, AsyncSpy}` with `use test_spy::{Spy, AsyncSpy}`
2. For async trait methods, ensure AsyncSpy is used and `.await` is called on `register_call()`
3. The API remains otherwise identical

## Limitations

- The `#[spy_mock]` macro requires all parameter types to implement `Clone`
- Return types must implement `Default` for automatic mock generation
- The macro works on trait definitions, not existing trait implementations from external crates

For existing traits you don't own, create mocks manually using the `Spy` and `AsyncSpy` types directly.