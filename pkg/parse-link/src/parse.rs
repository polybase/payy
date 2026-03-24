// lint-long-file-override allow-max-lines=500
use crate::{InviteLink, Link, RequestLink, SendLink, SendLinkHash};
use element::Element;
use ethnum::U256;
use std::borrow::Cow;
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur when parsing Payy links.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    /// The input URL was empty or only contained whitespace.
    #[error("[parse-link/parse] empty url input")]
    EmptyInput,
    /// The URL path did not include a leading segment (e.g. `/s`).
    #[error("[parse-link/parse] missing url prefix segment")]
    MissingPrefix,
    /// The URL prefix is not recognised.
    #[error("[parse-link/parse] unsupported prefix '{0}'")]
    UnknownPrefix(String),
    /// A required segment was absent for the provided prefix.
    #[error("[parse-link/parse] missing required segment for '{0}' link")]
    MissingSegment(String),
    /// A required hash fragment was absent.
    #[error("[parse-link/parse] missing hash fragment for '{0}' link")]
    MissingHash(String),
    /// A segment failed to parse into an `Element`.
    #[error("[parse-link/parse] invalid element in segment '{segment}': {value}")]
    InvalidElement {
        /// The segment name being parsed.
        segment: &'static str,
        /// The raw value that failed to parse.
        value: String,
    },
    /// The link did not match any known request formats.
    #[error("[parse-link/parse] invalid request link format")]
    InvalidRequestFormat,
    /// The link did not match any known send formats.
    #[error("[parse-link/parse] invalid send link format")]
    InvalidSendFormat,
}

/// Parse a URL into a [`Link`] returning detailed errors on failure.
pub fn parse_url(url: &str) -> Result<Link, ParseError> {
    let components = parse_components(url)?;

    match components.prefix {
        "r" => parse_request(&components.segments, components.hash),
        prefix if prefix.starts_with('s') => parse_send(&components.segments, components.hash),
        "invite" => parse_invite(&components.segments),
        prefix => Err(ParseError::UnknownPrefix(prefix.to_owned())),
    }
}

/// Parse only send links, returning `None` when the input is not a send link.
#[must_use]
pub fn parse_send_url(url: &str) -> Option<SendLink> {
    parse_url(url).ok().and_then(|link| match link {
        Link::Send(send) => Some(SendLink::Send(send)),
        _ => None,
    })
}

/// Extract an invite code (if available) from a parsed link.
#[must_use]
pub fn extract_invite_code(link: &Link) -> Option<&str> {
    match link {
        Link::Request(request) => request.invite_code.as_deref(),
        Link::Send(_) => None,
        Link::Invite(invite) => Some(invite.code.as_str()),
    }
}

/// Extract the memo text for a parsed link (empty string when absent).
#[must_use]
pub fn extract_memo(link: &Link) -> Cow<'_, str> {
    match link {
        Link::Request(request) => request
            .memo
            .as_deref()
            .map_or(Cow::Borrowed(""), Cow::Borrowed),
        Link::Send(send) => send
            .memo
            .as_deref()
            .map_or(Cow::Borrowed(""), Cow::Borrowed),
        Link::Invite(_) => Cow::Borrowed(""),
    }
}

struct UrlComponents<'a> {
    prefix: &'a str,
    segments: Vec<&'a str>,
    hash: Option<&'a str>,
}

