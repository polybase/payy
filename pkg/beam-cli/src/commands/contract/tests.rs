// lint-long-file-override allow-max-lines=280
mod export;

use serde_json::json;
use sourcify_interface::{
    CompilationSummary, ContractRecord, ContractResponse, MatchState, SourceFile,
};

use super::{
    error::{Error, RuntimeUnchecked},
    info::{failed_proxy, source_summary_output, sourcify_status_output, verified_output},
    proxy::{ProxyImplementation, ProxyInfo, ProxyStatus},
    source::{SourceMatchKind, match_source_path},
    target::{BytecodeInfo, build_target_from_entry},
};
use crate::{chains::ChainEntry, error::Error as BeamError};

const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

#[test]
fn parses_literal_address_and_checksums_output() {
    let target = build_target_from_entry(chain(), "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")
        .expect("target");

    assert_eq!(
        target.checksum_address,
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    );
    assert_eq!(
        target.input_address,
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
    );
}

#[test]
fn rejects_non_literal_contract_addresses() {
    let err = build_target_from_entry(chain(), "1111111111111111111111111111111111111111")
        .expect_err("missing 0x");
    assert!(matches!(err, Error::InvalidContractAddress { .. }));

    let err = build_target_from_entry(chain(), "alice.eth").expect_err("ens is not accepted");
    assert!(matches!(err, Error::InvalidContractAddress { .. }));
}

#[test]
fn source_path_matching_uses_exact_then_unique_basename() {
    let sources = source_map(&[
        ("contracts/Foo.sol", "contract Foo {}"),
        ("Bar.sol", "contract Bar {}"),
    ]);

    let exact = match_source_path(&sources, "contracts/Foo.sol").expect("exact match");
    assert_eq!(exact.kind, SourceMatchKind::Exact);
    assert_eq!(exact.content, "contract Foo {}");

    let basename = match_source_path(&sources, "Bar.sol").expect("basename match");
    assert_eq!(basename.kind, SourceMatchKind::Exact);

    let basename = match_source_path(&sources, "Foo.sol").expect("unique basename match");
    assert_eq!(basename.kind, SourceMatchKind::Basename);
}

#[test]
fn source_path_matching_rejects_ambiguous_basenames() {
    let sources = source_map(&[
        ("contracts/Foo.sol", "contract Foo {}"),
        ("lib/Foo.sol", "contract Foo2 {}"),
    ]);

    let err = match_source_path(&sources, "Foo.sol").expect_err("ambiguous basename");
    assert!(matches!(err, Error::SourcePathAmbiguous { .. }));
}

#[test]
fn info_renders_failed_proxy_lookup_distinctly() {
    let target = build_target_from_entry(chain(), ADDRESS).expect("target");
    let response = contract_response();
    let bytecode = BytecodeInfo {
        byte_len: 1,
        code_hash: "0x00".to_owned(),
        hex: "0x01".to_owned(),
    };
    let proxy = ProxyInfo {
        implementations: Vec::new(),
        proxy_type: None,
        status: ProxyStatus::Failed,
    };

    let output = verified_output(&target, &bytecode, &response.contract, &proxy);

    assert!(output.default.contains("Proxy: lookup failed"));
    assert!(!output.default.contains("Proxy: no"));
}

#[test]
fn info_runtime_not_verified_preserves_valid_proxy_data() {
    let target = build_target_from_entry(chain(), ADDRESS).expect("target");
    let mut response = contract_response();
    response.contract.runtime_match = None;
    let bytecode = BytecodeInfo {
        byte_len: 1,
        code_hash: "0x00".to_owned(),
        hex: "0x01".to_owned(),
    };
    let proxy = ProxyInfo {
        implementations: vec![ProxyImplementation {
            address: "0x2222222222222222222222222222222222222222".to_owned(),
            name: Some("Implementation".to_owned()),
        }],
        proxy_type: Some("EIP1967Proxy".to_owned()),
        status: ProxyStatus::Resolved,
    };

    let output = sourcify_status_output(&target, &bytecode, "runtime_not_verified", None, &proxy);

    assert_eq!(output.value["proxy"]["status"], json!("resolved"));
    assert_eq!(
        output.value["proxy"]["implementations"][0]["address"],
        json!("0x2222222222222222222222222222222222222222")
    );
}

#[test]
fn info_not_verified_reports_requested_proxy_lookup_failed() {
    let target = build_target_from_entry(chain(), ADDRESS).expect("target");
    let bytecode = BytecodeInfo {
        byte_len: 1,
        code_hash: "0x00".to_owned(),
        hex: "0x01".to_owned(),
    };

    let output = sourcify_status_output(&target, &bytecode, "not_verified", None, &failed_proxy());

    assert_eq!(output.value["proxy"]["status"], json!("failed"));
}

