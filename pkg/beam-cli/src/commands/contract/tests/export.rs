// lint-long-file-override allow-max-lines=300
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::{fs, path::Path};

use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::commands::contract::{
    error::Error,
    export::{
        commit_into_existing_for_test, export_bundle, flatten_source_key,
        flatten_source_keys_for_test,
    },
    target::build_target_from_entry,
};

use super::{ADDRESS, chain, contract_response, source_map};

#[test]
fn export_source_key_flattening_uses_portable_labels() {
    let usdc_source_key = "/Users/aloysius.chan/repo/contracts/v2/FiatTokenV2_2.sol";
    assert_eq!(
        flatten_source_key(usdc_source_key),
        format!(
            "Users_aloysius.chan_repo_contracts_v2_FiatTokenV2_2--{}.sol",
            source_key_hash16(usdc_source_key)
        )
    );

    let traversal_key = "contracts/../Foo.sol";
    assert_eq!(
        flatten_source_key(traversal_key),
        format!("contracts_.._Foo--{}.sol", source_key_hash16(traversal_key))
    );

    let unusual_key = "dir\\sub/Únicode\u{0007}.sol";
    let unusual_name = flatten_source_key(unusual_key);
    assert!(unusual_name.is_ascii());
    assert!(!unusual_name.contains('/'));
    assert!(!unusual_name.contains('\\'));
    assert!(!unusual_name.contains('\u{0007}'));
    assert!(unusual_name.ends_with(&format!("--{}.sol", source_key_hash16(unusual_key))));

    let empty_key = "\n/💥";
    assert_eq!(
        flatten_source_key(empty_key),
        format!("source--{}", source_key_hash16(empty_key))
    );

    let long_key = format!("{}.sol", "a".repeat(500));
    let long_name = flatten_source_key(&long_key);
    assert!(long_name.len() <= 240);
    assert!(long_name.ends_with(&format!("--{}.sol", source_key_hash16(&long_key))));

    let long_extension_key = format!("stem.{}", "b".repeat(500));
    let long_extension_name = flatten_source_key(&long_extension_key);
    assert!(long_extension_name.len() <= 240);
    assert_eq!(
        long_extension_name,
        format!("stem--{}", source_key_hash16(&long_extension_key))
    );
}

#[test]
fn export_bundle_writes_manifest_and_artifacts() {
    let temp_dir = TempDir::new().expect("temp dir");
    let destination = temp_dir.path().join("bundle");
    let target = build_target_from_entry(chain(), ADDRESS).expect("target");
    let response = contract_response();

    let result = export_bundle(&target, &response, destination.to_str().expect("utf8 path"))
        .expect("export bundle");

    assert!(result.written_files.contains(&"abi.json".to_owned()));
    assert!(result.written_files.contains(&"sourcify.json".to_owned()));
    let source_key = "contracts/Foo.sol";
    let source_path = flattened_source_path(source_key);
    assert!(result.written_files.contains(&source_path));
    assert_eq!(
        fs::read_to_string(destination.join(&source_path)).expect("source file"),
        "contract Foo {}\n"
    );

    let manifest = fs::read_to_string(destination.join("sourcify.json")).expect("manifest");
    let manifest: serde_json::Value = serde_json::from_str(&manifest).expect("manifest json");
    assert_eq!(manifest["chain_id"], json!(1));
    assert_eq!(
        manifest["files"]["abi.json"]
            .as_str()
            .expect("abi hash")
            .len(),
        64
    );
    let source_hash = sha256_hex("contract Foo {}\n".as_bytes());
    assert_eq!(manifest["files"][source_path.as_str()], json!(source_hash));
    assert_eq!(
        manifest["source_files"][source_key]["path"],
        json!(source_path)
    );
    assert_eq!(
        manifest["source_files"][source_key]["sha256"],
        json!(source_hash)
    );
    assert!(manifest["files"].get("sourcify.json").is_none());
}

#[test]
fn export_bundle_rejects_non_empty_destination() {
    let temp_dir = TempDir::new().expect("temp dir");
    let destination = temp_dir.path().join("bundle");
    fs::create_dir(&destination).expect("destination dir");
    fs::write(destination.join("existing"), b"keep").expect("existing file");
    let target = build_target_from_entry(chain(), ADDRESS).expect("target");
    let response = contract_response();

    let err = export_bundle(&target, &response, destination.to_str().expect("utf8 path"))
        .expect_err("non-empty destination");

    assert!(matches!(err, Error::ExportDestinationNotEmpty { .. }));
    assert_eq!(
        fs::read(destination.join("existing")).expect("existing file"),
        b"keep"
    );
}

