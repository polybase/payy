// lint-long-file-override allow-max-lines=300
use std::sync::Arc;
use std::time::{Duration, Instant};

use alloy::{
    primitives::{Address as AlloyAddress, B256 as AlloyB256},
    providers::{DynProvider, Provider},
    rpc::types::BlockNumberOrTag,
};
use async_trait::async_trait;
use contextful::ResultContextExt;
use payy_evm_client_interface::{
    Address, Error, PayyBlockTag, PayyEvmCallRequest, PayyEvmFeeData, PayyEvmLog, PayyEvmLogFilter,
    PayyEvmReadClient, PayyEvmSubmitter, PayyEvmTransactionRequest, PayyRawTransactionSubmitter,
    PayyTransactionCountArgs, PayyTransactionReceipt, PayyWaitForTransactionReceiptArgs, Result,
    TxHash,
};

use crate::convert::{alloy_filter, interface_log, interface_receipt};
use crate::transaction::{AlloyTransactionOptions, payy_to_alloy_transaction_request};

const DEFAULT_RECEIPT_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_RECEIPT_POLL_INTERVAL_MS: u64 = 1_000;

/// Build an Alloy provider-backed Payy read adapter.
#[must_use]
pub fn alloy_read_client(provider: DynProvider) -> Arc<dyn PayyEvmReadClient> {
    Arc::new(AlloyReadClient::new(provider))
}

/// Build an Alloy provider-backed delegated submitter.
#[must_use]
pub fn alloy_wallet_submitter(provider: DynProvider) -> Arc<dyn PayyEvmSubmitter> {
    Arc::new(AlloyWalletSubmitter::new(provider))
}

/// Build an Alloy provider-backed delegated submitter with an explicit sender.
#[must_use]
pub fn alloy_wallet_submitter_with_address(
    provider: DynProvider,
    address: Address,
) -> Arc<dyn PayyEvmSubmitter> {
    Arc::new(AlloyWalletSubmitter::with_address(provider, address))
}

/// Build an Alloy provider-backed raw transaction submitter.
#[must_use]
pub fn alloy_raw_transaction_submitter(
    provider: DynProvider,
) -> Arc<dyn PayyRawTransactionSubmitter> {
    Arc::new(AlloyRawTransactionSubmitter::new(provider))
}

/// Alloy provider-backed Payy read adapter.
#[derive(Clone)]
pub struct AlloyReadClient {
    provider: DynProvider,
}

impl AlloyReadClient {
    /// Create an Alloy read adapter.
    #[must_use]
    pub const fn new(provider: DynProvider) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl PayyEvmReadClient for AlloyReadClient {
    async fn get_chain_id(&self) -> Result<u64> {
        Ok(self.provider.get_chain_id().await.context("get chain id")?)
    }

    async fn get_block_number(&self) -> Result<u64> {
        Ok(self
            .provider
            .get_block_number()
            .await
            .context("get block number")?)
    }

    async fn read_contract(&self, request: PayyEvmCallRequest) -> Result<Vec<u8>> {
        let tx = payy_to_alloy_transaction_request(
            &PayyEvmTransactionRequest {
                from: None,
                to: request.to,
                data: request.data,
                value: 0,
                gas_limit: None,
            },
            AlloyTransactionOptions::default(),
        )?;
        let mut call = self.provider.call(tx);
        if let Some(block_number) = request.block_number {
            call = call.block(BlockNumberOrTag::Number(block_number).into());
        }
        Ok(call.await.context("read contract")?.to_vec())
    }

    async fn get_logs(&self, filter: PayyEvmLogFilter) -> Result<Vec<PayyEvmLog>> {
        let query = alloy_filter(&filter);
        let logs = self.provider.get_logs(&query).await.context("get logs")?;
        logs.iter().map(interface_log).collect()
    }

    async fn get_transaction_receipt(
        &self,
        hash: TxHash,
    ) -> Result<Option<PayyTransactionReceipt>> {
        let receipt = self
            .provider
            .get_transaction_receipt(AlloyB256::from(hash))
            .await
            .context("get transaction receipt")?;
        receipt.as_ref().map(interface_receipt).transpose()
    }

    async fn wait_for_transaction_receipt(
        &self,
        args: PayyWaitForTransactionReceiptArgs,
    ) -> Result<PayyTransactionReceipt> {
        let timeout_ms = args.timeout_ms.unwrap_or(DEFAULT_RECEIPT_TIMEOUT_MS);
        let timeout = Duration::from_millis(timeout_ms);
        let poll_interval = Duration::from_millis(
            args.poll_interval_ms
                .unwrap_or(DEFAULT_RECEIPT_POLL_INTERVAL_MS),
        );
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(receipt) = self.get_transaction_receipt(args.hash).await? {
                let latest_block = if confirmations_required(args.confirmations) <= 1 {
                    receipt.block_number
                } else {
                    self.get_block_number().await?
                };
                if receipt_confirmed(receipt.block_number, latest_block, args.confirmations) {
                    return Ok(receipt);
                }
            }
            if Instant::now() >= deadline {
                return Err(receipt_timeout_error(args.hash, timeout_ms));
            }
            tokio::time::sleep(poll_interval).await;
        }
    }
}