#[test]
fn human_summaries_sanitize_sourcify_control_characters() {
    let target = build_target_from_entry(chain(), ADDRESS).expect("target");
    let mut response = contract_response();
    response.contract.compilation = Some(CompilationSummary {
        compiler: Some("solc\nspoof".to_owned()),
        language: Some("Solidity\tInjected".to_owned()),
        contract_name: Some("Foo\rBar".to_owned()),
    });
    response.contract.verified_at = Some("2024-01-01\nBad".to_owned());
    let bytecode = BytecodeInfo {
        byte_len: 1,
        code_hash: "0x00".to_owned(),
        hex: "0x01".to_owned(),
    };
    let proxy = ProxyInfo {
        implementations: vec![ProxyImplementation {
            address: "0x2222222222222222222222222222222222222222".to_owned(),
            name: Some("Impl\nName".to_owned()),
        }],
        proxy_type: Some("Proxy\tType".to_owned()),
        status: ProxyStatus::Resolved,
    };

    let info_output = verified_output(&target, &bytecode, &response.contract, &proxy);
    assert!(info_output.default.contains("Contract: Foo Bar"));
    assert!(info_output.default.contains("Language: Solidity Injected"));
    assert!(info_output.default.contains("Compiler: solc spoof"));
    assert!(info_output.default.contains("Verified at: 2024-01-01 Bad"));
    assert!(info_output.default.contains("Proxy type: Proxy Type"));
    assert!(
        info_output
            .default
            .contains("Implementation name: Impl Name")
    );
    assert!(!info_output.default.contains("Foo\rBar"));
    assert!(!info_output.default.contains("Impl\nName"));

    let source_output = source_summary_output(
        &target,
        &response.contract,
        vec!["contracts/Foo\nBar.sol".to_owned()],
        &proxy,
    );
    assert!(source_output.default.contains("contracts/Foo Bar.sol"));
    assert_eq!(
        source_output.value["files"][0],
        json!("contracts/Foo\nBar.sol")
    );
}

#[test]
fn sourcify_miss_errors_carry_unchecked_rpc_context() {
    let err: BeamError = Error::SourcifyNotVerified {
        address: ADDRESS.to_owned(),
        artifact: "ABI".to_owned(),
        runtime_unchecked: Some(RuntimeUnchecked {
            reason: Some("connection refused".to_owned()),
        }),
    }
    .into();
    let message = err.to_string();

    assert!(message.contains("runtime code was not checked"));
    assert!(message.contains("connection refused"));
}

#[test]
fn sourcify_miss_errors_carry_no_rpc_unchecked_context() {
    let err: BeamError = Error::SourcifyNotVerified {
        address: ADDRESS.to_owned(),
        artifact: "Source".to_owned(),
        runtime_unchecked: Some(RuntimeUnchecked { reason: None }),
    }
    .into();
    let message = err.to_string();

    assert!(message.contains("runtime code was not checked"));
    assert!(!message.contains("RPC check failed"));
}

fn chain() -> ChainEntry {
    ChainEntry {
        aliases: Vec::new(),
        chain_id: 1,
        display_name: "Ethereum".to_owned(),
        is_builtin: true,
        key: "ethereum".to_owned(),
        native_symbol: "ETH".to_owned(),
        privacy: None,
    }
}

fn contract_response() -> ContractResponse {
    ContractResponse {
        endpoint:
            "https://sourcify.dev/server/v2/contract/1/0x1111111111111111111111111111111111111111"
                .to_owned(),
        requested_fields: vec!["sources".to_owned()],
        contract: ContractRecord {
            chain_id: "1".to_owned(),
            address: ADDRESS.to_owned(),
            match_state: MatchState::ExactMatch,
            creation_match: Some(MatchState::Match),
            runtime_match: Some(MatchState::ExactMatch),
            verified_at: Some("2024-08-08T13:20:07Z".to_owned()),
            abi: Some(Vec::new()),
            sources: Some(source_map(&[("contracts/Foo.sol", "contract Foo {}\n")])),
            metadata: Some(json!({"compiler": "solc"})),
            standard_json_input: Some(json!({"language": "Solidity"})),
            compilation: None,
            proxy_resolution: None,
        },
    }
}

fn source_map(items: &[(&str, &str)]) -> std::collections::BTreeMap<String, SourceFile> {
    items
        .iter()
        .map(|(path, content)| {
            (
                (*path).to_owned(),
                SourceFile {
                    content: (*content).to_owned(),
                },
            )
        })
        .collect()
}
