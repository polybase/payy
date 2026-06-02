// lint-long-file-override allow-max-lines=300
use std::path::{Path, PathBuf};

use crate::{
    apps::{
        model::{
            AppCatalogMetadata, AppManifest, AppPermissions, ChainOperation, ChainPermission,
            HostApi, HttpPermission, PrivacyCapability, RegistryIndex, RegistrySignature,
            StoragePermission, WalletPermissions, WasmArtifact,
        },
        privacy::reject_unsupported,
        registry::{DEFAULT_REGISTRY_URL, ensure_digest, signing_digest},
        validate::{ensure_beam_version, validate_index, validate_manifest},
    },
    cli::{AppsAction, Cli, Command},
};
use clap::Parser;

fn manifest() -> AppManifest {
    AppManifest {
        format_version: 1,
        id: "sample".to_string(),
        display_name: "Sample".to_string(),
        version: "1.0.0".to_string(),
        publisher: "Payy".to_string(),
        description: "Sample app".to_string(),
        min_beam_version: "0.0.1".to_string(),
        wasm: WasmArtifact {
            sha256: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            entrypoint: "beam_app_main".to_string(),
        },
        icon: None,
        catalog: AppCatalogMetadata::default(),
        commands: vec![crate::apps::model::AppCommand {
            name: "echo".to_string(),
            about: "Echo input".to_string(),
            usage: "echo <value>".to_string(),
            sensitive_args: Vec::new(),
            input_schema: serde_json::json!({ "type": "object" }),
            output_schema: serde_json::json!({ "type": "object" }),
            docs: None,
        }],
        permissions: AppPermissions {
            http: vec![HttpPermission {
                url: "https://api.example.com/*".to_string(),
            }],
            chains: vec![ChainPermission {
                chain: "base".to_string(),
                operations: vec![ChainOperation::Read],
                contracts: Some(vec!["uniswap-*".to_string()]),
                selectors: Some(vec!["0x12345678".to_string()]),
                spenders: None,
            }],
            wallet: WalletPermissions {
                read_balances: true,
                propose_transactions: false,
                erc20_approval: false,
            },
            storage: StoragePermission { app_local: true },
            privacy: Vec::new(),
        },
        host_api: HostApi {
            privacy_reserved: true,
        },
        signature: RegistrySignature {
            algorithm: "sha256-dev".to_string(),
            key_id: "test".to_string(),
            value: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
        },
    }
}

#[test]
fn validates_manifest_with_optional_glob_scopes() {
    validate_manifest(&manifest()).expect("validate manifest");
}

