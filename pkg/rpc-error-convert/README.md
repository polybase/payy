# RPC Error Convert

A procedural macro for automatic conversion between application errors and HTTP errors in RPC contexts.

## Overview

This package provides the `HTTPErrorConversion` derive macro that automatically implements conversions between your application error types and RPC HTTP errors. It supports:

- Unit variants (no data)
- Single unnamed fields 
- Multiple unnamed fields (creates tuple struct)
- Named fields (creates struct)

## Features

- Automatic generation of `From<YourError> for HTTPError`
- Automatic generation of `TryFrom<HTTPError> for YourError`
- Automatic generation of `TryFrom<ErrorOutput> for YourError`
- Support for serializing/deserializing error data
- Type-safe error conversions
- Works seamlessly with `Contextful<E>` wrappers via `HTTPError::from`

## Usage

```rust
use rpc::HTTPErrorConversion;
use rpc::code::ErrorCode;
use rpc::error::{ErrorOutput, HTTPError, Severity, TryFromHTTPError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, thiserror::Error, HTTPErrorConversion, Serialize, Deserialize)]
pub enum MyError {
    // Unit variant - no data
    #[bad_request("invalid-request")]
    #[error("invalid request")]
    InvalidRequest,

    // Single unnamed field - data passed directly
    #[not_found("user-not-found")]
    #[error("user not found: {0:?}")]
    UserNotFound(UserData),

    // Multiple unnamed fields - generates MultipleFieldsData tuple struct
    #[bad_request("multiple-fields")]
    #[error("multiple fields error")]
    MultipleFields(String, u32, bool),

    // Named fields - generates DuplicateEntryData struct
    #[already_exists("duplicate-entry")]
    #[error("duplicate entry with id {id}")]
    DuplicateEntry { 
        id: u64, 
        name: String 
    },

    // Unit variant with warn severity
    #[bad_request("quote-too-small", severity = "warn")]
    #[error("[my_error] quote output amount below minimum")]
    QuoteTooSmall,
}
```

## Generated Structures

For variants with multiple unnamed fields or named fields, the macro automatically generates data structures:

### Multiple Unnamed Fields
For `MultipleFields(String, u32, bool)`, generates:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MultipleFieldsData(pub String, pub u32, pub bool);
```

### Named Fields
For `DuplicateEntry { id: u64, name: String }`, generates:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DuplicateEntryData {
    pub id: u64,
    pub name: String,
}
```

## Supported Attributes

- `#[bad_request("error-code")]` - Maps to `ErrorCode::BadRequest`
- `#[not_found("error-code")]` - Maps to `ErrorCode::NotFound`
- `#[already_exists("error-code")]` - Maps to `ErrorCode::AlreadyExists`
- `#[failed_precondition("error-code")]` - Maps to `ErrorCode::FailedPrecondition`
- `#[internal("error-code")]` - Maps to `ErrorCode::FailedPrecondition`
- Optional `severity = "warn" | "error"` argument adjusts the generated `HTTPError` severity level (defaults to `"error"` when omitted)

## Severity Overrides

By default, generated `HTTPError` instances use `Severity::Error`. For expected validation failures where you want to downgrade the alerting noise, supply an explicit severity override:

```rust
#[derive(Debug, Clone, thiserror::Error, HTTPErrorConversion, Serialize, Deserialize)]
pub enum QuoteError {
    #[error("[quote] output amount below provider minimum")]
    #[bad_request("output-too-low", severity = "warn")]
    OutputTooLow,
}
```

This keeps the HTTP status and reason intact while ensuring `HTTPError::severity` is `Severity::Warn`, allowing downstream logging and alerting policies to treat the error as non-fatal. The derive emits fully-qualified severity paths, so no additional imports are required in the consuming crate.

## Requirements

All data types used in variants must implement:
- `Clone`
- `Serialize`
- `Deserialize`

## Example Conversion Flow

```rust
// Create an error
let error = MyError::DuplicateEntry { 
    id: 123, 
    name: "test".to_string() 
};

// Convert to HTTPError (automatic serialization of data)
let http_error: HTTPError = error.into();

// Convert back from HTTPError (automatic deserialization of data)
let recovered = MyError::try_from(http_error).unwrap();
```

## Contextful Support

Errors wrapped in `contextful::Contextful<E>` convert directly into `HTTPError` whenever `E` already implements `Into<HTTPError>`. This keeps the ergonomic `.context(...)` workflow while avoiding manual calls to `into_parts()` when delegating to the derived conversions.
