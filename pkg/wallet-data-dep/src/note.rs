// lint-long-file-override allow-max-lines=400
use element::Element;
use hash_poseidon::hash_merge;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;
use wallet_primitives::derive_private_key;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(rename = "WalletDataNote"))]
pub struct Note {
    pub address: Element,
    pub psi: Element,
    pub value: Element,
    #[serde(default)]
    pub source: Option<Element>,
    #[serde(default)]
    pub token: Option<String>,
}

impl Note {
    pub fn commitment(&self) -> Element {
        if self.value == Element::ZERO {
            return Element::ZERO;
        }

        hash_merge([
            self.value,
            self.address,
            self.psi,
            self.address,
            Element::ONE,
            Element::ONE,
        ])
    }
}

impl From<Note> for zk_primitives::Note {
    fn from(note: Note) -> Self {
        Self {
            kind: Element::new(2),
            contract: Element::new(1),
            value: note.value,
            psi: note.psi,
            address: note.address,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct StoredNote {
    pub note: Note,
    pub commitment: Element,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(type = "number"), ts(optional))]
    pub timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(type = "number"), ts(optional))]
    pub received: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(type = "number"), ts(optional))]
    pub spent: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub owner: Option<Element>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(type = "boolean"), ts(optional))]
    pub remote: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub private_key: Option<Element>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-rs", ts(optional))]
    pub invalidreason: Option<String>,
}

impl StoredNote {
    pub fn claimable_from_private_key(value: Element, private_key: Element) -> Self {
        let psi = derive_private_key(&[], private_key);
        Self::claimable_from_private_key_psi(value, private_key, psi)
    }

    pub fn claimable_from_private_key_psi(
        value: Element,
        private_key: Element,
        psi: Element,
    ) -> Self {
        let note = Note {
            value,
            address: hash_merge([private_key, Element::ZERO]),
            psi,
            source: None,
            token: None,
        };
        StoredNote {
            commitment: note.commitment(),
            note,
            private_key: Some(private_key),
            owner: None,
            timestamp: None,
            received: None,
            spent: None,
            remote: None,
            invalidreason: None,
        }
    }

