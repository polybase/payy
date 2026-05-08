#![warn(clippy::pedantic)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![deny(missing_docs)]

//! Interface crates for the Bungee domain service and its upstream API calque.

/// Typed mirror of the upstream Bungee REST API.
pub mod api;
/// Public domain trait, errors, and request/response types.
pub mod client;
