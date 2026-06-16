use std::env;

use serde_json::Value;

use crate::apps::host::HostRequest;

pub(super) fn app_debug_enabled() -> bool {
    env::var("BEAM_APP_DEBUG")
        .map(|value| {
            let value = value.to_ascii_lowercase();
            matches!(value.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

pub(super) fn app_debug(message: &str) {
    if app_debug_enabled() {
        eprintln!("[beam-cli/apps/debug] {message}");
    }
}

pub(super) fn host_request_summary(request: &HostRequest) -> String {
    match request {
        HostRequest::AppMetadata => "app-metadata".to_string(),
        HostRequest::Args { args } => format!("args count={}", args.len()),
        HostRequest::StructuredOutput { value } => {
            format!("structured-output {}", value_shape(value))
        }
        HostRequest::Diagnostic { level, message } => {
            format!("diagnostic level={level} message={message}")
        }
        HostRequest::HttpFetch(request) => format!(
            "http-fetch method={} url={} request_bytes={}",
            request.method,
            request.url,
            request.body.len()
        ),
        HostRequest::ChainRead(request) => format!(
            "chain-read operation={:?} chain={} target={} selector={}",
            request.operation,
            request.chain,
            optional_value(&request.target),
            optional_value(&request.selector)
        ),
        HostRequest::SimulateTransaction(transaction) => format!(
            "simulate-transaction chain={} target={} selector={} spender={}",
            transaction.chain,
            transaction.target,
            optional_value(&transaction.selector),
            optional_value(&transaction.spender)
        ),
        HostRequest::SubmitTransaction(transaction) => format!(
            "submit-transaction chain={} target={} selector={}",
            transaction.chain,
            transaction.target,
            optional_value(&transaction.selector)
        ),
        HostRequest::PollReceipt { tx_hash } => format!("poll-receipt tx_hash={tx_hash}"),
        HostRequest::ResolveAddress { value } => {
            format!("resolve-address provided={}", value.is_some())
        }
        HostRequest::AppStorageGet { key } => format!("storage-get key={key}"),
        HostRequest::AppStorageSet { key, value } => {
            format!("storage-set key={key} {}", value_shape(value))
        }
        HostRequest::AppStorageRemove { key } => format!("storage-remove key={key}"),
    }
}

pub(super) fn host_value_summary(value: &Value) -> String {
    if let Some(status) = value.get("status").and_then(Value::as_u64) {
        let body_bytes = value
            .get("body")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default();
        return format!("status={status} response_bytes={body_bytes}");
    }
    value_shape(value)
}

fn value_shape(value: &Value) -> String {
    match value {
        Value::Array(values) => format!("array_len={}", values.len()),
        Value::Object(values) => format!("object_keys={}", values.len()),
        Value::String(value) => format!("string_len={}", value.len()),
        Value::Number(_) => "number".to_string(),
        Value::Bool(_) => "bool".to_string(),
        Value::Null => "null".to_string(),
    }
}

fn optional_value(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("<none>")
}
