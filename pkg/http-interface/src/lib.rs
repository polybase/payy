#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! Abstractions for interacting with HTTP clients within the zk-rollup workspace.
//! The interfaces defined here enable dependency injection, mocking, and swapping
//! of HTTP client implementations.

/// Traits associated with building HTTP requests.
pub mod builder;
/// Core HTTP client abstraction traits.
pub mod client;
/// Shared HTTP error types and helpers.
pub mod error;
/// Response wrapper utilities shared across clients.
pub mod response;

pub use builder::HttpRequestBuilder;
pub use client::{
    ClientHttpAuth, HttpClient, HttpClientMock, MockRequestBuilder, RecordingHttpClient,
};
pub use error::{AuthError, Error, NoRpcError, Result, handle_error};
pub use response::{ClientResponse, HttpMetadata};
