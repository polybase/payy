#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! Public interface types and traits for the Payy EVM client.

/// Account selector models.
pub mod account;
/// Privacy-address encoding helpers.
pub mod address;
/// Interface error contract.
pub mod error;
/// EVM adapter models and traits.
pub mod evm;
/// Link and receive models.
pub mod link;
/// Network configuration.
pub mod network;
/// Note and state models.
pub mod note;
/// Operation and prepared-call models.
pub mod operation;
/// Privacy signer trait.
pub mod privacy;

pub use account::*;
pub use address::*;
pub use error::*;
pub use evm::*;
pub use link::*;
pub use network::*;
pub use note::*;
pub use operation::*;
pub use privacy::*;
