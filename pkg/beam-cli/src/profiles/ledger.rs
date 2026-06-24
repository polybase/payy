// lint-long-file-override allow-max-lines=230
use std::path::Path;

use contextful::ResultContextExt;
use json_store::{FileAccess, InvalidJsonBehavior, JsonStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::profiles::{
    Error, Result,
    model::{ProfileRecord, PublicSigningIntent},
    store::now,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerState {
    #[serde(default)]
    pub entries: Vec<LedgerEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerEntry {
    pub id: String,
    pub profile: String,
    pub grant_id: String,
    pub created_at: u64,
    pub status: LedgerStatus,
    pub amount: String,
    pub asset: String,
    pub chain: String,
    pub gas: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LedgerStatus {
    Signed,
    RolledBack,
}

pub async fn load(root: &Path) -> Result<JsonStore<LedgerState>> {
    Ok(JsonStore::new_with_invalid_json_behavior_and_access(
        root.join("profiles"),
        "ledger.json",
        InvalidJsonBehavior::Error,
        FileAccess::OwnerOnly,
    )
    .await
    .context("load beam profile ledger")?)
}

pub async fn append_signed(
    root: &Path,
    key: &[u8],
    profile: &ProfileRecord,
    grant_id: &str,
    intent: &PublicSigningIntent,
    gas: String,
    tx_hash: String,
) -> Result<LedgerEntry> {
    let store = load(root).await?;
    let mut state = verified_state(&store, key).await?;
    let entry = entry_for_intent(profile, grant_id, intent, gas, tx_hash);
    state.entries.push(entry.clone());
    sign_state(&mut state, key)?;
    store.set(state).await.context("persist profile ledger")?;
    Ok(entry)
}

pub async fn mark_rolled_back(root: &Path, key: &[u8], ledger_id: &str) -> Result<()> {
    let store = load(root).await?;
    let mut state = verified_state(&store, key).await?;
    for entry in &mut state.entries {
        if entry.id == ledger_id {
            entry.status = LedgerStatus::RolledBack;
        }
    }
    sign_state(&mut state, key)?;
    store
        .set(state)
        .await
        .context("persist profile ledger rollback")?;
    Ok(())
}

pub async fn verified_state(store: &JsonStore<LedgerState>, key: &[u8]) -> Result<LedgerState> {
    let state = store.get().await;
    verify_state(&state, key)?;
    Ok(state)
}

pub fn verify_state(state: &LedgerState, key: &[u8]) -> Result<()> {
    if state.entries.is_empty() && state.integrity.is_none() {
        return Ok(());
    }
    let expected = state_digest(state, key)?;
    match state.integrity.as_deref() {
        Some(actual) if actual == expected => Ok(()),
        _ => Err(Error::LedgerIntegrityFailed),
    }
}

pub fn sign_state(state: &mut LedgerState, key: &[u8]) -> Result<()> {
    state.integrity = None;
    state.integrity = Some(state_digest(state, key)?);
    Ok(())
}

pub fn spent_for_grant(state: &LedgerState, grant_id: &str, asset: &str) -> contracts::U256 {
    state
        .entries
        .iter()
        .filter(|entry| entry.grant_id == grant_id && entry.asset == asset)
        .filter(|entry| entry.status == LedgerStatus::Signed)
        .filter_map(|entry| parse_u256(&entry.amount).ok())
        .fold(contracts::U256::zero(), |left, right| left + right)
}

fn parse_u256(value: &str) -> Result<contracts::U256> {
    if let Some(value) = value.strip_prefix("0x") {
        return contracts::U256::from_str_radix(value, 16).map_err(|_| Error::InvalidAmount {
            value: format!("0x{value}"),
        });
    }
    contracts::U256::from_dec_str(value).map_err(|_| Error::InvalidAmount {
        value: value.to_string(),
    })
}

fn state_digest(state: &LedgerState, key: &[u8]) -> Result<String> {
    let mut clone = state.clone();
    clone.integrity = None;
    let bytes = serde_json::to_vec(&clone).context("encode profile ledger integrity payload")?;
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.update(bytes);
    Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
}

fn entry_for_intent(
    profile: &ProfileRecord,
    grant_id: &str,
    intent: &PublicSigningIntent,
    gas: String,
    tx_hash: String,
) -> LedgerEntry {
    let now = now();
    let mut entry = LedgerEntry {
        id: format!(
            "led_{}",
            short_hash(format!("{}:{grant_id}:{tx_hash}", profile.name))
        ),
        profile: profile.name.clone(),
        grant_id: grant_id.to_string(),
        created_at: now,
        status: LedgerStatus::Signed,
        amount: "0".to_string(),
        asset: "native".to_string(),
        chain: String::new(),
        gas,
        app: None,
        command: None,
        plan_hash: None,
        approval_id: None,
        tx_hash: Some(tx_hash),
    };
    apply_intent_fields(&mut entry, intent);
    entry
}

fn apply_intent_fields(entry: &mut LedgerEntry, intent: &PublicSigningIntent) {
    match intent {
        PublicSigningIntent::NativeTransfer(intent) => {
            entry.amount = intent.amount.clone();
            entry.asset = intent.asset.clone();
            entry.chain = intent.chain.clone();
            entry.command = Some("native-transfer".to_string());
        }
        PublicSigningIntent::Erc20Transfer(intent) => {
            entry.amount = intent.amount.clone();
            entry.asset = intent.token.clone();
            entry.chain = intent.chain.clone();
            entry.command = Some("erc20-transfer".to_string());
        }
        PublicSigningIntent::Erc20Approval(intent) => {
            entry.amount = intent.amount.clone();
            entry.asset = intent.token.clone();
            entry.chain = intent.chain.clone();
            entry.command = Some("erc20-approval".to_string());
        }
        PublicSigningIntent::ContractTransaction(intent) => {
            entry.amount = intent.native_value.clone();
            entry.asset = "native".to_string();
            entry.chain = intent.chain.clone();
            entry.command = Some("contract-transaction".to_string());
        }
        PublicSigningIntent::FetchPayment(intent) => {
            entry.amount = intent.amount.clone();
            entry.asset = intent.asset.clone();
            entry.chain = intent.chain.clone();
            entry.command = Some("fetch-payment".to_string());
        }
        PublicSigningIntent::AppActionPlan(intent) => {
            entry.app = Some(intent.app_id.clone());
            entry.chain = intent.chain.clone();
            entry.command = Some(intent.command.clone());
            entry.plan_hash = Some(intent.plan_hash.clone());
            entry.approval_id = intent.approval_id.clone();
        }
    }
}

fn short_hash(value: String) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(&digest[..8])
}
