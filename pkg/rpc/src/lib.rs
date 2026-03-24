#![warn(clippy::unwrap_used, clippy::expect_used)]
#![deny(clippy::disallowed_methods)]
extern crate self as rpc;
pub mod code;
pub mod error;
pub mod longpoll;
pub mod middleware;
pub mod tracing;

#[cfg(test)]
mod tests;

// Re-export the HTTPErrorConversion macro
pub use rpc_error_convert::HTTPErrorConversion;
