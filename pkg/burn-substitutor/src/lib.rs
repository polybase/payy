// lint-long-file-override allow-max-lines=300

use std::{collections::HashSet, time::Duration};

use contextful::{FromContextful, InternalError, prelude::*};
use contracts::{Address, ConfirmationType, RollupContract, U256, USDCContract};
use element::Element;
use eth_util::Eth;
use node_interface::{ListTxnsOrder, ListTxnsParams, ListTxnsPosition, NodeClient, TxnWithInfo};
use primitives::{
    block_height::BlockHeight,
    pagination::{CursorChoice, CursorChoiceAfter},
};
use thiserror::Error;
use zk_primitives::UtxoKindMessages;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, FromContextful)]
pub enum Error {
    #[error("[burn-substitutor] missing field: {name}")]
    MissingField { name: &'static str },

    #[error("[burn-substitutor] chain id mismatch: config={config_chain_id}, rpc={rpc_chain_id}")]
    ChainIdMismatch {
        config_chain_id: u64,
        rpc_chain_id: U256,
    },

    #[error("[burn-substitutor] invalid address: {address}")]
    InvalidAddress { address: String },

    #[error("[burn-substitutor] internal error")]
    Internal(#[from] InternalError),
}

pub struct BurnSubstitutor {
    rollup_contract: RollupContract,
    usdc_contract: USDCContract,
    node_client: Box<dyn NodeClient>,
    eth_txn_confirm_wait_interval: Duration,
    next_txn_cursor: Option<CursorChoiceAfter<ListTxnsPosition>>,
    excluded_burn_addresses: HashSet<Address>,
}

const TXNS_PAGE_LIMIT: usize = 100;

impl BurnSubstitutor {
    pub fn new(
        rollup_contract: RollupContract,
        usdc_contract: USDCContract,
        node_client: Box<dyn NodeClient>,
        eth_txn_confirm_wait_interval: Duration,
        excluded_burn_addresses: Vec<Address>,
    ) -> Self {
        BurnSubstitutor {
            rollup_contract,
            usdc_contract,
            node_client,
            eth_txn_confirm_wait_interval,
            next_txn_cursor: None,
            excluded_burn_addresses: excluded_burn_addresses.into_iter().collect(),
        }
    }

    pub async fn tick(&mut self) -> Result<Vec<Element>> {
        if self.next_txn_cursor.is_none() {
            let last_rollup = self
                .fetch_last_rollup_block()
                .await
                .context("fetch last rollup block")?;

            self.next_txn_cursor = Self::start_txn_cursor(last_rollup.next());
        }

        let (txns, next_txn_cursor) =
            Self::fetch_transactions(self.node_client.as_ref(), self.next_txn_cursor).await?;

        let mut substituted_burns = Vec::new();
        for txn in &txns {
            if let UtxoKindMessages::Burn(burn_msgs) = txn.proof.kind_messages() {
                let hash = burn_msgs.burn_hash;
                let burn_address =
                    Address::from_slice(&burn_msgs.burn_address.to_be_bytes()[12..32]);
                let amount = burn_msgs.value;
                let note_kind = burn_msgs.note_kind;
                let burn_block_height = txn.block_height.0;

                if self.excluded_burn_addresses.contains(&burn_address) {
                    tracing::info!(
                        ?burn_address,
                        "Skipping burn substitution for excluded address"
                    );
                    continue;
                }

                if self
                    .rollup_contract
                    .was_burn_substituted(&burn_address, &note_kind, &hash, &amount)
                    .await
                    .context("check was_burn_substituted")?
                {
                    continue;
                }

                let burn_value = burn_msgs.value.to_eth_u256();

                let usdc_balance = self
                    .usdc_contract
                    .balance(self.rollup_contract.signer_address)
                    .await
                    .context("fetch USDC balance for burn substitution")?;

                if burn_value > usdc_balance {
                    tracing::info!(
                        ?txn.proof.public_inputs,
                        %burn_value,
                        %usdc_balance,
                        "Skipping burn: value exceeds substitutor balance"
                    );
                    continue;
                }

                let txn = match self
                    .rollup_contract
                    .substitute_burn(&burn_address, &note_kind, &hash, &amount, burn_block_height)
                    .await
                {
                    Ok(txn) => txn,
                    Err(err) => {
                        tracing::error!(
                            ?txn.proof.public_inputs,
                            %burn_value,
                            %usdc_balance,
                            %err,
                            "Failed to submit burn substitution transaction",
                        );
                        continue;
                    }
                };

                self.rollup_contract
                    .client
                    .wait_for_confirm(
                        txn,
                        self.eth_txn_confirm_wait_interval,
                        ConfirmationType::Latest,
                    )
                    .await
                    .context("wait for burn substitution")?;

                substituted_burns.push(hash);
            }
        }

        self.next_txn_cursor = next_txn_cursor;

        Ok(substituted_burns)
    }

    async fn fetch_last_rollup_block(
        &mut self,
    ) -> std::result::Result<BlockHeight, contracts::Error> {
        self.rollup_contract.block_height().await.map(BlockHeight)
    }

    async fn fetch_transactions(
        node_client: &dyn NodeClient,
        cursor: Option<CursorChoiceAfter<ListTxnsPosition>>,
    ) -> Result<(
        Vec<TxnWithInfo>,
        Option<CursorChoiceAfter<ListTxnsPosition>>,
    )> {
        let response = node_client
            .list_transactions(ListTxnsParams {
                limit: TXNS_PAGE_LIMIT,
                cursor: cursor.map(|cursor| CursorChoice::After(cursor).opaque()),
                order: ListTxnsOrder::OldestToNewest,
                poll: true,
            })
            .await
            .context("fetch transactions")?;

        let next_cursor = response
            .cursor
            .after
            .map(|after| after.into_inner())
            .or(cursor);

        Ok((response.txns, next_cursor))
    }

    fn start_txn_cursor(start_height: BlockHeight) -> Option<CursorChoiceAfter<ListTxnsPosition>> {
        if start_height.0 == 0 {
            return None;
        }

        Some(CursorChoiceAfter::After(ListTxnsPosition {
            block: BlockHeight(start_height.0 - 1),
            txn: u64::MAX,
        }))
    }
}
