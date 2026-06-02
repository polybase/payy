use contextful::ResultContextExt;
use contracts::{Address, Client, U256};
use web3::types::{Bytes, CallRequest};

use super::FunctionCall;
use crate::{abi::encode_input, error::Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransactionGas {
    pub gas_limit: U256,
    pub gas_price: U256,
}

impl TransactionGas {
    pub fn fee(&self) -> U256 {
        self.gas_limit * self.gas_price
    }
}

pub async fn estimate_native_gas(
    client: &Client,
    from: Address,
    to: Address,
    amount: U256,
) -> Result<TransactionGas> {
    estimate_transaction_gas(client, from, to, &[], amount).await
}

pub async fn estimate_function_gas(
    client: &Client,
    from: Address,
    call: FunctionCall<'_>,
) -> Result<TransactionGas> {
    let data = encode_input(call.function, call.args)?;
    estimate_transaction_gas(client, from, call.contract, &data, call.value).await
}

pub(super) async fn resolve_transaction_gas(
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
