use contextful::{FromContextful, InternalError};
use currency::Currency;
use data::payment::Payment;
use element::Element;
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

/// Details for unsupported currency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsupportedCurrency {
    /// Currency that was requested but is not supported
    pub currency: Currency,
}

/// Details for invalid payment value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidPaymentValue {
    /// Payment value that was provided but is invalid
    pub value: Element,
}

/// RPC errors for guild
#[derive(
    Debug, Clone, thiserror::Error, HTTPErrorConversion, FromContextful, Serialize, Deserialize,
)]
pub enum Error {
    /// Invalid signature length, epected uncompressed signature
    #[bad_request("payments-unsupported-currency")]
    #[error("[guild-interface/payments] currency is not supported")]
    UnsupportedCurrency(UnsupportedCurrency),

    /// Payment requested is not found, will also be returned if
    /// user requests a payment they do not own
    #[not_found("payments-not-found")]
    #[error("[guild-interface/payments] payment not found")]
    PaymentNotFound,

    /// Invalid payment value provided
    #[bad_request("payments-invalid-payment-value")]
    #[error("[guild-interface/payments] invalid payment value")]
    InvalidPaymentValue(InvalidPaymentValue),

    /// Payment cannot be fulfilled - the payment kind is "out
    /// of stock"
    #[bad_request("payments-cannot-be-fulfilled")]
    #[error("[guild-interface/payments] payment cannot be fulfilled")]
    PaymentCannotBeFulfilled,

    /// Internal error
    #[error("[guild-interface/payments] internal error")]
    Internal(#[from] InternalError),
}

/// Result type for payment operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Kind of payment - reason for payment
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentKind {
    /// Payment is for a ramp deposit
    RampDeposit,
}

/// Input data for creating a payment
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentInput {
    /// The payment amount value - currently on USD supported
    /// 500 => $5
    pub value: Element,
    /// Currency - currently only USD is allowed
    pub currency: Currency,
    /// The kind/reason for the payment, so when the payment is
    /// approved the fulfilment can proceed
    pub kind: PaymentKind,
}

/// Response data for a payment request
#[derive(Debug, Serialize, Clone)]
pub struct PaymentResponse {
    /// Client secret for the payment - used by the client
    /// to submit payment request to Stripe
    pub client_secret: String,
    /// The payment record
    pub payment: Payment,
}
