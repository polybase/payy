use color_eyre::Result;
use ethereum_types::Address;
use secp256k1::{PublicKey, SecretKey, SECP256K1};
use sha3::{Digest, Keccak256};
use std::str::FromStr;

pub struct Wallet {
    secret_key: SecretKey,
}

impl Wallet {
    pub fn new(secret_key: SecretKey) -> Self {
        Self { secret_key }
    }

    pub fn new_from_str(secret_key: &str) -> Result<Self> {
        let secret_key = SecretKey::from_str(secret_key)?;
        Ok(Self { secret_key })
    }

    pub fn random() -> Self {
        Self {
            secret_key: SecretKey::new(&mut rand::thread_rng()),
        }
    }

    pub fn secret_key(&self) -> SecretKey {
        self.secret_key
    }

    pub fn web3_secret_key(&self) -> web3::signing::SecretKey {
        let secret_key = hex::encode(self.secret_key().as_ref());
        web3::signing::SecretKey::from_str(&secret_key).unwrap()
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey::from_secret_key(SECP256K1, &self.secret_key)
    }

    pub fn address(&self) -> [u8; 20] {
        let public_key = self.public_key();
        let serialized_pubkey = public_key.serialize_uncompressed();

        // Start from the 1st byte, to strip the 0x04 prefix from the public key.
        let hashed_pubkey = Keccak256::digest(&serialized_pubkey[1..]);

        // Get the last 20 bytes from the Keccak-256 hash. These last 20 bytes are the Ethereum address.
        let address_bytes = &hashed_pubkey[hashed_pubkey.len() - 20..];

        let mut address = [0u8; 20];
        address.copy_from_slice(address_bytes);
        address
    }

    pub fn to_eth_address(&self) -> Address {
        Address::from_slice(&self.address())
    }
}

pub fn gen_eth_wallet() -> (String, String) {
    let wallet = Wallet::random();
    (
        hex::encode(wallet.secret_key().as_ref()),
        hex::encode(wallet.address()),
    )
}
