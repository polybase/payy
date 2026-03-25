// lint-long-file-override allow-max-lines=300
use mockito::{Matcher, mock};
use serde_json::json;
use serial_test::serial;

use crate::update_client::available_update_from_releases_url;

#[tokio::test]
#[serial]
async fn available_update_falls_back_to_newest_complete_release_for_the_current_target() {
    let asset_name = current_asset_name();
    let page_1 = json!([
        {
            "tag_name": "beam-v1002.0.0",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": asset_name.clone(),
                    "digest": "sha1:not-valid",
                    "browser_download_url": "https://example.invalid/beam-v1002.0.0"
                }
            ]
        }
    ]);
    let page_2 = json!([
        {
            "tag_name": "beam-v1001.0.0",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": "beam-other-target",
                    "digest": format!("sha256:{}", "b".repeat(64)),
                    "browser_download_url": "https://example.invalid/beam-v1001.0.0"
                }
            ]
        }
    ]);
    let page_3 = json!([
        {
            "tag_name": "beam-v1000.0.0",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": asset_name.clone(),
                    "digest": format!("sha256:{}", "c".repeat(64)),
                    "browser_download_url": "https://example.invalid/beam-v1000.0.0"
                }
            ]
        }
    ]);

    let _pages = mock_release_pages([page_1, page_2, page_3, json!([])]);

    let update = available_update_from_releases_url(&releases_url())
        .await
        .expect("load update info")
        .expect("fallback release");

    assert_eq!(update.tag_name, "beam-v1000.0.0");
    assert_eq!(update.asset_name, asset_name);
    assert_eq!(update.asset_url, "https://example.invalid/beam-v1000.0.0");
    assert_eq!(update.asset_digest, format!("sha256:{}", "c".repeat(64)));
}

#[tokio::test]
#[serial]
async fn available_update_prefers_the_highest_complete_stable_version_across_pages() {
    let asset_name = current_asset_name();
    let page_1 = json!([
        {
            "tag_name": "beam-v1000.0.0",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": asset_name.clone(),
                    "digest": format!("sha256:{}", "a".repeat(64)),
                    "browser_download_url": "https://example.invalid/beam-v1000.0.0"
                }
            ]
        }
    ]);
    let page_2 = json!([
        {
            "tag_name": "beam-v1002.0.0",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": asset_name.clone(),
                    "digest": format!("sha256:{}", "b".repeat(64)),
                    "browser_download_url": "https://example.invalid/beam-v1002.0.0"
                }
            ]
        }
    ]);

    let _pages = mock_release_pages([page_1, page_2, json!([])]);

    let update = available_update_from_releases_url(&releases_url())
        .await
        .expect("load update info")
        .expect("highest stable release");

    assert_eq!(update.tag_name, "beam-v1002.0.0");
    assert_eq!(update.asset_name, asset_name);
    assert_eq!(update.asset_url, "https://example.invalid/beam-v1002.0.0");
    assert_eq!(update.asset_digest, format!("sha256:{}", "b".repeat(64)));
}

#[tokio::test]
#[serial]
async fn available_update_ignores_semver_prerelease_tags_even_if_github_flags_are_stable() {
    let asset_name = current_asset_name();
    let page_1 = json!([
        {
            "tag_name": "beam-v1002.0.0-rc.1",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": asset_name.clone(),
                    "digest": format!("sha256:{}", "a".repeat(64)),
                    "browser_download_url": "https://example.invalid/beam-v1002.0.0-rc.1"
                }
            ]
        }
    ]);
    let page_2 = json!([
        {
            "tag_name": "beam-v1001.0.0",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": asset_name.clone(),
                    "digest": format!("sha256:{}", "b".repeat(64)),
                    "browser_download_url": "https://example.invalid/beam-v1001.0.0"
                }
            ]
        }
    ]);

    let _pages = mock_release_pages([page_1, page_2, json!([])]);

    let update = available_update_from_releases_url(&releases_url())
        .await
        .expect("load update info")
        .expect("stable release after prerelease tag");

    assert_eq!(update.tag_name, "beam-v1001.0.0");
    assert_eq!(update.asset_name, asset_name);
    assert_eq!(update.asset_url, "https://example.invalid/beam-v1001.0.0");
    assert_eq!(update.asset_digest, format!("sha256:{}", "b".repeat(64)));
}

#[tokio::test]
#[serial]
async fn available_update_ignores_an_older_complete_release_if_a_newer_one_exists_later() {
    let asset_name = current_asset_name();
    let page_1 = json!([
        {
            "tag_name": "beam-v0.0.1",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": asset_name.clone(),
                    "digest": format!("sha256:{}", "a".repeat(64)),
                    "browser_download_url": "https://example.invalid/beam-v0.0.1"
                }
            ]
        }
    ]);
    let page_2 = json!([
        {
            "tag_name": "beam-v1000.0.0",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": asset_name.clone(),
                    "digest": format!("sha256:{}", "b".repeat(64)),
                    "browser_download_url": "https://example.invalid/beam-v1000.0.0"
                }
            ]
        }
    ]);

    let _pages = mock_release_pages([page_1, page_2, json!([])]);

    let update = available_update_from_releases_url(&releases_url())
        .await
        .expect("load update info")
        .expect("newer release after older current-or-lower release");

    assert_eq!(update.tag_name, "beam-v1000.0.0");
    assert_eq!(update.asset_name, asset_name);
    assert_eq!(update.asset_url, "https://example.invalid/beam-v1000.0.0");
    assert_eq!(update.asset_digest, format!("sha256:{}", "b".repeat(64)));
}

#[tokio::test]
#[serial]
async fn available_update_returns_none_when_all_releases_are_incomplete_for_the_current_target() {
    let asset_name = current_asset_name();
    let page_1 = json!([
        {
            "tag_name": "beam-v1002.0.0",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": asset_name.clone(),
                    "digest": "sha1:not-valid",
                    "browser_download_url": "https://example.invalid/beam-v1002.0.0"
                }
            ]
        }
    ]);
    let page_2 = json!([
        {
            "tag_name": "beam-v1001.0.0",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": asset_name,
                    "browser_download_url": "https://example.invalid/beam-v1001.0.0"
                }
            ]
        }
    ]);
    let page_3 = json!([
        {
            "tag_name": "beam-v1000.0.0",
            "draft": false,
            "prerelease": false,
            "assets": [
                {
                    "name": "beam-other-target",
                    "digest": format!("sha256:{}", "c".repeat(64)),
                    "browser_download_url": "https://example.invalid/beam-v1000.0.0"
                }
            ]
        }
    ]);

    let _pages = mock_release_pages([page_1, page_2, page_3, json!([])]);

    let update = available_update_from_releases_url(&releases_url())
        .await
        .expect("load update info");

    assert!(update.is_none());
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

fn mock_release_pages(pages: impl IntoIterator<Item = serde_json::Value>) -> Vec<mockito::Mock> {
    pages
        .into_iter()
        .enumerate()
        .map(|(index, page)| {
            mock("GET", "/releases")
                .match_query(Matcher::AllOf(vec![
                    Matcher::UrlEncoded("per_page".into(), "100".into()),
                    Matcher::UrlEncoded("page".into(), (index + 1).to_string()),
                ]))
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(page.to_string())
                .create()
        })
        .collect()
}

fn releases_url() -> String {
    format!("{}/releases", mockito::server_url())
}
