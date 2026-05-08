// lint-long-file-override allow-max-lines=300
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! Payy-EVM claim-link V3 parsing and encoding.

mod note;

use element::Element;
use payy_evm_client_interface::{
    ClaimLink, ClaimSourceKind, DirectLinkedNote, Error, IncomingTransfer, ParsedClaimLink, Result,
    ValidationErrorKind, owner_from_private_key,
};
use zk_primitives::EvmNote;

use crate::note::{append_compact_note, read_compact_note, read_word, validate_received_note};

const CLAIM_LINK_VERSION_V3: u8 = 3;

/// Claim-link parser and encoder helpers.
#[derive(Debug, Clone, Copy)]
pub struct LinksClient;

impl LinksClient {
    /// Parse a claim link.
    pub fn parse(&self, link: &str) -> Result<ParsedClaimLink> {
        parse_claim_link(link)
    }
}

/// Parse a V3 claim link.
pub fn parse_claim_link(link: &str) -> Result<ParsedClaimLink> {
    let (path, secret) = link.split_once('#').ok_or(Error::Validation {
        kind: ValidationErrorKind::InvalidClaimLink,
    })?;
    if path != "/s" && !path.starts_with("/s/") {
        return Err(Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        });
    }
    let message = parse_message(path)?;
    let payload = bs58::decode(secret)
        .into_vec()
        .map_err(|_| Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        })?;
    decode_payload(message, &payload)
}

/// Encode a direct claim link.
#[must_use]
pub fn encode_direct_claim_link(note: EvmNote, message: Option<&str>) -> ClaimLink {
    let mut payload = vec![CLAIM_LINK_VERSION_V3, 0];
    append_compact_note(note, &mut payload);
    encode_payload(message, &payload)
}

/// Encode an ephemeral claim link.
#[must_use]
pub fn encode_ephemeral_claim_link(
    incoming_transfer: &IncomingTransfer,
    message: Option<&str>,
) -> ClaimLink {
    let mut payload = vec![CLAIM_LINK_VERSION_V3, 1];
    append_compact_note(incoming_transfer.note, &mut payload);
    payload.extend_from_slice(&incoming_transfer.ephemeral_private_key);
    encode_payload(message, &payload)
}

/// Derive the canonical owner for an ephemeral private key.
pub fn ephemeral_owner(private_key: &[u8; 32]) -> Result<Element> {
    owner_from_private_key(*private_key)
}

fn decode_payload(message: Option<String>, payload: &[u8]) -> Result<ParsedClaimLink> {
    if payload.first().copied() != Some(CLAIM_LINK_VERSION_V3) {
        return Err(Error::Validation {
            kind: ValidationErrorKind::UnsupportedClaimLinkVersion,
        });
    }
    match payload.get(1).copied() {
        Some(0) => decode_direct(message, payload),
        Some(1) => decode_ephemeral(message, payload),
        _ => Err(Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        }),
    }
}

fn decode_direct(message: Option<String>, payload: &[u8]) -> Result<ParsedClaimLink> {
    let mut offset = 2;
    let note = read_compact_note(payload, &mut offset)?;
    if offset != payload.len() {
        return Err(Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        });
    }
    let commitment = element_to_b256(note.commitment());
    validate_received_note(note, commitment)?;
    Ok(ParsedClaimLink {
        message,
        claim_source_kind: ClaimSourceKind::Direct,
        direct_note: Some(DirectLinkedNote { note, commitment }),
        incoming_transfer: None,
    })
}

fn decode_ephemeral(message: Option<String>, payload: &[u8]) -> Result<ParsedClaimLink> {
    let mut offset = 2;
    let note = read_compact_note(payload, &mut offset)?;
    let commitment = element_to_b256(note.commitment());
    let ephemeral_private_key = read_word(payload, offset)?;
    offset += 32;
    if offset != payload.len() {
        return Err(Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        });
    }
    validate_received_note(note, commitment)?;
    if ephemeral_owner(&ephemeral_private_key)? != note.owner {
        return Err(Error::Validation {
            kind: ValidationErrorKind::EphemeralKeyMismatch,
        });
    }
    Ok(ParsedClaimLink {
        message,
        claim_source_kind: ClaimSourceKind::Ephemeral,
        direct_note: None,
        incoming_transfer: Some(IncomingTransfer {
            note,
            commitment,
            ephemeral_private_key,
            source_tx_hash: None,
            source_bridge_tx_hash: None,
        }),
    })
}

fn encode_payload(message: Option<&str>, payload: &[u8]) -> ClaimLink {
    let secret = bs58::encode(payload).into_string();
    let path = message.map_or_else(
        || "/s".to_owned(),
        |value| format!("/s/{}", percent_encode(value)),
    );
    ClaimLink {
        value: format!("{path}#{secret}"),
    }
}

fn parse_message(path: &str) -> Result<Option<String>> {
    if path == "/s" {
        return Ok(None);
    }
    let Some(message) = path.strip_prefix("/s/") else {
        return Err(Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        });
    };
    if message.contains('/') {
        return Err(Error::Validation {
            kind: ValidationErrorKind::InvalidClaimLink,
        });
    }
    if message.is_empty() {
        return Ok(None);
    }
    Ok(Some(percent_decode(message)?))
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                vec![char::from(byte)]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn percent_decode(value: &str) -> Result<String> {
    let mut out = Vec::with_capacity(value.len());
    let mut index = 0;
    let bytes = value.as_bytes();
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let Some(hex) = value.get(index + 1..index + 3) else {
                return Err(Error::Validation {
                    kind: ValidationErrorKind::InvalidClaimLink,
                });
            };
            let byte = u8::from_str_radix(hex, 16).map_err(|_| Error::Validation {
                kind: ValidationErrorKind::InvalidClaimLink,
            })?;
            out.push(byte);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).map_err(|_| Error::Validation {
        kind: ValidationErrorKind::InvalidClaimLink,
    })
}

fn element_to_b256(value: Element) -> [u8; 32] {
    value.to_be_bytes()
}
