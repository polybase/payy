use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, SeqAccess, Visitor},
};
use strum::Display;

use super::{Error, Result};
#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Hash, Display)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IDKind {
    Passport,
    NationalId,
    VoterId,
    ResidentCard,
    ResidentCardTemp,
    TemporaryProtectionPermit,
    DriversLicense,
    StateId,
    Selfie,
    SelfieLeft,
    SelfieRight,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum IDDocumentField {
    // for older apps using document bytes
    Bytes(Vec<u8>),
    // for newer apps using document ids
    Id { id: String, bytes: Vec<u8> },
}

impl IDDocumentField {
    pub fn get_bytes(&self) -> &[u8] {
        match self {
            Self::Bytes(bytes) => bytes,
            Self::Id { bytes, .. } => bytes,
        }
    }
}

pub fn deserialize_document_field_opt<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<IDDocumentField>, D::Error>
where
    D: Deserializer<'de>,
{
    struct FieldVisitor;

    impl<'de> Visitor<'de> for FieldVisitor {
        type Value = Option<IDDocumentField>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a base64 string, a string with ';', or a byte array")
        }

        fn visit_none<E>(self) -> std::result::Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> std::result::Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
        where
            E: DeError,
        {
            if value.contains(';') {
                Ok(Some(IDDocumentField::Id {
                    id: value.to_string(),
                    bytes: Vec::new(),
                }))
            } else {
                BASE64
                    .decode(value)
                    .map(IDDocumentField::Bytes)
                    .map(Some)
                    .map_err(DeError::custom)
            }
        }

        fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
        where
            E: DeError,
        {
            self.visit_str(&value)
        }

        fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut bytes = Vec::new();
            while let Some(byte) = seq.next_element::<u8>()? {
                bytes.push(byte);
            }
            Ok(Some(IDDocumentField::Bytes(bytes)))
        }
    }

    deserializer.deserialize_any(FieldVisitor)
}

pub fn serialize_document_field_opt<S>(
    value: &Option<IDDocumentField>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(IDDocumentField::Bytes(data)) => {
            let encoded = BASE64.encode(data);
            serializer.serialize_some(&encoded)
        }
        // serialise only the id
        Some(IDDocumentField::Id { id, .. }) => serializer.serialize_some(id),
        None => serializer.serialize_none(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IDDocument {
    #[serde(
        serialize_with = "serialize_document_field_opt",
        deserialize_with = "deserialize_document_field_opt"
    )]
    pub front: Option<IDDocumentField>,
    #[serde(
        default,
        serialize_with = "serialize_document_field_opt",
        deserialize_with = "deserialize_document_field_opt"
    )]
    pub back: Option<IDDocumentField>,
}

impl IDDocument {
    pub fn get_front(&self) -> Result<Vec<u8>> {
        let front = self
            .front
            .clone()
            .ok_or(Error::MissingKYCField("document_front".to_string()))?;

        Ok(match front {
            IDDocumentField::Bytes(bytes) => bytes,
            IDDocumentField::Id { bytes, .. } => bytes,
        })
    }

    pub fn get_back(&self) -> Result<Vec<u8>> {
        let back = self
            .back
            .clone()
            .ok_or(Error::MissingKYCField("document_back".to_string()))?;

        Ok(match back {
            IDDocumentField::Bytes(bytes) => bytes,
            IDDocumentField::Id { bytes, .. } => bytes,
        })
    }
}
