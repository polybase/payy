use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub type KnownTokensByChain = BTreeMap<String, BTreeMap<String, KnownToken>>;
pub type TrackedTokensByChain = BTreeMap<String, Vec<String>>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnownToken {
    pub address: String,
    pub decimals: u8,
    pub label: String,
}

pub fn default_known_tokens() -> KnownTokensByChain {
    let mut chains = BTreeMap::new();

    add_token(
        &mut chains,
        "ethereum",
        "USDC",
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        6,
    );
    add_token(
        &mut chains,
        "ethereum",
        "USDT",
        "0xdac17f958d2ee523a2206206994597c13d831ec7",
        6,
    );
    add_token(
        &mut chains,
        "base",
        "USDC",
        "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        6,
    );
    add_token(
        &mut chains,
        "polygon",
        "USDC",
        "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359",
        6,
    );
    add_token(
        &mut chains,
        "bnb",
        "USDC",
        "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d",
        18,
    );
    add_token(
        &mut chains,
        "bnb",
        "USDT",
        "0x55d398326f99059ff775485246999027b3197955",
        18,
    );
    add_token(
        &mut chains,
        "arbitrum",
        "USDC",
        "0xaf88d065e77c8cc2239327c5edb3a432268e5831",
        6,
    );
    add_token(
        &mut chains,
        "arbitrum",
        "USDT",
        "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9",
        6,
    );
    add_token(
        &mut chains,
        "sepolia",
        "USDC",
        "0x1c7d4b196cb0c7b01d743fbc6116a902379c7238",
        6,
    );
    add_token(
        &mut chains,
        "hardhat",
        "USDC",
        "0x5fbdb2315678afecb367f032d93f642f64180aa3",
        6,
    );

    chains
}

pub fn default_tracked_tokens() -> TrackedTokensByChain {
    default_known_tokens()
        .into_iter()
        .map(|(chain, tokens)| (chain, tokens.into_keys().collect()))
        .collect()
}

pub fn token_label_key(label: &str) -> String {
    label.trim().to_ascii_uppercase()
}

fn add_token(
    chains: &mut KnownTokensByChain,
    chain: &str,
    label: &str,
    address: &str,
    decimals: u8,
) {
    chains.entry(chain.to_string()).or_default().insert(
        label.to_string(),
        KnownToken {
            address: address.to_string(),
            decimals,
            label: label.to_string(),
        },
    );
}
