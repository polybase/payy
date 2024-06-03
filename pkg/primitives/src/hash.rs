use crate::u256;
use borsh::{BorshDeserialize, BorshSerialize};
use contracts::H256;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Borrow;
use std::fmt::{Debug, Display};
use std::str::FromStr;

#[derive(
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
// Serialize transparently with serde
// because otherwise it would be serialized as a tuple.
#[serde(transparent)]
pub struct CryptoHash(#[serde(with = "hex::serde")] pub [u8; 32]);

impl CryptoHash {
    pub const SIZE: usize = 32;

    pub fn new(v: [u8; 32]) -> Self {
        Self(v)
    }

    pub fn from_u64(n: u64) -> Self {
        let mut bytes = [0u8; 32];
        bytes[24..32].copy_from_slice(&n.to_be_bytes());
        Self(bytes)
    }

    pub fn as_u256(&self) -> u256::U256 {
        u256::U256::from_little_endian(&self.0)
    }

    pub fn inner(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn into_inner(self) -> [u8; 32] {
        self.0
    }

    pub fn from_vec_hash(v: Vec<u8>) -> Self {
        let bytes: [u8; 32] = Sha256::digest(v).into();
        Self(bytes)
    }

    pub fn genesis() -> Self {
        Self([0u8; 32])
    }

    pub fn to_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Display for CryptoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl Debug for CryptoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl Borrow<[u8]> for CryptoHash {
    fn borrow(&self) -> &[u8] {
        &self.0
    }
}

impl FromStr for CryptoHash {
    type Err = <H256 as FromStr>::Err;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        Ok(Self(H256::from_str(s)?.into()))
    }
}
