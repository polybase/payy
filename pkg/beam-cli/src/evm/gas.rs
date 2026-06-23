use contextful::ResultContextExt;
use contracts::{Address, Client, U256};
use web3::types::{Bytes, CallRequest};

use super::{
    FunctionCall,
    fees::{EvmFeeEstimate, estimate_fee},
};
use crate::{
    abi::encode_input,
    error::{Error, Result},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransactionGas {
    pub gas_limit: U256,
    pub fee: EvmFeeEstimate,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TransactionGasPolicy {
    pub gas_limit: Option<U256>,
    pub max_network_fee: Option<U256>,
}

impl TransactionGas {
    pub fn fee(&self) -> U256 {
        self.max_network_fee()
    }

    pub fn max_network_fee(&self) -> U256 {
        self.gas_limit * self.fee.max_fee_per_gas()
    }

    pub fn gas_price_for_display(&self) -> U256 {
        self.fee.max_fee_per_gas()
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

pub(crate) async fn resolve_transaction_gas(
    client: &Client,
    from: Address,
    to: Address,
    data: &[u8],
    value: U256,
    gas: Option<TransactionGasPolicy>,
) -> Result<TransactionGas> {
    let gas_policy = gas.unwrap_or_default();
    let gas_limit = match gas_policy.gas_limit {
        Some(gas_limit) => pad_gas_limit(gas_limit),
        None => estimate_gas_limit(client, from, to, data, value).await?,
    };
    let chain_id = client
        .chain_id()
        .await
        .context("fetch beam chain id for fee estimate")?
        .as_u64();
    let fee = estimate_fee(client, chain_id).await?;
    let resolved = TransactionGas { gas_limit, fee };

    if let Some(cap) = gas_policy.max_network_fee {
        let estimated = resolved.max_network_fee();
        if estimated > cap {
            return Err(Error::TransactionFeeCapExceeded { cap, estimated });
        }
    }

    Ok(resolved)
}

async fn estimate_transaction_gas(
    client: &Client,
    from: Address,
    to: Address,
    data: &[u8],
    value: U256,
) -> Result<TransactionGas> {
    let gas_limit = estimate_gas_limit(client, from, to, data, value).await?;
    let chain_id = client
        .chain_id()
        .await
        .context("fetch beam chain id for fee estimate")?
        .as_u64();
    let fee = estimate_fee(client, chain_id).await?;

    Ok(TransactionGas { gas_limit, fee })
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

    Ok(pad_gas_limit(gas))
}

fn pad_gas_limit(gas: U256) -> U256 {
    gas + gas / 5
}
