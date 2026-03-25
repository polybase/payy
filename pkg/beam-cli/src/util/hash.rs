// lint-long-file-override allow-max-lines=300
use contracts::{Address, U256};
use rlp::RlpStream;
use web3::{
    ethabi::{ParamType, encode},
    signing::{keccak256, namehash},
};

use crate::{
    abi::{read_param_type, tokenize_param},
    error::{Error, Result},
    runtime::parse_address,
    util::{bytes::decode_hex, numbers::parse_u256_value},
};

const CREATE2_DEPLOYER: &str = "0x4e59b44847b379578588920ca78fbf26c0b4956c";

pub fn address_zero() -> String {
    checksum_address(Address::zero(), None)
}

pub fn checksum_address(address: Address, chain_id: Option<u64>) -> String {
    let lower = hex::encode(address.as_bytes());
    let hash_input = match chain_id {
        Some(chain_id) => format!("{chain_id}0x{lower}"),
        None => lower.clone(),
    };
    let hash = keccak256(hash_input.as_bytes());
    let mut checksummed = String::with_capacity(42);
    checksummed.push_str("0x");

    for (index, ch) in lower.chars().enumerate() {
        if ch.is_ascii_digit() {
            checksummed.push(ch);
            continue;
        }

        let hash_byte = hash[index / 2];
        let nibble = if index % 2 == 0 {
            (hash_byte >> 4) & 0x0f
        } else {
            hash_byte & 0x0f
        };

        if nibble >= 8 {
            checksummed.push(ch.to_ascii_uppercase());
        } else {
            checksummed.push(ch);
        }
    }

    checksummed
}

pub fn compute_address(
    address: Option<&str>,
    nonce: Option<&str>,
    salt: Option<&str>,
    init_code: Option<&str>,
    init_code_hash: Option<&str>,
) -> Result<String> {
    let deployer = parse_address(address.ok_or_else(|| Error::MissingUtilInput {
        command: "compute-address <address>".to_string(),
    })?)?;

    if let Some(salt) = salt {
        return create2_address(
            Some(&format!("{deployer:#x}")),
            Some(salt),
            init_code,
            init_code_hash,
        );
    }

    let nonce = parse_u256_value(nonce.ok_or_else(|| Error::MissingUtilInput {
        command: "compute-address --nonce".to_string(),
    })?)?;
    let address = compute_create_address(deployer, nonce);
    Ok(checksum_address(address, None))
}

pub fn create2_address(
    deployer: Option<&str>,
    salt: Option<&str>,
    init_code: Option<&str>,
    init_code_hash: Option<&str>,
) -> Result<String> {
    let deployer = parse_address(deployer.unwrap_or(CREATE2_DEPLOYER))?;
    let salt = parse_fixed_32(salt.ok_or_else(|| Error::MissingUtilInput {
        command: "create2 --salt".to_string(),
    })?)?;
    let init_code_hash = resolve_init_code_hash(init_code, init_code_hash)?;
    let address = compute_create2_address(deployer, salt, init_code_hash);
    Ok(checksum_address(address, None))
}

pub fn erc7201_index(id: &str) -> String {
    let namespace = U256::from_big_endian(&keccak256(id.as_bytes()));
    let slot = namespace.overflowing_sub(U256::from(1u8)).0;
    let mut preimage = [0u8; 32];
    slot.to_big_endian(&mut preimage);
    let mut out = keccak256(&preimage);
    out[31] = 0u8;
    format!("0x{}", hex::encode(out))
}

pub fn hash_message(value: &str) -> String {
    let mut payload = format!("\x19Ethereum Signed Message:\n{}", value.len()).into_bytes();
    payload.extend_from_slice(value.as_bytes());
    format!("0x{}", hex::encode(keccak256(&payload)))
}

pub fn hash_zero() -> String {
    format!("0x{}", "0".repeat(64))
}

pub fn keccak_hex(value: &str) -> Result<String> {
    let bytes = if value.trim_start().starts_with("0x") {
        decode_hex(value)?
    } else {
        value.as_bytes().to_vec()
    };

    Ok(format!("0x{}", hex::encode(keccak256(&bytes))))
}

pub fn mapping_index(key_type: &str, key: &str, slot_number: &str) -> Result<String> {
    let kind = read_param_type(key_type.trim(), key_type)?;
    let key_bytes = mapping_key_bytes(&kind, key)?;
    let slot = parse_u256_value(slot_number)?;
    let mut slot_bytes = [0u8; 32];
    slot.to_big_endian(&mut slot_bytes);

    let mut preimage = key_bytes;
    preimage.extend_from_slice(&slot_bytes);
    Ok(format!("0x{}", hex::encode(keccak256(&preimage))))
}

pub fn namehash_hex(value: &str) -> String {
    format!("0x{}", hex::encode(namehash(value)))
}

pub fn selector(signature: &str) -> String {
    let hash = keccak256(signature.as_bytes());
    format!("0x{}", hex::encode(&hash[..4]))
}

pub fn selector_event(signature: &str) -> String {
    format!("0x{}", hex::encode(keccak256(signature.as_bytes())))
}

fn compute_create2_address(deployer: Address, salt: [u8; 32], init_code_hash: [u8; 32]) -> Address {
    let mut payload = Vec::with_capacity(85);
    payload.push(0xff);
    payload.extend_from_slice(deployer.as_bytes());
    payload.extend_from_slice(&salt);
    payload.extend_from_slice(&init_code_hash);
    let hash = keccak256(&payload);
    Address::from_slice(&hash[12..])
}

fn compute_create_address(deployer: Address, nonce: U256) -> Address {
    let mut stream = RlpStream::new_list(2);
    stream.append(&deployer.as_bytes());
    stream.append(&trimmed_u256_bytes(&nonce));
    let hash = keccak256(&stream.out());
    Address::from_slice(&hash[12..])
}

fn resolve_init_code_hash(
    init_code: Option<&str>,
    init_code_hash: Option<&str>,
) -> Result<[u8; 32]> {
    if let Some(init_code_hash) = init_code_hash {
        return parse_fixed_32(init_code_hash);
    }

    let init_code = decode_hex(init_code.ok_or_else(|| Error::MissingUtilInput {
        command: "create2 --init-code|--init-code-hash".to_string(),
    })?)?;

    Ok(keccak256(&init_code))
}

fn mapping_key_bytes(kind: &ParamType, key: &str) -> Result<Vec<u8>> {
    match kind {
        ParamType::String => Ok(key.as_bytes().to_vec()),
        ParamType::Bytes => decode_hex(key),
        _ => {
            let token = tokenize_param(kind, key)?;
            let encoded = encode(&[token]);
            if encoded.len() != 32 {
                return Err(Error::InvalidFunctionSignature {
                    signature: key.to_string(),
                });
            }
            Ok(encoded)
        }
    }
}

fn parse_fixed_32(value: &str) -> Result<[u8; 32]> {
    let bytes = decode_hex(value)?;
    if bytes.len() != 32 {
        return Err(Error::InvalidHexData {
            value: value.to_string(),
        });
    }

    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn trimmed_u256_bytes(value: &U256) -> Vec<u8> {
    if value.is_zero() {
        return Vec::new();
    }

    let mut bytes = [0u8; 32];
    value.to_big_endian(&mut bytes);
    let start = bytes.iter().position(|byte| *byte != 0).unwrap_or(31);
    bytes[start..].to_vec()
}
