use crate::client::Error;
// lint-long-file-override allow-max-lines=300
use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifier used to poll the Bungee status endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusIdentifier {
    /// Auto-route request hash returned from submit/build.
    RequestHash(String),
    /// Manual route source transaction hash.
    TxHash(String),
    /// Alternate identifier accepted by the public API.
    Id(String),
}

impl StatusIdentifier {
    /// Return the query parameter key expected by the public API.
    #[must_use]
    pub fn key(&self) -> &'static str {
        match self {
            Self::RequestHash(_) => "requestHash",
            Self::TxHash(_) => "txHash",
            Self::Id(_) => "id",
        }
    }

    /// Retrieve the underlying identifier value.
    #[must_use]
    pub fn value(&self) -> &str {
        match self {
            Self::RequestHash(value) | Self::TxHash(value) | Self::Id(value) => value,
        }
    }

    /// Convert into a [`GetStatusInput`].
    #[must_use]
    pub fn into_input(self) -> GetStatusInput {
        match self {
            Self::RequestHash(value) => GetStatusInput {
                request_hash: Some(value),
                ..GetStatusInput::default()
            },
            Self::TxHash(value) => GetStatusInput {
                tx_hash: Some(value),
                ..GetStatusInput::default()
            },
            Self::Id(value) => GetStatusInput {
                id: Some(value),
                ..GetStatusInput::default()
            },
        }
    }
}

/// Input payload for checking the status of a submitted bridge.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetStatusInput {
    /// Request hash returned by Bungee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_hash: Option<String>,
    /// Manual route source chain transaction hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    /// Alternate identifier accepted by the public API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl GetStatusInput {
    /// Create from a request hash.
    #[must_use]
    pub fn from_request_hash(request_hash: impl Into<String>) -> Self {
        Self {
            request_hash: Some(request_hash.into()),
            ..Self::default()
        }
    }

    /// Create from a transaction hash.
    #[must_use]
    pub fn from_tx_hash(tx_hash: impl Into<String>) -> Self {
        Self {
            tx_hash: Some(tx_hash.into()),
            ..Self::default()
        }
    }

    /// Create from an alternate identifier.
    #[must_use]
    pub fn from_id(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            ..Self::default()
        }
    }

    /// Resolve the identifier following Bungee's priority rules.
    pub fn identifier(&self) -> Result<StatusIdentifier, Error> {
        let pick = |value: &Option<String>, ctor: fn(String) -> StatusIdentifier| {
            value
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| ctor(s.to_owned()))
        };

        pick(&self.request_hash, StatusIdentifier::RequestHash)
            .or_else(|| pick(&self.tx_hash, StatusIdentifier::TxHash))
            .or_else(|| pick(&self.id, StatusIdentifier::Id))
            .ok_or(Error::MissingStatusIdentifier)
    }

    /// Build query pairs for the public API call.
    pub fn to_query_pairs(&self) -> Result<Vec<(String, String)>, Error> {
        let identifier = self.identifier()?;
        Ok(vec![(
            identifier.key().to_string(),
            identifier.value().to_string(),
        )])
    }
}

/// Status history returned by the Guild API.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct GetStatusOutput {
    /// Bungee status entries ordered with the most recent first.
    pub statuses: Vec<StatusEntry>,
}

impl GetStatusOutput {
    /// Return the most recent status entry.
    #[must_use]
    pub fn latest(&self) -> Option<&StatusEntry> {
        self.statuses.first()
    }
}

/// Individual status entry in the history.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct StatusEntry {
    /// Numeric status code.
    pub code: BungeeStatusCode,
    /// Optional status label provided by Bungee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Optional destination transaction hash once broadcast.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_tx_hash: Option<String>,
}

/// Enumeration of Bungee status codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BungeeStatusCode {
    /// Request submitted; waiting for solver assignment.
    Pending,
    /// Solver assigned and preparing execution.
    Assigned,
    /// Solver completed source-chain extraction.
    Extracted,
    /// Destination transaction broadcast and fulfilled.
    Fulfilled,
    /// Settlement completed on both chains.
    Settled,
    /// Request expired before completion.
    Expired,
    /// Request cancelled.
    Cancelled,
    /// Request refunded to the origin.
    Refunded,
    /// Unknown / forward-compatible status code.
    Unknown(u8),
}

impl BungeeStatusCode {
    /// Numeric representation used by Bungee.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Pending => 0,
            Self::Assigned => 1,
            Self::Extracted => 2,
            Self::Fulfilled => 3,
            Self::Settled => 4,
            Self::Expired => 5,
            Self::Cancelled => 6,
            Self::Refunded => 7,
            Self::Unknown(code) => code,
        }
    }

    /// Construct from the numeric representation.
    #[must_use]
    pub fn from_u8(code: u8) -> Self {
        match code {
            0 => Self::Pending,
            1 => Self::Assigned,
            2 => Self::Extracted,
            3 => Self::Fulfilled,
            4 => Self::Settled,
            5 => Self::Expired,
            6 => Self::Cancelled,
            7 => Self::Refunded,
            other => Self::Unknown(other),
        }
    }

    /// Human-readable label for the status code.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Assigned => "ASSIGNED",
            Self::Extracted => "EXTRACTED",
            Self::Fulfilled => "FULFILLED",
            Self::Settled => "SETTLED",
            Self::Expired => "EXPIRED",
            Self::Cancelled => "CANCELLED",
            Self::Refunded => "REFUNDED",
            Self::Unknown(_) => "UNKNOWN",
        }
    }
}

impl serde::Serialize for BungeeStatusCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(self.as_u8())
    }
}

impl<'de> serde::Deserialize<'de> for BungeeStatusCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = u8::deserialize(deserializer)?;
        Ok(Self::from_u8(code))
    }
}

impl fmt::Display for BungeeStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(code) => write!(f, "UNKNOWN({code})"),
            other => f.write_str(other.as_str()),
        }
    }
}
