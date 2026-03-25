// lint-long-file-override allow-max-lines=700
use std::{future::Future, time::Duration};

use contextful::ResultContextExt;

use crate::{Error, Result};
use ethereum_types::{Address, H256, U64};
use testutil::eth::EthNode;
use tokio::time::interval;
use web3::{
    Web3,
    contract::{Contract, Options, tokens::Tokenize},
    ethabi,
    signing::{SecretKey, keccak256},
    transports::Http,
    types::{
        Block, BlockId, BlockNumber, Bytes, CallRequest, Transaction, TransactionId,
        TransactionReceipt, U256,
    },
};

/// Configuration for different types of transaction confirmation requirements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmationType {
    /// Wait for transaction inclusion only (equivalent to current behavior).
    Latest,
    /// Wait for transaction inclusion plus N additional blocks for safety.
    LatestPlus(u64),
    /// Wait for transaction to be in a finalized block (chain-specific finality).
    Finalised,
}

#[derive(Debug, Clone)]
pub struct Client {
    client: Web3<Http>,
    minimum_gas_price: Option<U256>,
    pub use_latest_for_nonce: bool,
    rpc_url: String,
}

impl Client {
    pub fn try_new(rpc: &str, minimum_gas_price_gwei: Option<u64>) -> Result<Client> {
        let client = Web3::new(Http::new(rpc)?);
        let minimum_gas_price = minimum_gas_price_gwei.map(|gwei| U256::from(gwei) * 1_000_000_000);

        Ok(Client {
            client,
            minimum_gas_price,
            use_latest_for_nonce: false,
            rpc_url: rpc.to_string(),
        })
    }

    pub fn new(rpc: &str, minimum_gas_price_gwei: Option<u64>) -> Client {
        let client = Web3::new(Http::new(rpc).unwrap());
        let minimum_gas_price = minimum_gas_price_gwei.map(|gwei| U256::from(gwei) * 1_000_000_000);

        Client {
            client,
            minimum_gas_price,
            use_latest_for_nonce: false,
            rpc_url: rpc.to_string(),
        }
    }

    pub fn load_contract_from_str(
        &self,
        address: &str,
        contract_json: &str,
    ) -> Result<Contract<Http>> {
        let contract_json_value = serde_json::from_str::<serde_json::Value>(contract_json)?;
        // unwrap should be fine since the json is embedded at build time
        #[allow(clippy::unwrap_used)]
        let abi_value = contract_json_value.get("abi").unwrap();

        let contract_abi = serde_json::from_value::<ethabi::Contract>(abi_value.clone())?;

        Ok(Contract::new(
            self.client.eth(),
            address.parse()?,
            contract_abi,
        ))
    }

    pub fn from_eth_node(eth_node: &EthNode) -> Self {
        Self::new(&eth_node.rpc_url(), None)
    }

    pub async fn eth_balance(&self, address: Address) -> Result<U256> {
        let balance =
            retry_on_network_failure(move || self.client.eth().balance(address, None)).await?;
        Ok(balance)
    }

