// lint-long-file-override allow-max-lines=300
use std::{collections::BTreeMap, path::Path};

use contextful::ResultContextExt;
use json_store::{FileAccess, InvalidJsonBehavior, JsonStore};
use serde::{Deserialize, Serialize};

use crate::{
    chains::{ChainEntry, builtin_rpc_url, default_rpc_urls},
    error::Result,
    known_tokens::{KnownToken, default_known_tokens, default_tracked_tokens, token_label_key},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BeamConfig {
    pub default_chain: String,
    pub default_wallet: Option<String>,
    pub known_tokens: BTreeMap<String, BTreeMap<String, KnownToken>>,
    #[serde(default = "default_tracked_tokens")]
    pub tracked_tokens: BTreeMap<String, Vec<String>>,
    #[serde(default = "default_rpc_configs")]
    pub rpc_configs: BTreeMap<String, ChainRpcConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainRpcConfig {
    pub default_rpc: String,
    #[serde(default)]
    pub rpc_urls: Vec<String>,
}

impl BeamConfig {
    pub fn known_token_by_label(
        &self,
        chain_key: &str,
        label: &str,
    ) -> Option<(String, KnownToken)> {
        let key = token_label_key(label);
        self.known_tokens
            .get(chain_key)
            .and_then(|tokens| tokens.get(&key).cloned().map(|token| (key, token)))
    }

    pub fn known_token_by_address(
        &self,
        chain_key: &str,
        address: &str,
    ) -> Option<(String, KnownToken)> {
        self.known_tokens.get(chain_key).and_then(|tokens| {
            tokens
                .iter()
                .find(|(_, token)| token.address.eq_ignore_ascii_case(address))
                .map(|(key, token)| (key.clone(), token.clone()))
        })
    }

    pub fn rpc_config_for_chain(&self, chain: &ChainEntry) -> Option<ChainRpcConfig> {
        self.rpc_configs
            .get(&chain.key)
            .cloned()
            .map(ChainRpcConfig::normalized)
            .or_else(|| builtin_rpc_url(&chain.key).map(|rpc| ChainRpcConfig::new(rpc.to_string())))
    }

    pub fn tracked_token_keys_for_chain(&self, chain_key: &str) -> Vec<String> {
        let Some(tokens) = self.known_tokens.get(chain_key) else {
            return Vec::new();
        };
        let Some(tracked) = self.tracked_tokens.get(chain_key) else {
            return tokens.keys().cloned().collect();
        };

        let mut ordered = Vec::new();

        for label in tracked {
            if tokens.contains_key(label) && ordered.iter().all(|existing| existing != label) {
                ordered.push(label.clone());
            }
        }

        ordered
    }

    pub fn tracked_tokens_for_chain(&self, chain_key: &str) -> Vec<(String, KnownToken)> {
        self.tracked_token_keys_for_chain(chain_key)
            .into_iter()
            .filter_map(|label| {
                self.known_tokens
                    .get(chain_key)
                    .and_then(|tokens| tokens.get(&label).cloned().map(|token| (label, token)))
            })
            .collect()
    }

    pub fn track_token(&mut self, chain_key: &str, label: &str) -> bool {
        let tracked = self.tracked_token_keys_for_chain(chain_key);
        let entry = self
            .tracked_tokens
            .entry(chain_key.to_string())
            .or_insert(tracked);

        if entry.iter().any(|existing| existing == label) {
            return false;
        }

        entry.push(label.to_string());
        true
    }

    pub fn untrack_token(&mut self, chain_key: &str, label: &str) -> bool {
        let tracked = self.tracked_token_keys_for_chain(chain_key);
        let entry = self
            .tracked_tokens
            .entry(chain_key.to_string())
            .or_insert(tracked);
        let before = entry.len();

        entry.retain(|existing| existing != label);
        before != entry.len()
    }
}

impl ChainRpcConfig {
    pub fn new(default_rpc: impl Into<String>) -> Self {
        let default_rpc = default_rpc.into();

        Self {
            default_rpc: default_rpc.clone(),
            rpc_urls: vec![default_rpc],
        }
    }

    pub fn add_rpc(&mut self, rpc_url: &str) -> bool {
        if self.rpc_urls().iter().any(|value| value == rpc_url) {
            return false;
        }

        self.rpc_urls.push(rpc_url.to_string());
        true
    }

    pub fn remove_rpc(&mut self, rpc_url: &str) {
        let mut rpc_urls = self
            .rpc_urls()
            .into_iter()
            .filter(|value| value != rpc_url)
            .collect::<Vec<_>>();

        if self.default_rpc == rpc_url {
            self.default_rpc = rpc_urls.first().cloned().unwrap_or_default();
        }

        self.rpc_urls.clear();
        self.rpc_urls.append(&mut rpc_urls);
    }

    pub fn rpc_urls(&self) -> Vec<String> {
        dedup_rpcs(&self.rpc_urls, &self.default_rpc)
    }

    pub fn set_default_rpc(&mut self, rpc_url: &str) {
        self.default_rpc = rpc_url.to_string();
    }

    fn normalized(mut self) -> Self {
        self.rpc_urls = self.rpc_urls();
        self
    }
}

impl Default for BeamConfig {
    fn default() -> Self {
        Self {
            default_chain: "ethereum".to_string(),
            default_wallet: None,
            known_tokens: default_known_tokens(),
            tracked_tokens: default_tracked_tokens(),
            rpc_configs: default_rpc_configs(),
        }
    }
}

pub async fn load_config(root: &Path) -> Result<JsonStore<BeamConfig>> {
    let store = JsonStore::new_with_invalid_json_behavior_and_access(
        root,
        "config.json",
        InvalidJsonBehavior::Error,
        FileAccess::OwnerOnly,
    )
    .await
    .context("load beam config store")?;
    Ok(store)
}

fn default_rpc_configs() -> BTreeMap<String, ChainRpcConfig> {
    default_rpc_urls()
        .into_iter()
        .map(|(chain, rpc_url)| (chain, ChainRpcConfig::new(rpc_url)))
        .collect()
}

fn dedup_rpcs(rpc_urls: &[String], default_rpc: &str) -> Vec<String> {
    let mut ordered = Vec::new();

    for rpc_url in std::iter::once(default_rpc).chain(rpc_urls.iter().map(String::as_str)) {
        if ordered.iter().all(|existing| existing != rpc_url) {
            ordered.push(rpc_url.to_string());
        }
    }

    ordered
}
