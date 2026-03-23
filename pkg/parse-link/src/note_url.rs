// lint-long-file-override allow-max-lines=400
use element::Element;
use hash::hash_merge;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::str::FromStr;
use thiserror::Error;
#[cfg(feature = "ts-rs")]
use ts_rs::TS;
use web3::{signing::keccak256, types::H160};

const NOTE_KIND_TRAILER_MARKER: u8 = 0xff;

/// Errors returned when decoding activity URL payloads.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NoteUrlDecodeError {
    /// The payload could not be decoded from base58.
    #[error("[parse-link/note-url] failed to decode base58 payload: {0}")]
    Base58(String),
    /// The payload is too short to contain the required metadata.
    #[error("[parse-link/note-url] payload too short")]
    TooShort,
    /// The note URL version embedded in the payload is not supported.
    #[error("[parse-link/note-url] unsupported version {0}")]
    UnsupportedVersion(u8),
    /// The payload contains an invalid UTF-8 referral code.
    #[error("[parse-link/note-url] invalid referral code encoding")]
    InvalidReferralCode,
}

/// Result alias for note URL decoding operations.
pub type NoteUrlDecodeResult<T> = std::result::Result<T, NoteUrlDecodeError>;

/// NoteURLPayload is a struct that contains the data required to create a note URL.
///
/// These are used to send payments to a user e.g. `https://payy.link/s#<NoteURLPayload>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
pub struct NoteURLPayload {
    /// The version of the note URL payload.
    /// 0 -> old rollup note
    /// 1 -> old rollup note (derived psi)
    /// 2 -> new rollup (derived psi)
    pub version: u8,
    /// The private key of the note
    pub private_key: Element,
    /// The psi of the note, this is only kept for older note urls
    /// which included the psi
    pub psi: Option<Element>,
    /// The value of the note
    pub value: Element,
    /// The note kind of the note.
    ///
    /// When omitted the payload defaults to Polygon bridged USDC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_kind: Option<Element>,
    /// The referral code of the note
    pub referral_code: String,
}

impl NoteURLPayload {
    /// Gets the explicit or derived psi for the note url.
    #[must_use]
    pub fn psi(&self) -> Element {
        match self.version {
            0 => self
                .psi
                .expect("version 0 payloads must include explicit psi"),
            1 => {
                let private_key_bytes = self.private_key.to_be_bytes();
                Element::from_str(&hex::encode(keccak256(&private_key_bytes))).unwrap()
            }
            2 => hash_private_key_for_psi(self.private_key),
            version => panic!("unsupported note url version {version}"),
        }
    }

    /// Get the explicit or default note kind for the payload.
    #[must_use]
    pub fn note_kind(&self) -> Element {
        self.note_kind
            .unwrap_or_else(bridged_polygon_usdc_note_kind)
    }

    /// Derive the address for the payload's private key.
    #[must_use]
    pub fn address(&self) -> Element {
        get_address_for_private_key(self.private_key)
    }

    /// Compute the note commitment corresponding to this payload.
    #[must_use]
    pub fn commitment(&self) -> Element {
        if self.value == Element::ZERO {
            Element::ZERO
        } else {
            hash_merge([
                Element::new(2),
                self.note_kind(),
                self.value,
                self.address(),
                self.psi(),
                Element::ZERO,
                Element::ZERO,
            ])
        }
    }

    /// Encode a note URL payload to a base58-encoded string.
    #[must_use]
    pub fn encode_activity_url_payload(&self) -> String {
        let mut bytes = Vec::new();

        // Encode version
        bytes.push(self.version);

        // Encode private_key
        bytes.extend_from_slice(&self.private_key.to_be_bytes());

        // Encode psi if version is 0
        if let (0, Some(psi)) = (self.version, &self.psi) {
            bytes.extend_from_slice(&psi.to_be_bytes());
        }

        // Encode value with leading zeros
        let value_bytes = self.value.to_be_bytes();
        let leading_zeros = value_bytes.iter().take_while(|&&b| b == 0).count();
        #[allow(clippy::cast_possible_truncation)]
        bytes.push(leading_zeros as u8);
        bytes.extend_from_slice(&value_bytes[leading_zeros..]);

        // Encode referral_code as UTF-8
        bytes.extend_from_slice(self.referral_code.as_bytes());

        if let Some(note_kind) = self.encoded_note_kind() {
            bytes.push(NOTE_KIND_TRAILER_MARKER);
            bytes.extend_from_slice(&note_kind.to_be_bytes());
        }

        bs58::encode(bytes).into_string()
    }

    fn encoded_note_kind(&self) -> Option<Element> {
        let note_kind = self.note_kind();

        (note_kind != bridged_polygon_usdc_note_kind()).then_some(note_kind)
    }
}

