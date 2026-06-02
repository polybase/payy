// lint-long-file-override allow-max-lines=300
use crate::apps::{
    approvals::{ensure_approval_executable, plan_hash},
    host::{
        ChainReadOperation, ChainReadRequest, HostTransaction, ensure_chain_read_allowed,
        ensure_http_allowed, ensure_transaction_allowed,
    },
    model::{
        ActionBinding, ActionPlan, AppPermissions, ApprovalRecord, ApprovalStatus, ChainOperation,
        ChainPermission, HttpPermission,
    },
    runtime::validate_wasm_module,
    store::now,
};

#[test]
fn host_http_permissions_allow_declared_https_and_reject_private_hosts() {
    let permissions = AppPermissions {
        http: vec![HttpPermission {
            url: "https://trade-api.gateway.uniswap.org/v1/*".to_string(),
        }],
        ..Default::default()
    };

    ensure_http_allowed(
        &permissions,
        "https://trade-api.gateway.uniswap.org/v1/quote",
    )
    .expect("allow declared quote endpoint");
    ensure_http_allowed(&permissions, "https://127.0.0.1/v1/quote")
        .expect_err("reject private host");
    ensure_http_allowed(&permissions, "https://example.com/v1/quote")
        .expect_err("reject undeclared endpoint");
}

#[test]
fn host_transaction_permissions_enforce_selector_and_spender() {
    let permissions = AppPermissions {
        chains: vec![ChainPermission {
            chain: "base".to_string(),
            contracts: Some(vec!["0xrouter".to_string()]),
            operations: vec![ChainOperation::SendTransaction],
            selectors: Some(vec!["0x3593564c".to_string()]),
            spenders: Some(vec!["0xspender".to_string()]),
        }],
        ..Default::default()
    };
    let transaction = HostTransaction {
        chain: "base".to_string(),
        data: "0x3593564c".to_string(),
        selector: Some("0x3593564c".to_string()),
        spender: Some("0xspender".to_string()),
        target: "0xrouter".to_string(),
        value: "0".to_string(),
    };

    ensure_transaction_allowed(&permissions, &transaction, ChainOperation::SendTransaction)
        .expect("allow scoped transaction");

    let mut blocked = transaction;
    blocked.selector = Some("0xdeadbeef".to_string());
    ensure_transaction_allowed(&permissions, &blocked, ChainOperation::SendTransaction)
        .expect_err("reject blocked selector");
}

#[test]
fn host_chain_read_permissions_enforce_contract_scope() {
    let permissions = AppPermissions {
        chains: vec![ChainPermission {
            chain: "base".to_string(),
            contracts: Some(vec!["0xrouter".to_string()]),
            operations: vec![ChainOperation::Read],
            selectors: Some(vec!["0x70a08231".to_string()]),
            spenders: None,
        }],
        ..Default::default()
    };
    let request = ChainReadRequest {
        address: None,
        chain: "base".to_string(),
        data: None,
        operation: ChainReadOperation::Call,
        owner: None,
        selector: Some("0x70a08231".to_string()),
        spender: None,
        target: Some("0xrouter".to_string()),
        token: None,
        value: None,
    };

    ensure_chain_read_allowed(&permissions, &request).expect("allow scoped read");

    let mut blocked = request;
    blocked.target = Some("0xother".to_string());
    ensure_chain_read_allowed(&permissions, &blocked).expect_err("reject blocked read target");
}

#[test]
fn host_transaction_permissions_allow_broad_optional_globs() {
    let permissions = AppPermissions {
        chains: vec![ChainPermission {
            chain: "base".to_string(),
            contracts: None,
            operations: vec![ChainOperation::SendTransaction],
            selectors: None,
            spenders: None,
        }],
        ..Default::default()
    };
    let transaction = HostTransaction {
        chain: "base".to_string(),
        data: "0xdeadbeef".to_string(),
        selector: Some("0xdeadbeef".to_string()),
        spender: Some("0xspender".to_string()),
        target: "0xany".to_string(),
        value: "0".to_string(),
    };

    ensure_transaction_allowed(&permissions, &transaction, ChainOperation::SendTransaction)
        .expect("omitted optional scopes are broad wildcards");
}

#[test]
fn app_runtime_requires_declared_entrypoint() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("beam-apps/fixtures/valid/apps/uniswap/1.0.0/module.wasm");

    validate_wasm_module("uniswap", "beam_app_main", &path).expect("valid app wasm");
    validate_wasm_module("uniswap", "missing_entrypoint", &path)
        .expect_err("reject missing entrypoint");
}

#[test]
fn approval_integrity_rejects_tampered_plan() {
    let mut plan = action_plan();
    let plan_hash = plan_hash(&plan).expect("hash plan");
    plan.command = "swap USDC ETH 11".to_string();
    let approval = ApprovalRecord {
        id: "apr_test".to_string(),
        status: ApprovalStatus::Pending,
        plan,
        plan_hash,
        created_at: now(),
        updated_at: now(),
    };

    ensure_approval_executable(&approval).expect_err("reject tampered plan");
}

#[test]
fn action_plan_bindings_capture_continuation_inputs() {
    let plan = action_plan();

    assert!(
        plan.bindings
            .iter()
            .any(|binding| binding.key == "quote_id")
    );
    assert!(
        plan.bindings
            .iter()
            .any(|binding| binding.key == "swap_calldata_hash")
    );
    assert!(plan.bindings.iter().any(|binding| binding.key == "router"));
}

fn action_plan() -> ActionPlan {
    ActionPlan {
        app_id: "uniswap".to_string(),
        app_version: "1.0.0".to_string(),
        wasm_sha256: "sha256:wasm".to_string(),
        manifest_sha256: "sha256:manifest".to_string(),
        command: "swap USDC ETH 10".to_string(),
        wallet: Some("0x1111111111111111111111111111111111111111".to_string()),
        chain: "base".to_string(),
        steps: Vec::new(),
        bindings: vec![
            ActionBinding {
                key: "quote_id".to_string(),
                value: "quote-1".to_string(),
            },
            ActionBinding {
                key: "swap_calldata_hash".to_string(),
                value: "sha256:data".to_string(),
            },
            ActionBinding {
                key: "router".to_string(),
                value: "0x2222222222222222222222222222222222222222".to_string(),
            },
        ],
        constraints: Vec::new(),
        expires_at: now() + 60,
    }
}