#[cfg(unix)]
#[test]
fn export_bundle_rejects_symlink_destination() {
    let temp_dir = TempDir::new().expect("temp dir");
    let real_destination = temp_dir.path().join("real");
    let destination = temp_dir.path().join("bundle-link");
    fs::create_dir(&real_destination).expect("real destination");
    symlink(&real_destination, &destination).expect("destination symlink");
    let target = build_target_from_entry(chain(), ADDRESS).expect("target");
    let response = contract_response();

    let err = export_bundle(&target, &response, destination.to_str().expect("utf8 path"))
        .expect_err("symlink destination");

    assert!(matches!(err, Error::ExportDestinationInvalid { .. }));
}

#[test]
fn export_bundle_accepts_absolute_and_traversal_looking_source_keys() {
    let temp_dir = TempDir::new().expect("temp dir");
    let destination = temp_dir.path().join("bundle");
    let target = build_target_from_entry(chain(), ADDRESS).expect("target");
    let source_items = [
        (
            "/Users/aloysius.chan/repo/contracts/v2/FiatTokenV2_2.sol",
            "contract FiatTokenV2_2 {}",
        ),
        ("contracts/../Foo.sol", "contract Foo {}"),
    ];
    let response = contract_response_with_sources(&source_items);

    let result = export_bundle(&target, &response, destination.to_str().expect("utf8 path"))
        .expect("export bundle");

    for (source_key, content) in source_items {
        let source_path = flattened_source_path(source_key);
        assert!(result.written_files.contains(&source_path));
        assert_eq!(Path::new(&source_path).parent(), Some(Path::new("sources")));
        assert_eq!(
            fs::read_to_string(destination.join(&source_path)).expect("source file"),
            content
        );
    }
}

#[test]
fn export_source_key_flattening_rejects_duplicate_output_names() {
    let source_keys = ["same/Name.sol".to_owned(), "same\\Name.sol".to_owned()];

    let err = flatten_source_keys_for_test(source_keys.iter(), "0000000000000000")
        .expect_err("duplicate path");

    assert!(matches!(
        err,
        Error::ExportPathCollision { path } if path == "same_Name--0000000000000000.sol"
    ));
}

#[test]
fn export_commit_does_not_overwrite_files_that_appear_during_commit() {
    let temp_dir = TempDir::new().expect("temp dir");
    let prepared = temp_dir.path().join("prepared");
    let destination = temp_dir.path().join("destination");
    fs::create_dir(&prepared).expect("prepared dir");
    fs::create_dir(&destination).expect("destination dir");
    fs::write(prepared.join("a"), b"a").expect("prepared a");
    fs::write(prepared.join("b"), b"b").expect("prepared b");
    let mut injected_conflict = false;
    let err = {
        let mut after_move = |moved: &Path| {
            if moved.file_name().and_then(|name| name.to_str()) == Some("a") {
                fs::write(destination.join("b"), b"conflict").expect("conflict file");
                injected_conflict = true;
            }
        };
        commit_into_existing_for_test(
            &prepared,
            &destination,
            destination.to_str().expect("utf8 path"),
            None,
            Some(&mut after_move),
        )
        .expect_err("commit failure")
    };

    assert!(matches!(err, Error::ExportDestinationNotEmpty { .. }));
    assert!(injected_conflict);
    assert!(!destination.join("a").exists());
    assert_eq!(
        fs::read(destination.join("b")).expect("conflict file"),
        b"conflict"
    );
    assert_eq!(fs::read(prepared.join("b")).expect("prepared b"), b"b");
}

#[test]
fn export_commit_cleans_moved_files_after_write_failure() {
    let temp_dir = TempDir::new().expect("temp dir");
    let prepared = temp_dir.path().join("prepared");
    let destination = temp_dir.path().join("destination");
    fs::create_dir(&prepared).expect("prepared dir");
    fs::create_dir(&destination).expect("destination dir");
    fs::write(prepared.join("a"), b"a").expect("prepared a");
    fs::create_dir(prepared.join("b")).expect("prepared b");
    let mut injected_conflict = false;
    let err = {
        let mut before_move = |target: &Path| {
            if target.file_name().and_then(|name| name.to_str()) == Some("b") {
                fs::write(target, b"conflict").expect("conflict file");
                injected_conflict = true;
            }
        };
        commit_into_existing_for_test(
            &prepared,
            &destination,
            destination.to_str().expect("utf8 path"),
            Some(&mut before_move),
            None,
        )
        .expect_err("commit failure")
    };

    assert!(matches!(err, Error::ExportWriteFailed { .. }));
    assert!(injected_conflict);
    assert!(!destination.join("a").exists());
    assert_eq!(
        fs::read(destination.join("b")).expect("conflict file"),
        b"conflict"
    );
    assert!(prepared.join("b").is_dir());
}

fn contract_response_with_sources(items: &[(&str, &str)]) -> sourcify_interface::ContractResponse {
    let mut response = contract_response();
    response.contract.sources = Some(source_map(items));
    response
}

fn flattened_source_path(source_key: &str) -> String {
    format!("sources/{}", flatten_source_key(source_key))
}

fn source_key_hash16(source_key: &str) -> String {
    sha256_hex(source_key.as_bytes())[..16].to_owned()
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
