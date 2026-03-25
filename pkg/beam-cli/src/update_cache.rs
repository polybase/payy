// lint-long-file-override allow-max-lines=300
use std::{
    path::Path,
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use contextful::ResultContextExt;
use json_store::{FileAccess, JsonStore};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    display::{ColorMode, notice_message, warning_message},
    error::Result,
    update_client::{available_update, current_version},
};

const UPDATE_STATUS_CACHE_FILE: &str = "update-status.json";
const UPDATE_STATUS_REFRESH_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const UPDATE_STATUS_FAILURE_RETRY_INTERVAL: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct UpdateStatusCache {
    #[serde(default)]
    pub last_checked_at_secs: Option<u64>,
    #[serde(default)]
    pub last_refresh_failed_at_secs: Option<u64>,
    #[serde(default)]
    pub status: CachedUpdateStatus,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum CachedUpdateStatus {
    #[default]
    Unknown,
    UpToDate,
    UpdateAvailable {
        tag_name: String,
        version: String,
    },
}

impl UpdateStatusCache {
    pub(crate) fn up_to_date(last_checked_at_secs: u64) -> Self {
        Self {
            last_checked_at_secs: Some(last_checked_at_secs),
            last_refresh_failed_at_secs: None,
            status: CachedUpdateStatus::UpToDate,
        }
    }

    pub(crate) fn update_available(
        last_checked_at_secs: u64,
        tag_name: String,
        version: String,
    ) -> Self {
        Self {
            last_checked_at_secs: Some(last_checked_at_secs),
            last_refresh_failed_at_secs: None,
            status: CachedUpdateStatus::UpdateAvailable { tag_name, version },
        }
    }
}

pub(crate) fn skip_update_checks(disabled_by_flag: bool) -> bool {
    disabled_by_flag
        || matches!(
            std::env::var("BEAM_SKIP_UPDATE_CHECK")
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "yes"
        )
}

pub(crate) async fn maybe_warn_for_interactive_startup(
    root: &Path,
    color_mode: ColorMode,
) -> Result<()> {
    if let Some(message) =
        cached_update_message(&load_update_status_store(root).await?.get().await)?
    {
        eprintln!("{}", warning_message(&message, color_mode.colors_stderr()));
    }

    Ok(())
}

pub(crate) async fn maybe_print_cached_update_notice(
    root: &Path,
    color_mode: ColorMode,
) -> Result<()> {
    if let Some(message) =
        cached_update_message(&load_update_status_store(root).await?.get().await)?
    {
        eprintln!("{}", notice_message(&message, color_mode.colors_stderr()));
    }

    Ok(())
}

pub(crate) async fn spawn_background_refresh_if_stale(root: &Path) -> Result<()> {
    let cache = load_update_status_store(root).await?.get().await;
    if !needs_refresh(&cache, unix_timestamp_secs()?) {
        return Ok(());
    }

    let executable = std::env::current_exe().context("resolve beam executable path")?;
    Command::new(&executable)
        .args(["--no-update-check", "__refresh-update-status"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn beam background update refresh")?;

    Ok(())
}

pub(crate) async fn refresh_cached_update_status(root: &Path) -> Result<()> {
    let store = load_update_status_store(root).await?;
    let checked_at_secs = unix_timestamp_secs()?;

    match available_update().await {
        Ok(Some(update)) => {
            store
                .set(UpdateStatusCache::update_available(
                    checked_at_secs,
                    update.tag_name,
                    update.version.to_string(),
                ))
                .await
                .context("persist beam update status")?;
        }
        Ok(None) => {
            store
                .set(UpdateStatusCache::up_to_date(checked_at_secs))
                .await
                .context("persist beam update status")?;
        }
        Err(_) => {
            store
                .update(move |cache| {
                    cache.last_refresh_failed_at_secs = Some(checked_at_secs);
                })
                .await
                .context("persist beam update status failure timestamp")?;
        }
    }

    Ok(())
}

pub(crate) fn cached_update_message(cache: &UpdateStatusCache) -> Result<Option<String>> {
    let current_version = current_version()?;
    let Some(update_version) = cached_update_version(cache) else {
        return Ok(None);
    };

    if update_version <= current_version {
        return Ok(None);
    }

    Ok(Some(format!(
        "beam {update_version} is available. Run `beam update` to install it."
    )))
}

pub(crate) fn needs_refresh(cache: &UpdateStatusCache, now_secs: u64) -> bool {
    !refresh_within_window(
        cache.last_checked_at_secs,
        now_secs,
        UPDATE_STATUS_REFRESH_INTERVAL,
    ) && !refresh_within_window(
        cache.last_refresh_failed_at_secs,
        now_secs,
        UPDATE_STATUS_FAILURE_RETRY_INTERVAL,
    )
}

fn cached_update_version(cache: &UpdateStatusCache) -> Option<Version> {
    let CachedUpdateStatus::UpdateAvailable { version, .. } = &cache.status else {
        return None;
    };

    Version::parse(version).ok()
}

fn refresh_within_window(timestamp_secs: Option<u64>, now_secs: u64, interval: Duration) -> bool {
    timestamp_secs
        .and_then(|timestamp_secs| now_secs.checked_sub(timestamp_secs))
        .is_some_and(|age_secs| age_secs < interval.as_secs())
}

async fn load_update_status_store(root: &Path) -> Result<JsonStore<UpdateStatusCache>> {
    JsonStore::new_with_invalid_json_behavior_and_access(
        root,
        UPDATE_STATUS_CACHE_FILE,
        json_store::InvalidJsonBehavior::UseDefault,
        FileAccess::OwnerOnly,
    )
    .await
    .context("load beam update status store")
    .map_err(Into::into)
}

fn unix_timestamp_secs() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("compute beam update timestamp")?
        .as_secs())
}
