use std::path::{Path, PathBuf};

use super::fixtures::{test_app, test_app_with_output};
use crate::{
    apps::{
        Error,
        model::{AppManifest, InstalledApp, RegistryIndex, RegistryVersion},
        runtime::{AppRuntime, validate_wasm_module},
        store::{AppCache, now},
    },
    cli::AppRunArgs,
    commands::apps::run_app,
    output::OutputMode,
    runtime::InvocationOverrides,
};

const WASM_WITHOUT_COMMAND_ALLOC: &[u8] = b"\0asm\x01\0\0\0\
\x01\x04\x01\x60\0\0\
\x03\x02\x01\0\
\x05\x03\x01\0\x01\
\x07\x1a\x02\
\x06memory\x02\0\
\x0dbeam_app_main\0\0\
\x0a\x04\x01\x02\0\x0b";

#[test]
fn app_runtime_requires_declared_entrypoint() {
    let bundle = repo_root().join("beam-apps/fixtures/valid");
    let version = uniswap_fixture_version(&bundle);
    let path = artifact_path(&bundle, &version.module_url);

    validate_wasm_module("uniswap", "beam_app_main", &path).expect("valid app wasm");
    validate_wasm_module("uniswap", "missing_entrypoint", &path)
        .expect_err("reject missing entrypoint");
}

#[test]
fn app_runtime_rejects_missing_command_alloc_export() {
    let module = tempfile::NamedTempFile::new().expect("create module file");
    std::fs::write(module.path(), WASM_WITHOUT_COMMAND_ALLOC).expect("write module");

    let error = validate_wasm_module("uniswap", "beam_app_main", module.path())
        .expect_err("reject missing command allocator");

    assert!(matches!(
        error,
        Error::MissingWasmExport { export, .. } if export == "beam_alloc"
    ));
}

#[tokio::test]
async fn app_command_help_skips_stale_wasm_validation() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let bundle = repo_root().join("beam-apps/fixtures/valid");
    let version = uniswap_fixture_version(&bundle);
    let manifest_path = artifact_path(&bundle, &version.manifest_url);
    let manifest_bytes = std::fs::read(&manifest_path).expect("read manifest");
    let manifest = read_json::<AppManifest>(&manifest_path);
    let cache = AppCache::load(&app.paths.root)
        .await
        .expect("load app cache");
    cache
        .install(
            &manifest,
            &manifest_bytes,
            WASM_WITHOUT_COMMAND_ALLOC,
            &version.manifest_sha256,
            "sha256:stale",
            "https://registry.beam.payy.network",
        )
        .await
        .expect("install stale app module");

    run_app(
        &app,
        AppRunArgs {
            app: "uniswap".to_string(),
            prepare: false,
            no_prompt: false,
            max_network_fee_wei: None,
            args: vec!["swap".to_string(), "--help".to_string()],
        },
    )
    .await
    .expect("render command help from manifest");
}

#[tokio::test]
async fn app_run_checks_installed_manifest_minimum_version_before_wasm() {
    let (_temp_dir, app) =
        test_app_with_output(OutputMode::Quiet, InvocationOverrides::default()).await;
    let bundle = repo_root().join("beam-apps/fixtures/valid");
    let version = uniswap_fixture_version(&bundle);
    let manifest_path = artifact_path(&bundle, &version.manifest_url);
    let mut manifest = read_json::<AppManifest>(&manifest_path);
    manifest.min_beam_version = "999.0.0".to_string();
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).expect("encode manifest");
    let cache = AppCache::load(&app.paths.root)
        .await
        .expect("load app cache");
    cache
        .install(
            &manifest,
            &manifest_bytes,
            WASM_WITHOUT_COMMAND_ALLOC,
            &version.manifest_sha256,
            "sha256:stale",
            "https://registry.beam.payy.network",
        )
        .await
        .expect("install unsupported app module");

    let error = run_app(
        &app,
        AppRunArgs {
            app: "uniswap".to_string(),
            prepare: false,
            no_prompt: false,
            max_network_fee_wei: None,
            args: vec!["unknown".to_string()],
        },
    )
    .await
    .expect_err("reject unsupported app before wasm validation");

    assert!(matches!(
        error,
        crate::error::Error::App(Error::UnsupportedBeamVersion { required, .. })
            if required == "999.0.0"
    ));
}

#[tokio::test]
async fn app_runtime_invokes_guest_and_returns_structured_errors() {
    let (_temp_dir, app) = test_app(InvocationOverrides {
        chain: Some("base".to_string()),
        from: Some("0x1111111111111111111111111111111111111111".to_string()),
        ..InvocationOverrides::default()
    })
    .await;
    let bundle = repo_root().join("beam-apps/fixtures/valid");
    let version = uniswap_fixture_version(&bundle);
    let manifest_path = artifact_path(&bundle, &version.manifest_url);
    let module_path = artifact_path(&bundle, &version.module_url);
    let manifest = read_json(&manifest_path);
    let installed = InstalledApp {
        active_version: version.version.clone(),
        id: "uniswap".to_string(),
        installed_at: now(),
        manifest_sha256: version.manifest_sha256.clone(),
        module_sha256: version.module_sha256.clone(),
    };

    let error = AppRuntime::default()
        .run_command(
            &app,
            &manifest,
            &installed,
            &module_path,
            &["unknown".to_string()],
        )
        .await
        .expect_err("guest should reject unknown command");

    assert!(error.to_string().contains("unsupported command"));
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn uniswap_fixture_version(bundle: &Path) -> RegistryVersion {
    let index = read_json::<RegistryIndex>(&bundle.join("index.json"));
    index
        .apps
        .iter()
        .find(|app| app.id == "uniswap")
        .expect("find uniswap fixture")
        .versions[0]
        .clone()
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    serde_json::from_slice(&std::fs::read(path).expect("read json")).expect("decode json")
}

fn artifact_path(bundle: &Path, url: &str) -> PathBuf {
    let prefix = "https://registry.beam.payy.network/";
    bundle.join(url.strip_prefix(prefix).expect("registry url"))
}
