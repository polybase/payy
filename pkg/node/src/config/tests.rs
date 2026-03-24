use std::{io::Write, sync::Mutex};

use clap::Parser;
use tempfile::NamedTempFile;

use crate::Error;

use super::*;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

const TEST_ENV_VARS: [&str; 6] = [
    "POLY_CHAINS__0__CHAIN_ID",
    "POLY_CHAINS__0__ETH_RPC_URL",
    "POLY_CHAINS__0__ROLLUP_CONTRACT_ADDR",
    "POLY_CHAINS__0__SAFE_ETH_HEIGHT_OFFSET",
    "POLY_ETH_RPC_URL",
    "POLY_ROLLUP_CONTRACT_ADDR",
];

#[test]
fn can_parse_from_empty() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let _guard = EnvGuard::new(&TEST_ENV_VARS);
    let args = CliArgs::try_parse_from(["node"]).unwrap();
    let config = Config::from_env(args).unwrap();
    assert!(!config.chains.is_empty());
    assert_eq!(config.primary_chain_id(), Some(137));
}

#[test]
fn legacy_single_chain_translates_to_chains_vec() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let _guard = EnvGuard::new(&TEST_ENV_VARS);
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
env-name = "dev"
eth-rpc-url = "http://legacy.example"
rollup-contract-addr = "0x1234567890abcdef1234567890abcdef12345678"
"#
    )
    .unwrap();

    let args =
        CliArgs::try_parse_from(["node", "--config-path", file.path().to_str().unwrap()]).unwrap();

    let config = Config::from_env(args).unwrap();
    assert_eq!(config.chains.len(), 1);
    let chain = &config.chains[0];
    assert_eq!(chain.chain_id, 137);
    assert_eq!(chain.eth_rpc_url, "http://legacy.example");
    assert_eq!(
        chain.rollup_contract_addr,
        "0x1234567890abcdef1234567890abcdef12345678"
    );
}

#[test]
fn error_when_chains_are_missing() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let _guard = EnvGuard::new(&TEST_ENV_VARS);
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
chains = []
eth-rpc-url = "http://legacy.example"
rollup-contract-addr = "0x1234567890abcdef1234567890abcdef12345678"
"#
    )
    .unwrap();

    let args =
        CliArgs::try_parse_from(["node", "--config-path", file.path().to_str().unwrap()]).unwrap();

    let err = Config::from_env(args).unwrap_err();
    assert!(matches!(err, Error::MissingPrimaryChainConfig));
}

#[test]
fn env_chain_map_translates_to_vec() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let _guard = EnvGuard::new(&TEST_ENV_VARS);
    set_env_var("POLY_CHAINS__0__CHAIN_ID", "999");
    set_env_var("POLY_CHAINS__0__ETH_RPC_URL", "http://env-chain.example");
    set_env_var(
        "POLY_CHAINS__0__ROLLUP_CONTRACT_ADDR",
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    set_env_var("POLY_CHAINS__0__SAFE_ETH_HEIGHT_OFFSET", "42");
    set_env_var("POLY_ETH_RPC_URL", "http://env-chain.example");
    set_env_var(
        "POLY_ROLLUP_CONTRACT_ADDR",
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );

    let args = CliArgs::try_parse_from(["node"]).unwrap();
    let config = Config::from_env(args).unwrap();
    assert_eq!(config.chains.len(), 1);
    let chain = &config.chains[0];
    assert_eq!(chain.chain_id, 999);
    assert_eq!(chain.eth_rpc_url, "http://env-chain.example");
    assert_eq!(
        chain.rollup_contract_addr,
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert_eq!(chain.safe_eth_height_offset, 42);
}

#[test]
fn enable_rollup_worker_config() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let mut vars = TEST_ENV_VARS.to_vec();
    vars.push("POLY_ENABLE_ROLLUP_WORKER");
    let _guard = EnvGuard::new(&vars);

    // Test default
    let args = CliArgs::try_parse_from(["node"]).unwrap();
    let config = Config::from_env(args).unwrap();
    assert!(config.enable_rollup_worker);

    // Test env override
    set_env_var("POLY_ENABLE_ROLLUP_WORKER", "false");
    let args = CliArgs::try_parse_from(["node"]).unwrap();
    let config = Config::from_env(args).unwrap();
    assert!(!config.enable_rollup_worker);
}

struct EnvGuard {
    values: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    fn new(keys: &[&str]) -> Self {
        let mut values = Vec::with_capacity(keys.len());
        for key in keys {
            values.push(((*key).to_owned(), std::env::var(key).ok()));
            remove_env_var(key);
        }
        Self { values }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.values {
            if let Some(value) = value {
                set_env_var(key, value);
            } else {
                remove_env_var(key);
            }
        }
    }
}

fn set_env_var(key: &str, value: &str) {
    // SAFETY: tests serialize access to environment variables since they run single-threaded.
    unsafe {
        std::env::set_var(key, value);
    }
}

fn remove_env_var(key: &str) {
    // SAFETY: tests serialize access to environment variables since they run single-threaded.
    unsafe {
        std::env::remove_var(key);
    }
}
