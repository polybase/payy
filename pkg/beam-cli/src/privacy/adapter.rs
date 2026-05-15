// lint-long-file-override allow-max-lines=300
use std::time::{Duration, Instant};

use async_trait::async_trait;
use contextful::ResultContextExt;
use contracts::{Client, U256};
use payy_evm_client::{
    PayyBlockTag, PayyEvmCallRequest, PayyEvmFeeData, PayyEvmLog, PayyEvmLogFilter,
    PayyEvmReadClient, PayyEvmTransactionRequest, PayyRawTransactionSubmitter,
    PayyTransactionCountArgs, PayyTransactionReceipt, PayyTransactionStatus,
    PayyWaitForTransactionReceiptArgs, Result as PayyResult,
};
use web3::types::{
    BlockNumber, Bytes, CallRequest, FilterBuilder, H160, H256, TransactionReceipt, U64,
};

use crate::privacy::bytes_to_address;

const DEFAULT_RECEIPT_TIMEOUT: Duration = Duration::from_secs(600);
const DEFAULT_RECEIPT_POLL: Duration = Duration::from_millis(750);

pub struct BeamEvmAdapter {
    client: Client,
}

impl BeamEvmAdapter {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn client(&self) -> Client {
        self.client.clone()
    }
}

#[async_trait]
impl PayyEvmReadClient for BeamEvmAdapter {
    async fn get_chain_id(&self) -> PayyResult<u64> {
        Ok(self
            .client
            .chain_id()
            .await
            .context("fetch beam privacy chain id")?
            .as_u64())
    }

    async fn get_block_number(&self) -> PayyResult<u64> {
        Ok(self
            .client
            .block_number()
            .await
            .context("fetch beam privacy block number")?
            .as_u64())
    }

    async fn read_contract(&self, request: PayyEvmCallRequest) -> PayyResult<Vec<u8>> {
        let response = self
            .client
            .eth_call(
                CallRequest {
                    to: Some(bytes_to_address(request.to)),
                    data: Some(Bytes(request.data)),
                    ..Default::default()
                },
                request.block_number.map(|number| {
                    web3::types::BlockId::Number(BlockNumber::Number(U64::from(number)))
                }),
            )
            .await
            .context("execute beam privacy eth_call")?;
        Ok(response.0)
    }

    async fn get_logs(&self, filter: PayyEvmLogFilter) -> PayyResult<Vec<PayyEvmLog>> {
        let topics = filter
            .topics
            .iter()
            .map(|topic| topic.map(H256::from).map(|topic| vec![topic]))
            .collect::<Vec<_>>();
        let logs = self
            .client
            .logs(
                FilterBuilder::default()
                    .address(vec![H160::from(filter.address)])
                    .from_block(BlockNumber::Number(U64::from(filter.from_block)))
                    .to_block(BlockNumber::Number(U64::from(filter.to_block)))
                    .topics(
                        topics.first().cloned().unwrap_or(None),
                        topics.get(1).cloned().unwrap_or(None),
                        topics.get(2).cloned().unwrap_or(None),
                        topics.get(3).cloned().unwrap_or(None),
                    )
                    .build(),
            )
            .await
            .context("fetch beam privacy logs")?;

        logs.into_iter()
            .map(|log| {
                let address = log.address.0;
                let topics = log.topics.into_iter().map(|topic| topic.0).collect();
                Ok(PayyEvmLog {
                    address,
                    data: log.data.0,
                    topics,
                    block_number: required_u64(log.block_number, "log block number")?,
                    transaction_index: required_u64(
                        log.transaction_index,
                        "log transaction index",
                    )?,
                    log_index: required_u256(log.log_index, "log index")?,
                    transaction_hash: required_hash(log.transaction_hash, "log transaction hash")?,
                })
            })
            .collect()
    }

    async fn get_transaction_receipt(
        &self,
        hash: [u8; 32],
    ) -> PayyResult<Option<PayyTransactionReceipt>> {
        self.client
            .transaction_receipt(H256::from(hash))
            .await
            .context("fetch beam privacy transaction receipt")?
            .map(receipt_from_web3)
            .transpose()
    }

