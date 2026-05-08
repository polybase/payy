use poseidon::poseidon2::Poseidon2;
use std::embedded_curve_ops::{EmbeddedCurvePoint, EmbeddedCurveScalar, fixed_base_scalar_mul, multi_scalar_mul};

use crate::range::assert_u240;

// SPEC(docs/specs/privacy-protocol#encryption-three-purposes)
// SPEC(docs/specs/privacy-protocol#symmetric-encryption-poseidon-stream-cipher)
pub fn encrypt_payload<let N: u32>(payload: [Field; N], symmetric_key: Field) -> [Field; N] {
    // INVARIANT(docs/specs/privacy-protocol#symmetric-key-240-bit)
    assert_u240(symmetric_key);

    let mut ciphertext = [0; N];
    for i in 0..N {
        // SPEC(docs/specs/privacy-protocol#symmetric-encryption-poseidon-stream-cipher)
        let keystream = Poseidon2::hash([symmetric_key, i as Field], 2);
        ciphertext[i] = payload[i] + keystream;
    }

    ciphertext
}

// SPEC(docs/specs/privacy-protocol#design-principle-output-scoped-encryption)
//
// SPEC(docs/specs/privacy-protocol#chain-key-pke-scalar-is-sym-key)
// SPEC(docs/specs/privacy-protocol#chain-key-pke-tee-decryption)
// INVARIANT(docs/specs/privacy-protocol#chain-key-encryption-verified-in-zk)
pub fn encrypt_chain_key(
    symmetric_key: Field,
    chain_public_key_x: Field,
    chain_public_key_y: Field,
) -> [Field; 3] {
    // INVARIANT(docs/specs/privacy-protocol#symmetric-key-240-bit)
    assert_u240(symmetric_key);

    let chain_public_key = EmbeddedCurvePoint {
        x: chain_public_key_x,
        y: chain_public_key_y,
        is_infinite: false,
    };
    // SPEC(docs/specs/privacy-protocol#chain-key-pke-scalar-is-sym-key)
    let scalar = EmbeddedCurveScalar::from_field(symmetric_key);
    let shared_secret = multi_scalar_mul([chain_public_key], [scalar]);
    let ephemeral_public_key = fixed_base_scalar_mul(scalar);
    let encrypted_symmetric_key =
        symmetric_key + Poseidon2::hash([shared_secret.x, shared_secret.y], 2);

    [ephemeral_public_key.x, ephemeral_public_key.y, encrypted_symmetric_key]
}
