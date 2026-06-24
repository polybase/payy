// lint-long-file-override allow-max-lines=220
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfilesState {
    #[serde(default)]
    pub profiles: Vec<ProfileRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileRecord {
    pub name: String,
    pub wallet_address: String,
    pub wallet_selector: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub default_ttl_secs: u64,
    #[serde(default)]
    pub policy: ProfilePolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfilePolicy {
    #[serde(default)]
    pub command_grants: Vec<CommandGrant>,
    #[serde(default)]
    pub app_grants: Vec<AppGrant>,
    #[serde(default)]
    pub privacy: Vec<PrivacyGrant>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandGrant {
    pub id: String,
    pub command: PublicCommandKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spender: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_native_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_token_amount: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_gas: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cumulative_budget: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_secs: Option<u64>,
    #[serde(default)]
    pub allow_unlimited_approval: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PublicCommandKind {
    NativeTransfer,
    Erc20Transfer,
    Erc20Approval,
    ContractTransaction,
    FetchPayment,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppGrant {
    pub id: String,
    pub app_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wallet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_hash: Option<String>,
    #[serde(default)]
    pub bindings: Vec<ActionGrantBinding>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub steps: Vec<ActionGrantStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_gas: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cumulative_budget: Option<String>,
    #[serde(default)]
    pub allow_unlimited_approval: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionGrantBinding {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionGrantStep {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spender: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacyGrant {
    pub capability: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PublicSigningIntent {
    NativeTransfer(TransferIntent),
    Erc20Transfer(TokenTransferIntent),
    Erc20Approval(ApprovalIntent),
    ContractTransaction(ContractIntent),
    FetchPayment(FetchPaymentIntent),
    AppActionPlan(AppActionIntent),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferIntent {
    pub wallet: String,
    pub chain: String,
    pub recipient: String,
    pub amount: String,
    pub asset: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenTransferIntent {
    pub wallet: String,
    pub chain: String,
    pub token: String,
    pub recipient: String,
    pub amount: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalIntent {
    pub wallet: String,
    pub chain: String,
    pub token: String,
    pub spender: String,
    pub amount: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractIntent {
    pub wallet: String,
    pub chain: String,
    pub target: String,
    pub selector: String,
    pub native_value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FetchPaymentIntent {
    pub wallet: String,
    pub scheme: String,
    pub origin: String,
    pub chain: String,
    pub asset: String,
    pub recipient: String,
    pub amount: String,
    pub private_payment: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppActionIntent {
    pub app_id: String,
    pub app_version: String,
    pub registry_url: Option<String>,
    pub manifest_digest: String,
    pub module_digest: String,
    pub command: String,
    pub wallet: String,
    pub chain: String,
    pub plan_hash: String,
    pub expires_at: u64,
    #[serde(default)]
    pub bindings: Vec<ActionGrantBinding>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub steps: Vec<ActionGrantStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_id: Option<String>,
}