/// Decode a note URL payload from a base58-encoded string.
#[must_use]
pub fn decode_activity_url_payload(payload: &str) -> NoteURLPayload {
    try_decode_activity_url_payload(payload)
        .unwrap_or_else(|err| panic!("failed to decode note url payload: {err}"))
}

/// Attempt to decode a note URL payload from a base58-encoded string returning an error on failure.
pub fn try_decode_activity_url_payload(payload: &str) -> NoteUrlDecodeResult<NoteURLPayload> {
    let payload_bytes = bs58::decode(payload)
        .into_vec()
        .map_err(|err| NoteUrlDecodeError::Base58(err.to_string()))?;

    if payload_bytes.len() < 33 {
        return Err(NoteUrlDecodeError::TooShort);
    }

    let mut rest = &payload_bytes[..];

    let version = rest[0];
    rest = &rest[1..];

    if rest.len() < 32 {
        return Err(NoteUrlDecodeError::TooShort);
    }

    let private_key_bytes: [u8; 32] = rest[..32]
        .try_into()
        .expect("length already validated above");
    let private_key = Element::from_be_bytes(private_key_bytes);
    rest = &rest[32..];

    let psi = match version {
        0 => {
            if rest.len() < 32 {
                return Err(NoteUrlDecodeError::TooShort);
            }
            let psi_bytes: [u8; 32] = rest[..32]
                .try_into()
                .expect("length already validated above");
            rest = &rest[32..];
            Some(Element::from_be_bytes(psi_bytes))
        }
        1 => Some(Element::from_str(&hex::encode(keccak256(&private_key_bytes))).unwrap()),
        2 => Some(hash_private_key_for_psi(private_key)),
        unsupported => return Err(NoteUrlDecodeError::UnsupportedVersion(unsupported)),
    };

    if rest.is_empty() {
        return Err(NoteUrlDecodeError::TooShort);
    }

    let leading_zeros = rest[0] as usize;
    rest = &rest[1..];

    if leading_zeros > 32 || rest.len() < (32 - leading_zeros) {
        return Err(NoteUrlDecodeError::TooShort);
    }

    let value_len = 32 - leading_zeros;
    let value_without_leading_zeros = &rest[..value_len];
    rest = &rest[value_len..];

    let mut value_bytes = [0u8; 32];
    value_bytes[leading_zeros..].copy_from_slice(value_without_leading_zeros);
    let value = Element::from_be_bytes(value_bytes);

    let (referral_code_bytes, note_kind) = split_note_kind_trailer(rest);
    let referral_code = String::from_utf8(referral_code_bytes.to_vec())
        .map_err(|_| NoteUrlDecodeError::InvalidReferralCode)?;

    Ok(NoteURLPayload {
        version,
        private_key,
        psi,
        value,
        note_kind,
        referral_code,
    })
}

fn split_note_kind_trailer(rest: &[u8]) -> (&[u8], Option<Element>) {
    if rest.len() < 33 || rest[rest.len() - 33] != NOTE_KIND_TRAILER_MARKER {
        return (rest, None);
    }

    let note_kind = Element::from_be_bytes(
        rest[rest.len() - 32..]
            .try_into()
            .expect("note kind trailer length already validated"),
    );

    (&rest[..rest.len() - 33], normalize_note_kind(note_kind))
}

fn normalize_note_kind(note_kind: Element) -> Option<Element> {
    (note_kind != bridged_polygon_usdc_note_kind()).then_some(note_kind)
}

fn hash_private_key_for_psi(private_key: Element) -> Element {
    let mut hasher = Sha3_256::new();
    hasher.update(private_key.to_be_bytes());
    let result = hasher.finalize();

    Element::from_be_bytes(result.into())
}

fn get_address_for_private_key(private_key: Element) -> Element {
    hash_merge([private_key, Element::ZERO])
}

fn bridged_polygon_usdc_note_kind() -> Element {
    let chain = 137u64;
    let address =
        H160::from_slice(&hex::decode("3c499c542cef5e3811e1192ce70d8cc03d5c3359").unwrap());
    generate_note_kind_bridge_evm(chain, address)
}

fn generate_note_kind_bridge_evm(chain: u64, address: H160) -> Element {
    let mut bytes = [0u8; 32];

    bytes[0..2].copy_from_slice(&(2u16).to_be_bytes());
    bytes[2..10].copy_from_slice(&chain.to_be_bytes());
    bytes[10..30].copy_from_slice(address.as_bytes());

    Element::from_be_bytes(bytes)
}

#[cfg(test)]
mod tests;
