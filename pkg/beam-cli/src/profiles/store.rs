use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use contextful::ResultContextExt;
use json_store::{FileAccess, InvalidJsonBehavior, JsonStore};
use sha2::{Digest, Sha256};

use crate::profiles::{
    Error, Result,
    model::{ProfileRecord, ProfilesState},
};

pub async fn load(root: &Path) -> Result<JsonStore<ProfilesState>> {
    Ok(JsonStore::new_with_invalid_json_behavior_and_access(
        root.join("profiles"),
        "profiles.json",
        InvalidJsonBehavior::Error,
        FileAccess::OwnerOnly,
    )
    .await
    .context("load beam profiles")?)
}

pub async fn list(root: &Path) -> Result<Vec<ProfileRecord>> {
    Ok(load(root).await?.get().await.profiles)
}

pub async fn find(root: &Path, profile: &str) -> Result<ProfileRecord> {
    find_in_state(&load(root).await?.get().await, profile)
}

pub fn find_in_state(state: &ProfilesState, profile: &str) -> Result<ProfileRecord> {
    state
        .profiles
        .iter()
        .find(|candidate| candidate.name == profile)
        .cloned()
        .ok_or_else(|| Error::ProfileNotFound {
            profile: profile.to_string(),
        })
}

pub async fn insert(root: &Path, mut profile: ProfileRecord, key: &[u8]) -> Result<()> {
    let store = load(root).await?;
    let mut state = store.get().await;
    if state
        .profiles
        .iter()
        .any(|candidate| candidate.name == profile.name)
    {
        return Err(Error::ProfileAlreadyExists {
            profile: profile.name,
        });
    }
    sign_profile(&mut profile, key)?;
    state.profiles.push(profile);
    store.set(state).await.context("persist beam profile")?;
    Ok(())
}

pub async fn update_verified<F>(root: &Path, profile: &str, key: &[u8], update: F) -> Result<()>
where
    F: FnOnce(&mut ProfileRecord) -> Result<()>,
{
    let store = load(root).await?;
    let mut state = store.get().await;
    let record = state
        .profiles
        .iter_mut()
        .find(|candidate| candidate.name == profile)
        .ok_or_else(|| Error::ProfileNotFound {
            profile: profile.to_string(),
        })?;
    verify_profile(record, key)?;
    update(record)?;
    record.updated_at = now();
    sign_profile(record, key)?;
    store.set(state).await.context("persist beam profile")?;
    Ok(())
}

pub async fn remove_verified(root: &Path, profile: &str, key: &[u8]) -> Result<()> {
    let store = load(root).await?;
    let mut state = store.get().await;
    let record = find_in_state(&state, profile)?;
    verify_profile(&record, key)?;
    state.profiles.retain(|candidate| candidate.name != profile);
    store
        .set(state)
        .await
        .context("persist beam profile removal")?;
    Ok(())
}

pub fn verify_profile(profile: &ProfileRecord, key: &[u8]) -> Result<()> {
    let expected = profile_digest(profile, key)?;
    match profile.integrity.as_deref() {
        Some(actual) if actual == expected => Ok(()),
        _ => Err(Error::ProfileIntegrityFailed {
            profile: profile.name.clone(),
        }),
    }
}

pub fn sign_profile(profile: &mut ProfileRecord, key: &[u8]) -> Result<()> {
    profile.integrity = None;
    profile.integrity = Some(profile_digest(profile, key)?);
    Ok(())
}

pub fn integrity_key(secret_key: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(b"beam-cli/profile-integrity/v1");
    hasher.update(secret_key);
    hasher.finalize().to_vec()
}

pub fn validate_profile_name(profile: &str) -> Result<()> {
    if profile.trim().is_empty() {
        Err(Error::ProfileNameBlank)
    } else {
        Ok(())
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn parse_duration_secs(value: &str) -> Result<u64> {
    let (number, multiplier) = match value.chars().last() {
        Some('s') => (&value[..value.len() - 1], 1),
        Some('m') => (&value[..value.len() - 1], 60),
        Some('h') => (&value[..value.len() - 1], 60 * 60),
        Some('d') => (&value[..value.len() - 1], 24 * 60 * 60),
        _ => (value, 1),
    };
    number
        .parse::<u64>()
        .map(|seconds| seconds.saturating_mul(multiplier))
        .map_err(|_| Error::InvalidDuration {
            value: value.to_string(),
        })
}

fn profile_digest(profile: &ProfileRecord, key: &[u8]) -> Result<String> {
    let mut clone = profile.clone();
    clone.integrity = None;
    let bytes = serde_json::to_vec(&clone).context("encode profile integrity payload")?;
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.update(bytes);
    Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
}
