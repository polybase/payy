use crate::{hash::CryptoHash, sig::Signature};
use borsh::{BorshDeserialize, BorshSerialize};
use secp256k1::{Message, PublicKey, SecretKey, SECP256K1};
use serde::{Deserialize, Deserializer, Serialize};
use sha3::{Digest, Keccak256};
use std::{fmt::Display, str::FromStr};

#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize, BorshDeserialize,
)]
pub struct Address([u8; 20]);

impl Address {
    pub fn new(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    pub fn prefix(&self) -> String {
        self.0
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<String>>()[..4]
            .join("")
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_public_key(public_key: PublicKey) -> Address {
        let serialized_pubkey = public_key.serialize_uncompressed();

        // Start from the 1st byte, to strip the 0x04 prefix from the public key.
        let hashed_pubkey = Keccak256::digest(&serialized_pubkey[1..]);

        // Get the last 20 bytes from the Keccak-256 hash. These last 20 bytes are the Ethereum address.
        let address_bytes = &hashed_pubkey[hashed_pubkey.len() - 20..];

        let mut address = [0u8; 20];
        address.copy_from_slice(address_bytes);
        Self(address)
    }

    pub fn from_secret_key(secret_key: &SecretKey) -> Address {
        let public_key = PublicKey::from_secret_key(SECP256K1, secret_key);
        Self::from_public_key(public_key)
    }

    pub fn verify(&self, sig: Signature, msg: &CryptoHash) -> bool {
        sig.verify(msg).unwrap_or_default() == *self
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn to_u256(&self) -> crate::u256::U256 {
        crate::u256::U256::from_little_endian(&self.0)
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(&self.to_string(), serializer)
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <std::string::String as serde::Deserialize>::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl FromStr for Address {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s)?;
        let mut array_32 = [0u8; 20];
        array_32.copy_from_slice(&bytes);
        Ok(Self(array_32))
    }
}

impl From<contracts::Address> for Address {
    fn from(addr: contracts::Address) -> Self {
        Self(addr.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerIdSigner {
    peer_id: Address,
    secret_key: SecretKey,
}

impl PeerIdSigner {
    pub fn new(secret_key: SecretKey) -> Self {
        Self {
            peer_id: Address::from_secret_key(&secret_key),
            secret_key,
        }
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.secret_key.as_ref())
    }

    pub fn address(&self) -> Address {
        self.peer_id.clone()
    }

    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    pub fn sign(&self, msg: &CryptoHash) -> Signature {
        let mut hasher = Keccak256::new();
        hasher.update(b"Polybase".len().to_be_bytes());
        hasher.update(b"Polybase");
        hasher.update(msg.inner());
        let msg = Into::<[u8; 32]>::into(hasher.finalize());
        let msg = Message::from_digest(msg);

        let sig = SECP256K1.sign_ecdsa_recoverable(&msg, &self.secret_key);
        let mut sig_serialized = vec![0; 65];
        let (recovery, rest) = sig.serialize_compact();
        sig_serialized[0..64].copy_from_slice(&rest[0..64]);
        sig_serialized[64] = recovery.to_i32() as u8;

        Signature(sig_serialized.try_into().unwrap())
    }
}

impl FromStr for PeerIdSigner {
    type Err = secp256k1::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let secret_key = SecretKey::from_str(s)?;
        Ok(Self::new(secret_key))
    }
}

impl Display for PeerIdSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.peer_id)
    }
}

impl Default for PeerIdSigner {
    fn default() -> Self {
        Self::new(SecretKey::new(&mut rand::thread_rng()))
    }
}

impl Serialize for PeerIdSigner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(&self.to_string(), serializer)
    }
}

impl<'de> Deserialize<'de> for PeerIdSigner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <std::string::String as serde::Deserialize>::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use rand::RngCore;
//     use secp256k1::rand::thread_rng;

//     fn gen_peer() -> PeerIdSigner {
//         let mut rng = thread_rng();
//         let mut secret_key_bytes = [0u8; 32];
//         rng.fill_bytes(&mut secret_key_bytes);
//         secret_key_bytes[31] = 1;
//         let secret_key = SecretKey::from_slice(&secret_key_bytes).unwrap();

//         PeerIdSigner::new(secret_key)
//     }

//     #[test]
//     fn test_sign_verify() {
//         let signer = gen_peer();

//         let proposal = ProposalAcceptSigData::new(
//             ProposalHeader {
//                 hash: ProposalHash::new([123u8; 32]),
//                 height: 0,
//                 skips: 0,
//             },
//             0,
//         );
//         let sig = proposal.sign(&signer);
//         assert!(proposal.verify(&signer.peer(), &sig));
//     }

//     #[test]
//     fn test_sign_verify_fail_if_different_block_hash() {
//         let signer = gen_peer();

//         let proposal = ProposalAcceptSigData::new(
//             ProposalHeader {
//                 hash: ProposalHash::new([123u8; 32]),
//                 height: 0,
//                 skips: 0,
//             },
//             0,
//         );
//         let sig = proposal.sign(&signer);

//         let proposal_with_different_hash = ProposalAcceptSigData::new(
//             ProposalHeader {
//                 hash: ProposalHash::new([124u8; 32]),
//                 height: 0,
//                 skips: 0,
//             },
//             0,
//         );
//         assert!(!proposal_with_different_hash.verify(&signer.peer(), &sig));
//     }

//     #[test]
//     fn test_sign_verify_fail_if_different_peer() {
//         let signer = gen_peer();
//         let signer2 = gen_peer();

//         let proposal = ProposalAcceptSigData::new(
//             ProposalHeader {
//                 hash: ProposalHash::new([123u8; 32]),
//                 height: 0,
//                 skips: 0,
//             },
//             0,
//         );
//         let sig = proposal.sign(&signer);
//         assert!(!proposal.verify(&signer2.peer(), &sig));
//     }
// }
