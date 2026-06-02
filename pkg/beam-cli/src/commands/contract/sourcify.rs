// lint-long-file-override allow-max-lines=250
use serde_json::{Map, Value, json};
use sourcify_interface::{
    CompilationSummary, ContractField, ContractLookup, ContractRecord, ContractResponse,
    Error as SourcifyError, MatchState, SourcifyClient,
};

use crate::human_output::sanitize_control_chars;

use super::{
    error::{Error, Result},
    target::InspectionTarget,
};

#[derive(Clone, Copy, Debug)]
pub(super) enum Artifact {
    Abi,
    Source,
    Export,
}

pub(super) const INFO_CAP_BYTES: usize = 10 * 1024 * 1024;
pub(super) const ABI_CAP_BYTES: usize = 10 * 1024 * 1024;
pub(super) const SOURCE_CAP_BYTES: usize = 100 * 1024 * 1024;
pub(super) const EXPORT_CAP_BYTES: usize = 100 * 1024 * 1024;

pub(super) fn info_fields() -> Vec<ContractField> {
    vec![
        ContractField::CreationMatch,
        ContractField::RuntimeMatch,
        ContractField::VerifiedAt,
        ContractField::Compilation,
        ContractField::ProxyResolution,
    ]
}

pub(super) fn abi_fields() -> Vec<ContractField> {
    vec![
        ContractField::Abi,
        ContractField::ProxyResolution,
        ContractField::Compilation,
        ContractField::CreationMatch,
        ContractField::RuntimeMatch,
        ContractField::VerifiedAt,
    ]
}

pub(super) fn source_fields() -> Vec<ContractField> {
    vec![
        ContractField::Sources,
        ContractField::Metadata,
        ContractField::Compilation,
        ContractField::ProxyResolution,
        ContractField::CreationMatch,
        ContractField::RuntimeMatch,
        ContractField::VerifiedAt,
    ]
}

pub(super) fn export_fields() -> Vec<ContractField> {
    vec![
        ContractField::Abi,
        ContractField::Sources,
        ContractField::Metadata,
        ContractField::StandardJsonInput,
        ContractField::Compilation,
        ContractField::ProxyResolution,
        ContractField::CreationMatch,
        ContractField::RuntimeMatch,
        ContractField::VerifiedAt,
    ]
}

pub(super) async fn lookup_contract(
    client: &dyn SourcifyClient,
    target: &InspectionTarget,
    fields: Vec<ContractField>,
    cap_bytes: usize,
) -> Result<ContractResponse> {
    client
        .contract(&ContractLookup {
            chain_id: target.chain_id,
            address: target.checksum_address.clone(),
            fields,
            response_cap_bytes: cap_bytes,
        })
        .await
        .map_err(|err| map_sourcify_error(err, target, cap_bytes))
}

pub(super) fn sourcify_not_checked_value() -> Value {
    json!({
        "status": "not_checked",
        "checked": false,
        "verified": false,
    })
}

pub(super) fn sourcify_record_value(record: &ContractRecord) -> Value {
    let mut object = Map::new();
    object.insert("status".to_owned(), json!("runtime_verified"));
    object.insert("checked".to_owned(), json!(true));
    object.insert("verified".to_owned(), json!(true));
    object.insert("match".to_owned(), json!(record.match_state.as_str()));
    object.insert(
        "creation_match".to_owned(),
        match record.creation_match {
            Some(value) => json!(value.as_str()),
            None => Value::Null,
        },
    );
    object.insert(
        "runtime_match".to_owned(),
        match record.runtime_match {
            Some(value) => json!(value.as_str()),
            None => Value::Null,
        },
    );

    insert_optional_string(&mut object, "verified_at", record.verified_at.as_ref());
    if let Some(compilation) = record.compilation.as_ref() {
        insert_compilation(&mut object, compilation);
    }

    Value::Object(object)
}

pub(super) fn sourcify_status_value(status: &str, error: Option<&str>) -> Value {
    let mut object = Map::new();
    object.insert("status".to_owned(), json!(status));
    object.insert("checked".to_owned(), json!(true));
    object.insert("verified".to_owned(), json!(false));
    if let Some(error) = error {
        object.insert("error".to_owned(), json!(error));
    }

    Value::Object(object)
}

pub(super) fn artifact_label(artifact: Artifact) -> &'static str {
    match artifact {
        Artifact::Abi => "ABI",
        Artifact::Source => "Source",
        Artifact::Export => "Source bundle",
    }
}

pub(super) fn match_summary(record: &ContractRecord) -> String {
    format!(
        "runtime {}, creation {}",
        match_label(record.runtime_match),
        match_label(record.creation_match),
    )
}

pub(super) fn compilation_lines(record: &ContractRecord) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(compilation) = record.compilation.as_ref() {
        if let Some(contract_name) = compilation.contract_name.as_ref() {
            lines.push(format!(
                "Contract: {}",
                sanitize_control_chars(contract_name)
            ));
        }
        if let Some(language) = compilation.language.as_ref() {
            lines.push(format!("Language: {}", sanitize_control_chars(language)));
        }
        if let Some(compiler) = compilation.compiler.as_ref() {
            lines.push(format!("Compiler: {}", sanitize_control_chars(compiler)));
        }
    }
    if let Some(verified_at) = record.verified_at.as_ref() {
        lines.push(format!(
            "Verified at: {}",
            sanitize_control_chars(verified_at)
        ));
    }

    lines
}

fn map_sourcify_error(err: SourcifyError, target: &InspectionTarget, cap_bytes: usize) -> Error {
    match err {
        SourcifyError::NotVerified => Error::SourcifyNotVerified {
            address: target.checksum_address.clone(),
            artifact: "artifact".to_owned(),
            runtime_unchecked: None,
        },
        SourcifyError::ChainUnsupported { chain_id } => {
            Error::SourcifyChainUnsupported { chain_id }
        }
        SourcifyError::LookupFailed { reason } => Error::SourcifyLookupFailed {
            address: target.checksum_address.clone(),
            reason,
        },
        SourcifyError::ResponseTooLarge { .. } => Error::SourcifyResponseTooLarge { cap_bytes },
        SourcifyError::MalformedResponse { reason } => Error::SourcifyMalformedResponse { reason },
        SourcifyError::Internal(internal) => Error::SourcifyLookupFailed {
            address: target.checksum_address.clone(),
            reason: internal.to_string(),
        },
    }
}

fn insert_compilation(object: &mut Map<String, Value>, compilation: &CompilationSummary) {
    insert_optional_string(object, "contract_name", compilation.contract_name.as_ref());
    insert_optional_string(object, "language", compilation.language.as_ref());
    insert_optional_string(object, "compiler", compilation.compiler.as_ref());
}

fn insert_optional_string(object: &mut Map<String, Value>, key: &str, value: Option<&String>) {
    if let Some(value) = value {
        object.insert(key.to_owned(), json!(value));
    }
}

fn match_label(value: Option<MatchState>) -> &'static str {
    match value {
        Some(MatchState::ExactMatch) => "exact match",
        Some(MatchState::Match) => "match",
        None => "not verified",
    }
}
