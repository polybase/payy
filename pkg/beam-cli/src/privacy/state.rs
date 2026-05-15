use std::collections::BTreeMap;
use std::path::Path;

use contextful::ResultContextExt;
use json_store::{FileAccess, InvalidJsonBehavior, JsonStore};
use payy_evm_client::{IncomingNote, OwnedNoteState};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    privacy::hex32,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacyState {
    #[serde(default)]
    pub entries: BTreeMap<String, PrivacyStateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacyStateEntry {
    pub bridge: String,
    pub chain: String,
    pub chain_id: u64,
    #[serde(default)]
    pub incoming: BTreeMap<String, IncomingNote>,
    #[serde(default)]
    pub incoming_next_block: u64,
    #[serde(default)]
    pub pending: Vec<PendingPrivacyOperation>,
    pub privacy_address: String,
    pub standard: String,
    pub standard_version: u32,
    #[serde(default)]
    pub tokens: BTreeMap<String, TokenPrivacyState>,
    pub wallet_address: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenPrivacyState {
    pub checkpoint: Option<OwnedNoteState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingPrivacyOperation {
    pub operation: String,
    pub token: Option<String>,
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct PrivacyStateKey {
    pub bridge: String,
    pub chain: String,
    pub chain_id: u64,
    pub privacy_address: String,
    pub standard: String,
    pub standard_version: u32,
    pub wallet_address: String,
}

impl PrivacyState {
    pub fn entry_mut(&mut self, key: &PrivacyStateKey) -> Result<&mut PrivacyStateEntry> {
        let id = key.id();
        if let Some(entry) = self.entries.get(&id) {
            key.validate(entry)?;
        }
        Ok(self
            .entries
            .entry(id)
            .or_insert_with(|| PrivacyStateEntry::new(key)))
    }

    pub fn entry(&self, key: &PrivacyStateKey) -> Result<Option<&PrivacyStateEntry>> {
        let Some(entry) = self.entries.get(&key.id()) else {
            return Ok(None);
        };
        key.validate(entry)?;
        Ok(Some(entry))
    }
}

impl PrivacyStateEntry {
    fn new(key: &PrivacyStateKey) -> Self {
        Self {
            bridge: key.bridge.clone(),
            chain: key.chain.clone(),
            chain_id: key.chain_id,
            incoming: BTreeMap::new(),
            incoming_next_block: 0,
            pending: Vec::new(),
            privacy_address: key.privacy_address.clone(),
            standard: key.standard.clone(),
            standard_version: key.standard_version,
            tokens: BTreeMap::new(),
            wallet_address: key.wallet_address.clone(),
        }
    }

    pub fn token_mut(&mut self, token: &str) -> &mut TokenPrivacyState {
        self.tokens.entry(token.to_string()).or_default()
    }

    pub fn checkpoint(&self, token: &str) -> Option<OwnedNoteState> {
        self.tokens
            .get(token)
            .and_then(|state| state.checkpoint.clone())
    }

    pub fn remember_incoming(&mut self, note: IncomingNote) -> String {
        let id = hex32(&note.commitment);
        self.incoming.insert(id.clone(), note);
        id
    }

    pub fn incoming_note(&self, id: &str) -> Result<IncomingNote> {
        self.incoming
            .get(id)
            .cloned()
            .or_else(|| {
                let normalized = id.strip_prefix("0x").map(|raw| format!("0x{raw}"));
                normalized.and_then(|normalized| self.incoming.get(&normalized).cloned())
            })
            .ok_or_else(|| Error::PrivacyStateNotFound { id: id.to_string() })
    }
}

impl PrivacyStateKey {
    pub fn id(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.chain_id, self.bridge, self.wallet_address, self.privacy_address
        )
    }

    fn validate(&self, entry: &PrivacyStateEntry) -> Result<()> {
        if entry.chain_id != self.chain_id
            || entry.bridge != self.bridge
            || entry.standard != self.standard
            || entry.standard_version != self.standard_version
            || entry.wallet_address != self.wallet_address
            || entry.privacy_address != self.privacy_address
        {
            return Err(Error::PrivacyStateNotFound { id: self.id() });
        }
        Ok(())
    }
}

pub async fn load_privacy_state(root: &Path) -> Result<JsonStore<PrivacyState>> {
    JsonStore::new_with_invalid_json_behavior_and_access(
        root,
        "privacy-state.json",
        InvalidJsonBehavior::Error,
        FileAccess::OwnerOnly,
    )
    .await
    .context("load beam privacy state")
    .map_err(Into::into)
}
