# contextful

Utilities for wrapping error values with human-friendly context that preserves the original source error. The crate exposes a lightweight `Contextful<E>` wrapper together with extension traits that make attaching context ergonomic in the places where you already return `Result<T, E>` values or need to rewrap a bare `E`.

## Overview

`Contextful<E>` stores a context message alongside an underlying error. It implements `std::error::Error`, `Display`, `Debug`, `Serialize`, and `Deserialize`, making it suitable for propagating through error enums or reporting pipelines.

Import the prelude to get the extension traits:

```rust
use contextful::prelude::*;
```

## API

### `Contextful<E>`

- Construct via `Contextful::new(msg, err)`
- Access pieces with `context_message()`, `source_ref()`, or `into_parts()`
- Map the inner error with `map_source`

### `ResultContextExt<T, E>`

Adds `.context(msg)`, `.with_context(|| msg)`, and `.without_context()` to any `Result<T, E>` so you can attach context right where the `?` operator would otherwise return the bare error.

```rust
use contextful::ResultContextExt;

fn read_user(id: i64) -> Result<User, Contextful<sqlx::Error>> {
    repo.fetch(id).context("load user by id")
}

fn parse_user(input: &str) -> Result<User, Contextful<ParseError>> {
    Parser::new(input).with_context(|| format!("parse user payload `{input}`"))
}
```

### `ErrorContextExt`

Adds `.wrap_err(msg)`, `.wrap_err_with(|| msg)`, and `.without_context()` to any error value. This is ideal when you already have an `E` (for example after matching on an enum variant) and need to rewrap it before returning.

```rust
use contextful::ErrorContextExt;

fn reconcile(user: User) -> Result<(), Contextful<BusinessError>> {
    match validate(user) {
        Ok(()) => Ok(()),
        Err(err) => Err(err.wrap_err("validate user before reconcile")),
    }
}

fn call_external() -> Result<(), Contextful<ExternalError>> {
    let err = ExternalError::Timeout;
    Err(err.wrap_err_with(|| format!("external timeout at {}", chrono::Utc::now())))
}
```

`wrap_err` evaluates its message eagerly. `wrap_err_with` only evaluates the closure when the error is wrapped, which helps avoid formatting costs in success paths.

## Best Practices

- Use `ResultContextExt` when working with a `Result` and let the trait handle wrapping automatically.
- Use `ErrorContextExt` when you are holding a bare error value.
- Keep context strings concise but actionable. Include identifiers (user id, external resource) that aid debugging.
- Propagate `Contextful<E>` variants through your error enums using `#[from]` to preserve error chains.
