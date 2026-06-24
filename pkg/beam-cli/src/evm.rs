// lint-long-file-override allow-max-lines=400
mod fees;
mod gas;

use contextful::ResultContextExt;
use contracts::{Address, Client, ERC20Contract, U256};
use web3::{
    ethabi::{Function, StateMutability},
    types::{Bytes, CallRequest, TransactionParameters, TransactionReceipt, U64},
};

pub use crate::units::{format_units, parse_units, validate_unit_decimals};
use crate::{
    abi::{decode_output, encode_input, parse_function, tokens_to_json},
    error::{Error, Result},
    signer::Signer,
    transaction::{TransactionExecution, TransactionStatusUpdate, submit_and_wait},
};
pub use fees::{EvmFeeEstimate, EvmFeeMode};
pub(crate) use gas::resolve_transaction_gas;
pub use gas::{TransactionGas, TransactionGasPolicy, estimate_function_gas, estimate_native_gas};

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

#[derive(Clone, Debug)]
pub struct CalldataTransaction {
    pub data: Vec<u8>,
    pub to: Address,
    pub value: U256,
    pub gas: Option<TransactionGasPolicy>,
}

#[derive(Clone, Debug)]
pub struct CalldataExecution {
    pub execution: TransactionExecution,
    pub gas: TransactionGas,
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

pub async fn erc20_allowance(
    client: &Client,
    token: Address,
    owner: Address,
    spender: Address,
) -> Result<U256> {
    let function = parse_function(
        "allowance(address,address):(uint256)",
        StateMutability::View,
    )?;
    let outcome = call_function(
        client,
        Some(owner),
        token,
        &function,
        &[format!("{owner:#x}"), format!("{spender:#x}")],
    )
    .await?;
    let decoded = outcome
        .decoded
        .ok_or_else(|| Error::InvalidFunctionSignature {
            signature: "allowance(address,address):(uint256)".to_string(),
        })?;
    let value = decoded[0]
        .as_str()
        .ok_or_else(|| Error::InvalidFunctionSignature {
            signature: "allowance(address,address):(uint256)".to_string(),
        })?
        .parse::<U256>()
        .context("parse beam erc20 allowance")?;
    Ok(value)
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

pub async fn send_native<S: Signer + ?Sized>(
    client: &Client,
    signer: &S,
    to: Address,
    amount: U256,
    on_status: impl FnMut(TransactionStatusUpdate),
    cancel: impl std::future::Future,
) -> Result<TransactionExecution> {
    send_native_with_gas(client, signer, to, amount, None, on_status, cancel).await
}

pub async fn send_native_with_gas<S: Signer + ?Sized>(
    client: &Client,
    signer: &S,
    to: Address,
    amount: U256,
    gas: Option<TransactionGasPolicy>,
    on_status: impl FnMut(TransactionStatusUpdate),
    cancel: impl std::future::Future,
) -> Result<TransactionExecution> {
    let (tx, _) =
        prepare_transaction(client, signer.address(), to, Vec::new(), amount, gas).await?;
    submit_transaction(client, signer, tx, on_status, cancel).await
}

pub async fn send_function<S: Signer + ?Sized>(
    client: &Client,
    signer: &S,
    call: FunctionCall<'_>,
    on_status: impl FnMut(TransactionStatusUpdate),
    cancel: impl std::future::Future,
) -> Result<TransactionExecution> {
    send_function_with_gas(client, signer, call, None, on_status, cancel).await
}

pub async fn send_function_with_gas<S: Signer + ?Sized>(
    client: &Client,
    signer: &S,
    call: FunctionCall<'_>,
    gas: Option<TransactionGasPolicy>,
    on_status: impl FnMut(TransactionStatusUpdate),
    cancel: impl std::future::Future,
) -> Result<TransactionExecution> {
    let data = encode_input(call.function, call.args)?;
    let (tx, _) = prepare_transaction(
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

pub async fn send_calldata_with_fee_report<S: Signer + ?Sized>(
    client: &Client,
    signer: &S,
    transaction: CalldataTransaction,
    on_status: impl FnMut(TransactionStatusUpdate),
    cancel: impl std::future::Future,
) -> Result<CalldataExecution> {
    let (tx, gas) = prepare_transaction(
        client,
        signer.address(),
        transaction.to,
        transaction.data,
        transaction.value,
        transaction.gas,
    )
    .await?;
    let execution = submit_transaction(client, signer, tx, on_status, cancel).await?;
    Ok(CalldataExecution { execution, gas })
}

pub async fn simulate_calldata(
    client: &Client,
    from: Address,
    to: Address,
    data: Vec<u8>,
    value: U256,
) -> Result<()> {
    client
        .eth_call(
            CallRequest {
                data: Some(Bytes(data)),
                from: Some(from),
                to: Some(to),
                value: Some(value),
                ..Default::default()
            },
            None,
        )
        .await
        .context("simulate beam transaction")?;

    Ok(())
}

async fn prepare_transaction(
    client: &Client,
    from: Address,
    to: Address,
    data: Vec<u8>,
    value: U256,
    gas: Option<TransactionGasPolicy>,
) -> Result<(TransactionParameters, TransactionGas)> {
    let gas = resolve_transaction_gas(client, from, to, &data, value, gas).await?;
    let transaction = fill_transaction(client, from, to, data, value, gas).await?;
    Ok((transaction, gas))
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
        nonce: Some(nonce),
        to: Some(to),
        value,
        ..transaction_fee_parameters(&gas)
    })
}

fn transaction_fee_parameters(gas: &TransactionGas) -> TransactionParameters {
    match &gas.fee {
        EvmFeeEstimate::Legacy { gas_price } => TransactionParameters {
            gas_price: Some(*gas_price),
            ..Default::default()
        },
        EvmFeeEstimate::Eip1559 {
            max_fee_per_gas,
            max_priority_fee_per_gas,
        } => TransactionParameters {
            transaction_type: Some(U64::from(2)),
            max_fee_per_gas: Some(*max_fee_per_gas),
            max_priority_fee_per_gas: Some(*max_priority_fee_per_gas),
            ..Default::default()
        },
    }
}

pub fn transaction_fee_json(gas: &TransactionGas) -> serde_json::Value {
    match &gas.fee {
        EvmFeeEstimate::Legacy { gas_price } => serde_json::json!({
            "fee_mode": "legacy",
            "gas_price": gas_price.to_string(),
            "max_network_fee_wei": gas.max_network_fee().to_string(),
        }),
        EvmFeeEstimate::Eip1559 {
            max_fee_per_gas,
            max_priority_fee_per_gas,
        } => serde_json::json!({
            "fee_mode": "eip1559",
            "max_fee_per_gas": max_fee_per_gas.to_string(),
            "max_priority_fee_per_gas": max_priority_fee_per_gas.to_string(),
            "max_network_fee_wei": gas.max_network_fee().to_string(),
        }),
    }
}

async fn submit_transaction<S: Signer + ?Sized>(
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
