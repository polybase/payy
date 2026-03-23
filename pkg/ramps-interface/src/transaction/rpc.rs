use currency::Currency;
use element::Element;
use network::Network;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::provider::Provider;

use super::{FundingStatus, Status, TransactionUpdate};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateTransactionRequest {
    pub quote_id: Uuid,
    pub from_network_identifier: Option<network::NetworkIdentifier>,
    pub to_network_identifier: Option<network::NetworkIdentifier>,
    pub evm_address: Option<String>,
    pub external_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateTransactionRequest {
    pub status: Option<Status>,
    pub local_id: Option<String>,
    pub evm_address: Option<String>,
    pub memo: Option<String>,
}

impl From<UpdateTransactionRequest> for TransactionUpdate {
    fn from(req: UpdateTransactionRequest) -> Self {
        Self {
            status: req.status,
            memo: req.memo,
            ..Default::default()
        }
    }
}

pub struct FundTransactionRequest {
    pub external_id: String,
    pub from_currency: Currency,
    pub from_amount: Element,
    pub from_network: Network,
    pub default_to_currency: Currency,
    pub default_to_network: Network,
    pub default_method_id: Option<Uuid>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ListRampsTransactionsQuery {
    pub wait: Option<u64>,
    pub after: Option<u64>,
    pub provider: Option<String>,
    pub status: Option<Status>,
    pub funding_status: Option<FundingStatus>,
    pub network: Option<Network>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitQuery {
    pub provider: Provider,
    pub network: Network,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemainingLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekly: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_daily: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_weekly: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_monthly: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<u64>,
}
