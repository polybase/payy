use evm_common::{
    EvmNote, MerklePath, OwnerSignature, assert_owner_signature, assert_u240, encrypt_chain_key,
    encrypt_payload, next_nonce_hash, note_commitment, note_nullifier, receive_prefix_from_owner,
    tx_commitment,
};

// SPEC(docs/specs/privacy-protocol#transfer-category-purpose)
// SPEC(docs/specs/privacy-protocol#transfer-send-direct-recipient-distribution)
// SPEC(docs/specs/privacy-protocol#transfer-send-variant-io-and-constraints)
// INVARIANT(docs/specs/privacy-protocol#transfer-send-recipient-bound)
fn main(
    input_note: EvmNote,
    input_merkle_path: MerklePath,
    output_note_self: EvmNote,
    output_note_recv: EvmNote,
    owner_signature: OwnerSignature,
    sender_symmetric_key: Field,
    recipient_symmetric_key: Field,
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
    assert(input_note.kind == 1, "input kind");
    assert(output_note_self.kind == 1, "self kind");
    assert(output_note_recv.kind == 1, "recv kind");
    // INVARIANT(docs/specs/privacy-protocol#note-value-240-bit)
    assert_u240(input_note.value);
    assert_u240(output_note_self.value);
    assert_u240(output_note_recv.value);
    assert(user_encrypted_key_hash != 0, "user key hash");
    assert(recipient_encrypted_key_hash != 0, "recipient key hash");
    // INVARIANT(docs/specs/privacy-protocol#transfer-value-conservation)
    assert(
        input_note.value == output_note_self.value + output_note_recv.value,
        "value conservation",
    );
    // INVARIANT(docs/specs/privacy-protocol#transfer-send-recipient-value-positive)
    assert(output_note_recv.value != 0, "recv value");
    // SPEC(docs/specs/privacy-protocol#nonce-chain-sequential-ordering)
    assert(output_note_self.nonce == input_note.nonce + 1, "nonce increment");
    // SPEC(docs/specs/privacy-protocol#output-notes-authenticated-via-signer-binding)
    assert(output_note_self.owner == input_note.owner, "owner chain");
    assert(output_note_self.token == input_note.token, "self token");
    assert(output_note_recv.token == input_note.token, "recv token");
    assert(output_note_recv.nonce == 0, "recv nonce");
    assert(input_nullifiers[1] == 0, "unused nullifier");
    assert(token == 0, "unused token");
    assert(burn_recipient == 0, "unused burn recipient");
    assert(value == 0, "unused value");
    assert(mint_from == 0, "unused mint from");
    assert(receive_prefix == receive_prefix_from_owner(output_note_recv.owner), "receive prefix");

    let input_commitment = note_commitment(input_note);
    let signed_commitment = tx_commitment(
        chain_id,
        bridge_address,
        input_commitment,
        0,
        output_commitments[0],
        output_commitments[1],
        0,
        0,
        user_encrypted_key_hash,
        recipient_encrypted_key_hash,
        receive_prefix,
    );
    // SPEC(docs/specs/privacy-protocol#auth-rules-per-circuit-table)
    // SPEC(docs/specs/privacy-protocol#auth-model-single-primitive)
    assert_owner_signature(owner_signature, input_note.owner, signed_commitment);
    assert(input_merkle_path.validate(input_commitment, recent_root), "input path");
    assert(
        note_nullifier(input_commitment, input_note.psi) == input_nullifiers[0],
        "input nullifier",
    );
    assert(note_commitment(output_note_self) == output_commitments[0], "self commitment");
    assert(note_commitment(output_note_recv) == output_commitments[1], "recv commitment");
    // INVARIANT(docs/specs/privacy-protocol#nonce-hash-computed-in-circuit)
    // INVARIANT(docs/specs/privacy-protocol#nonce-hash-targets-change-note)
    assert(
        nonce_hash
            == next_nonce_hash(
                output_note_self.kind,
                output_note_self.token,
                output_note_self.owner,
                output_note_self.nonce,
                input_note.psi,
            ),
        "nonce hash",
    );
    assert(
        // SPEC(docs/specs/privacy-protocol#encrypted-payload-plaintext-layout)
        // SPEC(docs/specs/privacy-protocol#encrypted-payload-length-per-variant)
        encrypt_payload(
            [
                output_note_self.token,
                output_note_self.nonce,
                output_note_self.psi,
                output_note_self.owner,
                output_note_self.value,
            ],
            sender_symmetric_key,
        ) == sender_encrypted_note,
        "sender encrypted note",
    );
    assert(
        encrypt_payload(
            [
                output_note_recv.token,
                output_note_recv.nonce,
                output_note_recv.psi,
                output_note_recv.owner,
                output_note_recv.value,
            ],
            recipient_symmetric_key,
        ) == recipient_encrypted_note,
        "recipient encrypted note",
    );
    assert(
        encrypt_chain_key(sender_symmetric_key, chain_public_key[0], chain_public_key[1])
            == sender_chain_encrypted_key,
        "sender chain encrypted key",
    );
    assert(
        encrypt_chain_key(recipient_symmetric_key, chain_public_key[0], chain_public_key[1])
            == recipient_chain_encrypted_key,
        "recipient chain encrypted key",
    );
}
