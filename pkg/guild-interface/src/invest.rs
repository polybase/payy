use contextful::{FromContextful, InternalError};
use element::Element;
use rpc::error::{ErrorOutput, HTTPError, TryFromHTTPError};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

/// RPC errors for guild invest / yield operations.
#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, FromContextful, Serialize, Deserialize,
)]
pub enum Error {
    /// Catch-all internal error wrapper.
    #[error("[guild-interface/invest] internal error")]
    Internal(#[from] InternalError),
}

/// Aggregate invested / withdrawn totals used to derive all-time gains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldPosition {
    /// Sum of completed USDC -> USTB swap inputs.
    pub invested_total: Element,
    /// Sum of completed USTB -> USDC swap outputs.
    pub withdrawn_total: Element,
}
