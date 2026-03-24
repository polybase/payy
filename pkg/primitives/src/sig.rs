use crate::{hash::CryptoHash, peer::Address};
use borsh::{BorshDeserialize, BorshSerialize};
use secp256k1::{
    Message, SECP256K1,
    ecdsa::{self, RecoveryId},
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

const NETWORK: &str = "Payy";

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Signature(#[serde(with = "hex::serde")] pub [u8; 65]);

impl Signature {
    pub fn inner(&self) -> &[u8] {
        &self.0
    }

    pub fn verify(&self, msg: &CryptoHash) -> Option<Address> {
        let mut hasher = Keccak256::new();
        hasher.update(NETWORK.len().to_be_bytes());
        hasher.update(NETWORK);
        hasher.update(msg.inner());
        let msg = Into::<[u8; 32]>::into(hasher.finalize());
        let msg = Message::from_digest(msg);

        let sig = self.inner();
        let sig = ecdsa::RecoverableSignature::from_compact(
            &sig[0..64],
            RecoveryId::from_i32(sig[64] as i32).ok()?,
        )
        .unwrap();

        let public_key = SECP256K1.recover_ecdsa(&msg, &sig).ok()?;
        Some(Address::from_public_key(public_key))
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Default for Signature {
    fn default() -> Self {
        Self([0u8; 65])
    }
}
