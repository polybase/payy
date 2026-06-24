use std::path::{Path, PathBuf};

use contextful::ResultContextExt;
use json_store::{FileAccess, InvalidJsonBehavior, JsonStore};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::profiles::{Error, Result, store::now};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionsState {
    #[serde(default)]
    pub sessions: Vec<ProfileSession>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileSession {
    pub profile: String,
    pub wallet_address: String,
    pub socket: PathBuf,
    pub token: String,
    pub created_at: u64,
    pub expires_at: u64,
}

pub async fn load(root: &Path) -> Result<JsonStore<SessionsState>> {
    Ok(JsonStore::new_with_invalid_json_behavior_and_access(
        root.join("profiles"),
        "sessions.json",
        InvalidJsonBehavior::Error,
        FileAccess::OwnerOnly,
    )
    .await
    .context("load beam profile sessions")?)
}

pub async fn upsert(root: &Path, session: ProfileSession) -> Result<()> {
    let store = load(root).await?;
    store
        .update(|state| {
            state
                .sessions
                .retain(|candidate| candidate.profile != session.profile);
            state.sessions.push(session);
        })
        .await
        .context("persist beam profile session")?;
    Ok(())
}

pub async fn remove(root: &Path, profile: &str) -> Result<()> {
    let store = load(root).await?;
    store
        .update(|state| state.sessions.retain(|session| session.profile != profile))
        .await
        .context("remove beam profile session")?;
    Ok(())
}

pub async fn active(root: &Path, profile: &str) -> Result<ProfileSession> {
    let state = load(root).await?.get().await;
    let session = state
        .sessions
        .into_iter()
        .find(|session| session.profile == profile)
        .ok_or_else(|| Error::SessionNotFound {
            profile: profile.to_string(),
        })?;
    if session.expires_at <= now() {
        return Err(Error::SessionExpired {
            profile: profile.to_string(),
        });
    }
    Ok(session)
}

pub async fn list(root: &Path) -> Result<Vec<ProfileSession>> {
    Ok(load(root).await?.get().await.sessions)
}

pub fn socket_path(root: &Path, profile: &str) -> PathBuf {
    let root_digest = Sha256::digest(root.to_string_lossy().as_bytes());
    let profile_digest = Sha256::digest(profile.as_bytes());
    std::env::temp_dir().join(format!(
        "beam-profile-{}-{}.sock",
        hex::encode(&root_digest[..4]),
        hex::encode(&profile_digest[..4])
    ))
}

pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("tok_{}", hex::encode(bytes))
}
