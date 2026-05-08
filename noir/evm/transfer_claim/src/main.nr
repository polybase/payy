use evm_common::{
    EvmNote, MerklePath, OwnerSignature, assert_owner_signature, assert_u240, encrypt_chain_key,
    encrypt_payload, first_nonce_hash, next_nonce_hash, note_commitment, note_nullifier,
    tx_commitment,
};

// SPEC(docs/specs/privacy-protocol#transfer-claim-dual-signature-purpose)
// SPEC(docs/specs/privacy-protocol#transfer-claim-variant-io-and-constraints)
fn main(
    input_note_own: EvmNote,
    input_note_incoming: EvmNote,
    input_merkle_path_own: MerklePath,
    input_merkle_path_incoming: MerklePath,
    output_note: EvmNote,
    recipient_signature: OwnerSignature,
    incoming_note_signature: OwnerSignature,
    sender_symmetric_key: Field,
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
    assert(output_note.kind == 1, "output kind");
    assert(input_note_incoming.kind == 1, "incoming kind");
    // INVARIANT(docs/specs/privacy-protocol#note-value-240-bit)
    assert_u240(input_note_own.value);
    assert_u240(input_note_incoming.value);
    assert_u240(output_note.value);
    assert(user_encrypted_key_hash != 0, "user key hash");
    assert(recipient_encrypted_key_hash == 0, "recipient key hash");
    assert(receive_prefix == 0, "receive prefix");
    // Claim-side of transfer-value-conservation:
    // Send invariant covers value preservation between sender input and outputs in transfer_send;
    // transfer_claim covers the symmetric side, enforcing conservation between the two inputs
    // (own + ephemeral) and the merged output.
    assert(
        input_note_own.value + input_note_incoming.value == output_note.value,
        "value conservation",
    );
    assert(output_note.token == input_note_incoming.token, "incoming token");
    assert(output_commitments[1] == 0, "unused commitment");
    assert(token == 0, "unused token");
    assert(burn_recipient == 0, "unused burn recipient");
    assert(value == 0, "unused value");
    assert(mint_from == 0, "unused mint from");
    // INVARIANT(docs/specs/privacy-protocol#encrypted-data-literal-zero-tail)
    assert(recipient_encrypted_note == [0; 5], "unused recipient note");
    // INVARIANT(docs/specs/privacy-protocol#encrypted-data-literal-zero-tail)
    assert(recipient_chain_encrypted_key == [0; 3], "unused recipient chain key");

    let input_commitment_0 = note_commitment(input_note_own);
    let input_commitment_1 = note_commitment(input_note_incoming);
    let signed_commitment = tx_commitment(
        chain_id,
        bridge_address,
        input_commitment_0,
        input_commitment_1,
        output_commitments[0],
        0,
        0,
        0,
        user_encrypted_key_hash,
        0,
        0,
    );
    // SPEC(docs/specs/privacy-protocol#auth-rules-per-circuit-table)
    // SPEC(docs/specs/privacy-protocol#auth-model-single-primitive)
    // SPEC(docs/specs/privacy-protocol#output-notes-authenticated-via-signer-binding)
    // INVARIANT(docs/specs/privacy-protocol#transfer-claim-requires-recipient-signature)
    assert_owner_signature(recipient_signature, output_note.owner, signed_commitment);
    assert_owner_signature(incoming_note_signature, input_note_incoming.owner, signed_commitment);
    assert(input_merkle_path_incoming.validate(input_commitment_1, recent_root), "incoming path");
    assert(
        note_nullifier(input_commitment_1, input_note_incoming.psi) == input_nullifiers[1],
        "incoming nullifier",
    );

    // INVARIANT(docs/specs/privacy-protocol#nonce-hash-computed-in-circuit)
    // INVARIANT(docs/specs/privacy-protocol#nonce-hash-targets-change-note)
    if input_note_own.kind == 0 {
        // SPEC(docs/specs/privacy-protocol#nonce-chain-sequential-ordering)
        assert(output_note.nonce == 0, "first nonce");
        // INVARIANT(docs/specs/privacy-protocol#padding-slot-nullifiers-circuit-zero)
        assert(input_nullifiers[0] == 0, "padding nullifier");
        assert(
            nonce_hash == first_nonce_hash(output_note.kind, output_note.token, output_note.owner),
            "first nonce hash",
        );
    } else {
        assert(input_note_own.kind == 1, "own kind");
        // SPEC(docs/specs/privacy-protocol#nonce-chain-sequential-ordering)
        assert(output_note.nonce == input_note_own.nonce + 1, "nonce increment");
        assert(output_note.owner == input_note_own.owner, "owner chain");
        assert(output_note.token == input_note_own.token, "token chain");
        assert(input_note_incoming.token == input_note_own.token, "shared token");
        assert(input_merkle_path_own.validate(input_commitment_0, recent_root), "own path");
        assert(
            note_nullifier(input_commitment_0, input_note_own.psi) == input_nullifiers[0],
            "own nullifier",
        );
        assert(
            nonce_hash
                == next_nonce_hash(
                    output_note.kind,
                    output_note.token,
                    output_note.owner,
                    output_note.nonce,
                    input_note_own.psi,
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
            sender_symmetric_key,
        ) == sender_encrypted_note,
        "sender encrypted note",
    );
    assert(
        encrypt_chain_key(sender_symmetric_key, chain_public_key[0], chain_public_key[1])
            == sender_chain_encrypted_key,
        "sender chain encrypted key",
    );
}
