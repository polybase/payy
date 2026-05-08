mod mpp;
mod x402;

use base64::{
    Engine,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;

use crate::error::{Error, Result};

use super::payment::ExecutedPayment;

pub(super) const PAYMENT_REQUIRED_HEADER: &str = "payment-required";
pub(super) const PAYMENT_SIGNATURE_HEADER: &str = "payment-signature";
pub(super) const X_PAYMENT_HEADER: &str = "x-payment";
pub(super) const WWW_AUTHENTICATE_HEADER: &str = "www-authenticate";
pub(super) const MPP_PROBLEM_TYPE: &str = "https://paymentauth.org/problems/payment-required";

#[derive(Clone, Debug)]
pub(crate) enum PaymentChallenge {
    X402(X402Challenge),
    Mpp(Box<MppChallenge>),
}

#[derive(Clone, Debug)]
pub(crate) struct X402Challenge {
    pub offers: Vec<X402Offer>,
    pub resource: Option<Value>,
    pub version: u8,
}

#[derive(Clone, Debug)]
pub(crate) struct X402Offer {
    pub amount: AmountValue,
    pub asset: String,
    pub network: String,
    pub pay_to: String,
    pub raw: Value,
    pub scheme: String,
}

#[derive(Clone, Debug)]
pub(crate) struct MppChallenge {
    pub auth: Option<MppAuthChallenge>,
    pub problem: MppProblem,
    pub request: MppPaymentRequest,
}

#[derive(Clone, Debug)]
pub(crate) struct MppProblem {
    pub challenge_id: String,
    pub detail: Option<String>,
    pub title: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct MppAuthChallenge {
    pub description: Option<String>,
    pub digest: Option<String>,
    pub expires: Option<String>,
    pub id: String,
    pub intent: String,
    pub method: String,
    pub opaque: Option<String>,
    pub realm: String,
    pub request: String,
}

#[derive(Clone, Debug)]
pub(crate) struct MppPaymentRequest {
    pub amount: AmountValue,
    pub chain_id: Option<u64>,
    pub currency: String,
    pub description: Option<String>,
    pub recipient: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AmountValue {
    Atomic(String),
    Human(String),
}

#[derive(Clone, Debug)]
pub(crate) struct RetryHeader {
    pub name: HeaderName,
    pub value: HeaderValue,
}

impl PaymentChallenge {
    pub(crate) fn protocol_name(&self) -> &'static str {
        match self {
            Self::X402(_) => "x402",
            Self::Mpp(_) => "MPP",
        }
    }

    pub(crate) fn describe(&self) -> String {
        match self {
            Self::X402(challenge) => x402::describe_x402(challenge),
            Self::Mpp(challenge) => mpp::describe_mpp(challenge),
        }
    }

    pub(crate) fn retry_header(&self, executed: &ExecutedPayment) -> Result<RetryHeader> {
        match self {
            Self::X402(challenge) => x402::build_x402_retry_header(challenge, executed),
            Self::Mpp(challenge) => mpp::build_mpp_retry_header(challenge, executed),
        }
    }
}

impl AmountValue {
    pub(crate) fn raw(&self) -> &str {
        match self {
            Self::Atomic(value) | Self::Human(value) => value,
        }
    }
}

pub(crate) fn parse_payment_challenge(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Option<PaymentChallenge>> {
    if let Some(encoded) = header_value(headers, PAYMENT_REQUIRED_HEADER) {
        return x402::parse_x402_from_header(&encoded).map(Some);
    }

    if body.is_empty() {
        return Ok(None);
    }

    let value = match serde_json::from_slice::<Value>(body) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    if mpp::is_mpp_problem(&value) {
        return mpp::parse_mpp(headers, value).map(Some);
    }

    if value.get("accepts").is_some() {
        return x402::parse_x402_from_value(value).map(Some);
    }

    Ok(None)
}

pub(super) fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
}

pub(super) fn decode_base64(value: &str) -> Result<Vec<u8>> {
    for engine in [STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD] {
        if let Ok(bytes) = engine.decode(value.trim()) {
            return Ok(bytes);
        }
    }

    Err(Error::FetchInvalidPaymentResponse)
}
