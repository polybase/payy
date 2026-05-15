use base64::Engine;
use contextful::ResultContextExt;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    error::{Error, Result},
    human_output::sanitize_control_chars,
};

use super::{
    AmountValue, ExecutedPayment, PAYMENT_SIGNATURE_HEADER, PaymentChallenge, RetryHeader,
    X_PAYMENT_HEADER, X402Challenge, X402Offer, decode_base64,
};

pub(super) fn parse_x402_from_header(encoded: &str) -> Result<PaymentChallenge> {
    let bytes = decode_base64(encoded)?;
    let value = serde_json::from_slice::<Value>(&bytes).context("parse beam x402 header json")?;
    parse_x402_from_value(value)
}

pub(super) fn parse_x402_from_value(value: Value) -> Result<PaymentChallenge> {
    let raw = serde_json::from_value::<RawX402Challenge>(value)
        .map_err(|_| Error::FetchInvalidPaymentResponse)?;
    if raw.accepts.is_empty() {
        return Err(Error::FetchInvalidPaymentResponse);
    }

    let version = raw
        .x402_version
        .unwrap_or_else(|| infer_x402_version(raw.resource.as_ref()));
    let mut offers = Vec::with_capacity(raw.accepts.len());

    for raw_offer in raw.accepts {
        offers.push(parse_x402_offer(raw_offer)?);
    }

    Ok(PaymentChallenge::X402(X402Challenge {
        offers,
        resource: raw.resource,
        version,
    }))
}

pub(super) fn describe_x402(challenge: &X402Challenge) -> String {
    let mut lines = vec![
        "Payment required via x402".to_string(),
        format!("Offers: {}", challenge.offers.len()),
    ];

    for offer in &challenge.offers {
        lines.push(format!(
            "- {} {} on {} to {} ({})",
            sanitize_control_chars(offer.amount.raw()),
            sanitize_control_chars(&offer.asset),
            sanitize_control_chars(&offer.network),
            sanitize_control_chars(offer.private_address.as_deref().unwrap_or(&offer.pay_to)),
            sanitize_control_chars(&offer.scheme),
        ));
    }

    lines.join("\n")
}

pub(super) fn build_x402_retry_header(
    challenge: &X402Challenge,
    executed: &ExecutedPayment,
) -> Result<RetryHeader> {
    let payload = if challenge.version >= 2 {
        json!({
            "x402Version": 2,
            "resource": challenge.resource.clone(),
            "accepted": executed.accepted.clone(),
            "payload": executed.proof.clone(),
        })
    } else {
        json!({
            "x402Version": 1,
            "scheme": executed.scheme,
            "network": executed.network,
            "payload": executed.proof.clone(),
        })
    };
    let encoded = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_vec(&payload).context("serialize beam x402 proof")?);
    let header_name = if challenge.version >= 2 {
        reqwest::header::HeaderName::from_static(PAYMENT_SIGNATURE_HEADER)
    } else {
        reqwest::header::HeaderName::from_static(X_PAYMENT_HEADER)
    };
    let header_value = reqwest::header::HeaderValue::from_str(&encoded)
        .map_err(|_| Error::FetchInvalidPaymentResponse)?;

    Ok(RetryHeader {
        name: header_name,
        value: header_value,
    })
}

fn infer_x402_version(resource: Option<&Value>) -> u8 {
    match resource {
        Some(Value::Object(_)) => 2,
        _ => 1,
    }
}

fn parse_x402_offer(raw: Value) -> Result<X402Offer> {
    let parsed = serde_json::from_value::<RawX402Offer>(raw.clone())
        .map_err(|_| Error::FetchInvalidPaymentResponse)?;
    let amount = parsed
        .amount
        .as_ref()
        .or(parsed.max_amount_required.as_ref())
        .ok_or(Error::FetchInvalidPaymentResponse)
        .and_then(amount_from_json)?;

    Ok(X402Offer {
        amount,
        asset: parsed.asset,
        network: parsed.network,
        pay_to: parsed.pay_to,
        private_address: privacy_recipient_from_value(&raw),
        raw,
        scheme: parsed.scheme,
    })
}

fn privacy_recipient_from_value(value: &Value) -> Option<String> {
    value
        .get("payToPrivateAddress")
        .or_else(|| value.get("privateAddress"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| {
            value
                .get("privacy")
                .and_then(|privacy| privacy.get("privateAddress"))
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
}

fn amount_from_json(value: &Value) -> Result<AmountValue> {
    match value {
        Value::Number(number) => Ok(AmountValue::Atomic(number.to_string())),
        Value::String(value) if value.contains('.') => Ok(AmountValue::Human(value.clone())),
        Value::String(value) => Ok(AmountValue::Atomic(value.clone())),
        _ => Err(Error::FetchInvalidPaymentResponse),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawX402Challenge {
    accepts: Vec<Value>,
    resource: Option<Value>,
    x402_version: Option<u8>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawX402Offer {
    amount: Option<Value>,
    asset: String,
    max_amount_required: Option<Value>,
    network: String,
    pay_to: String,
    scheme: String,
}
