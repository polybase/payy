pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("[beam-app-erc8004] unsupported command: {command}")]
    UnsupportedCommand { command: String },

    #[error("[beam-app-erc8004] invalid argument: {reason}")]
    InvalidArgument { reason: String },

    #[error("[beam-app-erc8004] unsupported ERC-8004 chain id: {chain_id}")]
    UnsupportedChain { chain_id: u64 },

    #[error("[beam-app-erc8004] invalid agent uri: {uri}")]
    InvalidAgentUri { uri: String },

    #[error("[beam-app-erc8004] invalid agent id: {value}")]
    InvalidAgentId { value: String },

    #[error("[beam-app-erc8004] address value is invalid: {value}")]
    InvalidAddress { value: String },

    #[error("[beam-app-erc8004] integer value is invalid: {value}")]
    InvalidInteger { value: String },

    #[error("[beam-app-erc8004] host call failed: {message}")]
    HostCallFailed { message: String },

    #[error("[beam-app-erc8004] host response is invalid: {reason}")]
    InvalidHostResponse { reason: String },

    #[error("[beam-app-erc8004] serialization failed: {reason}")]
    Serialization { reason: String },
}
