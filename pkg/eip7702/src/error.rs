use client_http::{Error as ClientHttpError, NoRpcError};
use contextful::Contextful;
use ethereum_types::{FromStrRadixErr, U256};
use hex::FromHexError;
use thiserror::Error;

/// EIP-7702 result type alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Typed errors for the EIP-7702 crate.
#[derive(Debug, Error)]
pub enum Error {
    /// Relayer secret key was required but not configured.
    #[error("[eip7702] relayer secret key not configured")]
    RelayerNotConfigured,

    /// Authorization chain id does not match the network chain id.
    #[error(
        "[eip7702] chain id mismatch: authorization has {authorization:#x}, network has {network:#x}"
    )]
    ChainIdMismatch {
        /// Chain id encoded within the authorization tuple.
        authorization: U256,
        /// Chain id returned by the connected network.
        network: U256,
    },

    /// RPC call returned an error field instead of a successful result.
    #[error("[eip7702] rpc error: {response}")]
    RpcError {
        /// Full JSON value returned by the RPC endpoint.
        response: String,
    },

    /// Raw transaction submission failed and returned an error.
    #[error("[eip7702] rpc raw error: {response}")]
    RpcRawError {
        /// Full JSON value returned by the RPC endpoint.
        response: String,
    },

    /// Gas estimation RPC returned an error field.
    #[error("[eip7702] gas estimation error: {response}")]
    EstimateGasError {
        /// Full JSON value returned by the RPC endpoint.
        response: String,
    },

    /// Hex string could not be parsed into a U256 value.
    #[error("[eip7702] failed to parse hex value into U256")]
    HexU256Parse(#[from] Contextful<FromStrRadixErr>),

    /// Transaction hash string was malformed.
    #[error("[eip7702] invalid transaction hash")]
    TxHashParse(#[from] Contextful<FromHexError>),

    /// Transaction hash decoded to an unexpected length.
    #[error("[eip7702] invalid transaction hash length {length} for {hash}")]
    TxHashLength {
        /// Original hash string that decoded to an unexpected length.
        hash: String,
        /// Number of bytes produced from the decoded hash.
        length: usize,
    },

    /// Delegation was not visible on-chain within the allotted timeout.
    #[error("[eip7702] delegated code not visible after timeout")]
    DelegationTimeout,

    /// Underlying web3 client error.
    #[error("[eip7702] web3 error")]
    Web3(#[from] Contextful<web3::Error>),

    /// HTTP client level error.
    #[error("[eip7702] http error")]
    Reqwest(#[from] Contextful<reqwest::Error>),

    /// JSON serialization/deserialization error.
    #[error("[eip7702] json error")]
    Json(#[from] Contextful<serde_json::Error>),

    /// Higher-level HTTP client error when delegating to external relayers.
    #[error("[eip7702] client http error")]
    ClientHttp(#[from] Contextful<ClientHttpError<NoRpcError>>),
}
