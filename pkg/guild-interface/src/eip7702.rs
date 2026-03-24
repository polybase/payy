use contracts::{Address, H256, U256};
use primitives::serde::{deserialize_hex_0x_prefixed, serialize_hex_0x_prefixed};
use serde::{Deserialize, Serialize};

/// Authorization tuple submitted by the client.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AuthorizationInput {
    /// Chain id included in the authorization tuple
    pub chain_id: U256,
    /// Delegate implementation address authorized by the user
    pub delegate: Address,
    /// User transaction nonce at time of signing
    pub nonce: U256,
    /// y-parity (aka recovery id parity) of the signature (0 or 1)
    pub y_parity: u8,
    /// r component of the signature
    pub r: H256,
    /// s component of the signature
    pub s: H256,
}

/// Input to relay a SetCode (type 0x04) upgrade using a signed authorization
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayUpgradeInput {
    /// User address to upgrade
    pub user: Address,
    /// Signed authorization
    pub authorization: AuthorizationInput,
}

/// One meta-transaction call
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CallInput {
    /// Target contract to call
    pub target: Address,
    /// Value in wei
    pub value: U256,
    /// Calldata for target (hex)
    #[serde(
        serialize_with = "serialize_hex_0x_prefixed",
        deserialize_with = "deserialize_hex_0x_prefixed"
    )]
    pub data: Vec<u8>,
}

/// Input to relay a meta transaction batch (executeMeta)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayMetaInput {
    /// Chain id to relay on
    pub chain_id: U256,
    /// User address (EOA executing delegate code)
    pub user: Address,
    /// Calls to execute in order
    pub calls: Vec<CallInput>,
    /// Meta-txn nonce
    pub nonce: U256,
    /// Validity window start (unix seconds)
    pub valid_after: U256,
    /// Validity window end (unix seconds)
    pub valid_until: U256,
    /// User signature over EIP-712 digest
    #[serde(
        serialize_with = "serialize_hex_0x_prefixed",
        deserialize_with = "deserialize_hex_0x_prefixed"
    )]
    pub signature: Vec<u8>,
}

/// Output with a transaction hash
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayTxOutput {
    /// Transaction hash of the relayed transaction
    pub txn: H256,
}

/// Output for account nonce
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountNonceOutput {
    /// Current meta-transaction nonce read from the delegate contract
    pub nonce: U256,
}
