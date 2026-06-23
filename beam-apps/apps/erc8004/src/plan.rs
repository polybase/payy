use serde_json::json;

use crate::{
    Result,
    abi::{AgentId, EvmAddress, address_hex, calldata_hash, selector},
    host::{ActionBinding, ActionPlan, ActionStep, DynamicContractScope, PlanContext},
};

#[derive(Clone, Debug)]
pub struct TransactionPlanInput {
    pub bindings: Vec<ActionBinding>,
    pub calldata: String,
    pub command: String,
    pub context: PlanContext,
    pub dynamic_contracts: Vec<DynamicContractScope>,
    pub expires_at: u64,
    pub registry: String,
    pub summary: String,
    pub value: String,
}

pub fn transaction_plan(input: TransactionPlanInput) -> Result<ActionPlan> {
    let selector = selector_from_calldata(&input.calldata);
    let mut bindings = input.bindings;
    bindings.push(binding("calldata_hash", &calldata_hash(&input.calldata)));
    let registry = input.registry;
    let value = input.value;
    let step = ActionStep {
        kind: "transaction".to_string(),
        metadata: json!({
            "transaction": {
                "data": input.calldata,
                "to": registry.clone(),
                "value": value.clone(),
            },
        }),
        selector,
        spender: None,
        summary: input.summary,
        target: Some(registry),
        value: Some(value),
    };

    Ok(ActionPlan {
        app_id: input.context.app_id,
        app_version: input.context.app_version,
        wasm_sha256: input.context.wasm_sha256,
        manifest_sha256: input.context.manifest_sha256,
        command: input.command,
        wallet: Some(input.context.wallet),
        chain: input.context.chain,
        steps: vec![step],
        bindings,
        constraints: Vec::new(),
        dynamic_contracts: input.dynamic_contracts,
        expires_at: input.expires_at,
    })
}

pub fn binding(key: &str, value: &str) -> ActionBinding {
    ActionBinding {
        key: key.to_string(),
        value: value.to_string(),
    }
}

pub fn agent_binding(agent_id: AgentId) -> ActionBinding {
    binding("agent_id", &agent_id.to_string())
}

pub fn wallet_binding(key: &str, wallet: EvmAddress) -> ActionBinding {
    binding(key, &address_hex(wallet))
}

fn selector_from_calldata(data: &str) -> Option<String> {
    let data = data.strip_prefix("0x").unwrap_or(data);
    (data.len() >= 8).then(|| format!("0x{}", &data[..8]))
}

pub fn selector_binding(signature: &str) -> ActionBinding {
    binding("selector", &selector(signature))
}
