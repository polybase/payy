// lint-long-file-override allow-max-lines=300
use mockito::{Matcher, mock};
use serde_json::json;
use serial_test::serial;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::{
    cli::{Cli, Command},
    display::ColorMode,
    error::Error,
    output::OutputMode,
    run_cli_with_paths,
    runtime::BeamPaths,
    update_cache::{UpdateStatusCache, cached_update_message, needs_refresh},
    update_client::{
        available_update_from_releases_url, override_releases_url_for_tests,
        parse_release_asset_sha256, verify_release_asset_bytes,
    },
};

const ASSET_NAME: &str = "beam-x86_64-unknown-linux-gnu";

#[test]
fn parses_sha256_release_asset_digest() {
    let expected_sha256 = hex::encode(Sha256::digest(b"beam"));
    let digest = format!("sha256:{expected_sha256}");

    let actual_sha256 = parse_release_asset_sha256(ASSET_NAME, &digest).expect("parse sha256");

    assert_eq!(actual_sha256, expected_sha256);
}

#[test]
fn rejects_invalid_release_asset_digest() {
    let err = parse_release_asset_sha256(ASSET_NAME, "sha1:1234").expect_err("reject sha1");

    assert!(matches!(
        err,
        Error::InvalidReleaseAssetDigest { asset, digest }
            if asset == ASSET_NAME && digest == "sha1:1234"
    ));
}

#[test]
fn verifies_matching_release_asset_checksum() {
    let bytes = b"beam";
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(bytes)));

    verify_release_asset_bytes(ASSET_NAME, bytes, &digest).expect("accept matching checksum");
}

#[test]
fn rejects_mismatched_release_asset_checksum() {
    let err = verify_release_asset_bytes(
        ASSET_NAME,
        b"beam",
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    )
    .expect_err("reject checksum mismatch");

    assert!(matches!(
        err,
        Error::ReleaseAssetChecksumMismatch {
            asset,
            expected,
            actual,
        } if asset == ASSET_NAME
            && expected == "0000000000000000000000000000000000000000000000000000000000000000"
            && actual == "ae4b867cf2eeb128ceab8c7df148df2eacfe2be35dbd40856a77bfc74f882236"
    ));
}

#[test]
fn emits_cached_update_message_for_newer_version() {
    let message = cached_update_message(&UpdateStatusCache::update_available(
        1,
        "beam-v999.0.0".to_string(),
        "999.0.0".to_string(),
    ))
    .expect("build cached update message");

    assert_eq!(
        message.as_deref(),
        Some("beam 999.0.0 is available. Run `beam update` to install it.")
    );
}

#[test]
fn suppresses_cached_update_message_for_current_version() {
    let version = env!("CARGO_PKG_VERSION");
    let message = cached_update_message(&UpdateStatusCache::update_available(
        1,
        format!("beam-v{version}"),
        version.to_string(),
    ))
    .expect("compute cached update message");

    assert_eq!(message, None);
}

#[test]
fn refreshes_update_status_entries_only_every_24_hours() {
    assert!(needs_refresh(&UpdateStatusCache::default(), 10));
    assert!(!needs_refresh(
        &UpdateStatusCache {
            last_checked_at_secs: Some(100),
            ..UpdateStatusCache::default()
        },
        100 + 24 * 60 * 60 - 1
    ));
    assert!(needs_refresh(
        &UpdateStatusCache {
            last_checked_at_secs: Some(100),
            ..UpdateStatusCache::default()
        },
        100 + 24 * 60 * 60
    ));
    assert!(!needs_refresh(&UpdateStatusCache::up_to_date(100), 101));
    assert!(needs_refresh(
        &UpdateStatusCache::up_to_date(100),
        100 + 24 * 60 * 60
    ));
    assert!(!needs_refresh(
        &UpdateStatusCache::update_available(
            100,
            "beam-v999.0.0".to_string(),
            "999.0.0".to_string(),
        ),
        100 + 24 * 60 * 60 - 1
    ));
    assert!(needs_refresh(
        &UpdateStatusCache::update_available(
            100,
            "beam-v999.0.0".to_string(),
            "999.0.0".to_string(),
        ),
        100 + 24 * 60 * 60
    ));
}

#[tokio::test]
#[serial]
async fn available_update_scans_past_non_beam_release_pages() {
    let asset_name = current_asset_name();
    let page_1 = json!([
        {
            "tag_name": "wallet-v9.9.9",
            "draft": false,
            "prerelease": false,
            "assets": []
        }
    ]);
    let page_2 = json!([
        {
            "tag_name": "beam-v0.3.0",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": asset_name,
                    "digest": format!("sha256:{}", "a".repeat(64)),
                    "browser_download_url": "https://example.invalid/beam-v0.3.0"
                }
            ]
        }
    ]);
    let page_3 = json!([]);

    let _page_1 = mock("GET", "/releases")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("per_page".into(), "100".into()),
            Matcher::UrlEncoded("page".into(), "1".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page_1.to_string())
        .create();
    let _page_2 = mock("GET", "/releases")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("per_page".into(), "100".into()),
            Matcher::UrlEncoded("page".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page_2.to_string())
        .create();
    let _page_3 = mock("GET", "/releases")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("per_page".into(), "100".into()),
            Matcher::UrlEncoded("page".into(), "3".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page_3.to_string())
        .create();

    let update = available_update_from_releases_url(&format!("{}/releases", mockito::server_url()))
        .await
        .expect("load update info");

    assert_eq!(update.expect("beam release").tag_name, "beam-v0.3.0");
}

#[tokio::test]
#[serial]
async fn run_cli_update_skips_corrupted_local_state_bootstrap() {
    let temp_dir = TempDir::new().expect("create beam home");
    write_invalid_state_file(temp_dir.path(), "config.json");
    write_invalid_state_file(temp_dir.path(), "chains.json");
    write_invalid_state_file(temp_dir.path(), "wallets.json");

    let _releases_url_guard =
        override_releases_url_for_tests(format!("{}/releases", mockito::server_url()));
    let releases = mock("GET", "/releases")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("per_page".into(), "100".into()),
            Matcher::UrlEncoded("page".into(), "1".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    run_cli_with_paths(
        Cli {
            command: Some(Command::Update),
            rpc: None,
            from: None,
            chain: None,
            output: OutputMode::Quiet,
            color: ColorMode::Never,
            no_update_check: false,
        },
        Some(BeamPaths::new(temp_dir.path().to_path_buf())),
    )
    .await
    .expect("run beam update without loading corrupted state");

    releases.assert();
}

fn current_asset_name() -> String {
    let target = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        (os, arch) => panic!("unsupported test platform: {os} {arch}"),
    };

    format!("beam-{target}")
}

fn write_invalid_state_file(root: &std::path::Path, name: &str) {
    std::fs::write(root.join(name), "{ invalid json").expect("write invalid beam state");
}
