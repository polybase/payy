use contracts::Address;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const PRIVACY_STANDARD_ID: &str = "payy-evm-privacy";
pub const PRIVACY_STANDARD_VERSION: u32 = 1;
pub const PRIVACY_BRIDGE_ADDRESS: &str = "0x3100000000000000000000000000000000000000";
pub const PRIVACY_DEPLOYMENT_PREDEPLOY: &str = "predeploy";
pub const PRIVACY_PROVER_BB_CLI: &str = "bb-cli";
pub const PRIVACY_TOKEN_POLICY_ERC20_POOLS: &str = "erc20-pools";
pub const PRIVACY_STATE_POLICY_PERSISTED: &str = "persisted";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacyProfile {
    #[serde(default = "default_standard")]
    pub standard: String,
    #[serde(default = "default_version")]
    pub version: u32,
    pub bridge: String,
    #[serde(default = "default_deployment")]
    pub deployment: String,
    #[serde(default = "default_prover")]
    pub prover: String,
    #[serde(default = "default_token_policy")]
    pub token_policy: String,
    #[serde(default = "default_state_policy")]
    pub state_policy: String,
    #[serde(default)]
    pub features: PrivacyFeatureFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacyFeatureFlags {
    #[serde(default = "default_true")]
    pub mint: bool,
    #[serde(default = "default_true")]
    pub burn: bool,
    #[serde(default = "default_true")]
    pub direct_send: bool,
    #[serde(default = "default_true")]
    pub incoming: bool,
    #[serde(default = "default_true")]
    pub claim: bool,
    #[serde(default = "default_true")]
    pub claim_links: bool,
    #[serde(default = "default_true")]
    pub ephemeral: bool,
    #[serde(default = "default_true")]
    pub private_payments: bool,
}

impl PrivacyProfile {
    pub fn payy_default() -> Self {
        Self {
            standard: PRIVACY_STANDARD_ID.to_string(),
            version: PRIVACY_STANDARD_VERSION,
            bridge: PRIVACY_BRIDGE_ADDRESS.to_string(),
            deployment: PRIVACY_DEPLOYMENT_PREDEPLOY.to_string(),
            prover: PRIVACY_PROVER_BB_CLI.to_string(),
            token_policy: PRIVACY_TOKEN_POLICY_ERC20_POOLS.to_string(),
            state_policy: PRIVACY_STATE_POLICY_PERSISTED.to_string(),
            features: PrivacyFeatureFlags::default(),
        }
    }

    pub fn bridge_address(&self) -> Result<Address> {
        self.bridge.parse().map_err(|_| Error::InvalidAddress {
            value: self.bridge.clone(),
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.standard != PRIVACY_STANDARD_ID || self.version != PRIVACY_STANDARD_VERSION {
            return Err(Error::UnsupportedPrivacyStandard {
                standard: self.standard.clone(),
                version: self.version,
            });
        }

        let _ = self.bridge_address()?;
        Ok(())
    }
}

impl Default for PrivacyFeatureFlags {
    fn default() -> Self {
        Self {
            mint: true,
            burn: true,
            direct_send: true,
            incoming: true,
            claim: true,
            claim_links: true,
            ephemeral: true,
            private_payments: true,
        }
    }
}

pub fn builtin_privacy_profile(chain_key: &str) -> Option<PrivacyProfile> {
    match chain_key {
        "payy-testnet" | "payy-dev" => Some(PrivacyProfile::payy_default()),
        _ => None,
    }
}

fn default_true() -> bool {
    true
}

fn default_standard() -> String {
    PRIVACY_STANDARD_ID.to_string()
}

fn default_version() -> u32 {
    PRIVACY_STANDARD_VERSION
}

fn default_deployment() -> String {
    PRIVACY_DEPLOYMENT_PREDEPLOY.to_string()
}

fn default_prover() -> String {
    PRIVACY_PROVER_BB_CLI.to_string()
}

fn default_token_policy() -> String {
    PRIVACY_TOKEN_POLICY_ERC20_POOLS.to_string()
}

fn default_state_policy() -> String {
    PRIVACY_STATE_POLICY_PERSISTED.to_string()
}
