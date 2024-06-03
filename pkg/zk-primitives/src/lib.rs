#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::explicit_deref_methods)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! A set of core primitives for use with polybase's zk circuits

mod element;
mod hash;
mod path;

#[cfg(feature = "test-api")]
pub use hash::{hash_count, hash_element_count, reset_hash_count, reset_hash_element_count};

#[cfg(feature = "rand")]
pub use element::Insecure;
pub use element::{Element, Lsb};
pub use hash::{hash_bytes, hash_merge};
pub use path::compute_merkle_root;

/// The base element used by cryptographic operations on this tree
///
/// This is (roughly) an integer modulo `p` where `p` is [`Element::MODULUS`]
pub type Base = poseidon_circuit::Bn256Fr;
