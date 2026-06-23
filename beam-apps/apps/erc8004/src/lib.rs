// lint-long-file-override allow-max-lines=700
mod abi;
mod args;
mod config;
mod error;
mod host;
mod plan;

pub use abi::selector;
pub use error::{Error, Result};
pub use host::{ActionBinding, ActionPlan, ActionStep, GuestInvocation, PlanContext};

use abi::{
    AgentId, EvmAddress, address_hex, agent_wallet_hashes, decode_address, decode_string,
    get_agent_wallet_calldata, owner_of_calldata, parse_address, parse_agent_id,
    parse_registered_event, register_calldata, registered_topic, set_uri_calldata,
    set_wallet_calldata, token_uri_calldata, unset_wallet_calldata,
};
use args::{Command, ConnectionMode};
use ethabi::ethereum_types::Address;
use plan::{TransactionPlanInput, agent_binding, binding, transaction_plan, wallet_binding};
use serde_json::{Value, json};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Agent {
    agent_id: AgentId,
    agent_wallet: EvmAddress,
    owner: EvmAddress,
    uri: String,
}

#[unsafe(no_mangle)]
pub extern "C" fn beam_alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    core::mem::forget(buffer);
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn beam_free(ptr: *mut u8, capacity: usize) {
    if ptr.is_null() || capacity == 0 {
        return;
    }
    unsafe {
        let _ = Vec::<u8>::from_raw_parts(ptr, 0, capacity);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn beam_app_main(input_ptr: *const u8, input_len: usize) -> u64 {
    let result = run_guest(input_ptr, input_len).unwrap_or_else(|error| GuestResponse::Error {
        message: error.to_string(),
    });
    pack_response(&result)
}

fn run_guest(input_ptr: *const u8, input_len: usize) -> Result<GuestResponse> {
    if input_ptr.is_null() {
        return Err(Error::InvalidHostResponse {
            reason: "guest invocation pointer is null".to_string(),
        });
    }
    let input = unsafe { core::slice::from_raw_parts(input_ptr, input_len) };
    let invocation =
        serde_json::from_slice::<GuestInvocation>(input).map_err(|err| Error::Serialization {
            reason: err.to_string(),
        })?;
    host::ensure_host_abi(&invocation)?;
    match args::parse(&invocation.args)? {
        Command::Support => output(run_support(&invocation)?),
        Command::ConfigShow => output(run_config_show(&invocation)?),
        Command::ConfigSet(args) => output(run_config_set(&invocation, args)?),
        Command::Register(args) => action_plan(run_register(&invocation, args)?),
        Command::Show(args) => output(run_show(&invocation, args)?),
        Command::List(args) => output(run_list(&invocation, args)?),
        Command::SetUri(args) => action_plan(run_set_uri(&invocation, args)?),
        Command::SetWallet(args) => action_plan(run_set_wallet(&invocation, args)?),
        Command::UnsetWallet(args) => action_plan(run_unset_wallet(&invocation, args)?),
    }
}

fn run_support(invocation: &GuestInvocation) -> Result<Value> {
    let selection = config::select(
        invocation.metadata.chain_id,
        &invocation.metadata.chain,
        None,
    )?;
    let value = json!({
        "message": format!(
            "ERC-8004 is supported on {} ({})\nidentity registry: {}",
            selection.config.display_name,
            selection.config.chain_id,
            selection.config.identity_registry
        ),
        "supported": true,
        "chain": invocation.metadata.chain,
        "chain_id": invocation.metadata.chain_id,
        "registry": config::to_json(&selection.config),
    });
    Ok(value)
}

fn run_config_show(invocation: &GuestInvocation) -> Result<Value> {
    let config = config::show(invocation.metadata.chain_id, &invocation.metadata.chain)?;
    Ok(json!({
        "message": format!("ERC-8004 identity registry: {}", config.identity_registry),
        "registry": config::to_json(&config),
    }))
}

fn run_config_set(invocation: &GuestInvocation, args: args::ConfigSetArgs) -> Result<Value> {
    let config = config::set(
        invocation.metadata.chain_id,
        &invocation.metadata.chain,
        &args.identity_registry,
        args.reputation_registry.as_deref(),
    )?;
    Ok(json!({
        "message": format!("Updated ERC-8004 registry config: {}", config.identity_registry),
        "registry": config::to_json(&config),
    }))
}

fn run_register(invocation: &GuestInvocation, args: args::RegisterArgs) -> Result<ActionPlan> {
    if let Some(uri) = args.uri.as_deref() {
        validate_agent_uri(uri)?;
    }
    let selection = config::select(
        invocation.metadata.chain_id,
        &invocation.metadata.chain,
        args.identity_registry.as_deref(),
    )?;
    let calldata = register_calldata(args.uri.as_deref());
    let mut bindings = vec![binding(
        "agent_uri",
        args.uri.as_deref().unwrap_or("<empty>"),
    )];
    bindings.push(plan::selector_binding(if args.uri.is_some() {
        "register(string)"
    } else {
        "register()"
    }));
    transaction_plan(TransactionPlanInput {
        bindings,
        calldata,
        command: "register".to_string(),
        context: invocation.metadata.plan_context(),
        dynamic_contracts: selection.dynamic_contracts,
        expires_at: invocation.metadata.now + 15 * 60,
        registry: selection.config.identity_registry,
        summary: "Register ERC-8004 agent".to_string(),
        value: "0".to_string(),
    })
}

fn run_show(invocation: &GuestInvocation, args: args::ShowArgs) -> Result<Value> {
    let selection = config::select(
        invocation.metadata.chain_id,
        &invocation.metadata.chain,
        args.identity_registry.as_deref(),
    )?;
    let agent_id = parse_agent_id(&args.agent_id)?;
    let agent = read_agent(
        &invocation.metadata.chain,
        &selection.config.identity_registry,
        &selection.dynamic_contracts,
        agent_id,
    )?;
    let uri_body = if args.fetch_uri {
        fetch_uri(agent.uri.as_str())?
    } else {
        None
    };

    Ok(json!({
        "message": format!("ERC-8004 agent {} owned by {}", agent.agent_id, address_hex(agent.owner)),
        "agent": agent_json(&agent),
        "agent_uri_body": uri_body,
        "identity_registry": selection.config.identity_registry,
    }))
}

fn run_list(invocation: &GuestInvocation, args: args::ListArgs) -> Result<Value> {
    let selection = config::select(
        invocation.metadata.chain_id,
        &invocation.metadata.chain,
        args.identity_registry.as_deref(),
    )?;
    let wallet = host::resolve_address(args.wallet.as_deref())?;
    let wallet = parse_address(&wallet)?;
    let owner_topic = if matches!(args.connection, ConnectionMode::Owner) {
        Some(vec![address_topic(wallet)])
    } else {
        None
    };
    let logs = host::logs(
        &invocation.metadata.chain,
        &selection.config.identity_registry,
        vec![Some(vec![registered_topic()]), None, owner_topic],
        args.from_block,
        args.to_block,
        &selection.dynamic_contracts,
    )?;
    let mut agents = Vec::new();
    for event in logs
        .logs
        .iter()
        .filter_map(|log| parse_registered_event(log, &selection.config.identity_registry))
    {
        let agent = read_agent(
            &invocation.metadata.chain,
            &selection.config.identity_registry,
            &selection.dynamic_contracts,
            event.agent_id,
        )?;
        if connects(&agent, wallet, &args.connection) {
            agents.push(agent_json(&agent));
        }
    }

    Ok(json!({
        "message": format!("Found {} ERC-8004 agents", agents.len()),
        "agents": agents,
        "connection": args.connection.label(),
        "identity_registry": selection.config.identity_registry,
        "wallet": address_hex(wallet),
    }))
}

fn run_set_uri(invocation: &GuestInvocation, args: args::SetUriArgs) -> Result<ActionPlan> {
    validate_agent_uri(&args.uri)?;
    let selection = config::select(
        invocation.metadata.chain_id,
        &invocation.metadata.chain,
        args.identity_registry.as_deref(),
    )?;
    let agent_id = parse_agent_id(&args.agent_id)?;
    let calldata = set_uri_calldata(agent_id, &args.uri);
    transaction_plan(TransactionPlanInput {
        bindings: vec![
            agent_binding(agent_id),
            binding("agent_uri", &args.uri),
            plan::selector_binding("setAgentURI(uint256,string)"),
        ],
        calldata,
        command: format!("set-uri {agent_id}"),
        context: invocation.metadata.plan_context(),
        dynamic_contracts: selection.dynamic_contracts,
        expires_at: invocation.metadata.now + 15 * 60,
        registry: selection.config.identity_registry,
        summary: format!("Update ERC-8004 agent {agent_id} URI"),
        value: "0".to_string(),
    })
}

fn run_unset_wallet(invocation: &GuestInvocation, args: args::UnsetWalletArgs) -> Result<ActionPlan> {
    let selection = config::select(
        invocation.metadata.chain_id,
        &invocation.metadata.chain,
        args.identity_registry.as_deref(),
    )?;
    let agent_id = parse_agent_id(&args.agent_id)?;
    let calldata = unset_wallet_calldata(agent_id);
    transaction_plan(TransactionPlanInput {
        bindings: vec![
            agent_binding(agent_id),
            plan::selector_binding("unsetAgentWallet(uint256)"),
        ],
        calldata,
        command: format!("unset-wallet {agent_id}"),
        context: invocation.metadata.plan_context(),
        dynamic_contracts: selection.dynamic_contracts,
        expires_at: invocation.metadata.now + 15 * 60,
        registry: selection.config.identity_registry,
        summary: format!("Clear ERC-8004 agent {agent_id} wallet"),
        value: "0".to_string(),
    })
}

fn run_set_wallet(invocation: &GuestInvocation, args: args::SetWalletArgs) -> Result<ActionPlan> {
    let selection = config::select(
        invocation.metadata.chain_id,
        &invocation.metadata.chain,
        args.identity_registry.as_deref(),
    )?;
    let agent_id = parse_agent_id(&args.agent_id)?;
    let target_wallet = host::resolve_address(Some(&args.wallet))?;
    let target_wallet = parse_address(&target_wallet)?;
    let registry = parse_address(&selection.config.identity_registry)?;
    let agent = read_agent(
        &invocation.metadata.chain,
        &selection.config.identity_registry,
        &selection.dynamic_contracts,
        agent_id,
    )?;
    let deadline = invocation
        .metadata
        .now
        .saturating_add(args.deadline_seconds);
    let (domain_separator, struct_hash) = agent_wallet_hashes(
        invocation.metadata.chain_id,
        registry,
        agent_id,
        target_wallet,
        agent.owner,
        deadline,
    );
    let signature = host::sign_typed_data(
        &invocation.metadata.chain,
        &args.wallet,
        &selection.config.identity_registry,
        &domain_separator,
        &struct_hash,
        vec![
            ("uint256", "agentId", agent_id.to_string()),
            ("address", "newWallet", address_hex(target_wallet)),
            ("address", "owner", address_hex(agent.owner)),
            ("uint256", "deadline", deadline.to_string()),
        ],
        &selection.dynamic_contracts,
    )?;
    let calldata = set_wallet_calldata(agent_id, target_wallet, deadline, &signature.signature)?;
    transaction_plan(TransactionPlanInput {
        bindings: vec![
            agent_binding(agent_id),
            wallet_binding("agent_wallet", target_wallet),
            wallet_binding("owner", agent.owner),
            binding("deadline", &deadline.to_string()),
            binding("signed_by", &signature.signer),
            binding("typed_data_digest", &signature.digest),
            binding("signature_hash", &abi::calldata_hash(&signature.signature)),
            plan::selector_binding("setAgentWallet(uint256,address,uint256,bytes)"),
        ],
        calldata,
        command: format!("set-wallet {agent_id} {}", args.wallet),
        context: invocation.metadata.plan_context(),
        dynamic_contracts: selection.dynamic_contracts,
        expires_at: deadline,
        registry: selection.config.identity_registry,
        summary: format!("Update ERC-8004 agent {agent_id} wallet"),
        value: "0".to_string(),
    })
}

fn read_agent(
    chain: &str,
    registry: &str,
    dynamic_contracts: &[host::DynamicContractScope],
    agent_id: AgentId,
) -> Result<Agent> {
    let owner = decode_address(&host::eth_call(
        chain,
        registry,
        &owner_of_calldata(agent_id),
        dynamic_contracts,
    )?)?;
    let uri = decode_string(&host::eth_call(
        chain,
        registry,
        &token_uri_calldata(agent_id),
        dynamic_contracts,
    )?)?;
    let agent_wallet = decode_address(&host::eth_call(
        chain,
        registry,
        &get_agent_wallet_calldata(agent_id),
        dynamic_contracts,
    )?)?;

    Ok(Agent {
        agent_id,
        agent_wallet,
        owner,
        uri,
    })
}

fn fetch_uri(uri: &str) -> Result<Option<Value>> {
    if !uri.starts_with("https://") {
        return Ok(None);
    }
    let response = host::http_get(uri)?;
    let text = String::from_utf8(response.body).map_err(|err| Error::InvalidHostResponse {
        reason: err.to_string(),
    })?;
    if !(200..300).contains(&response.status) {
        return Ok(Some(json!({
            "body": sanitize_control_chars(&text),
            "status": response.status,
            "url": response.url,
        })));
    }
    match serde_json::from_str::<Value>(&text) {
        Ok(value) => Ok(Some(value)),
        Err(_) => Ok(Some(json!(sanitize_control_chars(&text)))),
    }
}

fn connects(agent: &Agent, wallet: Address, mode: &ConnectionMode) -> bool {
    matches!(mode, ConnectionMode::Owner | ConnectionMode::Both) && agent.owner == wallet
        || matches!(mode, ConnectionMode::AgentWallet | ConnectionMode::Both)
            && agent.agent_wallet == wallet
}

fn agent_json(agent: &Agent) -> Value {
    json!({
        "agent_id": agent.agent_id.to_string(),
        "agent_uri": agent.uri,
        "agent_wallet": address_hex(agent.agent_wallet),
        "owner": address_hex(agent.owner),
    })
}

fn validate_agent_uri(uri: &str) -> Result<()> {
    if uri.starts_with("https://") || uri.starts_with("ipfs://") || uri.starts_with("data:") {
        Ok(())
    } else {
        Err(Error::InvalidAgentUri {
            uri: uri.to_string(),
        })
    }
}

fn address_topic(address: Address) -> String {
    format!("0x{:0>64}", hex::encode(address.as_bytes()))
}

fn sanitize_control_chars(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_control()).collect()
}

enum GuestResponse {
    Output { value: Value },
    ActionPlan { plan: ActionPlan },
    Error { message: String },
}

fn output(value: Value) -> Result<GuestResponse> {
    Ok(GuestResponse::Output { value })
}

fn action_plan(plan: ActionPlan) -> Result<GuestResponse> {
    Ok(GuestResponse::ActionPlan { plan })
}

fn pack_response(value: &GuestResponse) -> u64 {
    let bytes = response_bytes(value).unwrap_or_else(|err| {
        format!(
            r#"{{"kind":"error","message":"[beam-app-erc8004] serialization failed: {}"}}"#,
            err
        )
        .into_bytes()
    });
    let ptr = beam_alloc(bytes.len());
    if ptr.is_null() {
        return 0;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    }
    ((ptr as u64) << 32) | bytes.len() as u64
}

fn response_bytes(value: &GuestResponse) -> std::result::Result<Vec<u8>, serde_json::Error> {
    match value {
        GuestResponse::Output { value } => serde_json::to_vec(&json!({
            "kind": "output",
            "value": value,
        })),
        GuestResponse::ActionPlan { plan } => serde_json::to_vec(&json!({
            "kind": "action-plan",
            "plan": plan,
        })),
        GuestResponse::Error { message } => serde_json::to_vec(&json!({
            "kind": "error",
            "message": message,
        })),
    }
}
