use element::Element;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

#[cfg(feature = "ts-rs")]
use ts_rs::TS;

/// Canonical note shape for the EVM privacy protocol.
// SPEC(docs/specs/privacy-protocol#note-six-field-structure)
// SPEC(docs/specs/privacy-protocol#note-field-definitions-table)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
pub struct EvmNote {
    /// Note kind/version. `0` is padding, `1` is the current real-note format.
    pub kind: Element,
    /// ERC-20 token address encoded as a field element.
    pub token: Element,
    // SPEC(docs/specs/privacy-protocol#nonce-chain-sequential-ordering)
    /// Sequential nonce for the `(owner, token)` note chain.
    pub nonce: Element,
    /// Per-note entropy used in commitments and nullifiers.
    pub psi: Element,
    /// Poseidon hash of the owner public key coordinates.
    pub owner: Element,
    /// Token balance carried by the note.
    pub value: Element,
}

impl EvmNote {
    /// Returns the canonical all-zero padding note.
    #[must_use]
    pub fn padding_note() -> Self {
        Self {
            kind: Element::ZERO,
            token: Element::ZERO,
            nonce: Element::ZERO,
            psi: Element::ZERO,
            owner: Element::ZERO,
            value: Element::ZERO,
        }
    }

    /// Computes the note commitment used in the rollup tree.
    #[must_use]
    pub fn commitment(&self) -> Element {
        if self.kind.is_zero() {
            debug_assert_eq!(*self, Self::padding_note());
            Element::ZERO
        } else {
            hash::hash_merge([
                self.kind, self.token, self.nonce, self.psi, self.owner, self.value,
            ])
        }
    }

    /// Computes the spend nullifier for the note.
    #[must_use]
    pub fn nullifier(&self) -> Element {
        let commitment = self.commitment();
        if commitment.is_zero() {
            Element::ZERO
        } else {
            hash::hash_merge([commitment, self.psi])
        }
    }
}

/// Computes the first nonce-hash entry for a new `(owner, token)` chain.
// SPEC(docs/specs/privacy-protocol#nonce-hash-formula)
// INVARIANT(docs/specs/privacy-protocol#first-note-input-psi-zero)
#[must_use]
pub fn first_nonce_hash(kind: Element, token: Element, owner: Element) -> Element {
    hash::hash_merge([kind, token, owner, Element::ZERO, Element::ZERO])
}

/// Computes the next nonce-hash entry after spending an existing note.
// SPEC(docs/specs/privacy-protocol#nonce-hash-formula)
// SPEC(docs/specs/privacy-protocol#nonce-hash-chain-requires-prev-psi)
#[must_use]
pub fn next_nonce_hash(
    kind: Element,
    token: Element,
    owner: Element,
    output_nonce: Element,
    input_psi: Element,
) -> Element {
    hash::hash_merge([kind, token, owner, output_nonce, input_psi])
}

/// Computes the EVM privacy transaction commitment signed by note owners.
// SPEC(docs/specs/privacy-protocol#tx-commitment-twelve-element-poseidon)
// SPEC(docs/specs/privacy-protocol#tx-commitment-field-inclusion-rationale)
// INVARIANT(docs/specs/privacy-protocol#tx-commitment-kind-tag)
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn tx_commitment(
    chain_id: Element,
    bridge_address: Element,
    input_commitment_0: Element,
    input_commitment_1: Element,
    output_commitment_0: Element,
    output_commitment_1: Element,
    burn_recipient: Element,
    mint_from: Element,
    user_encrypted_key_hash: Element,
    recipient_encrypted_key_hash: Element,
    receive_prefix: Element,
) -> Element {
    hash::hash_merge([
        Element::ONE,
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
    ])
}

/// Encodes a 20-byte EVM address into a field element.
#[must_use]
pub fn address_to_field(address: [u8; 20]) -> Element {
    let mut bytes = [0u8; 32];
    bytes[12..].copy_from_slice(&address);
    Element::from_be_bytes(bytes)
}

/// Decodes the low 20 bytes of a field element as an EVM address.
#[must_use]
pub fn field_to_address(field: Element) -> [u8; 20] {
    let bytes = field.to_be_bytes();
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes[12..]);
    out
}

/// Derives the 6-byte receive prefix from the canonical big-endian owner encoding.
// SPEC(docs/specs/privacy-protocol#incoming-note-log-prefix)
#[must_use]
pub fn receive_prefix(owner: Element) -> Element {
    let bytes = owner.to_be_bytes();
    let mut prefix = [0u8; 32];
    prefix[26..].copy_from_slice(&bytes[..6]);
    Element::from_be_bytes(prefix)
}

/// Reduces the calldata user-encrypted key bundle to a field-safe hash.
// SPEC(docs/specs/privacy-protocol#user-encrypted-key-hash-derivation)
#[must_use]
pub fn encrypted_key_hash(words: &[[u8; 32]; 4]) -> Element {
    let mut hasher = Keccak256::new();
    for word in words {
        hasher.update(word);
    }
    let digest = hasher.finalize();
    let hash = ethnum::U256::from_be_bytes(digest.into());
    Element::from(hash % Element::MODULUS.to_u256())
}

/// Reduces the calldata user-encrypted key bundle to a field-safe hash.
// SPEC(docs/specs/privacy-protocol#user-encrypted-key-hash-derivation)
#[must_use]
pub fn user_encrypted_key_hash(words: &[[u8; 32]; 4]) -> Element {
    encrypted_key_hash(words)
}

#[cfg(test)]
mod tests;
