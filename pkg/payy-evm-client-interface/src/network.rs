use serde::{Deserialize, Serialize};

use crate::evm::Address;

/// Payy network configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayyNetworkConfig {
    /// EVM chain ID.
    pub chain_id: u64,
    /// PrivacyBridge predeploy address.
    pub privacy_bridge: Address,
}

/// Built-in network preset identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayyNetworkPreset {
    /// Local development chain.
    Dev,
    /// Shared testnet chain.
    Testnet,
}

impl PayyNetworkPreset {
    /// Resolve the preset into concrete network config.
    #[must_use]
    pub fn config(self) -> PayyNetworkConfig {
        match self {
            Self::Dev => PayyNetworkConfig {
                chain_id: 7297,
                privacy_bridge: [
                    0x31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
            },
            Self::Testnet => PayyNetworkConfig {
                chain_id: 7298,
                privacy_bridge: [
                    0x31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
            },
        }
    }
}
