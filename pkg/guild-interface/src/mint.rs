use contextful::{FromContextful, InternalError};
use element::Element;
use ethereum_types::{Address, H256, U256};
use primitives::serde::{deserialize_hex_0x_prefixed, serialize_hex_0x_prefixed};
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

/// Details for insufficient balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsufficientBalance {
    /// Required balance for the mint
    pub required: U256,
    /// Available balance for the mint
    pub available: U256,
    /// Token kind
    pub token: Element,
}

/// RPC errors for guild
#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, FromContextful, Serialize, Deserialize,
)]
pub enum Error {
    /// Invalid signature length, epected uncompressed signature
    #[bad_request("invalid-signature-length")]
    #[error("[guild-interface/mint] invalid signature length, expected 65, got {length}")]
    InvalidSignatureLength {
        /// Signature length received
        length: usize,
    },

    /// Mints with zero value are not allowed
    #[bad_request("zero-mint-value")]
    #[error("[guild-interface/mint] mint value cannot be zero")]
    ZeroMintValue,

    /// EVM deposit address does not have the required mint funds
    #[failed_precondition("insufficient-balance")]
    #[error("[guild-interface/mint] insufficient balance {0:?}")]
    InsufficientBalance(InsufficientBalance),

    /// Internal error
    #[error("[guild-interface/mint] internal error")]
    Internal(#[from] InternalError),
}

/// Result type for mint operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Raw mint input, includes the core data to be used by the mint contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintInput {
    /// Mint hash used to identify mints in the smart contract. Note: mint_hash can
    /// be re-used once the mint resolves in the smart contract.
    pub mint_hash: Element,
    /// Value of the mint
    pub value: Element,
    /// Kind of note being minted
    pub note_kind: Element,
}

/// Mint request, including signed data for submission to EVM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintRequest {
    /// Mint input
    pub mint: MintInput,
    /// Depsoit address to mint funds from
    pub deposit_addr: Address,
    /// USDC signature for transfer via signature
    #[serde(
        serialize_with = "serialize_hex_0x_prefixed",
        deserialize_with = "deserialize_hex_0x_prefixed"
    )]
    pub usdc_sig: Vec<u8>,
    /// Mint signature to prevent frontrunning the mint
    #[serde(
        serialize_with = "serialize_hex_0x_prefixed",
        deserialize_with = "deserialize_hex_0x_prefixed"
    )]
    pub mint_sig: Vec<u8>,
    /// Valid after expiration for signature
    pub valid_after: U256,
    /// Valid before expiration for signature
    pub valid_before: U256,
    /// Nonce for randomness
    pub nonce: H256,
}

impl MintRequest {
    /// Validate the mint request is valid
    pub fn validate(&self) -> Result<()> {
        if self.usdc_sig.len() != 65 {
            return Err(Error::InvalidSignatureLength {
                length: self.usdc_sig.len(),
            });
        }

        if self.mint_sig.len() != 65 {
            return Err(Error::InvalidSignatureLength {
                length: self.mint_sig.len(),
            });
        }

        Ok(())
    }
}

/// Response for mint if successful
#[derive(Debug, Serialize, Deserialize)]
pub struct MintResponse {
    /// EVM txn hash
    pub txn_hash: H256,
    /// The kind of note to be minted
    pub note_kind: Element,
    /// Value that was minted
    pub value: Element,
    /// Has the mint already been spent
    pub spent: bool,
}
