//! Utilities for handling Payy V2 Notes

use element::Element;

use crate::circuits::generated::erc20_transfer::Note;

pub fn hash_note(note: &Note) -> Element {
    let &Note {
        kind,
        address,
        token,
        value,
        psi,
    } = note;
    hash::hash_merge([
        kind,
        address_to_element(address),
        address_to_element(token),
        value,
        psi,
    ])
}

/// Helper to convert 20-byte address to Element
fn address_to_element(addr: [u8; 20]) -> Element {
    let mut bytes = [0u8; 32];
    bytes[12..32].copy_from_slice(&addr);
    Element::from_be_bytes(bytes)
}