fn parse_components(url: &str) -> Result<UrlComponents<'_>, ParseError> {
    let mut path = url.trim();
    if path.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    for prefix in [
        "https://payy.link/",
        "http://payy.link/",
        "payy.link/",
        "https://www.payy.link/",
        "http://www.payy.link/",
        "www.payy.link/",
    ] {
        if let Some(stripped) = path.strip_prefix(prefix) {
            path = stripped;
            break;
        }
    }

    while path.starts_with('/') {
        path = &path[1..];
    }

    let (path_no_query, hash_query) = path.split_once('?').unwrap_or((path, ""));
    let (path_no_hash, hash_part) =
        path_no_query
            .split_once('#')
            .map_or((path_no_query, None), |(p, h)| {
                let (hash_without_query, _) = h.split_once('?').unwrap_or((h, ""));
                (p, Some(hash_without_query))
            });

    let mut iter = path_no_hash.split('/');
    let prefix = iter.next().unwrap_or("");
    if prefix.is_empty() {
        return Err(ParseError::MissingPrefix);
    }

    let mut segments: Vec<&str> = iter.collect();

    // Preserve trailing empty segment when the path ends with `/` by inspecting the
    // original hash-less path string.
    if path_no_hash.ends_with('/') {
        segments.push("");
    }

    Ok(UrlComponents {
        prefix,
        segments,
        hash: hash_part.or_else(|| hash_query.split_once('#').map(|(_, h)| h)),
    })
}

fn parse_request(segments: &[&str], hash: Option<&str>) -> Result<Link, ParseError> {
    let a = segments.first().copied().unwrap_or("");
    let b = segments.get(1).copied().unwrap_or("");

    if a == "1" && !b.is_empty() {
        let hash = hash.ok_or_else(|| ParseError::MissingHash("r".to_owned()))?;
        if hash.is_empty() {
            return Err(ParseError::MissingHash("r".to_owned()));
        }

        let mut hash_parts = hash.split('/');
        let public_key_str = hash_parts
            .next()
            .filter(|segment| !segment.is_empty())
            .ok_or(ParseError::InvalidRequestFormat)?;
        let public_key = parse_element("public_key", public_key_str)?;

        let invite_code = hash_parts
            .next()
            .filter(|segment| !segment.is_empty())
            .map(std::string::ToString::to_string);

        let value = parse_element("value", b)?;
        let memo = segments.get(2).map(|memo| (*memo).to_owned());

        return Ok(Link::Request(RequestLink {
            value,
            public_key,
            invite_code,
            memo,
        }));
    }

    Err(ParseError::InvalidRequestFormat)
}

fn parse_send(segments: &[&str], hash: Option<&str>) -> Result<Link, ParseError> {
    if let Some(secret_hash) = hash {
        if secret_hash.is_empty() {
            return Err(ParseError::InvalidSendFormat);
        }

        let memo = segments
            .first()
            .map(|segment| (*segment).to_owned())
            .filter(|memo| !memo.is_empty());

        return Ok(Link::Send(SendLinkHash {
            secret_hash: secret_hash.to_owned(),
            memo,
        }));
    }

    Err(ParseError::InvalidSendFormat)
}

fn parse_invite(segments: &[&str]) -> Result<Link, ParseError> {
    let code = segments
        .first()
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| ParseError::MissingSegment("invite".to_owned()))?;

    Ok(Link::Invite(InviteLink {
        code: (*code).to_owned(),
    }))
}

fn parse_element(segment: &'static str, value: &str) -> Result<Element, ParseError> {
    try_parse_element(value).ok_or_else(|| ParseError::InvalidElement {
        segment,
        value: value.to_owned(),
    })
}

