#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! Simple data format for working with encrypted data
//!
mod asymmetric;
mod error;
mod symmetric;
mod util;

pub use asymmetric::*;
pub use error::*;
pub use symmetric::*;

pub use crypto_secretbox::Key;
pub use x25519_dalek::{PublicKey, StaticSecret};

const VERSION: u8 = 1;
const NONCE_SIZE: usize = 24;
const EPHEMERAL_PUBLIC_KEY_SIZE: usize = 32;
