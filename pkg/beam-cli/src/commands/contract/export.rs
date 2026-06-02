// lint-long-file-override allow-max-lines=300
pub(super) mod fs_ops;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use sourcify_interface::ContractResponse;

use super::{
    error::{Error, Result},
    source::require_sources,
    target::InspectionTarget,
};
use fs_ops::{
    commit_into_existing, prepare_temp_dir, validate_destination, write_failed, write_files,
};

#[cfg(test)]
pub(super) use fs_ops::commit_into_existing_for_test;

const HASH_SUFFIX_HEX_LEN: usize = 16;
const MAX_SOURCE_FILENAME_BYTES: usize = 240;

#[derive(Clone, Debug)]
pub(super) struct ExportResult {
    pub(super) destination: String,
    pub(super) written_files: Vec<String>,
}

#[derive(Clone, Debug)]
struct ExportFiles {
    files: BTreeMap<String, Vec<u8>>,
}

#[derive(Clone, Debug)]
struct SourceFileManifest {
    path: String,
    sha256: String,
}

pub(super) fn export_bundle(
    target: &InspectionTarget,
    response: &ContractResponse,
    destination: &str,
) -> Result<ExportResult> {
    let destination_path = PathBuf::from(destination);
    validate_destination(&destination_path, destination)?;

    let export_files = build_export_files(target, response)?;
    let temp_dir = prepare_temp_dir(&destination_path)?;
    write_files(temp_dir.path(), &export_files.files)?;

    if destination_path.exists() {
        commit_into_existing(temp_dir.path(), &destination_path, destination)?;
    } else {
        fs::rename(temp_dir.path(), &destination_path).map_err(write_failed)?;
    }

    let mut written_files = export_files.files.keys().cloned().collect::<Vec<_>>();
    written_files.sort();

    Ok(ExportResult {
        destination: destination.to_owned(),
        written_files,
    })
}

fn build_export_files(
    target: &InspectionTarget,
    response: &ContractResponse,
) -> Result<ExportFiles> {
    let sources = require_sources(response.contract.sources.as_ref())?;
    let mut source_filenames = BTreeSet::new();
    let mut files = BTreeMap::new();
    let mut source_files = BTreeMap::new();

    if let Some(abi) = response.contract.abi.as_ref() {
        insert_json_file(&mut files, "abi.json", &Value::Array(abi.clone()))?;
    }
    if let Some(metadata) = response.contract.metadata.as_ref() {
        insert_value_file(&mut files, "metadata.json", metadata)?;
    }
    if let Some(input) = response.contract.standard_json_input.as_ref() {
        insert_value_file(&mut files, "standard-json-input.json", input)?;
    }

    for (source_path, source) in sources {
        let output_path = flatten_source_key(source_path);
        if !source_filenames.insert(output_path.clone()) {
            return Err(Error::ExportPathCollision { path: output_path });
        }
        let relative_path = format!("sources/{output_path}");
        let bytes = source.content.as_bytes().to_vec();
        let sha256 = sha256_hex(&bytes);
        files.insert(relative_path.clone(), bytes);
        source_files.insert(
            source_path.to_owned(),
            SourceFileManifest {
                path: relative_path,
                sha256,
            },
        );
    }

    let manifest = build_manifest(target, response, &files, &source_files);
    insert_json_file(&mut files, "sourcify.json", &manifest)?;

    Ok(ExportFiles { files })
}

fn build_manifest(
    target: &InspectionTarget,
    response: &ContractResponse,
    files: &BTreeMap<String, Vec<u8>>,
    source_files: &BTreeMap<String, SourceFileManifest>,
) -> Value {
    let mut hashes = Map::new();
    for (path, bytes) in files {
        hashes.insert(path.clone(), json!(sha256_hex(bytes)));
    }

    let mut source_map = Map::new();
    for (source_key, source_file) in source_files {
        source_map.insert(
            source_key.clone(),
            json!({
                "path": source_file.path.clone(),
                "sha256": source_file.sha256.clone(),
            }),
        );
    }

    json!({
        "endpoint": response.endpoint.clone(),
        "requested_fields": response.requested_fields.clone(),
        "chain_id": target.chain_id,
        "address": target.checksum_address.clone(),
        "match": response.contract.match_state.as_str(),
        "creation_match": response.contract.creation_match.map(|value| value.as_str()),
        "runtime_match": response.contract.runtime_match.map(|value| value.as_str()),
        "verified_at": response.contract.verified_at.clone(),
        "compilation": response.contract.compilation.clone(),
        "files": hashes,
        "source_files": source_map,
    })
}

pub(super) fn flatten_source_key(source_key: &str) -> String {
    flatten_source_key_with_hash(source_key, &source_key_hash16(source_key))
}

#[cfg(test)]
pub(super) fn flatten_source_keys_for_test<'a>(
    paths: impl Iterator<Item = &'a String>,
    hash: &str,
) -> Result<BTreeMap<String, String>> {
    let mut output = BTreeMap::new();
    let mut exact = BTreeSet::new();

    for path in paths {
        let flattened = flatten_source_key_with_hash(path, hash);
        if !exact.insert(flattened.clone()) {
            return Err(Error::ExportPathCollision { path: flattened });
        }
        output.insert(path.clone(), flattened);
    }

    Ok(output)
}

fn flatten_source_key_with_hash(source_key: &str, hash: &str) -> String {
    let sanitized = sanitized_source_label(source_key);
    let (stem, extension) = split_final_extension(&sanitized);
    let extension = if 2 + hash.len() + extension.len() <= MAX_SOURCE_FILENAME_BYTES {
        extension
    } else {
        ""
    };
    let suffix_len = 2 + hash.len() + extension.len();
    let max_stem_len = MAX_SOURCE_FILENAME_BYTES.saturating_sub(suffix_len);

    let mut stem = stem.to_owned();
    stem.truncate(max_stem_len);
    format!("{stem}--{hash}{extension}")
}

fn sanitized_source_label(source_key: &str) -> String {
    let mut output = String::new();
    let mut previous_underscore = false;

    for ch in source_key.chars() {
        let mapped = if is_allowed_source_label_char(ch) {
            ch
        } else {
            '_'
        };
        if mapped == '_' {
            if !previous_underscore {
                output.push('_');
                previous_underscore = true;
            }
        } else {
            output.push(mapped);
            previous_underscore = false;
        }
    }

    let output = output.trim_matches('_');
    if output.is_empty() {
        "source".to_owned()
    } else {
        output.to_owned()
    }
}

fn split_final_extension(filename: &str) -> (&str, &str) {
    let Some(index) = filename.rfind('.') else {
        return (filename, "");
    };
    if index == 0 || index + 1 == filename.len() {
        return (filename, "");
    }

    (&filename[..index], &filename[index..])
}

fn source_key_hash16(source_key: &str) -> String {
    sha256_hex(source_key.as_bytes())[..HASH_SUFFIX_HEX_LEN].to_owned()
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn is_allowed_source_label_char(ch: char) -> bool {
    matches!(
        ch,
        'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | '-' | '@' | '+'
    )
}

fn insert_json_file(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    value: &Value,
) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|err| Error::ExportWriteFailed {
        reason: err.to_string(),
    })?;
    bytes.push(b'\n');
    files.insert(path.to_owned(), bytes);

    Ok(())
}

fn insert_value_file(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    value: &Value,
) -> Result<()> {
    match value {
        Value::String(value) => {
            files.insert(path.to_owned(), value.as_bytes().to_vec());
            Ok(())
        }
        _ => insert_json_file(files, path, value),
    }
}
