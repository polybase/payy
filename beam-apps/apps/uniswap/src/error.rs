pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("[beam-app-uniswap] unsupported command: {command}")]
    UnsupportedCommand { command: String },

    #[error("[beam-app-uniswap] invalid argument: {reason}")]
    InvalidArgument { reason: String },

    #[error("[beam-app-uniswap] invalid Uniswap response: {reason}")]
    InvalidUniswapResponse { reason: String },

    #[error("[beam-app-uniswap] unsupported Uniswap route: {route}")]
    UnsupportedUniswapRoute { route: String },

    #[error("[beam-app-uniswap] quote expired")]
    QuoteExpired,

    #[error("[beam-app-uniswap] insufficient {token} balance")]
    InsufficientBalance { token: String },

    #[error("[beam-app-uniswap] integer value is invalid: {value}")]
    InvalidInteger { value: String },

    #[error("[beam-app-uniswap] address value is invalid: {value}")]
    InvalidAddress { value: String },

    #[error("[beam-app-uniswap] host call failed: {message}")]
    HostCallFailed { message: String },

    #[error("[beam-app-uniswap] host response is invalid: {reason}")]
    InvalidHostResponse { reason: String },

    #[error("[beam-app-uniswap] serialization failed: {reason}")]
    Serialization { reason: String },
}