    pub fn client(&self) -> &Web3<Http> {
        &self.client
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    pub fn with_latest_nonce(mut self) -> Self {
        self.use_latest_for_nonce = true;
        self
    }

    pub async fn get_latest_block_height(&self) -> Result<U64> {
        let block_number = self.block_number().await?;
        Ok(block_number)
    }

    pub async fn fast_gas_price(&self) -> Result<U256, web3::Error> {
        let gas_price: U256 =
            retry_on_network_failure(move || self.client.eth().gas_price()).await?;
        let ten_percent = gas_price / 10;
        let fast_gas_price = gas_price + ten_percent;

        match self.minimum_gas_price {
            Some(minimum_gas_price) if fast_gas_price < minimum_gas_price => Ok(minimum_gas_price),
            _ => Ok(fast_gas_price),
        }
    }

    /// Returns the current chain id with network-failure retries.
    pub async fn chain_id(&self) -> Result<U256, web3::Error> {
        retry_on_network_failure(move || self.client.eth().chain_id()).await
    }

    /// Returns the current chain id using the contracts error type.
    pub async fn chain_id_contracts(&self) -> Result<U256> {
        let chain_id = self.chain_id().await.context("fetch chain id")?;
        Ok(chain_id)
    }

    /// Returns the latest block number with network-failure retries.
    pub async fn block_number(&self) -> Result<U64, web3::Error> {
        retry_on_network_failure(move || self.client.eth().block_number()).await
    }

    /// Returns the latest block number with instant network-failure retries.
    pub async fn block_number_with_fast_retries(&self) -> Result<U64, web3::Error> {
        retry_on_network_failure_instant(move || self.client.eth().block_number()).await
    }

    /// Fetch logs for a given filter with network-failure retries.
    pub async fn logs(
        &self,
        filter: web3::types::Filter,
    ) -> Result<Vec<web3::types::Log>, web3::Error> {
        retry_on_network_failure({
            let filter = filter.clone();
            move || self.client.eth().logs(filter)
        })
        .await
    }

    pub async fn eth_call(
        &self,
        request: CallRequest,
        block: Option<BlockId>,
    ) -> Result<Bytes, web3::Error> {
        retry_on_network_failure(move || self.client.eth().call(request.clone(), block)).await
    }

    pub async fn estimate_gas(
        &self,
        request: CallRequest,
        block: Option<BlockNumber>,
    ) -> Result<U256, web3::Error> {
        retry_on_network_failure(move || self.client.eth().estimate_gas(request.clone(), block))
            .await
    }

    pub async fn send_raw_transaction(&self, transaction: Bytes) -> Result<H256, web3::Error> {
        let local_tx_hash = H256::from_slice(&keccak256(&transaction.0));

        match retry_on_network_failure(move || {
            self.client.eth().send_raw_transaction(transaction.clone())
        })
        .await
        {
            Ok(tx_hash) => Ok(tx_hash),
            Err(err)
                if should_recover_failed_submission(
                    &err,
                    self.submitted_transaction_exists(local_tx_hash).await,
                ) =>
            {
                Ok(local_tx_hash)
            }
            Err(err) => Err(err),
        }
    }

    async fn submitted_transaction_exists(&self, tx_hash: H256) -> bool {
        matches!(self.transaction(tx_hash).await, Ok(Some(_)))
            || matches!(self.transaction_receipt(tx_hash).await, Ok(Some(_)))
    }

    pub async fn transaction(&self, tx_hash: H256) -> Result<Option<Transaction>, web3::Error> {
        retry_on_network_failure(move || {
            self.client.eth().transaction(TransactionId::Hash(tx_hash))
        })
        .await
    }

    pub async fn transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> Result<Option<TransactionReceipt>, web3::Error> {
        retry_on_network_failure(move || self.client.eth().transaction_receipt(tx_hash)).await
    }

    pub async fn block(&self, block_id: BlockId) -> Result<Option<Block<H256>>, web3::Error> {
        retry_on_network_failure(move || self.client.eth().block(block_id)).await
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn get_nonce(
        &self,
        address: Address,
        block: web3::types::BlockNumber,
    ) -> Result<U256, web3::Error> {
        self.client
            .eth()
            .transaction_count(address, Some(block))
            .await
    }

    #[tracing::instrument(err, ret, skip(self))]
    pub async fn nonce(&self, address: Address) -> Result<U256, web3::Error> {
        retry_on_network_failure(move || {
            self.get_nonce(
                address,
                match self.use_latest_for_nonce {
                    true => web3::types::BlockNumber::Latest,
                    false => web3::types::BlockNumber::Pending,
                },
            )
        })
        .await
    }

    pub(crate) async fn options(&self, address: Address) -> Result<Options, web3::Error> {
        let gas_price = self.fast_gas_price().await?;
        let nonce = self.nonce(address).await?;

        Ok(Options {
            gas: Some(10_000_000.into()),
            gas_price: Some(gas_price),
            nonce: Some(nonce),
            ..Default::default()
        })
    }

    pub async fn call(
        &self,
        contract: &Contract<Http>,
        func: &str,
        params: impl Tokenize + Clone,
        signer: &SecretKey,
        signer_address: Address,
    ) -> Result<H256> {
        let options = self.options(signer_address).await?;
        let gas = retry_on_network_failure(|| {
            contract.estimate_gas(func, params.clone(), signer_address, options.clone())
        })
        .await?;

        let call_tx = retry_on_network_failure(move || {
            contract.signed_call(
                func,
                params,
                web3::contract::Options {
                    gas: Some(gas + gas / 2),
                    ..options
                },
                signer,
            )
        })
        .await?;

        Ok(call_tx)
    }

    pub async fn query<R, A, B, P>(
        &self,
        contract: &Contract<Http>,
        func: &str,
        params: P,
        from: A,
        options: Options,
        block: B,
    ) -> Result<R, web3::contract::Error>
    where
        R: web3::contract::tokens::Detokenize,
        A: Into<Option<Address>> + Clone,
        B: Into<Option<web3::types::BlockId>> + Clone,
        P: Tokenize + Clone,
    {
        let result =
            retry_on_network_failure(move || contract.query(func, params, from, options, block))
                .await?;

        Ok(result)
    }

    /// Wait for a transaction to be confirmed and returns the block number.
    ///
    /// Times out if a transaction has been unknown (not in mempool) for 60 seconds.
    ///
    /// The confirmation type determines when the transaction is considered confirmed:
    /// - `Latest`: Returns immediately when transaction is included in any block
    /// - `LatestPlus(n)`: Waits for transaction block + n additional confirmations
    /// - `Finalised`: Waits for transaction to be in a finalized block
    #[tracing::instrument(err, skip(self))]
    pub async fn wait_for_confirm(
        &self,
        txn_hash: H256,
        interval_period: Duration,
        confirmation_type: ConfirmationType,
    ) -> Result<U64> {
        let mut interval = interval(interval_period);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        // First, wait for transaction to be included in a block
        let transaction_block_number = self
            .wait_for_transaction_inclusion(txn_hash, &mut interval)
            .await?;

        // Now apply confirmation type logic
        match confirmation_type {
            ConfirmationType::Latest => {
                tracing::debug!(
                    ?txn_hash,
                    block_number = ?transaction_block_number,
                    "Transaction confirmed with Latest confirmation type"
                );
                Ok(transaction_block_number)
            }
            ConfirmationType::LatestPlus(additional_blocks) => {
                self.wait_for_additional_confirmations(
                    txn_hash,
                    transaction_block_number,
                    additional_blocks,
                    &mut interval,
                )
                .await?;
                Ok(transaction_block_number)
            }
            ConfirmationType::Finalised => {
                self.wait_for_finalized_confirmation(
                    txn_hash,
                    transaction_block_number,
                    &mut interval,
                )
                .await?;
                Ok(transaction_block_number)
            }
        }
    }

    /// Wait for a transaction receipt.
    ///
    /// Polls at the given interval. Returns Error::UnknownTransaction if the transaction is dropped.
    pub async fn wait_for_receipt(
        &self,
        tx_hash: H256,
        poll_interval: Duration,
        timeout: Duration,
    ) -> Result<TransactionReceipt> {
        let start = std::time::Instant::now();

        loop {
            let receipt = self.transaction_receipt(tx_hash).await?;

            if let Some(receipt) = receipt {
                return Ok(receipt);
            }

            let transaction = self.transaction(tx_hash).await?;

            if transaction.is_none() {
                return Err(Error::UnknownTransaction(tx_hash));
            }

            if start.elapsed() > timeout {
                return Err(Error::Timeout);
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Wait for a transaction to be included in a block.
    /// Returns the block number when the transaction is included.
    /// Times out if the transaction has been unknown for 60 seconds.
    async fn wait_for_transaction_inclusion(
        &self,
        txn_hash: H256,
        interval: &mut tokio::time::Interval,
    ) -> Result<U64> {
        let unknown_timeout = std::time::Instant::now() + Duration::from_secs(60);

        loop {
            interval.tick().await;

            let txn = self.transaction(txn_hash).await?;

            match txn {
                None => {
                    // Transaction doesn't exist / is unknown
                    if std::time::Instant::now() > unknown_timeout {
                        return Err(Error::UnknownTransaction(txn_hash));
                    }
                }
                Some(Transaction {
                    block_number: None, ..
                }) => {
                    // Transaction is pending
                }
                Some(Transaction {
                    block_number: Some(block_number),
                    ..
                }) => {
                    // Transaction is included in a block
                    return Ok(block_number);
                }
            }
        }
    }

    /// Wait for additional block confirmations after transaction inclusion.
    /// Waits until the latest block number is >= transaction_block + additional_blocks.
    async fn wait_for_additional_confirmations(
        &self,
        txn_hash: H256,
        transaction_block_number: U64,
        additional_blocks: u64,
        interval: &mut tokio::time::Interval,
    ) -> Result<()> {
        tracing::debug!(
            ?txn_hash,
            block_number = ?transaction_block_number,
            additional_blocks,
            "Waiting for additional block confirmations"
        );

        let required_block_number = transaction_block_number + U64::from(additional_blocks);

        loop {
            interval.tick().await;

            let latest_block = self.block_number().await?;

            if latest_block >= required_block_number {
                tracing::debug!(
                    ?txn_hash,
                    transaction_block = ?transaction_block_number,
                    latest_block = ?latest_block,
                    "Transaction confirmed with required additional blocks"
                );
                return Ok(());
            }

            tracing::trace!(
                ?txn_hash,
                transaction_block = ?transaction_block_number,
                latest_block = ?latest_block,
                required_block = ?required_block_number,
                "Waiting for additional confirmations"
            );
        }
    }

    /// Wait for the transaction's block to be finalized.
    /// Waits until the finalized block number is >= transaction_block_number.
    async fn wait_for_finalized_confirmation(
        &self,
        txn_hash: H256,
        transaction_block_number: U64,
        interval: &mut tokio::time::Interval,
    ) -> Result<()> {
        tracing::debug!(
            ?txn_hash,
            block_number = ?transaction_block_number,
            "Waiting for finalized block confirmation"
        );

        loop {
            interval.tick().await;

            let finalized_block = self
                .block(BlockId::Number(web3::types::BlockNumber::Finalized))
                .await?;

            if let Some(finalized_block) = finalized_block
                && let Some(finalized_number) = finalized_block.number
                && finalized_number >= transaction_block_number
            {
                tracing::debug!(
                    ?txn_hash,
                    transaction_block = ?transaction_block_number,
                    finalized_block = ?finalized_number,
                    "Transaction confirmed in finalized block"
                );
                return Ok(());
            }

            tracing::trace!(
                ?txn_hash,
                transaction_block = ?transaction_block_number,
                "Waiting for block to be finalized"
            );
        }
    }
}

pub(crate) trait IsNetworkFailure {
    fn is_network_failure(&self) -> bool;
}

impl IsNetworkFailure for web3::error::Error {
    fn is_network_failure(&self) -> bool {
        matches!(self, web3::error::Error::Transport(_))
    }
}

impl IsNetworkFailure for web3::contract::Error {
    fn is_network_failure(&self) -> bool {
        matches!(
            self,
            web3::contract::Error::Api(web3::error::Error::Transport(_))
        )
    }
}

const DUPLICATE_SUBMISSION_RPC_PHRASES: &[&str] = &[
    "already known",
    "same hash was already imported",
    "transaction already imported",
];

fn is_duplicate_submission_rpc_error(error: &web3::Error) -> bool {
    let web3::Error::Rpc(error) = error else {
        return false;
    };

    is_duplicate_submission_rpc_message(&error.message)
}

fn is_duplicate_submission_rpc_message(message: &str) -> bool {
    let message = message.to_ascii_lowercase();

    DUPLICATE_SUBMISSION_RPC_PHRASES
        .iter()
        .any(|phrase| message.contains(phrase))
}

fn should_recover_failed_submission(error: &web3::Error, transaction_found: bool) -> bool {
    transaction_found || is_duplicate_submission_rpc_error(error)
}

async fn retry_internal<T, E: IsNetworkFailure, Fut: Future<Output = std::result::Result<T, E>>>(
    f: impl FnOnce() -> Fut + Clone,
    delays: &[Duration],
) -> std::result::Result<T, E> {
    for (i, delay) in delays
        .iter()
        .chain(std::iter::once(&Duration::ZERO))
        .enumerate()
    {
        let res = (f.clone())().await;

        if res.as_ref().is_err_and(|err| err.is_network_failure()) {
            let was_last_try = i == delays.len();
            if was_last_try {
                return res;
            }

            tokio::time::sleep(*delay).await;
        } else {
            return res;
        }
    }

    unreachable!()
}

/// Retries 4 times for a maximum of ~16s on transport-level failures.
pub(crate) async fn retry_on_network_failure<
    T,
    E: IsNetworkFailure,
    Fut: Future<Output = std::result::Result<T, E>>,
>(
    f: impl FnOnce() -> Fut + Clone,
) -> std::result::Result<T, E> {
    const DELAYS: &[Duration] = &[
        Duration::from_secs(1),
        Duration::from_secs(5),
        Duration::from_secs(10),
    ];
    retry_internal(f, DELAYS).await
}

/// Retries 4 times with minimal delay on transport-level failures.
pub(crate) async fn retry_on_network_failure_instant<
    T,
    E: IsNetworkFailure,
    Fut: Future<Output = std::result::Result<T, E>>,
>(
    f: impl FnOnce() -> Fut + Clone,
) -> std::result::Result<T, E> {
    const DELAYS: &[Duration] = &[Duration::ZERO; 3];
    retry_internal(f, DELAYS).await
}

#[cfg(test)]
mod tests;
