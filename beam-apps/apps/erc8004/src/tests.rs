use crate::{
    abi::{
        address_hex, agent_wallet_hashes, parse_address, parse_agent_id, register_calldata,
        selector, set_wallet_calldata,
    },
    args::{Command, ConnectionMode, parse},
    host::{GuestInvocation, HostMetadata, ensure_host_abi},
};

#[test]
fn parses_named_wallet_set_wallet() {
    let args = vec![
        "set-wallet".to_string(),
        "7".to_string(),
        "alice".to_string(),
        "--deadline-seconds".to_string(),
        "120".to_string(),
    ];

    let command = parse(&args).expect("parse set-wallet");

    match command {
        Command::SetWallet(args) => {
            assert_eq!(args.agent_id, "7");
            assert_eq!(args.wallet, "alice");
            assert_eq!(args.deadline_seconds, 120);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn list_defaults_to_owner_mode() {
    let args = vec!["list".to_string()];

    let command = parse(&args).expect("parse list");

    match command {
        Command::List(args) => assert_eq!(args.connection, ConnectionMode::Owner),
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_mutating_commands_with_registry_override() {
    let registry = "0x2222222222222222222222222222222222222222";
    let set_uri = parse(&[
        "set-uri".to_string(),
        "7".to_string(),
        "https://agent.example/new.json".to_string(),
        "--identity-registry".to_string(),
        registry.to_string(),
    ])
    .expect("parse set-uri override");
    match set_uri {
        Command::SetUri(args) => {
            assert_eq!(args.agent_id, "7");
            assert_eq!(args.identity_registry.as_deref(), Some(registry));
            assert_eq!(args.uri, "https://agent.example/new.json");
        }
        other => panic!("unexpected command: {other:?}"),
    }

    let unset_wallet = parse(&[
        "unset-wallet".to_string(),
        "7".to_string(),
        "--identity-registry".to_string(),
        registry.to_string(),
    ])
    .expect("parse unset-wallet override");
    match unset_wallet {
        Command::UnsetWallet(args) => {
            assert_eq!(args.agent_id, "7");
            assert_eq!(args.identity_registry.as_deref(), Some(registry));
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn selectors_match_registry_abi() {
    assert_eq!(selector("register()"), "0x1aa3a008");
    assert_eq!(selector("register(string)"), "0xf2c298be");
    assert_eq!(
        selector("setAgentWallet(uint256,address,uint256,bytes)"),
        "0x2d1ef5ae"
    );
    assert_eq!(selector("getAgentWallet(uint256)"), "0x00339509");
}

#[test]
fn host_abi_requires_signature_and_logs_surface() {
    let invocation = GuestInvocation {
        args: vec!["support".to_string()],
        host_api_version: 1,
        metadata: HostMetadata {
            app_id: "erc8004".to_string(),
            app_version: "1.0.0".to_string(),
            chain: "base".to_string(),
            chain_id: 8453,
            debug: false,
            host_api_version: 1,
            manifest_sha256: "sha256:manifest".to_string(),
            now: 1_000,
            wallet: "0x3333333333333333333333333333333333333333".to_string(),
            wasm_sha256: "sha256:wasm".to_string(),
        },
        output_mode: "default".to_string(),
    };

    ensure_host_abi(&invocation).expect("erc8004 uses the current host api");
}

#[test]
fn encodes_register_and_set_wallet_calldata() {
    let register = register_calldata(Some("https://agent.example/agent.json"));
    assert!(register.starts_with("0xf2c298be"));

    let wallet = parse_address("0x1111111111111111111111111111111111111111").expect("wallet");
    let data = set_wallet_calldata(
        parse_agent_id("1").expect("agent id"),
        wallet,
        42,
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa1b",
    )
    .expect("set wallet calldata");
    assert!(data.starts_with("0x2d1ef5ae"));
}

#[test]
fn hashes_agent_wallet_typed_data() {
    let registry = parse_address("0x8004A818BFB912233c491871b3d84c89A494BD9e").expect("registry");
    let wallet = parse_address("0x1111111111111111111111111111111111111111").expect("wallet");
    let owner = parse_address("0x2222222222222222222222222222222222222222").expect("owner");

    let (domain_separator, struct_hash) = agent_wallet_hashes(
        11155111,
        registry,
        parse_agent_id("1").unwrap(),
        wallet,
        owner,
        300,
    );

    assert!(domain_separator.starts_with("0x"));
    assert_eq!(domain_separator.len(), 66);
    assert!(struct_hash.starts_with("0x"));
    assert_eq!(struct_hash.len(), 66);
    assert_eq!(
        address_hex(wallet),
        "0x1111111111111111111111111111111111111111"
    );
}
