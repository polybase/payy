use std::{fs, path::Path};

use mockito::{Matcher, mock};
use serde_json::from_str;
use serial_test::serial;
use tempfile::TempDir;

use crate::{
    update_cache::{
        CachedUpdateStatus, UpdateStatusCache, needs_refresh, refresh_cached_update_status,
    },
    update_client::override_releases_url_for_tests,
};

const UPDATE_STATUS_CACHE_FILE: &str = "update-status.json";
const FAILURE_RETRY_WINDOW_SECS: u64 = 5 * 60;
const SUCCESS_REFRESH_WINDOW_SECS: u64 = 24 * 60 * 60;

#[test]
fn failed_refreshes_retry_after_short_window() {
    let cache = UpdateStatusCache {
        last_checked_at_secs: Some(100),
        last_refresh_failed_at_secs: Some(100 + SUCCESS_REFRESH_WINDOW_SECS),
        status: CachedUpdateStatus::UpToDate,
    };

    assert!(!needs_refresh(
        &cache,
        100 + SUCCESS_REFRESH_WINDOW_SECS + FAILURE_RETRY_WINDOW_SECS - 1
    ));
    assert!(needs_refresh(
        &cache,
        100 + SUCCESS_REFRESH_WINDOW_SECS + FAILURE_RETRY_WINDOW_SECS
    ));
}

#[tokio::test]
#[serial]
async fn refresh_cached_update_status_records_failures_separately() {
    let temp_dir = TempDir::new().expect("create beam home");
    write_update_status_cache(temp_dir.path(), &UpdateStatusCache::up_to_date(100));

    let _releases_url_guard =
        override_releases_url_for_tests(format!("{}/releases", mockito::server_url()));
    let failed_releases = mock("GET", "/releases")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("per_page".into(), "100".into()),
            Matcher::UrlEncoded("page".into(), "1".into()),
        ]))
        .with_status(500)
        .create();

    refresh_cached_update_status(temp_dir.path())
        .await
        .expect("record failed refresh");

    failed_releases.assert();

    let failed_cache = read_update_status_cache(temp_dir.path());
    assert_eq!(failed_cache.last_checked_at_secs, Some(100));
    assert!(failed_cache.last_refresh_failed_at_secs.is_some());
    assert_eq!(failed_cache.status, CachedUpdateStatus::UpToDate);
}

#[tokio::test]
#[serial]
async fn successful_refresh_clears_failed_refresh_timestamp() {
    let temp_dir = TempDir::new().expect("create beam home");
    write_update_status_cache(
        temp_dir.path(),
        &UpdateStatusCache {
            last_checked_at_secs: Some(100),
            last_refresh_failed_at_secs: Some(200),
            status: CachedUpdateStatus::Unknown,
        },
    );

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

    refresh_cached_update_status(temp_dir.path())
        .await
        .expect("record successful refresh");

    releases.assert();

    let refreshed_cache = read_update_status_cache(temp_dir.path());
    assert!(refreshed_cache.last_checked_at_secs.is_some());
    assert_eq!(refreshed_cache.last_refresh_failed_at_secs, None);
    assert_eq!(refreshed_cache.status, CachedUpdateStatus::UpToDate);
}

fn read_update_status_cache(root: &Path) -> UpdateStatusCache {
    let json = fs::read_to_string(root.join(UPDATE_STATUS_CACHE_FILE))
        .expect("read cached update status file");

    from_str(&json).expect("parse cached update status")
}

fn write_update_status_cache(root: &Path, cache: &UpdateStatusCache) {
    let json = serde_json::to_string_pretty(cache).expect("serialize update status cache");
    fs::write(root.join(UPDATE_STATUS_CACHE_FILE), json).expect("write update status cache");
}