#[test]
fn parses_apps_lifecycle_commands() {
    let cli = Cli::try_parse_from(["beam", "apps", "install", "sample", "--dry-run"])
        .expect("parse apps install");
    match cli.command {
        Some(Command::Apps {
            action: AppsAction::Install(args),
        }) if args.app == "sample" && args.dry_run => {}
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_x_alias_with_trailing_args() {
    let cli = Cli::try_parse_from([
        "beam",
        "x",
        "uniswap",
        "swap",
        "USDC",
        "ETH",
        "10",
        "--prepare",
    ])
    .expect("parse x alias");
    match cli.command {
        Some(Command::X(args)) if args.app == "uniswap" && args.args.contains(&"swap".into()) => {}
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn privacy_capabilities_parse_and_fail_closed() {
    let mut manifest = manifest();
    manifest.permissions.privacy = vec![PrivacyCapability::PrivateBalance];
    validate_manifest(&manifest).expect("validate privacy manifest");
    let error = reject_unsupported(PrivacyCapability::PrivateBalance).expect_err("unsupported");
    assert!(
        error
            .to_string()
            .contains("unsupported privacy app capability")
    );
}

#[test]
fn valid_registry_fixture_signatures_are_valid() {
    let bundle = repo_root().join("beam-apps/fixtures/valid");
    let index = read_json::<RegistryIndex>(&bundle.join("index.json"));
    validate_index(&index, DEFAULT_REGISTRY_URL).expect("validate index");
    assert_eq!(
        index.signature.value,
        signing_digest(&index).expect("sign index")
    );

    for app in &index.apps {
        for version in &app.versions {
            let manifest_path = artifact_path(&bundle, &version.manifest_url);
            let module_path = artifact_path(&bundle, &version.module_url);
            ensure_digest(
                "manifest",
                &std::fs::read(&manifest_path).expect("read manifest"),
                &version.manifest_sha256,
            )
            .expect("manifest digest");
            ensure_digest(
                "module",
                &std::fs::read(&module_path).expect("read module"),
                &version.module_sha256,
            )
            .expect("module digest");

            let manifest = read_json::<AppManifest>(&manifest_path);
            validate_manifest(&manifest).expect("validate generated manifest");
            assert!(manifest.icon.is_some());
            assert!(manifest.commands[0].docs.is_some());
            assert_eq!(
                manifest.signature.value,
                signing_digest(&manifest).expect("sign manifest")
            );
        }
    }
}

#[test]
fn local_loopback_registry_artifacts_are_valid_for_dev() {
    let mut index =
        read_json::<RegistryIndex>(&repo_root().join("beam-apps/fixtures/valid/index.json"));
    rewrite_registry_urls(&mut index, DEFAULT_REGISTRY_URL, "http://127.0.0.1:8787");

    validate_index(&index, "http://127.0.0.1:8787").expect("validate local index");
    validate_index(&index, DEFAULT_REGISTRY_URL).expect_err("reject wrong registry origin");
    validate_index(&index, "http://192.168.0.10:8787").expect_err("reject non-loopback http");
}

#[test]
fn registry_fixtures_cover_invalid_and_broad_permissions() {
    let fixtures = repo_root().join("beam-apps/fixtures");
    let invalid = read_json::<RegistryIndex>(&fixtures.join("invalid-digest/index.json"));
    let invalid_version = &invalid.apps[0].versions[0];
    let invalid_module = artifact_path(
        &fixtures.join("invalid-digest"),
        &invalid_version.module_url,
    );
    ensure_digest(
        "module",
        &std::fs::read(invalid_module).expect("read invalid module"),
        &invalid_version.module_sha256,
    )
    .expect_err("invalid digest");

    let missing = std::fs::read(first_fixture_manifest(&fixtures.join("missing-fields")))
        .expect("read missing fields manifest");
    serde_json::from_slice::<AppManifest>(&missing).expect_err("missing required field");

    let unsupported =
        read_json::<AppManifest>(&first_fixture_manifest(&fixtures.join("unsupported-beam")));
    ensure_beam_version(&unsupported.id, &unsupported.min_beam_version)
        .expect_err("unsupported beam version");

    let malformed = read_json::<AppManifest>(&first_fixture_manifest(
        &fixtures.join("malformed-permissions"),
    ));
    validate_manifest(&malformed).expect_err("malformed permission");

    let broad = read_json::<AppManifest>(&first_fixture_manifest(&fixtures.join("broad-wildcard")));
    validate_manifest(&broad).expect("broad wildcard permission");
    assert!(broad.permissions.chains[0].contracts.is_none());
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    serde_json::from_slice(&std::fs::read(path).expect("read json")).expect("decode json")
}

fn artifact_path(bundle: &Path, url: &str) -> PathBuf {
    let prefix = "https://registry.beam.payy.network/";
    bundle.join(url.strip_prefix(prefix).expect("registry url"))
}

fn rewrite_registry_urls(index: &mut RegistryIndex, from: &str, to: &str) {
    let from = from.trim_end_matches('/');
    let to = to.trim_end_matches('/');
    for app in &mut index.apps {
        for version in &mut app.versions {
            version.manifest_url = version.manifest_url.replace(from, to);
            version.module_url = version.module_url.replace(from, to);
        }
    }
}

fn first_fixture_manifest(bundle: &Path) -> PathBuf {
    let index = read_json::<RegistryIndex>(&bundle.join("index.json"));
    artifact_path(bundle, &index.apps[0].versions[0].manifest_url)
}
