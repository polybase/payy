// lint-long-file-override allow-max-lines=300
use element::Element;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "stage", content = "data", rename_all = "lowercase")]
pub enum WalletActivityRampStage {
    Kyc(WalletActivityRampInitData),
    Transaction(WalletActivityRampInitData),
    Fund(Box<WalletActivityRampFundCombinedData>),
}

impl WalletActivityRampStage {
    pub fn id(&self) -> Option<Uuid> {
        match self {
            WalletActivityRampStage::Fund(fund) => {
                Uuid::parse_str(&fund.fund_data.transaction_id).ok()
            }
            _ => None,
        }
    }

    pub fn init_data(&self) -> WalletActivityRampInitData {
        match self {
            WalletActivityRampStage::Kyc(init_data) => init_data.clone(),
            WalletActivityRampStage::Transaction(init_data) => init_data.clone(),
            WalletActivityRampStage::Fund(fund) => fund.init_data.clone(),
        }
    }

    pub fn txn_data(&self) -> Option<WalletActivityRampFundTransactionData> {
        self.init_data()
            .transaction
            .or(self.init_data().transaction2)
    }

    pub fn ramp_kind(&self) -> Option<RampKind> {
        match self.txn_data().map(|t| (t.from_network, t.to_network))? {
            (Network::Card, _) => Some(RampKind::Card),
            (_, Network::Card) => Some(RampKind::Card),
            (Network::Payy, _) => Some(RampKind::Withdraw),
            (_, Network::Payy) => Some(RampKind::Deposit),
            _ => None,
        }
    }

    pub fn balance_movement(&self) -> Option<(Element, bool)> {
        let txn = self.txn_data()?;
        let is_credit = matches!(txn.to_network, Network::Payy);
        let value = match is_credit {
            true => txn.from_amount,
            false => txn.to_amount,
        };
        Some((value, is_credit))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Network {
    Payy,
    Polygon,
    Ethereum,
    Coelsa,
    Card,
}

pub enum RampKind {
    Deposit,
    Withdraw,
    Card,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Category {
    Bills,
    Charity,
    EatingOut,
    Entertainment,
    Expenses,
    Family,
    Finances,
    General,
    Gifts,
    Groceries,
    Holidays,
    Income,
    PersonalCare,
    Savings,
    Shopping,
    Transfer,
    Transport,
    PetCare,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkIdentifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accountnumber: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routingnumber: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardnumber: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardexpiration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardcvv: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phonenumber: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methodid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivityRampInitData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub account_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_added_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<RampMethod>,
    pub category: Option<Category>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_refund_amount: Option<Element>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint_burn_complete: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<WalletActivityRampFundTransactionData>,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub transaction2: Option<WalletActivityRampFundTransactionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RampMethod {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_id: Option<String>,
    pub network: String,
    pub identifier: NetworkIdentifier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivityRampFundCombinedData {
    #[serde(flatten)]
    pub init_data: WalletActivityRampInitData,
    #[serde(flatten)]
    pub fund_data: WalletActivityRampFundData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivityRampFundData {
    #[serde(alias = "rampTransactionId", alias = "transactionId")]
    pub transaction_id: String,
    pub has_remote_evm_address: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalletActivityRampFundTransactionData {
    pub kind: Option<String>,
    pub provider: String,
    #[serde(alias = "to_amount")]
    pub to_amount: Element,
    #[serde(alias = "to_network")]
    pub to_network: Network,
    #[serde(alias = "from_amount")]
    pub from_amount: Element,
    #[serde(alias = "to_currency")]
    pub to_currency: String,
    #[serde(alias = "from_network")]
    pub from_network: Network,
    #[serde(alias = "from_currency")]
    pub from_currency: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "to_network_identifier"
    )]
    pub to_network_identifier: Option<NetworkIdentifier>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "from_network_identifier"
    )]
    pub from_network_identifier: Option<NetworkIdentifier>,
}
