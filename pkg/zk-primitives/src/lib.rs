#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::match_bool)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::explicit_deref_methods)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! A set of core primitives for use with polybase's zk circuits

mod address;
mod agg_agg;
mod agg_final;
mod agg_utxo;
mod burn;
mod input_note;
mod merkle_path;
mod migrate;
mod note;
mod points;
mod proof_bytes;
mod signature;
mod traits;
mod util;
mod utxo;

pub use address::*;
pub use agg_agg::*;
pub use agg_final::*;
pub use agg_utxo::*;
pub use burn::*;
pub use input_note::*;
pub use merkle_path::*;
pub use migrate::*;
pub use note::*;
pub use parse_link::{
    NoteURLPayload, NoteUrlDecodeError, NoteUrlDecodeResult, decode_activity_url_payload,
    try_decode_activity_url_payload,
};
pub use points::*;
pub use proof_bytes::*;
pub use signature::*;
pub use traits::*;
pub use util::*;
pub use utxo::*;
