// lint-long-file-override allow-max-lines=280
#[cfg(test)]
mod test_support;

use std::time::Duration;

use contextful::ResultContextExt;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

const RELEASE_PREFIX: &str = "beam-v";
const RELEASES_URL: &str = "https://api.github.com/repos/polybase/payy/releases";
const RELEASES_PAGE_SIZE: usize = 100;
const UPDATE_CHECK_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const UPDATE_CHECK_TIMEOUT: Duration = Duration::from_secs(10);
const UPDATE_DOWNLOAD_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const UPDATE_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);
const USER_AGENT: &str = concat!("beam/", env!("CARGO_PKG_VERSION"));

#[cfg(test)]
pub(crate) use test_support::override_releases_url_for_tests;

#[derive(Debug, Deserialize)]
struct Release {
    assets: Vec<ReleaseAsset>,
    draft: bool,
    prerelease: bool,
    tag_name: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    browser_download_url: String,
    digest: Option<String>,
    name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UpdateInfo {
    pub asset_digest: String,
    pub asset_name: String,
    pub asset_url: String,
    pub tag_name: String,
    pub version: Version,
}

pub(crate) async fn available_update() -> Result<Option<UpdateInfo>> {
    available_update_from_releases_url(&releases_url()).await
}

pub(crate) async fn available_update_from_releases_url(
    releases_url: &str,
) -> Result<Option<UpdateInfo>> {
    let current = current_version()?;
    let target = current_target()?;
    let client = update_client(UPDATE_CHECK_CONNECT_TIMEOUT, UPDATE_CHECK_TIMEOUT)?;
    let Some(update) = latest_stable_update(&client, releases_url, target).await? else {
        return Ok(None);
    };

    if update.version <= current {
        return Ok(None);
    }

    Ok(Some(update))
}

async fn latest_stable_update(
    client: &reqwest::Client,
    releases_url: &str,
    target: &str,
) -> Result<Option<UpdateInfo>> {
    let asset_name = format!("beam-{target}");
    let mut best_update = None;
    let mut page = 1;

    loop {
        let releases = release_page(client, releases_url, page).await?;
        if releases.is_empty() {
            return Ok(best_update);
        }

        for release in releases {
            let Some(version) = release_version(&release) else {
                continue;
            };

            if let Ok(update) = update_info_for_release(&release, &version, target, &asset_name)
                && best_update
                    .as_ref()
                    .is_none_or(|current: &UpdateInfo| update.version > current.version)
            {
                best_update = Some(update);
            }
        }

        page += 1;
    }
}

fn update_info_for_release(
    release: &Release,
    version: &Version,
    target: &str,
    asset_name: &str,
) -> Result<UpdateInfo> {
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| Error::ReleaseAssetNotFound {
            target: target.to_string(),
        })?;
    let asset_digest = asset
        .digest
        .as_deref()
        .ok_or_else(|| Error::ReleaseAssetDigestMissing {
            asset: asset_name.to_string(),
        })?;
    parse_release_asset_sha256(asset_name, asset_digest)?;

    Ok(UpdateInfo {
        asset_digest: asset_digest.to_string(),
        asset_name: asset.name.clone(),
        asset_url: asset.browser_download_url.clone(),
        tag_name: release.tag_name.clone(),
        version: version.clone(),
    })
}

async fn release_page(
    client: &reqwest::Client,
    releases_url: &str,
    page: usize,
) -> Result<Vec<Release>> {
    Ok(client
        .get(format!(
            "{releases_url}?per_page={RELEASES_PAGE_SIZE}&page={page}"
        ))
        .send()
        .await
        .with_context(|| format!("fetch beam releases page {page}"))?
        .error_for_status()
        .with_context(|| format!("validate beam releases page {page} response"))?
        .json::<Vec<Release>>()
        .await
        .with_context(|| format!("decode beam releases page {page} response"))?)
}

fn release_version(release: &Release) -> Option<Version> {
    if release.draft || release.prerelease {
        return None;
    }

    let version = release
        .tag_name
        .strip_prefix(RELEASE_PREFIX)
        .and_then(|version| Version::parse(version).ok())?;

    version.pre.is_empty().then_some(version)
}

pub(crate) async fn download_update_bytes(update: &UpdateInfo) -> Result<Vec<u8>> {
    let bytes = update_client(UPDATE_DOWNLOAD_CONNECT_TIMEOUT, UPDATE_DOWNLOAD_TIMEOUT)?
        .get(&update.asset_url)
        .send()
        .await
        .context("download beam update asset")?
        .error_for_status()
        .context("validate beam update asset response")?
        .bytes()
        .await
        .context("read beam update asset bytes")?;

    Ok(bytes.to_vec())
}

pub(crate) fn current_version() -> Result<Version> {
    Ok(Version::parse(env!("CARGO_PKG_VERSION")).context("parse current beam version")?)
}

pub(crate) fn current_version_string() -> Result<String> {
    Ok(current_version()?.to_string())
}

pub(crate) fn parse_release_asset_sha256(asset_name: &str, digest: &str) -> Result<String> {
    let sha256 =
        digest
            .strip_prefix("sha256:")
            .ok_or_else(|| Error::InvalidReleaseAssetDigest {
                asset: asset_name.to_string(),
                digest: digest.to_string(),
            })?;

    if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidReleaseAssetDigest {
            asset: asset_name.to_string(),
            digest: digest.to_string(),
        });
    }

    Ok(sha256.to_ascii_lowercase())
}

pub(crate) fn verify_release_asset_bytes(
    asset_name: &str,
    bytes: &[u8],
    digest: &str,
) -> Result<()> {
    let expected_sha256 = parse_release_asset_sha256(asset_name, digest)?;
    let actual_sha256 = hex::encode(Sha256::digest(bytes));

    if actual_sha256 != expected_sha256 {
        return Err(Error::ReleaseAssetChecksumMismatch {
            actual: actual_sha256,
            asset: asset_name.to_string(),
            expected: expected_sha256,
        });
    }

    Ok(())
}

fn current_target() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        (os, arch) => Err(Error::UnsupportedPlatform {
            arch: arch.to_string(),
            os: os.to_string(),
        }),
    }
}

fn update_client(connect_timeout: Duration, timeout: Duration) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .connect_timeout(connect_timeout)
        .timeout(timeout)
        .user_agent(USER_AGENT)
        .build()
        .context("build beam update reqwest client")?)
}

fn releases_url() -> String {
    #[cfg(test)]
    if let Some(releases_url) = test_support::releases_url_override() {
        return releases_url;
    }

    RELEASES_URL.to_string()
}
