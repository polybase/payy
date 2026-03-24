mod bytes;
mod field_utils;
mod merkle_path;
mod note;
mod utxo;

pub type Address = [u8; 20];

pub struct Note {
    pub kind: Field,
    pub value: Field,
    pub address: Field,
    pub psi: Field,
}

pub struct InputNote {
    pub note: Note,
    pub secret_key: Field,
}

pub struct MerklePath {
    pub path: [Field; 160],
}

pub use bytes::slice;
pub use field_utils::{field_from_bytes, is_one_of};
pub use note::{
    check_commitment, check_input_note_ownership, get_address, get_note_commitment, is_multiple_kinds,
};
pub use utxo::Utxo;
