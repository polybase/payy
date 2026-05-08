// lint-long-file-override allow-max-lines=280
#[path = "mpp_auth.rs"]
mod auth;

use base64::Engine;
use contextful::ResultContextExt;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    error::{Error, Result},
    human_output::sanitize_control_chars,
};

use super::{
    AmountValue, ExecutedPayment, MPP_PROBLEM_TYPE, MppAuthChallenge, MppChallenge,
    MppPaymentRequest, MppProblem, PaymentChallenge, RetryHeader, decode_base64,
};

pub(super) fn is_mpp_problem(value: &Value) -> bool {
    value
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|value| value == MPP_PROBLEM_TYPE)
        && value.get("challengeId").is_some()
}

pub(super) fn parse_mpp(
    headers: &reqwest::header::HeaderMap,
    value: Value,
) -> Result<PaymentChallenge> {
    let problem = serde_json::from_value::<RawMppProblem>(value)
        .map_err(|_| Error::FetchInvalidPaymentResponse)?;
    let auth = auth::parse_mpp_auth_challenge(headers)?;
    if problem.challenge_id != auth.id {
        return Err(Error::FetchInvalidPaymentResponse);
    }
    let request = parse_mpp_request(&auth.request)?;

    Ok(PaymentChallenge::Mpp(Box::new(MppChallenge {
        auth: Some(auth),
        problem: MppProblem {
            challenge_id: problem.challenge_id,
            detail: problem.detail,
            title: problem.title,
        },
        request,
    })))
}

pub(super) fn describe_mpp(challenge: &MppChallenge) -> String {
    let mut lines = vec![
        "Payment required via MPP".to_string(),
        format!(
            "Challenge: {}",
            sanitize_control_chars(&challenge.problem.challenge_id)
        ),
    ];

    if let Some(title) = challenge.problem.title.as_ref() {
        lines.push(format!("Title: {}", sanitize_control_chars(title)));
    }

    if let Some(detail) = challenge.problem.detail.as_ref() {
        lines.push(format!("Detail: {}", sanitize_control_chars(detail)));
    }

    if let Some(auth) = challenge.auth.as_ref() {
        lines.push(format!(
            "Method: {} {} {} {}",
            sanitize_control_chars(&auth.method),
            sanitize_control_chars(challenge.request.amount.raw()),
            sanitize_control_chars(&challenge.request.currency),
            sanitize_control_chars(&challenge.request.recipient),
        ));
    }

    lines.join("\n")
}

pub(super) fn build_mpp_retry_header(
    challenge: &MppChallenge,
    executed: &ExecutedPayment,
) -> Result<RetryHeader> {
    let auth = challenge
        .auth
        .as_ref()
        .ok_or(Error::FetchInvalidPaymentResponse)?;
    let credential = json!({
        "challenge": auth.challenge_json(),
        "payload": executed.proof.clone(),
        "source": executed.source.clone(),
    });
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&credential).context("serialize beam mpp credential")?);
    let value = format!("Payment {encoded}");
    let header_value = reqwest::header::HeaderValue::from_str(&value)
        .map_err(|_| Error::FetchInvalidPaymentResponse)?;

    Ok(RetryHeader {
        name: reqwest::header::HeaderName::from_static("authorization"),
        value: header_value,
    })
}

fn parse_mpp_request(encoded_request: &str) -> Result<MppPaymentRequest> {
    let bytes = decode_base64(encoded_request)?;
    let value = serde_json::from_slice::<Value>(&bytes).context("parse beam mpp request json")?;
    let raw = serde_json::from_value::<RawMppRequest>(value)
        .map_err(|_| Error::FetchInvalidPaymentResponse)?;
    let amount = amount_from_json(&raw.amount)?;

    Ok(MppPaymentRequest {
        amount,
        chain_id: raw
            .method_details
            .as_ref()
            .and_then(|details| details.chain_id)
            .or(raw.chain_id),
        currency: raw.currency.ok_or(Error::FetchInvalidPaymentResponse)?,
        description: raw.description,
        recipient: raw.recipient.ok_or(Error::FetchInvalidPaymentResponse)?,
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

impl MppAuthChallenge {
    fn challenge_json(&self) -> Value {
        let mut value = json!({
            "id": self.id,
            "realm": self.realm,
            "method": self.method,
            "intent": self.intent,
            "request": self.request,
        });

        if let Some(object) = value.as_object_mut() {
            if let Some(description) = self.description.as_ref() {
                object.insert(
                    "description".to_string(),
                    Value::String(description.clone()),
                );
            }
            if let Some(digest) = self.digest.as_ref() {
                object.insert("digest".to_string(), Value::String(digest.clone()));
            }
            if let Some(expires) = self.expires.as_ref() {
                object.insert("expires".to_string(), Value::String(expires.clone()));
            }
            if let Some(opaque) = self.opaque.as_ref() {
                object.insert("opaque".to_string(), Value::String(opaque.clone()));
            }
        }

        value
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMppProblem {
    challenge_id: String,
    detail: Option<String>,
    title: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMppRequest {
    amount: Value,
    chain_id: Option<u64>,
    currency: Option<String>,
    description: Option<String>,
    method_details: Option<RawMppMethodDetails>,
    recipient: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMppMethodDetails {
    chain_id: Option<u64>,
}
