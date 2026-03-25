use async_trait::async_trait;
use contextful::ResultContextExt;
use contracts::Address;
use web3::{
    Web3,
    signing::{Key, SecretKey, SecretKeyRef},
    transports::Http,
    types::{SignedTransaction, TransactionParameters},
};

use crate::error::{Error, Result};

#[async_trait]
pub trait Signer: Send + Sync {
    fn address(&self) -> Address;

    async fn sign_transaction(
        &self,
        client: &Web3<Http>,
        transaction: TransactionParameters,
    ) -> Result<SignedTransaction>;
}

#[derive(Clone)]
pub struct KeySigner {
    address: Address,
    secret_key: SecretKey,
}

impl KeySigner {
    pub fn from_secret_key(secret_key: SecretKey) -> Self {
        let address = SecretKeyRef::new(&secret_key).address();
        Self {
            address,
            secret_key,
        }
    }

    pub fn from_slice(secret_key: &[u8]) -> Result<Self> {
        let secret_key = SecretKey::from_slice(secret_key).map_err(|_| Error::InvalidPrivateKey)?;
        Ok(Self::from_secret_key(secret_key))
    }
}

#[async_trait]
impl Signer for KeySigner {
    fn address(&self) -> Address {
        self.address
    }

    async fn sign_transaction(
        &self,
        client: &Web3<Http>,
        transaction: TransactionParameters,
    ) -> Result<SignedTransaction> {
        let signed = client
            .accounts()
            .sign_transaction(transaction, SecretKeyRef::new(&self.secret_key))
            .await
            .context("sign beam transaction")?;

        Ok(signed)
    }
}
