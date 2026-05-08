use std::collections::BTreeMap;

use crate::error::{Error, Result};

use super::super::WWW_AUTHENTICATE_HEADER;
use super::MppAuthChallenge;

pub(super) fn parse_mpp_auth_challenge(
    headers: &reqwest::header::HeaderMap,
) -> Result<MppAuthChallenge> {
    for value in headers.get_all(WWW_AUTHENTICATE_HEADER) {
        let value = value
            .to_str()
            .map_err(|_| Error::FetchInvalidPaymentResponse)?;
        if let Some(auth) = extract_payment_auth_params(value) {
            return parse_payment_auth_params(auth);
        }
    }

    Err(Error::FetchInvalidPaymentResponse)
}

fn parse_payment_auth_params(input: &str) -> Result<MppAuthChallenge> {
    let params = parse_auth_params(input)?;

    Ok(MppAuthChallenge {
        description: params.get("description").cloned(),
        digest: params.get("digest").cloned(),
        expires: params.get("expires").cloned(),
        id: required_auth_param(&params, "id")?,
        intent: required_auth_param(&params, "intent")?,
        method: required_auth_param(&params, "method")?,
        opaque: params.get("opaque").cloned(),
        realm: required_auth_param(&params, "realm")?,
        request: required_auth_param(&params, "request")?,
    })
}

fn extract_payment_auth_params(header: &str) -> Option<&str> {
    let mut candidate_starts = vec![0];
    let mut escaped = false;
    let mut in_quotes = false;

    for (index, ch) in header.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_quotes => escaped = true,
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => candidate_starts.push(index + 1),
            _ => {}
        }
    }

    for start in candidate_starts {
        let candidate = header[start..].trim_start();
        let Some(auth) = strip_auth_scheme(candidate, "payment") else {
            continue;
        };
        let auth_end = payment_auth_params_len(auth);
        return Some(auth[..auth_end].trim_end());
    }

    None
}

fn strip_auth_scheme<'a>(input: &'a str, scheme: &str) -> Option<&'a str> {
    let input = input.trim_start();
    let scheme_end = input.find(char::is_whitespace)?;
    let auth_scheme = &input[..scheme_end];
    if !auth_scheme.eq_ignore_ascii_case(scheme) {
        return None;
    }

    let params = input[scheme_end..].trim_start();
    (!params.is_empty()).then_some(params)
}

fn payment_auth_params_len(input: &str) -> usize {
    let mut escaped = false;
    let mut in_quotes = false;

    for (index, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_quotes => escaped = true,
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes && !starts_auth_param(&input[index + 1..]) => return index,
            _ => {}
        }
    }

    input.len()
}

fn starts_auth_param(input: &str) -> bool {
    let input = input.trim_start();
    let token_end = input
        .find(|ch: char| ch.is_whitespace() || ch == ',' || ch == '=')
        .unwrap_or(input.len());
    if token_end == 0 {
        return false;
    }

    input[token_end..].trim_start().starts_with('=')
}

fn parse_auth_params(input: &str) -> Result<BTreeMap<String, String>> {
    let mut params = BTreeMap::new();
    let mut cursor = input.trim();

    while !cursor.is_empty() {
        let eq = cursor.find('=').ok_or(Error::FetchInvalidPaymentResponse)?;
        let key = cursor[..eq].trim();
        if key.is_empty() {
            return Err(Error::FetchInvalidPaymentResponse);
        }

        cursor = cursor[eq + 1..].trim_start();
        let (value, rest) = parse_auth_value(cursor)?;
        params.insert(key.to_string(), value);
        cursor = rest.trim_start();

        if let Some(remaining) = cursor.strip_prefix(',') {
            cursor = remaining.trim_start();
        } else if !cursor.is_empty() {
            return Err(Error::FetchInvalidPaymentResponse);
        }
    }

    Ok(params)
}

fn parse_auth_value(input: &str) -> Result<(String, &str)> {
    if let Some(rest) = input.strip_prefix('"') {
        let mut escaped = false;
        let mut value = String::new();

        for (index, ch) in rest.char_indices() {
            if escaped {
                value.push(ch);
                escaped = false;
                continue;
            }

            match ch {
                '\\' => escaped = true,
                '"' => return Ok((value, &rest[index + 1..])),
                _ => value.push(ch),
            }
        }

        return Err(Error::FetchInvalidPaymentResponse);
    }

    let next_comma = input.find(',').unwrap_or(input.len());
    Ok((input[..next_comma].trim().to_string(), &input[next_comma..]))
}

fn required_auth_param(params: &BTreeMap<String, String>, name: &str) -> Result<String> {
    params
        .get(name)
        .cloned()
        .ok_or(Error::FetchInvalidPaymentResponse)
}
