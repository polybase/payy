use ethabi::{
    ParamType, Token, decode, encode,
    ethereum_types::{Address, U256},
};
use sha3::{Digest, Keccak256};

use crate::{Error, Result, host::LogEntry};

pub const AGENT_WALLET_DOMAIN_NAME: &str = "ERC8004IdentityRegistry";
pub const AGENT_WALLET_DOMAIN_VERSION: &str = "1";
pub const REGISTERED_EVENT_SIGNATURE: &str = "Registered(uint256,string,address)";

pub fn selector(signature: &str) -> String {
    let hash = keccak(signature.as_bytes());
    format!("0x{}", hex::encode(&hash[..4]))
}

pub fn register_calldata(uri: Option<&str>) -> String {
    match uri {
        Some(uri) => calldata("register(string)", &[Token::String(uri.to_string())]),
        None => calldata("register()", &[]),
    }
}

pub fn set_uri_calldata(agent_id: U256, uri: &str) -> String {
    calldata(
        "setAgentURI(uint256,string)",
        &[Token::Uint(agent_id), Token::String(uri.to_string())],
    )
}

pub fn unset_wallet_calldata(agent_id: U256) -> String {
    calldata("unsetAgentWallet(uint256)", &[Token::Uint(agent_id)])
}

pub fn set_wallet_calldata(
    agent_id: U256,
    wallet: Address,
    deadline: u64,
    signature: &str,
) -> Result<String> {
    let signature = hex_bytes(signature)?;
    Ok(calldata(
        "setAgentWallet(uint256,address,uint256,bytes)",
        &[
            Token::Uint(agent_id),
            Token::Address(wallet),
            Token::Uint(U256::from(deadline)),
            Token::Bytes(signature),
        ],
    ))
}

pub fn owner_of_calldata(agent_id: U256) -> String {
    calldata("ownerOf(uint256)", &[Token::Uint(agent_id)])
}

pub fn token_uri_calldata(agent_id: U256) -> String {
    calldata("tokenURI(uint256)", &[Token::Uint(agent_id)])
}

pub fn get_agent_wallet_calldata(agent_id: U256) -> String {
    calldata("getAgentWallet(uint256)", &[Token::Uint(agent_id)])
}

pub fn decode_address(raw: &str) -> Result<Address> {
    let tokens = decode(&[ParamType::Address], &hex_bytes(raw)?).map_err(|err| {
        Error::InvalidHostResponse {
            reason: format!("{err:?}"),
        }
    })?;
    match tokens.as_slice() {
        [Token::Address(value)] => Ok(*value),
        _ => Err(Error::InvalidHostResponse {
            reason: "address response had wrong ABI shape".to_string(),
        }),
    }
}

pub fn decode_string(raw: &str) -> Result<String> {
    let tokens = decode(&[ParamType::String], &hex_bytes(raw)?).map_err(|err| {
        Error::InvalidHostResponse {
            reason: format!("{err:?}"),
        }
    })?;
    match tokens.as_slice() {
        [Token::String(value)] => Ok(value.clone()),
        _ => Err(Error::InvalidHostResponse {
            reason: "string response had wrong ABI shape".to_string(),
        }),
    }
}

pub fn registered_topic() -> String {
    format!(
        "0x{}",
        hex::encode(keccak(REGISTERED_EVENT_SIGNATURE.as_bytes()))
    )
}

pub fn parse_registered_event(log: &LogEntry, registry: &str) -> Option<RegisteredEvent> {
    if !log.address.eq_ignore_ascii_case(registry) || log.topics.len() != 3 {
        return None;
    }
    if log.topics.first()?.to_ascii_lowercase() != registered_topic() {
        return None;
    }

    let agent_id = parse_u256_word(log.topics.get(1)?)?;
    let owner = address_from_topic(log.topics.get(2)?)?;
    let uri = decode(&[ParamType::String], &hex_bytes(&log.data).ok()?)
        .ok()
        .and_then(|tokens| match tokens.as_slice() {
            [Token::String(value)] => Some(value.clone()),
            _ => None,
        })?;

    Some(RegisteredEvent {
        agent_id,
        owner,
        uri,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredEvent {
    pub agent_id: U256,
    pub owner: Address,
    pub uri: String,
}

pub fn agent_wallet_hashes(
    chain_id: u64,
    verifying_contract: Address,
    agent_id: U256,
    new_wallet: Address,
    owner: Address,
    deadline: u64,
) -> (String, String) {
    let domain_separator = keccak(&encode(&[
        bytes32_token(&keccak(
            b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
        )),
        bytes32_token(&keccak(AGENT_WALLET_DOMAIN_NAME.as_bytes())),
        bytes32_token(&keccak(AGENT_WALLET_DOMAIN_VERSION.as_bytes())),
        Token::Uint(U256::from(chain_id)),
        Token::Address(verifying_contract),
    ]));
    let struct_hash = keccak(&encode(&[
        bytes32_token(&keccak(
            b"AgentWalletSet(uint256 agentId,address newWallet,address owner,uint256 deadline)",
        )),
        Token::Uint(agent_id),
        Token::Address(new_wallet),
        Token::Address(owner),
        Token::Uint(U256::from(deadline)),
    ]));

    (
        format!("0x{}", hex::encode(domain_separator)),
        format!("0x{}", hex::encode(struct_hash)),
    )
}

pub fn parse_address(value: &str) -> Result<Address> {
    value.parse::<Address>().map_err(|_| Error::InvalidAddress {
        value: value.to_string(),
    })
}

pub fn parse_agent_id(value: &str) -> Result<U256> {
    if let Some(value) = value.strip_prefix("0x") {
        return U256::from_str_radix(value, 16).map_err(|_| Error::InvalidAgentId {
            value: format!("0x{value}"),
        });
    }
    value.parse::<U256>().map_err(|_| Error::InvalidAgentId {
        value: value.to_string(),
    })
}

pub fn address_hex(address: Address) -> String {
    format!("{address:#x}")
}

pub fn calldata_hash(data: &str) -> String {
    format!(
        "sha256:{}",
        hex::encode(sha2::Sha256::digest(data.as_bytes()))
    )
}

fn calldata(signature: &str, tokens: &[Token]) -> String {
    let selector = selector(signature);
    format!("{selector}{}", hex::encode(encode(tokens)))
}

fn bytes32_token(bytes: &[u8; 32]) -> Token {
    Token::FixedBytes(bytes.to_vec())
}

fn keccak(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn hex_bytes(value: &str) -> Result<Vec<u8>> {
    hex::decode(value.strip_prefix("0x").unwrap_or(value)).map_err(|_| Error::InvalidHostResponse {
        reason: format!("invalid hex value {value}"),
    })
}

fn parse_u256_word(value: &str) -> Option<U256> {
    let bytes = hex::decode(value.strip_prefix("0x").unwrap_or(value)).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    Some(U256::from_big_endian(&bytes))
}

fn address_from_topic(value: &str) -> Option<Address> {
    let bytes = hex::decode(value.strip_prefix("0x").unwrap_or(value)).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    Some(Address::from_slice(&bytes[12..]))
}

pub type AgentId = U256;
pub type EvmAddress = Address;
