use poseidon::poseidon2::Poseidon2;

// INVARIANT(docs/specs/privacy-protocol#tx-commitment-kind-tag)
pub global TX_COMMITMENT_KIND: Field = 1;

// SPEC(docs/specs/privacy-protocol#tx-commitment-twelve-element-poseidon)
// SPEC(docs/specs/privacy-protocol#tx-commitment-field-inclusion-rationale)
pub fn tx_commitment(
    chain_id: Field,
    bridge_address: Field,
    input_commitment_0: Field,
    input_commitment_1: Field,
    output_commitment_0: Field,
    output_commitment_1: Field,
    burn_recipient: Field,
    mint_from: Field,
    user_encrypted_key_hash: Field,
    recipient_encrypted_key_hash: Field,
    receive_prefix: Field,
) -> Field {
    Poseidon2::hash(
        [
            TX_COMMITMENT_KIND,
            chain_id,
            bridge_address,
            input_commitment_0,
            input_commitment_1,
            output_commitment_0,
            output_commitment_1,
            burn_recipient,
            mint_from,
            user_encrypted_key_hash,
            recipient_encrypted_key_hash,
            receive_prefix,
        ],
        12,
    )
}

// SPEC(docs/specs/privacy-protocol#incoming-note-log-prefix)
pub fn receive_prefix_from_owner(owner: Field) -> Field {
    let bytes = owner.to_be_bytes::<32>();
    (bytes[0] as Field) * 0x10000000000
        + (bytes[1] as Field) * 0x100000000
        + (bytes[2] as Field) * 0x1000000
        + (bytes[3] as Field) * 0x10000
        + (bytes[4] as Field) * 0x100
        + (bytes[5] as Field)
}

#[test]
fn tx_commitment_changes_with_bindings() {
    let tx_a = tx_commitment(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
    let tx_b = tx_commitment(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12);
    assert(tx_a != tx_b);
}

#[test]
fn receive_prefix_from_owner_uses_first_six_bytes() {
    let owner = Field::from_be_bytes::<8>([0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0, 1]);
    assert(receive_prefix_from_owner(owner) == 0);

    let high_owner = Field::from_be_bytes::<32>([
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    assert(receive_prefix_from_owner(high_owner) == 0x123456789abc);
}
