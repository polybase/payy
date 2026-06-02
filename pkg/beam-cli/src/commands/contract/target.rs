// lint-long-file-override allow-max-lines=250
use contracts::{Address, Client};
use web3::{
    signing::keccak256,
    types::{BlockNumber, Bytes},
};

use crate::{chains::ChainEntry, error::Error as BeamError, runtime::BeamApp};

use super::error::{Error, Result};

#[derive(Clone, Debug)]
pub(super) struct InspectionTarget {
    pub(super) address: Address,
    pub(super) chain: String,
    pub(super) chain_id: u64,
    pub(super) checksum_address: String,
    pub(super) entry: ChainEntry,
    pub(super) input_address: String,
}

#[derive(Clone, Debug)]
pub(super) struct BytecodeInfo {
    pub(super) byte_len: usize,
    pub(super) code_hash: String,
    pub(super) hex: String,
}

#[derive(Clone, Debug)]
pub(super) struct ContractBlock {
    pub(super) number: BlockNumber,
    pub(super) selector: String,
}

#[derive(Clone, Debug)]
pub(super) enum RpcProbe {
    RuntimeCode,
    NoRuntimeCode,
    Unchecked { reason: Option<String> },
}

pub(super) async fn build_target(app: &BeamApp, input_address: &str) -> Result<InspectionTarget> {
    let entry = app
        .active_chain_entry()
        .await
        .map_err(map_rpc_lookup_error)?;
    build_target_from_entry(entry, input_address)
}

pub(super) fn build_target_from_entry(
    entry: ChainEntry,
    input_address: &str,
) -> Result<InspectionTarget> {
    let address = parse_literal_address(input_address)?;
    let checksum_address = checksum_address(address);

    Ok(InspectionTarget {
        address,
        chain: entry.key.clone(),
        chain_id: entry.chain_id,
        checksum_address,
        entry,
        input_address: input_address.to_owned(),
    })
}

pub(super) async fn required_rpc_client(
    app: &BeamApp,
    target: &InspectionTarget,
) -> Result<Client> {
    let rpc_url = app
        .active_rpc_url_for_chain(&target.entry)
        .await
        .map_err(map_rpc_lookup_error)?;
    let client = Client::try_new(&rpc_url, None).map_err(|_| Error::RpcLookupFailed {
        reason: "invalid RPC URL".to_owned(),
    })?;
    validate_rpc_chain_id(&client, target).await?;

    Ok(client)
}

pub(super) async fn optional_rpc_probe(
    app: &BeamApp,
    target: &InspectionTarget,
) -> Result<RpcProbe> {
    let rpc_url = match app.active_rpc_url_for_chain(&target.entry).await {
        Ok(rpc_url) => rpc_url,
        Err(BeamError::NoRpcConfigured { .. }) => {
            return Ok(RpcProbe::Unchecked { reason: None });
        }
        Err(err) => {
            return Ok(RpcProbe::Unchecked {
                reason: Some(err.to_string()),
            });
        }
    };
    let Ok(client) = Client::try_new(&rpc_url, None) else {
        return Ok(RpcProbe::Unchecked {
            reason: Some("invalid RPC URL".to_owned()),
        });
    };
    if let Err(err) = validate_rpc_chain_id(&client, target).await {
        return rpc_probe_error(err);
    }
    let code = match fetch_bytecode(&client, target.address, BlockNumber::Latest).await {
        Ok(code) => code,
        Err(err) => return rpc_probe_error(err),
    };
    if code.byte_len == 0 {
        return Ok(RpcProbe::NoRuntimeCode);
    }

    Ok(RpcProbe::RuntimeCode)
}

fn rpc_probe_error(err: Error) -> Result<RpcProbe> {
    match err {
        Error::RpcChainMismatch {
            actual,
            chain,
            expected,
        } => Err(Error::RpcChainMismatch {
            actual,
            chain,
            expected,
        }),
        Error::RpcLookupFailed { reason } => Ok(RpcProbe::Unchecked {
            reason: Some(reason),
        }),
        err => Ok(RpcProbe::Unchecked {
            reason: Some(err.to_string()),
        }),
    }
}

pub(super) async fn validate_rpc_chain_id(
    client: &Client,
    target: &InspectionTarget,
) -> Result<()> {
    let actual = client
        .chain_id_contracts()
        .await
        .map_err(|err| Error::RpcLookupFailed {
            reason: err.to_string(),
        })?
        .low_u64();
    if actual != target.chain_id {
        return Err(Error::RpcChainMismatch {
            actual,
            chain: target.chain.clone(),
            expected: target.chain_id,
        });
    }

    Ok(())
}

pub(super) async fn fetch_bytecode(
    client: &Client,
    address: Address,
    block: BlockNumber,
) -> Result<BytecodeInfo> {
    let bytes = client
        .client()
        .eth()
        .code(address, Some(block))
        .await
        .map_err(|err| Error::RpcLookupFailed {
            reason: err.to_string(),
        })?;

    Ok(bytecode_info(bytes))
}

pub(super) fn parse_block(value: Option<&str>) -> Result<ContractBlock> {
    let value = value.unwrap_or("latest");
    let number = match value {
        "latest" => BlockNumber::Latest,
        "pending" => BlockNumber::Pending,
        "safe" => BlockNumber::Safe,
        "finalized" => BlockNumber::Finalized,
        value => {
            let number = value.parse::<u64>().map_err(|_| Error::RpcLookupFailed {
                reason: format!("invalid block selector: {value}"),
            })?;
            BlockNumber::Number(number.into())
        }
    };

    Ok(ContractBlock {
        number,
        selector: value.to_owned(),
    })
}

pub(super) fn bytecode_info(bytes: Bytes) -> BytecodeInfo {
    let raw = bytes.0;
    BytecodeInfo {
        byte_len: raw.len(),
        code_hash: format!("0x{}", hex::encode(keccak256(&raw))),
        hex: format!("0x{}", hex::encode(raw)),
    }
}

fn parse_literal_address(value: &str) -> Result<Address> {
    if value.len() != 42
        || !value.starts_with("0x")
        || !value[2..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(Error::InvalidContractAddress {
            value: value.to_owned(),
        });
    }
    let bytes = hex::decode(&value[2..]).map_err(|_| Error::InvalidContractAddress {
        value: value.to_owned(),
    })?;

    Ok(Address::from_slice(&bytes))
}

pub(super) fn checksum_address(address: Address) -> String {
    let lower = hex::encode(address.as_bytes());
    let hash = keccak256(lower.as_bytes());
    let mut checksum = String::with_capacity(42);
    checksum.push_str("0x");

    for (index, byte) in lower.bytes().enumerate() {
        let hash_byte = hash[index / 2];
        let nibble = if index.is_multiple_of(2) {
            hash_byte >> 4
        } else {
            hash_byte & 0x0f
        };
        if byte.is_ascii_alphabetic() && nibble >= 8 {
            checksum.push((byte as char).to_ascii_uppercase());
        } else {
            checksum.push(byte as char);
        }
    }

    checksum
}

fn map_rpc_lookup_error(err: BeamError) -> Error {
    Error::RpcLookupFailed {
        reason: err.to_string(),
    }
}
