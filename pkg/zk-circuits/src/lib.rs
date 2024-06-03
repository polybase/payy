#![allow(clippy::assign_op_pattern)]
#![deny(clippy::disallowed_methods)]
#![feature(once_cell)]

pub mod aggregate_agg;
pub mod aggregate_utxo;
mod burn;
pub mod chips;
pub mod compliance;
pub mod constants;
pub mod evm_verifier;
pub(crate) mod fr;
pub mod insert;
pub mod mint;
pub mod points;
pub mod proof;
pub mod proof_format;
pub(crate) mod signature;
pub mod util;
mod utxo;

mod error;

#[cfg(feature = "test")]
pub mod test;

/// Simple data types used as inputs to the proofs
pub mod data;

mod keys;
mod params;

pub(crate) use crate::chips::aggregation::snark::Snark;
pub use constants::{UTXO_INPUTS, UTXO_OUTPUTS};
pub use keys::CircuitKind;

pub use error::{Error, Result};
pub use zk_primitives::Base;

