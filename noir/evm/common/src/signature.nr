use std::embedded_curve_ops::EmbeddedCurvePoint;

use crate::note::owner_from_public_key;

// SPEC(docs/specs/privacy-protocol#owner-signature-shape)
pub struct OwnerSignature {
    pub signature: [u8; 64],
    pub public_key_x: Field,
    pub public_key_y: Field,
}

// SPEC(docs/specs/privacy-protocol#owner-auth-schnorr-steps)
// INVARIANT(docs/specs/privacy-protocol#spend-auth-schnorr-grumpkin)
pub fn assert_owner_signature(signature: OwnerSignature, expected_owner: Field, message: Field) {
    let public_key = EmbeddedCurvePoint {
        x: signature.public_key_x,
        y: signature.public_key_y,
        is_infinite: false,
    };

    assert(
        // SPEC(docs/specs/privacy-protocol#tx-commitment-raw-32-byte-big-endian-signing)
        schnorr::verify_signature(public_key, signature.signature, message.to_be_bytes::<32>()),
        "invalid owner signature",
    );
    assert(
        owner_from_public_key(signature.public_key_x, signature.public_key_y) == expected_owner,
        "owner mismatch",
    );
}
