use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanContext {
    pub app_id: String,
    pub app_version: String,
    pub chain: String,
    pub manifest_sha256: String,
    pub wallet: String,
    pub wasm_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapToken {
    pub address: String,
    pub decimals: u8,
    pub is_native: bool,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ActionBinding {
    pub key: String,
    pub value: String,
}
