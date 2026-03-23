use kyc::Kyc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Query parameters for get wallet
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct GetWalletQuery {
    /// Long poll duration
    pub wait: Option<u64>,
    /// Get changes since unix timestamps (in microseconds)
    pub after: Option<u64>,
}

/// Wallet data for the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// ID of the users wallet
    pub id: Uuid,
    /// Kyc data for the user
    pub kyc: Option<Kyc>,
    /// Features enabled for the user
    pub features: Features,
}

/// Features the user has enabled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Features {
    /// Is card program is enabled for the user
    pub card: bool,
}
