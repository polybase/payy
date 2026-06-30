use contracts::{Address, U256};
use primitives::serde::{deserialize_hex_0x_prefixed, serialize_hex_0x_prefixed};
use serde::{Deserialize, Serialize};

/// Input for getting a Bungee quote.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetQuoteInput {
    /// Source chain id.
    pub source_chain_id: u128,
    /// Destination chain id.
    pub destination_chain_id: u128,
    /// Input token address.
    pub input_token: Address,
    /// Output token address.
    pub output_token: Address,
    /// Input amount.
    pub input_amount: U256,
    /// Receiver wallet address on the destination chain.
    pub receiver_address: Address,
    /// User wallet address on the source chain.
    pub user_address: Address,
}

/// Output for getting a Bungee quote.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetQuoteOutput {
    /// Expected output amount.
    pub output_amount: U256,
    /// Guaranteed minimum output amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_output_amount: Option<U256>,
    /// Inbox transaction target.
    pub tx_to: Address,
    /// Inbox transaction value in wei.
    pub tx_value: U256,
    /// Inbox transaction calldata.
    #[serde(
        serialize_with = "serialize_hex_0x_prefixed",
        deserialize_with = "deserialize_hex_0x_prefixed"
    )]
    pub tx_data: Vec<u8>,
    /// Optional approval spender.
    pub approval_spender: Option<Address>,
    /// Optional approval amount.
    pub approval_amount: Option<U256>,
    /// Optional provider quote id.
    pub quote_id: Option<String>,
    /// Optional provider request hash.
    pub request_hash: Option<String>,
}
