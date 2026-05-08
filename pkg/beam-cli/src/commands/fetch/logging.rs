use reqwest::{
    Response,
    header::{HeaderMap, HeaderName, HeaderValue},
};

use super::{
    RequestSpec,
    protocol::{PAYMENT_SIGNATURE_HEADER, X_PAYMENT_HEADER},
};

const REDACTED_REQUEST_HEADER_VALUE: &str = "<redacted>";

pub(super) fn print_request(spec: &RequestSpec, headers: &HeaderMap) {
    eprintln!("> {} {}", spec.method, spec.url);
    for (name, value) in headers {
        eprintln!(
            "> {}: {}",
            name.as_str(),
            printable_request_header_value(name, value)
        );
    }
}

pub(super) fn print_response(response: &Response) {
    let reason = response.status().canonical_reason().unwrap_or("Unknown");
    eprintln!("< {} {}", response.status().as_u16(), reason);
    for (name, value) in response.headers() {
        eprintln!("< {}: {}", name.as_str(), printable_header_value(value));
    }
}

fn printable_header_value(value: &HeaderValue) -> String {
    value
        .to_str()
        .map_or_else(|_| "<binary>".to_string(), ToString::to_string)
}

fn printable_request_header_value(name: &HeaderName, value: &HeaderValue) -> String {
    if is_sensitive_request_header(name) {
        return REDACTED_REQUEST_HEADER_VALUE.to_string();
    }

    printable_header_value(value)
}

fn is_sensitive_request_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "authorization"
            | "proxy-authorization"
            | "cookie"
            | PAYMENT_SIGNATURE_HEADER
            | X_PAYMENT_HEADER
    )
}

#[cfg(test)]
pub(crate) fn printable_request_header_value_for_test(name: &str, value: &str) -> String {
    let name = HeaderName::from_bytes(name.as_bytes()).expect("test header name");
    let value = HeaderValue::from_str(value).expect("test header value");

    printable_request_header_value(&name, &value)
}
