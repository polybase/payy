use contextful::ResultContextExt;
use contracts::{Client, U256};
use web3::types::BlockNumber;

use crate::error::Result;

const FEE_HISTORY_BLOCKS: u64 = 20;
const PRIORITY_REWARD_PERCENTILE: f64 = 50.0;
const BASE_FEE_MULTIPLIER: u64 = 2;
const WEI_PER_GWEI: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvmFeeMode {
    Legacy,
    Eip1559,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvmFeeEstimate {
    Legacy {
        gas_price: U256,
    },
    Eip1559 {
        max_fee_per_gas: U256,
        max_priority_fee_per_gas: U256,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvmFeePolicy {
    pub base_fee_multiplier: u64,
    pub priority_fee_floor: U256,
}

impl EvmFeeEstimate {
    pub fn mode(&self) -> EvmFeeMode {
        match self {
            Self::Legacy { .. } => EvmFeeMode::Legacy,
            Self::Eip1559 { .. } => EvmFeeMode::Eip1559,
        }
    }

    pub fn max_fee_per_gas(&self) -> U256 {
        match self {
            Self::Legacy { gas_price } => *gas_price,
            Self::Eip1559 {
                max_fee_per_gas, ..
            } => *max_fee_per_gas,
        }
    }
}

pub async fn estimate_fee(client: &Client, chain_id: u64) -> Result<EvmFeeEstimate> {
    let policy = EvmFeePolicy {
        base_fee_multiplier: BASE_FEE_MULTIPLIER,
        priority_fee_floor: priority_fee_floor(chain_id),
    };

    if let Some(estimate) = estimate_eip1559_fee(client, policy).await? {
        return Ok(estimate);
    }

    let gas_price = client
        .fast_gas_price()
        .await
        .context("fetch beam legacy gas price")?;
    Ok(EvmFeeEstimate::Legacy { gas_price })
}

async fn estimate_eip1559_fee(
    client: &Client,
    policy: EvmFeePolicy,
) -> Result<Option<EvmFeeEstimate>> {
    let history = match client
        .client()
        .eth()
        .fee_history(
            U256::from(FEE_HISTORY_BLOCKS),
            BlockNumber::Latest,
            Some(vec![PRIORITY_REWARD_PERCENTILE]),
        )
        .await
    {
        Ok(history) => history,
        Err(_) => return Ok(None),
    };

    let Some(base_fee) = history
        .base_fee_per_gas
        .last()
        .copied()
        .filter(|fee| !fee.is_zero())
    else {
        return Ok(None);
    };

    let reward = history
        .reward
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.first().copied())
        .filter(|fee| !fee.is_zero())
        .collect::<Vec<_>>();
    let priority_fee = std::cmp::max(median_reward(reward), policy.priority_fee_floor);
    let max_fee_per_gas = base_fee * U256::from(policy.base_fee_multiplier) + priority_fee;

    Ok(Some(EvmFeeEstimate::Eip1559 {
        max_fee_per_gas,
        max_priority_fee_per_gas: priority_fee,
    }))
}

fn median_reward(mut rewards: Vec<U256>) -> U256 {
    if rewards.is_empty() {
        return U256::zero();
    }

    rewards.sort_unstable();
    rewards[rewards.len() / 2]
}

fn priority_fee_floor(chain_id: u64) -> U256 {
    match chain_id {
        // Ethereum mainnet and Sepolia should not produce dust priority fees.
        1 | 11155111 => gwei(1),
        // L2s normally need much lower priority fees than Ethereum mainnet.
        8453 | 42161 => U256::from(1_000_000u64),
        // Polygon and BNB need non-dust defaults when they expose EIP-1559 data.
        56 | 137 => gwei(1),
        _ => U256::from(10_000_000u64),
    }
}

fn gwei(value: u64) -> U256 {
    U256::from(value) * U256::from(WEI_PER_GWEI)
}