pub(crate) const fn receipt_timeout_error(hash: TxHash, timeout_ms: u64) -> Error {
    Error::ReceiptTimeout { hash, timeout_ms }
}

pub(crate) fn confirmations_required(confirmations: Option<u64>) -> u64 {
    confirmations.unwrap_or(1).max(1)
}

pub(crate) fn receipt_confirmed(
    receipt_block_number: u64,
    latest_block_number: u64,
    confirmations: Option<u64>,
) -> bool {
    let required = confirmations_required(confirmations);
    let Some(target_block) = receipt_block_number.checked_add(required - 1) else {
        return false;
    };
    latest_block_number >= target_block
}

/// Alloy provider-backed delegated submitter.
#[derive(Clone)]
pub struct AlloyWalletSubmitter {
    provider: DynProvider,
    address: Option<Address>,
}

impl AlloyWalletSubmitter {
    /// Create an Alloy submitter without a fixed sender address.
    #[must_use]
    pub const fn new(provider: DynProvider) -> Self {
        Self {
            provider,
            address: None,
        }
    }

    /// Create an Alloy submitter with a fixed sender address.
    #[must_use]
    pub const fn with_address(provider: DynProvider, address: Address) -> Self {
        Self {
            provider,
            address: Some(address),
        }
    }
}

#[async_trait]
impl PayyEvmSubmitter for AlloyWalletSubmitter {
    async fn get_chain_id(&self) -> Result<u64> {
        Ok(self.provider.get_chain_id().await.context("get chain id")?)
    }

    async fn get_address(&self) -> Result<Option<Address>> {
        if let Some(address) = self.address {
            return Ok(Some(address));
        }
        let accounts = self.provider.get_accounts().await.context("get accounts")?;
        Ok(accounts.first().map(|address| *address.as_ref()))
    }

    async fn send_transaction(&self, request: PayyEvmTransactionRequest) -> Result<TxHash> {
        let chain_id = self.get_chain_id().await?;
        let tx = payy_to_alloy_transaction_request(
            &request,
            AlloyTransactionOptions {
                chain_id: Some(chain_id),
                from: self.address,
            },
        )?;
        let pending = self
            .provider
            .send_transaction(tx)
            .await
            .context("send transaction")?;
        Ok(*pending.tx_hash().as_ref())
    }
}

/// Alloy provider-backed raw transaction submitter.
#[derive(Clone)]
pub struct AlloyRawTransactionSubmitter {
    provider: DynProvider,
}

impl AlloyRawTransactionSubmitter {
    /// Create an Alloy raw transaction submitter.
    #[must_use]
    pub const fn new(provider: DynProvider) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl PayyRawTransactionSubmitter for AlloyRawTransactionSubmitter {
    async fn get_chain_id(&self) -> Result<u64> {
        Ok(self.provider.get_chain_id().await.context("get chain id")?)
    }

    async fn get_transaction_count(&self, args: PayyTransactionCountArgs) -> Result<u64> {
        let mut call = self
            .provider
            .get_transaction_count(AlloyAddress::from(args.address));
        if args.block_tag == PayyBlockTag::Pending {
            call = call.block_id(BlockNumberOrTag::Pending.into());
        }
        Ok(call.await.context("get transaction count")?)
    }

    async fn estimate_gas(&self, request: PayyEvmTransactionRequest) -> Result<u64> {
        let tx = payy_to_alloy_transaction_request(&request, AlloyTransactionOptions::default())?;
        Ok(self
            .provider
            .estimate_gas(tx)
            .await
            .context("estimate gas")?)
    }

    async fn get_fee_data(&self) -> Result<PayyEvmFeeData> {
        let estimate = self
            .provider
            .estimate_eip1559_fees()
            .await
            .context("estimate eip1559 fees")?;
        Ok(PayyEvmFeeData {
            max_fee_per_gas: estimate.max_fee_per_gas,
            max_priority_fee_per_gas: estimate.max_priority_fee_per_gas,
        })
    }

    async fn send_raw_transaction(&self, raw_transaction: Vec<u8>) -> Result<TxHash> {
        let pending = self
            .provider
            .send_raw_transaction(&raw_transaction)
            .await
            .context("send raw transaction")?;
        Ok(*pending.tx_hash().as_ref())
    }
}
