use crate::util::u256;
use borsh::{BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256};
use std::borrow::Borrow;
use std::fmt::{Debug, Display};

#[derive(Clone, Default, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub struct ProposalHash([u8; 32]);

impl ProposalHash {
    pub fn new(v: [u8; 32]) -> Self {
        ProposalHash(v)
    }

    pub fn as_u256(&self) -> u256::U256 {
        u256::U256::from_little_endian(&self.0)
    }

    pub fn inner(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn from_vec_hash(v: Vec<u8>) -> Self {
        let bytes: [u8; 32] = Sha256::digest(v).into();
        ProposalHash(bytes)
    }

    pub fn genesis() -> Self {
        ProposalHash([0u8; 32])
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Display for ProposalHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl Debug for ProposalHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl Borrow<[u8]> for ProposalHash {
    fn borrow(&self) -> &[u8] {
        &self.0
    }
}
