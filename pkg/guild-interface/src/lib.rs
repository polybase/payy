#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! Interface for requests/responses to Guild
/// Across interface
pub mod across;
/// Auth interface
pub mod auth;
/// EIP-7702 interface
pub mod eip7702;
mod error;
/// Invest / yield interface
pub mod invest;
/// Migration interface
pub mod migrate;
/// Mint interface
pub mod mint;
/// Note interface
pub mod notes;
/// Payments interface
pub mod payments;
/// Prices interface
pub mod prices;
/// Ramps interface
pub mod ramps;
/// Registry interface
pub mod registry;
/// Support interface
pub mod support;
/// Utility fns
mod util;
/// Wallet
pub mod wallet;
/// Wallet activity interface
pub mod wallet_activity;
/// Wallet notes interface
pub mod wallet_notes;

pub use eip7702::*;
pub use error::*;
