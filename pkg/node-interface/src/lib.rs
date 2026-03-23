#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! Interface for requests to Payy Network

mod blocks;
mod client;
mod elements;
mod error;
mod height;
mod merkle;
mod transaction;

pub use blocks::*;
pub use client::{NodeClient, NodeClientMock};
pub use elements::*;
pub use error::*;
pub use height::*;
pub use merkle::*;
pub use transaction::*;
