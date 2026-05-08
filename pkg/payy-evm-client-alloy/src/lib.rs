#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! Alloy adapters for the Payy EVM client.

mod adapter;
mod convert;
mod transaction;

#[cfg(test)]
mod tests;

pub use adapter::*;
pub use transaction::*;
