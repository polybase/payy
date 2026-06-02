#![warn(clippy::pedantic)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![deny(missing_docs)]

//! Typed interface for Sourcify contract lookups.

mod client;
mod contract;
mod error;

pub use client::{ContractLookup, SourcifyClient};
pub use contract::{
    CompilationSummary, ContractField, ContractRecord, ContractResponse, MatchState, SourceFile,
};
pub use error::{Error, Result};
