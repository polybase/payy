use element::Element;

use super::{
    EvmNote, address_to_field, encrypted_key_hash, field_to_address, first_nonce_hash,
    next_nonce_hash, receive_prefix, tx_commitment, user_encrypted_key_hash,
};

#[test]
fn padding_note_commits_to_zero() {
    assert_eq!(EvmNote::padding_note().commitment(), Element::ZERO);
    assert_eq!(EvmNote::padding_note().nullifier(), Element::ZERO);
}

#[test]
fn nonce_hash_chain_changes_with_nonce_or_psi() {
    let first = first_nonce_hash(Element::ONE, Element::new(9), Element::new(11));
    let second = next_nonce_hash(
        Element::ONE,
        Element::new(9),
        Element::new(11),
        Element::ONE,
        Element::new(7),
    );

    assert_ne!(first, second);
}

#[test]
fn address_field_roundtrip_preserves_low_20_bytes() {
    let address = [0x11u8; 20];
    assert_eq!(field_to_address(address_to_field(address)), address);
}

#[test]
fn user_encrypted_key_hash_is_stable() {
    let words = [[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
    assert_eq!(
        user_encrypted_key_hash(&words),
        user_encrypted_key_hash(&words),
    );
    assert_eq!(user_encrypted_key_hash(&words), encrypted_key_hash(&words));
}

#[test]
fn receive_prefix_uses_first_six_owner_bytes() {
    let owner = Element::from_be_bytes([
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    ]);

    assert_eq!(receive_prefix(owner), Element::from(0x1234_5678_9abc_u64));
}

#[test]
fn tx_commitment_binds_recipient_key_hash_and_prefix() {
    let base = tx_commitment(
        Element::new(1),
        Element::new(2),
        Element::new(3),
        Element::new(4),
        Element::new(5),
        Element::new(6),
        Element::new(7),
        Element::new(8),
        Element::new(9),
        Element::new(10),
        Element::new(11),
    );
    let changed_recipient_key = tx_commitment(
        Element::new(1),
        Element::new(2),
        Element::new(3),
        Element::new(4),
        Element::new(5),
        Element::new(6),
        Element::new(7),
        Element::new(8),
        Element::new(9),
        Element::new(12),
        Element::new(11),
    );
    let changed_prefix = tx_commitment(
        Element::new(1),
        Element::new(2),
        Element::new(3),
        Element::new(4),
        Element::new(5),
        Element::new(6),
        Element::new(7),
        Element::new(8),
        Element::new(9),
        Element::new(10),
        Element::new(12),
    );

    assert_ne!(base, changed_recipient_key);
    assert_ne!(base, changed_prefix);
}