    pub fn get_private_key(&self, wallet_private_key: Element) -> Option<Element> {
        // We have an explicit private key, try to use that
        if let Some(pk) = self.private_key {
            let addr = hash_merge([pk, Element::ZERO]);
            if self.note.address == addr {
                return Some(pk);
            }
        }

        // Next try using a derived private key using the psi
        let psi_pk = derive_private_key(
            format!("0x{}", self.note.psi.to_hex()).as_bytes(),
            wallet_private_key,
        );
        let psi_addr = hash_merge([psi_pk, Element::ZERO]);
        if self.note.address == psi_addr {
            return Some(psi_pk);
        }

        // Otherwise check if the raw wallet pk is used
        let wallet_addr = hash_merge([wallet_private_key, Element::ZERO]);
        if self.note.address == wallet_addr {
            return Some(wallet_private_key);
        }

        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct MerklePath {
    pub siblings: Vec<Element>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct ClaimableNote {
    pub note: StoredNote,
    pub secret_key: Element,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct InputNote {
    pub note: Note,
    pub secret_key: Element,
    pub merkle_path: MerklePath,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct SnarkWitness {
    #[serde(rename = "V1")]
    pub v1: SnarkWitnessV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct SnarkWitnessV1 {
    pub proof: String,
    pub instances: Vec<Vec<Element>>,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use element::Element;

    #[test]
    fn test_get_private_key() {
        let wallet_private_key =
            Element::from_str("0x39c8c2ca6c49019ab3f8670968008e54678d41ebf3254ab5773f2b772a31aa69")
                .unwrap();
        let psi =
            Element::from_str("0x0b8ffa9a1a747c202f8291bb8c1ec7aceedb010da1ca7fa788ed77d036bc2e5d")
                .unwrap();
        let derived_address =
            Element::from_str("1603fb25f420f4cfdc81a9962e2bdc6b2da279498f02392329bcb48b6070b8cf")
                .unwrap();
        let derived_private_key =
            Element::from_str("0x267537a04de9d21de3e6bdb61f0d2d0e0862891ce4e6bcc2c9fd6c7de60b9aa7")
                .unwrap();

        let note = Note {
            address: derived_address,
            psi,
            value: Element::new(100),
            source: None,
            token: None,
        };

        let stored_note = StoredNote {
            note,
            commitment: Element::new(0),
            timestamp: None,
            received: None,
            spent: None,
            owner: None,
            remote: None,
            private_key: None,
            invalidreason: None,
        };

        let result = stored_note.get_private_key(wallet_private_key);
        assert_eq!(result, Some(derived_private_key));
    }

    #[test]
    fn test_claimable_from_private_key() {
        let private_key =
            Element::from_str("0xade2737d8245c6e45e906cdeeb127aa066187e98822f6eed31098bac13d68936")
                .unwrap();
        let expected_psi =
            Element::from_str("0x82a54bb547a18cdf3036ba399f01bd8f99136d8f05417cf503b4f4e5b6ac265a")
                .unwrap();
        let value = Element::new(100);

        let stored_note = StoredNote::claimable_from_private_key(value, private_key);

        assert_eq!(stored_note.note.psi, expected_psi);
        assert_eq!(stored_note.note.value, value);
        assert_eq!(stored_note.private_key, Some(private_key));
        assert_eq!(
            stored_note.note.address,
            hash_merge([private_key, Element::ZERO])
        );
    }

    #[test]
    fn test_note_deserialize_missing_source_field() {
        // Test JSON without source field (old format)
        let json_without_source = r#"{
            "address": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "psi": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            "value": "0000000000000000000000000000000000000000000000000000000000000064"
        }"#;

        let note: Result<Note, _> = serde_json::from_str(json_without_source);
        assert!(
            note.is_ok(),
            "Failed to deserialize note without source field"
        );

        let note = note.unwrap();
        assert_eq!(note.source, None);
        assert_eq!(note.token, None);
    }

    #[test]
    fn test_note_deserialize_missing_token_field() {
        // Test JSON without token field (old format)
        let json_without_token = r#"{
            "address": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "psi": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            "value": "0000000000000000000000000000000000000000000000000000000000000064",
            "source": "1111111111111111111111111111111111111111111111111111111111111111"
        }"#;

        let note: Result<Note, _> = serde_json::from_str(json_without_token);
        assert!(
            note.is_ok(),
            "Failed to deserialize note without token field"
        );

        let note = note.unwrap();
        assert_eq!(note.token, None);
        assert!(note.source.is_some());
    }

    #[test]
    fn test_note_deserialize_missing_both_fields() {
        // Test JSON without both source and token fields (very old format)
        let json_without_both = r#"{
            "address": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "psi": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            "value": "0000000000000000000000000000000000000000000000000000000000000064"
        }"#;

        let note: Result<Note, _> = serde_json::from_str(json_without_both);
        assert!(
            note.is_ok(),
            "Failed to deserialize note without source and token fields"
        );

        let note = note.unwrap();
        assert_eq!(note.source, None);
        assert_eq!(note.token, None);
    }

    #[test]
    fn test_note_serialize_deserialize_with_all_fields() {
        // Test complete note with all fields present
        let original_note = Note {
            address: Element::from_str(
                "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            )
            .unwrap(),
            psi: Element::from_str(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            )
            .unwrap(),
            value: Element::new(100),
            source: Some(
                Element::from_str(
                    "1111111111111111111111111111111111111111111111111111111111111111",
                )
                .unwrap(),
            ),
            token: Some("USDC".to_string()),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&original_note).unwrap();

        // Deserialize back
        let deserialized_note: Note = serde_json::from_str(&json).unwrap();

        // Verify all fields match
        assert_eq!(original_note.address, deserialized_note.address);
        assert_eq!(original_note.psi, deserialized_note.psi);
        assert_eq!(original_note.value, deserialized_note.value);
        assert_eq!(original_note.source, deserialized_note.source);
        assert_eq!(original_note.token, deserialized_note.token);
    }

    #[test]
    fn test_stored_note_deserialize_missing_fields() {
        // Test StoredNote with note missing optional fields
        let json_stored_note = r#"{
            "note": {
                "address": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                "psi": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                "value": "0000000000000000000000000000000000000000000000000000000000000064"
            },
            "commitment": "2222222222222222222222222222222222222222222222222222222222222222"
        }"#;

        let stored_note: Result<StoredNote, _> = serde_json::from_str(json_stored_note);
        assert!(
            stored_note.is_ok(),
            "Failed to deserialize StoredNote with note missing optional fields"
        );

        let stored_note = stored_note.unwrap();
        assert_eq!(stored_note.note.source, None);
        assert_eq!(stored_note.note.token, None);
    }
}
