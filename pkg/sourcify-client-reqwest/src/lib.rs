#![warn(clippy::pedantic)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![deny(missing_docs)]

//! Reqwest-backed Sourcify client.

mod client;
mod error;

#[cfg(test)]
mod tests;

pub use client::{SourcifyReqwestClient, SourcifyReqwestClientOptions};
pub use error::Error;
