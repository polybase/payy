use evm_common::{
    EvmNote, MerklePath, OwnerSignature, assert_owner_signature, assert_u240, encrypt_chain_key,
    encrypt_payload, first_nonce_hash, next_nonce_hash, note_commitment, note_nullifier,
    tx_commitment,
};

// SPEC(docs/specs/privacy-protocol#mint-category-purpose)
// SPEC(docs/specs/privacy-protocol#mint-variant-io-and-constraints)
fn main(
    input_note: EvmNote,
    input_merkle_path: MerklePath,
    output_note: EvmNote,
    owner_signature: OwnerSignature,
    symmetric_key: Field,
    chain_id: pub Field,
    bridge_address: pub Field,
    recent_root: pub Field,
    input_nullifiers: pub [Field; 2],
    output_commitments: pub [Field; 2],
    nonce_hash: pub Field,
    user_encrypted_key_hash: pub Field,
    recipient_encrypted_key_hash: pub Field,
    sender_encrypted_note: pub [Field; 5],
    recipient_encrypted_note: pub [Field; 5],
    sender_chain_encrypted_key: pub [Field; 3],
    recipient_chain_encrypted_key: pub [Field; 3],
    chain_public_key: pub [Field; 2],
    token: pub Field,
    burn_recipient: pub Field,
    value: pub Field,
    mint_from: pub Field,
    receive_prefix: pub Field,
) {
    // INVARIANT(docs/specs/privacy-protocol#note-value-240-bit)
    assert_u240(input_note.value);
    assert_u240(output_note.value);
    assert_u240(value);
    assert(value != 0, "mint value");
    assert(mint_from != 0, "mint from");
    assert(user_encrypted_key_hash != 0, "user key hash");
    assert(output_note.kind == 1, "output kind");
    assert(output_note.token == token, "output token");
    assert(recipient_encrypted_key_hash == 0, "recipient key hash");
    assert(receive_prefix == 0, "receive prefix");
    // INVARIANT(docs/specs/privacy-protocol#mint-value-conservation)
    assert(output_note.value == input_note.value + value, "value conservation");
    // INVARIANT(docs/specs/privacy-protocol#unused-public-input-slots-constrained-zero)
    assert(input_nullifiers[1] == 0, "unused nullifier");
    // INVARIANT(docs/specs/privacy-protocol#unused-public-input-slots-constrained-zero)
    assert(output_commitments[1] == 0, "unused commitment");
    // INVARIANT(docs/specs/privacy-protocol#unused-public-input-slots-constrained-zero)
    assert(burn_recipient == 0, "unused burn recipient");
    // INVARIANT(docs/specs/privacy-protocol#encrypted-data-literal-zero-tail)
    assert(recipient_encrypted_note == [0; 5], "unused recipient note");
    // INVARIANT(docs/specs/privacy-protocol#encrypted-data-literal-zero-tail)
    assert(recipient_chain_encrypted_key == [0; 3], "unused recipient chain key");

    let input_commitment = note_commitment(input_note);
    let signed_commitment = tx_commitment(
        chain_id,
        bridge_address,
        input_commitment,
        0,
        output_commitments[0],
        0,
        0,
        mint_from,
        user_encrypted_key_hash,
        0,
        0,
    );
    // SPEC(docs/specs/privacy-protocol#auth-rules-per-circuit-table)
    // SPEC(docs/specs/privacy-protocol#auth-model-single-primitive)
    // SPEC(docs/specs/privacy-protocol#output-notes-authenticated-via-signer-binding)
    // INVARIANT(docs/specs/privacy-protocol#mint-requires-output-owner-signature)
    assert_owner_signature(owner_signature, output_note.owner, signed_commitment);

    // INVARIANT(docs/specs/privacy-protocol#nonce-hash-computed-in-circuit)
    // INVARIANT(docs/specs/privacy-protocol#nonce-hash-targets-change-note)
    if input_note.kind == 0 {
        // SPEC(docs/specs/privacy-protocol#nonce-chain-sequential-ordering)
        assert(output_note.nonce == 0, "first nonce");
        // INVARIANT(docs/specs/privacy-protocol#padding-slot-nullifiers-circuit-zero)
        assert(input_nullifiers[0] == 0, "padding nullifier");
        assert(
            nonce_hash == first_nonce_hash(output_note.kind, output_note.token, output_note.owner),
            "first nonce hash",
        );
    } else {
        // SPEC(docs/specs/privacy-protocol#zero-value-note-can-be-spent-via-mint)
        assert(input_note.kind == 1, "input kind");
        // SPEC(docs/specs/privacy-protocol#nonce-chain-sequential-ordering)
        assert(output_note.nonce == input_note.nonce + 1, "nonce increment");
        assert(output_note.owner == input_note.owner, "owner chain");
        assert(output_note.token == input_note.token, "token chain");
        assert(input_merkle_path.validate(input_commitment, recent_root), "input path");
        assert(
            note_nullifier(input_commitment, input_note.psi) == input_nullifiers[0],
            "input nullifier",
        );
        assert(
            nonce_hash
                == next_nonce_hash(
                    output_note.kind,
                    output_note.token,
                    output_note.owner,
                    output_note.nonce,
                    input_note.psi,
                ),
            "nonce hash",
        );
    }

    assert(note_commitment(output_note) == output_commitments[0], "output commitment");
    assert(
        // SPEC(docs/specs/privacy-protocol#encrypted-payload-plaintext-layout)
        // SPEC(docs/specs/privacy-protocol#encrypted-payload-length-per-variant)
        encrypt_payload(
            [
                output_note.token,
                output_note.nonce,
                output_note.psi,
                output_note.owner,
                output_note.value,
            ],
            symmetric_key,
        ) == sender_encrypted_note,
        "sender encrypted note",
    );
    assert(
        encrypt_chain_key(symmetric_key, chain_public_key[0], chain_public_key[1])
            == sender_chain_encrypted_key,
        "sender chain encrypted key",
    );
}
