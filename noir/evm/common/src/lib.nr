// Module index
// SPEC(docs/specs/privacy-protocol#evm-specific-common-modules)

mod encryption;
mod nonce;
mod note;
mod range;
mod signature;
mod tx;

// Design principles

pub use common::MerklePath;
pub use encryption::{encrypt_chain_key, encrypt_payload};
pub use nonce::{first_nonce_hash, next_nonce_hash};
pub use note::{EvmNote, note_commitment, note_nullifier, owner_from_public_key};
pub use range::assert_u240;
pub use signature::{OwnerSignature, assert_owner_signature};
pub use tx::{TX_COMMITMENT_KIND, receive_prefix_from_owner, tx_commitment};
