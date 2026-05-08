// lint-long-file-override allow-max-lines=300
use contextful::ResultContextExt;
use contracts::{Address, Client, ERC20Contract, U256};
use web3::{
    ethabi::{Function, StateMutability},
    types::{Bytes, CallRequest, TransactionParameters, TransactionReceipt},
};

pub use crate::units::{format_units, parse_units, validate_unit_decimals};
use crate::{
    abi::{decode_output, encode_input, parse_function, tokens_to_json},
    error::{Error, Result},
    signer::Signer,
    transaction::{TransactionExecution, TransactionStatusUpdate, submit_and_wait},
};

#[derive(Clone, Debug)]
pub struct CallOutcome {
    pub decoded: Option<serde_json::Value>,
    pub raw: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionOutcome {
    pub block_number: Option<u64>,
    pub status: Option<u64>,
    pub tx_hash: String,
}

#[derive(Clone, Debug)]
pub struct FunctionCall<'a> {
    pub args: &'a [String],
    pub contract: Address,
    pub function: &'a Function,
    pub value: U256,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransactionGas {
    pub gas_limit: U256,
    pub gas_price: U256,
}

pub async fn native_balance(client: &Client, address: Address) -> Result<U256> {
    let balance = client
        .eth_balance(address)
        .await
        .context("fetch beam native balance")?;
    Ok(balance)
}

pub async fn erc20_balance(client: &Client, token: Address, owner: Address) -> Result<U256> {
    let contract = ERC20Contract::load(client.clone(), &format!("{token:#x}"))
        .await
        .context("load beam erc20 contract")?;
    let balance = contract
        .balance(owner)
        .await
        .context("fetch beam erc20 balance")?;
    Ok(balance)
}

pub async fn erc20_decimals(client: &Client, token: Address) -> Result<u8> {
    let function = parse_function("decimals():(uint8)", StateMutability::View)?;
    let outcome = call_function(client, None, token, &function, &[]).await?;
    let decoded = outcome
        .decoded
        .ok_or_else(|| Error::InvalidFunctionSignature {
            signature: "decimals():(uint8)".to_string(),
        })?;
    let value = decoded[0]
        .as_str()
        .ok_or_else(|| Error::InvalidFunctionSignature {
            signature: "decimals():(uint8)".to_string(),
        })?
        .parse::<u8>()
        .context("parse beam erc20 decimals")?;
    Ok(value)
}

pub async fn call_function(
    client: &Client,
    from: Option<Address>,
    contract: Address,
    function: &Function,
    args: &[String],
) -> Result<CallOutcome> {
    let data = encode_input(function, args)?;
    let request = CallRequest {
        data: Some(Bytes(data)),
        from,
        to: Some(contract),
        ..Default::default()
    };
    let raw = client
        .eth_call(request, None)
        .await
        .context("execute beam eth_call")?;

    let decoded = if function.outputs.is_empty() {
        None
    } else {
        Some(tokens_to_json(&decode_output(function, &raw.0)?))
    };

    Ok(CallOutcome {
        decoded,
        raw: format!("0x{}", hex::encode(raw.0)),
    })
}

pub async fn send_native<S: Signer>(
    client: &Client,
    signer: &S,
    to: Address,
    amount: U256,
    on_status: impl FnMut(TransactionStatusUpdate),
    cancel: impl std::future::Future,
) -> Result<TransactionExecution> {
    send_native_with_gas(client, signer, to, amount, None, on_status, cancel).await
}

pub async fn send_native_with_gas<S: Signer>(
    client: &Client,
    signer: &S,
    to: Address,
    amount: U256,
    gas: Option<TransactionGas>,
    on_status: impl FnMut(TransactionStatusUpdate),
    cancel: impl std::future::Future,
) -> Result<TransactionExecution> {
    let tx = prepare_transaction(client, signer.address(), to, Vec::new(), amount, gas).await?;
    submit_transaction(client, signer, tx, on_status, cancel).await
}

pub async fn send_function<S: Signer>(
    client: &Client,
    signer: &S,
    call: FunctionCall<'_>,
    on_status: impl FnMut(TransactionStatusUpdate),
    cancel: impl std::future::Future,
) -> Result<TransactionExecution> {
    send_function_with_gas(client, signer, call, None, on_status, cancel).await
}

pub async fn send_function_with_gas<S: Signer>(
    client: &Client,
    signer: &S,
    call: FunctionCall<'_>,
    gas: Option<TransactionGas>,
    on_status: impl FnMut(TransactionStatusUpdate),
    cancel: impl std::future::Future,
) -> Result<TransactionExecution> {
    let data = encode_input(call.function, call.args)?;
    let tx = prepare_transaction(
        client,
        signer.address(),
        call.contract,
        data,
        call.value,
        gas,
    )
    .await?;
    submit_transaction(client, signer, tx, on_status, cancel).await
}

async fn prepare_transaction(
    client: &Client,
    from: Address,
    to: Address,
    data: Vec<u8>,
    value: U256,
    gas: Option<TransactionGas>,
) -> Result<TransactionParameters> {
    let gas = resolve_transaction_gas(client, from, to, &data, value, gas).await?;
    fill_transaction(client, from, to, data, value, gas).await
}

async fn fill_transaction(
    client: &Client,
    from: Address,
    to: Address,
    data: Vec<u8>,
    value: U256,
    gas: TransactionGas,
) -> Result<TransactionParameters> {
    let nonce = client.nonce(from).await.context("fetch beam nonce")?;
    let chain_id = client
        .chain_id()
        .await
        .context("fetch beam chain id")?
        .as_u64();

    Ok(TransactionParameters {
        chain_id: Some(chain_id),
        data: Bytes(data),
        gas: gas.gas_limit,
        gas_price: Some(gas.gas_price),
        nonce: Some(nonce),
        to: Some(to),
        value,
        ..Default::default()
    })
}

async fn resolve_transaction_gas(
    client: &Client,
    from: Address,
    to: Address,
    data: &[u8],
    value: U256,
    gas: Option<TransactionGas>,
) -> Result<TransactionGas> {
    match gas {
        Some(gas) => Ok(gas),
        None => estimate_transaction_gas(client, from, to, data, value).await,
    }
}

async fn estimate_transaction_gas(
    client: &Client,
    from: Address,
    to: Address,
    data: &[u8],
    value: U256,
) -> Result<TransactionGas> {
    let gas_limit = estimate_gas_limit(client, from, to, data, value).await?;
    let gas_price = client
        .fast_gas_price()
        .await
        .context("fetch beam gas price")?;

    Ok(TransactionGas {
        gas_limit,
        gas_price,
    })
}

async fn estimate_gas_limit(
    client: &Client,
    from: Address,
    to: Address,
    data: &[u8],
    value: U256,
) -> Result<U256> {
    let gas = client
        .estimate_gas(
            CallRequest {
                data: Some(Bytes(data.to_vec())),
                from: Some(from),
                to: Some(to),
                value: Some(value),
                ..Default::default()
            },
            None,
        )
        .await
        .context("estimate beam transaction gas")?;

    Ok(gas + gas / 5)
}

async fn submit_transaction<S: Signer>(
    client: &Client,
    signer: &S,
    transaction: TransactionParameters,
    on_status: impl FnMut(TransactionStatusUpdate),
    cancel: impl std::future::Future,
) -> Result<TransactionExecution> {
    submit_and_wait(client, signer, transaction, on_status, cancel).await
}

pub(crate) fn outcome_from_receipt(receipt: TransactionReceipt) -> Result<TransactionOutcome> {
    let outcome = TransactionOutcome {
        block_number: receipt.block_number.map(|value| value.as_u64()),
        status: receipt.status.map(|value| value.as_u64()),
        tx_hash: format!("{:#x}", receipt.transaction_hash),
    };

    match outcome.status {
        Some(1) => Ok(outcome),
        Some(status) => Err(Error::TransactionFailed {
            status,
            tx_hash: outcome.tx_hash,
        }),
        None => Err(Error::TransactionStatusMissing {
            tx_hash: outcome.tx_hash,
        }),
    }
}
