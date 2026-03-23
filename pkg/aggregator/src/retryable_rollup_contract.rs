use std::time::Duration;

use aggregator_interface::{ContractError, RollupContract, RollupInput};
use async_trait::async_trait;
use element::Element;
use tokio::time::sleep;
use tracing::warn;

pub struct RetryableRollupContract {
    inner: Box<dyn RollupContract>,
    retry_attempts: usize,
    retry_delay: Duration,
}

impl RetryableRollupContract {
    pub fn new(
        inner: Box<dyn RollupContract>,
        retry_attempts: usize,
        retry_delay: Duration,
    ) -> Self {
        Self {
            inner,
            retry_attempts,
            retry_delay,
        }
    }
}

#[async_trait]
impl RollupContract for RetryableRollupContract {
    async fn height(&self) -> Result<u64, ContractError> {
        self.inner.height().await
    }

    async fn root_hash(&self) -> Result<Element, ContractError> {
        self.inner.root_hash().await
    }

    async fn submit_rollup(&self, rollup: &RollupInput) -> Result<(), ContractError> {
        let mut attempts = 0;
        loop {
            // RollupInput implements Clone, so we can clone it for each attempt
            match self.inner.submit_rollup(&rollup.clone()).await {
                Ok(()) => return Ok(()),
                Err(err) => {
                    attempts += 1;
                    if attempts >= self.retry_attempts {
                        return Err(err);
                    }
                    warn!(
                        error = ?err,
                        attempts,
                        delay = ?self.retry_delay,
                        "failed to submit rollup, retrying..."
                    );
                    sleep(self.retry_delay).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use aggregator_interface::RollupContractMock;
    use unimock::*;

    use super::*;

    fn mock_input() -> RollupInput {
        RollupInput {
            proof: vec![],
            old_root: Element::ZERO,
            new_root: Element::ZERO,
            commit_hash: Element::ZERO,
            utxo_messages: vec![],
            kzg: vec![],
            other_hash: [0; 32],
            height: 1,
            signatures: vec![],
            gas_per_burn_call: 100,
        }
    }

    #[tokio::test]
    async fn submit_rollup_succeeds_immediately() {
        let mock = RollupContractMock::submit_rollup
            .next_call(matching!(_))
            .returns(Ok(()));

        let contract =
            RetryableRollupContract::new(Box::new(Unimock::new(mock)), 3, Duration::from_millis(1));

        contract.submit_rollup(&mock_input()).await.unwrap();
    }

    #[tokio::test]
    async fn submit_rollup_retries_and_succeeds() {
        let mock = Unimock::new((
            RollupContractMock::submit_rollup
                .next_call(matching!(_))
                .returns(Err(ContractError::ImplementationSpecific(Box::new(
                    std::io::Error::other("fail 1"),
                )))),
            RollupContractMock::submit_rollup
                .next_call(matching!(_))
                .returns(Err(ContractError::ImplementationSpecific(Box::new(
                    std::io::Error::other("fail 2"),
                )))),
            RollupContractMock::submit_rollup
                .next_call(matching!(_))
                .returns(Ok(())),
        ));

        let contract = RetryableRollupContract::new(Box::new(mock), 3, Duration::from_millis(1));

        contract.submit_rollup(&mock_input()).await.unwrap();
    }

    #[tokio::test]
    async fn submit_rollup_fails_after_retries() {
        let mock = Unimock::new((
            RollupContractMock::submit_rollup
                .next_call(matching!(_))
                .returns(Err(ContractError::ImplementationSpecific(Box::new(
                    std::io::Error::other("fail"),
                )))),
            RollupContractMock::submit_rollup
                .next_call(matching!(_))
                .returns(Err(ContractError::ImplementationSpecific(Box::new(
                    std::io::Error::other("fail"),
                )))),
            RollupContractMock::submit_rollup
                .next_call(matching!(_))
                .returns(Err(ContractError::ImplementationSpecific(Box::new(
                    std::io::Error::other("fail"),
                )))),
        ));

        let contract = RetryableRollupContract::new(Box::new(mock), 3, Duration::from_millis(1));

        let result = contract.submit_rollup(&mock_input()).await;
        assert!(result.is_err());
    }
}
