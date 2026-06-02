// lint-long-file-override allow-max-lines=250
use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Sourcify v2 contract fields supported by Beam.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractField {
    /// ABI JSON array.
    Abi,
    /// Compilation summary.
    Compilation,
    /// Creation-bytecode match state.
    CreationMatch,
    /// Overall match state.
    ///
    /// Sourcify returns this field in contract responses, but does not accept it as a `fields`
    /// selector.
    Match,
    /// Metadata JSON.
    Metadata,
    /// Proxy resolution summary.
    ProxyResolution,
    /// Runtime-bytecode match state.
    RuntimeMatch,
    /// Solidity standard JSON input.
    StandardJsonInput,
    /// Source file map.
    Sources,
    /// Verification timestamp.
    VerifiedAt,
}

impl ContractField {
    /// Sourcify v2 response field name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Abi => "abi",
            Self::Compilation => "compilation",
            Self::CreationMatch => "creationMatch",
            Self::Match => "match",
            Self::Metadata => "metadata",
            Self::ProxyResolution => "proxyResolution",
            Self::RuntimeMatch => "runtimeMatch",
            Self::StandardJsonInput => "stdJsonInput",
            Self::Sources => "sources",
            Self::VerifiedAt => "verifiedAt",
        }
    }

    /// Sourcify v2 query field name, when this field is requestable.
    #[must_use]
    pub fn as_query_str(self) -> Option<&'static str> {
        match self {
            Self::Match => None,
            _ => Some(self.as_str()),
        }
    }
}

impl fmt::Display for ContractField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Sourcify match state for accepted non-null match values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchState {
    /// Runtime or creation bytecode matched exactly.
    ExactMatch,
    /// Runtime or creation bytecode matched after Sourcify transformations.
    Match,
}

impl MatchState {
    /// Whether this state is accepted as runtime verification.
    #[must_use]
    pub fn is_runtime_verified(self) -> bool {
        matches!(self, Self::ExactMatch | Self::Match)
    }

    /// Sourcify string representation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExactMatch => "exact_match",
            Self::Match => "match",
        }
    }
}

impl<'de> Deserialize<'de> for MatchState {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "exact_match" => Ok(Self::ExactMatch),
            "match" => Ok(Self::Match),
            other => Err(serde::de::Error::custom(format!(
                "unknown Sourcify match state `{other}`"
            ))),
        }
    }
}

impl fmt::Display for MatchState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Source file entry returned by Sourcify.
#[derive(Clone, Debug, Deserialize)]
pub struct SourceFile {
    /// Decoded UTF-8 source content.
    pub content: String,
}

/// Compiler and contract summary returned by Sourcify.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilationSummary {
    /// Compiler version or identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiler: Option<String>,
    /// Contract language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Contract name.
    #[serde(
        default,
        alias = "contractName",
        alias = "name",
        skip_serializing_if = "Option::is_none"
    )]
    pub contract_name: Option<String>,
}

/// Typed common Sourcify v2 contract record.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractRecord {
    /// Decimal chain id string from Sourcify.
    pub chain_id: String,
    /// Address string from Sourcify.
    pub address: String,
    /// Overall match state.
    #[serde(rename = "match")]
    pub match_state: MatchState,
    /// Creation match state.
    pub creation_match: Option<MatchState>,
    /// Runtime match state.
    pub runtime_match: Option<MatchState>,
    /// Verification timestamp.
    #[serde(default)]
    pub verified_at: Option<String>,
    /// ABI JSON array.
    #[serde(default, deserialize_with = "deserialize_optional_array")]
    pub abi: Option<Vec<Value>>,
    /// Source files keyed by Sourcify source path.
    #[serde(default)]
    pub sources: Option<BTreeMap<String, SourceFile>>,
    /// Metadata JSON object or JSON string.
    #[serde(default)]
    pub metadata: Option<Value>,
    /// Solidity standard JSON input.
    #[serde(default, rename = "stdJsonInput")]
    pub standard_json_input: Option<Value>,
    /// Compilation summary.
    #[serde(default)]
    pub compilation: Option<CompilationSummary>,
    /// Raw proxy resolution object.
    #[serde(default)]
    pub proxy_resolution: Option<Value>,
}

impl ContractRecord {
    /// Validates that Sourcify echoed the requested target.
    pub fn validate_target(&self, chain_id: u64, address: &str) -> Result<(), String> {
        let response_chain_id = self
            .chain_id
            .parse::<u64>()
            .map_err(|_| "chainId is not a decimal string".to_owned())?;
        if response_chain_id != chain_id {
            return Err(format!(
                "chainId mismatch: expected {chain_id}, got {response_chain_id}"
            ));
        }

        if !self.address.eq_ignore_ascii_case(address) {
            return Err(format!(
                "address mismatch: expected {address}, got {}",
                self.address
            ));
        }

        Ok(())
    }
}

/// Response metadata and typed contract record.
#[derive(Clone, Debug)]
pub struct ContractResponse {
    /// Final URL used for the request.
    pub endpoint: String,
    /// Sourcify fields requested.
    pub requested_fields: Vec<String>,
    /// Contract record.
    pub contract: ContractRecord,
}

fn deserialize_optional_array<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<Value>>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(value) = Option::<Value>::deserialize(deserializer)? else {
        return Ok(None);
    };

    match value {
        Value::Array(values) => Ok(Some(values)),
        _ => Ok(None),
    }
}
