// lint-long-file-override allow-max-lines=300
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryIndex {
    pub format_version: u32,
    pub generated_at: String,
    pub apps: Vec<RegistryApp>,
    pub signature: RegistrySignature,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryApp {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub description: String,
    pub versions: Vec<RegistryVersion>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryVersion {
    pub version: String,
    pub min_beam_version: String,
    pub manifest_url: String,
    pub manifest_sha256: String,
    pub module_url: String,
    pub module_sha256: String,
    pub signature: RegistrySignature,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistrySignature {
    pub algorithm: String,
    pub key_id: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppManifest {
    pub format_version: u32,
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub publisher: String,
    pub description: String,
    pub min_beam_version: String,
    pub wasm: WasmArtifact,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<AppIcon>,
    #[serde(default)]
    pub catalog: AppCatalogMetadata,
    pub commands: Vec<AppCommand>,
    #[serde(default)]
    pub permissions: AppPermissions,
    #[serde(default)]
    pub host_api: HostApi,
    pub signature: RegistrySignature,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WasmArtifact {
    pub sha256: String,
    pub entrypoint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppIcon {
    pub url: String,
    pub sha256: String,
    pub media_type: String,
    pub alt: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppCatalogMetadata {
    #[serde(default)]
    pub capability_badges: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppCommand {
    pub name: String,
    pub about: String,
    #[serde(default)]
    pub usage: String,
    #[serde(default)]
    pub sensitive_args: Vec<String>,
    #[serde(default)]
    pub input_schema: Value,
    #[serde(default)]
    pub output_schema: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<AppCommandDocs>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppCommandDocs {
    pub summary: String,
    pub invocation: String,
    #[serde(default)]
    pub arguments: Vec<AppCommandParameter>,
    #[serde(default)]
    pub options: Vec<AppCommandParameter>,
    #[serde(default)]
    pub examples: Vec<AppCommandExample>,
    #[serde(default)]
    pub output_notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppCommandParameter {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_name: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default)]
    pub sensitive: bool,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppCommandExample {
    pub title: String,
    pub command: String,
    pub description: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppPermissions {
    #[serde(default)]
    pub http: Vec<HttpPermission>,
    #[serde(default)]
    pub chains: Vec<ChainPermission>,
    #[serde(default)]
    pub wallet: WalletPermissions,
    #[serde(default)]
    pub storage: StoragePermission,
    #[serde(default)]
    pub privacy: Vec<PrivacyCapability>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpPermission {
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainPermission {
    pub chain: String,
    #[serde(default)]
    pub operations: Vec<ChainOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contracts: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selectors: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spenders: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ChainOperation {
    Read,
    Simulate,
    SendTransaction,
    Erc20Approval,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WalletPermissions {
    #[serde(default)]
    pub read_balances: bool,
    #[serde(default)]
    pub propose_transactions: bool,
    #[serde(default)]
    pub erc20_approval: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoragePermission {
    #[serde(default)]
    pub app_local: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PrivacyCapability {
    PrivateAddress,
    PrivateBalance,
    PrivateTransfer,
    MintProof,
    BurnProof,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostApi {
    #[serde(default)]
    pub privacy_reserved: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledApp {
    pub id: String,
    pub active_version: String,
    pub manifest_sha256: String,
    pub module_sha256: String,
    pub installed_at: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppsState {
    #[serde(default)]
    pub installed: Vec<InstalledApp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppLock {
    pub id: String,
    pub version: String,
    pub manifest_sha256: String,
    pub module_sha256: String,
    pub registry_url: String,
    pub installed_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionPlan {
    pub app_id: String,
    pub app_version: String,
    pub wasm_sha256: String,
    pub manifest_sha256: String,
    pub command: String,
    pub wallet: Option<String>,
    pub chain: String,
    #[serde(default)]
    pub steps: Vec<ActionStep>,
    #[serde(default)]
    pub bindings: Vec<ActionBinding>,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionBinding {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionStep {
    pub kind: String,
    pub summary: String,
    pub target: Option<String>,
    pub selector: Option<String>,
    pub spender: Option<String>,
    pub value: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalRecord {
    pub id: String,
    pub status: ApprovalStatus,
    pub plan: ActionPlan,
    pub plan_hash: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalsState {
    #[serde(default)]
    pub approvals: Vec<ApprovalRecord>,
}
