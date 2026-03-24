use element::Element;
use ethereum_types::{Address, H160, H256, U256};
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use sha3::{Digest, Keccak256};

pub trait Eth {
    fn to_secp256k1_secret_key(&self) -> SecretKey;
    fn to_eth_address(&self) -> Address;
    fn from_160(h160: &H160) -> Self;
    fn to_h256(&self) -> H256;
    fn from_u256(u256: U256) -> Self;
    fn to_eth_u256(&self) -> U256;
}

impl Eth for Element {
    fn to_secp256k1_secret_key(&self) -> SecretKey {
        SecretKey::from_slice(&self.to_be_bytes()).expect("secret key must be random")
    }

    fn to_eth_address(&self) -> Address {
        secret_key_to_address(&self.to_secp256k1_secret_key())
    }

    fn from_160(h160: &H160) -> Element {
        let mut h256 = [0u8; 32];
        h256[12..32].copy_from_slice(&h160.0);
        Element::from_be_bytes(h256)
    }

    fn to_h256(&self) -> H256 {
        H256::from_slice(self.to_be_bytes().as_slice())
    }

    fn from_u256(u256: U256) -> Self {
        Self::from_u64_array(u256.0)
    }

    fn to_eth_u256(&self) -> U256 {
        ethereum_types::U256(self.to_u64_array())
    }
}

pub fn secret_key_to_address(secret_key: &SecretKey) -> Address {
    // Create a secp256k1 context
    let secp = Secp256k1::new();

    // Derive public key from private key
    let public_key = PublicKey::from_secret_key(&secp, secret_key);

    // Serialize the public key in uncompressed format
    let public_key_serialized = public_key.serialize_uncompressed();

    // Hash the public key using Keccak-256 (skip the first byte which is the format byte)
    let public_key_hash = Keccak256::digest(&public_key_serialized[1..]);

    // Take the last 20 bytes of the hash to get the Ethereum address
    let mut address_bytes = [0u8; 20];
    address_bytes.copy_from_slice(&public_key_hash[12..32]);

    Address::from(address_bytes)
}