fn try_parse_element(value: &str) -> Option<Element> {
    if value.is_empty() {
        return None;
    }

    if value.chars().all(|c| c.is_ascii_digit())
        && let Ok(decimal) = U256::from_str_radix(value, 10)
    {
        return Some(Element::from(decimal));
    }

    Element::from_str(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_invite_link() {
        let link = parse_url("https://payy.link/invite/9KJLMZ").unwrap();
        match link {
            Link::Invite(invite) => assert_eq!(invite.code, "9KJLMZ"),
            other => panic!("unexpected link parsed: {other:?}"),
        }

        let link = parse_url("https://payy.link/invite/9KJLMZ?posthog_id=abc").unwrap();
        match link {
            Link::Invite(invite) => assert_eq!(invite.code, "9KJLMZ"),
            other => panic!("unexpected link parsed: {other:?}"),
        }
    }

    #[test]
    fn parse_send_link_with_memo() {
        let link = parse_url("https://payy.link/s/howdy#").unwrap_err();
        assert_eq!(link, ParseError::InvalidSendFormat);

        let link = parse_url(
            "https://payy.link/s/howdy#52wMdUKJXCEc7Bp4VeAf55QF9g7rC4Usez6p3RhK5a25xgEQqD",
        )
        .unwrap();
        match link {
            Link::Send(send) => {
                assert_eq!(
                    send.secret_hash,
                    "52wMdUKJXCEc7Bp4VeAf55QF9g7rC4Usez6p3RhK5a25xgEQqD"
                );
                assert_eq!(send.memo, Some("howdy".to_owned()));
            }
            other => panic!("unexpected link parsed: {other:?}"),
        }

        let link = parse_url(
            "https://payy.link/s/howdy#52wMdUKJXCEc7Bp4VeAf55QF9g7rC4Usez6p3RhK5a25xgEQqD?posthog=1",
        )
        .unwrap();
        assert!(matches!(link, Link::Send(_)));
    }

    #[test]
    fn parse_request_links() {
        let modern = parse_url("https://payy.link/r/1/1000000/send_money#0x00c6/8D9GEF").unwrap();
        match modern {
            Link::Request(request) => {
                assert_eq!(
                    request.value,
                    Element::from(U256::from_str_radix("1000000", 10).unwrap())
                );
                assert_eq!(request.public_key, Element::from_str("0x00c6").unwrap());
                assert_eq!(request.invite_code.as_deref(), Some("8D9GEF"));
                assert_eq!(request.memo.as_deref(), Some("send_money"));
            }
            other => panic!("unexpected link parsed: {other:?}"),
        }
    }

    #[test]
    fn parse_request_v1_without_memo_has_empty_string() {
        let link = parse_url("https://payy.link/r/1/1000000/#0x00c6/8D9GEF").unwrap();

        match link {
            Link::Request(request) => {
                assert_eq!(request.memo.as_deref(), Some(""));
                assert_eq!(
                    extract_invite_code(&Link::Request(request.clone())),
                    Some("8D9GEF")
                );
            }
            other => panic!("unexpected link parsed: {other:?}"),
        }
    }

    #[test]
    fn extract_memo_defaults_to_empty_string() {
        let link = parse_url("https://payy.link/invite/9KJLMZ").unwrap();
        assert_eq!(extract_memo(&link), Cow::Borrowed(""));
    }

    #[test]
    fn extract_invite_code_variants() {
        let invite = parse_url("https://payy.link/invite/9KJLMZ").unwrap();
        assert_eq!(extract_invite_code(&invite), Some("9KJLMZ"));

        let send = parse_url(
            "https://payy.link/s/howdy#52wMdUKJXCEc7Bp4VeAf55QF9g7rC4Usez6p3RhK5a25xgEQqD",
        )
        .unwrap();
        assert_eq!(extract_invite_code(&send), None);
    }

    #[test]
    fn parse_send_url_helper_matches_kinds() {
        let send = parse_send_url(
            "https://payy.link/s/howdy#52wMdUKJXCEc7Bp4VeAf55QF9g7rC4Usez6p3RhK5a25xgEQqD",
        )
        .unwrap();
        let SendLink::Send(send) = send;
        assert_eq!(send.memo.as_deref(), Some("howdy"));
    }

    #[test]
    fn parse_url_returns_error_for_unknown() {
        assert!(parse_url("https://payy.link/unknown/abc123").is_err());
    }

    #[test]
    fn parse_unknown_prefix_fails() {
        let err = parse_url("https://payy.link/unknown/abc123").unwrap_err();
        assert!(matches!(err, ParseError::UnknownPrefix(prefix) if prefix == "unknown"));
    }
}
