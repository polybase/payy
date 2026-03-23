use std::{sync::Arc, time::Duration};

use aggregator_interface::{ContractError, RollupInput};
use async_trait::async_trait;
use element::Element;
use thiserror::Error;
use tracing::warn;
use web3::types::U64;

pub struct ContractsRollupContract {
    contract: Arc<contracts::RollupContract>,
    receipt_timeout: Duration,
    receipt_poll_interval: Duration,
}

impl ContractsRollupContract {
    pub fn new(
        inner: Arc<contracts::RollupContract>,
        receipt_timeout: Duration,
        receipt_poll_interval: Duration,
    ) -> Self {
        Self {
            contract: inner,
            receipt_timeout,
            receipt_poll_interval,
        }
    }

    fn map_contract_error<E>(err: E) -> ContractError
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        ContractError::ImplementationSpecific(Box::new(err))
    }

    async fn do_submit_rollup(&self, rollup: &RollupInput) -> Result<(), ContractError> {
        let signature_refs = rollup
            .signatures
            .iter()
            .map(|sig| sig.as_slice())
            .collect::<Vec<_>>();

        let tx_hash = self
            .contract
            .verify_block(
                &rollup.proof,
                &rollup.old_root,
                &rollup.new_root,
                &rollup.commit_hash,
                &rollup.utxo_messages,
                &rollup.kzg,
                rollup.other_hash,
                rollup.height,
                &signature_refs,
                rollup.gas_per_burn_call,
            )
            .await
            .map_err(Self::map_contract_error)?;

        let receipt = self
            .contract
            .client
            .wait_for_receipt(tx_hash, self.receipt_poll_interval, self.receipt_timeout)
            .await
            .map_err(|e| match e {
                contracts::Error::UnknownTransaction(_) => {
                    Self::map_contract_error(SubmitRollupError::Dropped)
                }
                contracts::Error::Timeout => Self::map_contract_error(SubmitRollupError::Timeout),
                _ => Self::map_contract_error(e),
            })?;

        if let Some(status) = receipt.status.filter(|status| *status != U64::from(1u64)) {
            return Err(Self::map_contract_error(SubmitRollupError::Failed {
                status,
            }));
        }

        Ok(())
    }
}

#[async_trait]
impl aggregator_interface::RollupContract for ContractsRollupContract {
    async fn height(&self) -> Result<u64, ContractError> {
        self.contract
            .block_height()
            .await
            .map_err(|e| ContractError::ImplementationSpecific(Box::new(e)))
    }

    async fn root_hash(&self) -> Result<Element, ContractError> {
        let root_hash = self
            .contract
            .root_hash()
            .await
            .map_err(|e| ContractError::ImplementationSpecific(Box::new(e)))?;

        Ok(Element::from_be_bytes(root_hash.to_fixed_bytes()))
    }

    async fn submit_rollup(&self, rollup: &RollupInput) -> Result<(), ContractError> {
        if let Err(submit_rollup_err) = self.do_submit_rollup(rollup).await {
            match self.height().await {
                Ok(current_height) if current_height >= rollup.height => {
                    warn!(
                        current_height,
                        rollup_height = rollup.height,
                        ?submit_rollup_err,
                        "rollup submission failed but contract height is already at or above target, assuming success"
                    );
                    return Ok(());
                }
                _ => return Err(submit_rollup_err),
            }
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
enum SubmitRollupError {
    #[error(
        "[aggregator/contracts_adapter] rollup transaction dropped before receipt was produced"
    )]
    Dropped,
    #[error("[aggregator/contracts_adapter] timed out waiting for rollup receipt")]
    Timeout,
    #[error("[aggregator/contracts_adapter] rollup transaction reverted with status {status:?}")]
    Failed { status: U64 },
}