    async fn wait_for_transaction_receipt(
        &self,
        args: PayyWaitForTransactionReceiptArgs,
    ) -> PayyResult<PayyTransactionReceipt> {
        let timeout = Duration::from_millis(
            args.timeout_ms
                .unwrap_or_else(|| DEFAULT_RECEIPT_TIMEOUT.as_millis() as u64),
        );
        let poll = Duration::from_millis(
            args.poll_interval_ms
                .unwrap_or_else(|| DEFAULT_RECEIPT_POLL.as_millis() as u64),
        );
        let started = Instant::now();
        loop {
            if let Some(receipt) = self.get_transaction_receipt(args.hash).await? {
                return Ok(receipt);
            }
            if started.elapsed() >= timeout {
                return Err(payy_evm_client::Error::ReceiptTimeout {
                    hash: args.hash,
                    timeout_ms: timeout.as_millis() as u64,
                });
            }
            tokio::time::sleep(poll).await;
        }
    }
}

#[async_trait]
impl PayyRawTransactionSubmitter for BeamEvmAdapter {
    async fn get_chain_id(&self) -> PayyResult<u64> {
        PayyEvmReadClient::get_chain_id(self).await
    }

    async fn get_transaction_count(&self, args: PayyTransactionCountArgs) -> PayyResult<u64> {
        let block = match args.block_tag {
            PayyBlockTag::Latest => BlockNumber::Latest,
            PayyBlockTag::Pending => BlockNumber::Pending,
        };
        Ok(self
            .client
            .get_nonce(bytes_to_address(args.address), block)
            .await
            .context("fetch beam privacy nonce")?
            .as_u64())
    }

    async fn estimate_gas(&self, request: PayyEvmTransactionRequest) -> PayyResult<u64> {
        let gas = self
            .client
            .estimate_gas(
                CallRequest {
                    data: Some(Bytes(request.data)),
                    from: request.from.map(bytes_to_address),
                    to: Some(bytes_to_address(request.to)),
                    value: Some(U256::from(request.value)),
                    ..Default::default()
                },
                None,
            )
            .await
            .context("estimate beam privacy transaction gas")?;
        Ok((gas + gas / 5).as_u64())
    }

    async fn get_fee_data(&self) -> PayyResult<PayyEvmFeeData> {
        let gas_price = self
            .client
            .fast_gas_price()
            .await
            .context("fetch beam privacy fee data")?
            .as_u128();
        Ok(PayyEvmFeeData {
            max_fee_per_gas: gas_price,
            max_priority_fee_per_gas: gas_price,
        })
    }

    async fn send_raw_transaction(&self, raw_transaction: Vec<u8>) -> PayyResult<[u8; 32]> {
        Ok(self
            .client
            .send_raw_transaction(Bytes(raw_transaction))
            .await
            .context("submit beam privacy raw transaction")?
            .0)
    }
}

fn receipt_from_web3(receipt: TransactionReceipt) -> PayyResult<PayyTransactionReceipt> {
    Ok(PayyTransactionReceipt {
        transaction_hash: receipt.transaction_hash.0,
        block_number: required_u64(receipt.block_number, "receipt block number")?,
        status: match receipt.status.map(|status| status.as_u64()) {
            Some(1) => PayyTransactionStatus::Success,
            _ => PayyTransactionStatus::Reverted,
        },
    })
}

fn required_hash(value: Option<H256>, label: &'static str) -> PayyResult<[u8; 32]> {
    value
        .map(|hash| hash.0)
        .ok_or_else(|| payy_evm_client::Error::SignerCommandFailed {
            message: format!("missing {label}"),
        })
}

fn required_u64(value: Option<U64>, label: &'static str) -> PayyResult<u64> {
    value
        .map(|number| number.as_u64())
        .ok_or_else(|| payy_evm_client::Error::SignerCommandFailed {
            message: format!("missing {label}"),
        })
}

fn required_u256(value: Option<U256>, label: &'static str) -> PayyResult<u64> {
    value
        .map(|number| number.as_u64())
        .ok_or_else(|| payy_evm_client::Error::SignerCommandFailed {
            message: format!("missing {label}"),
        })
}
