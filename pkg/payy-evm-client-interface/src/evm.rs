use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;

/// Canonical 32-byte value.
pub type B256 = [u8; 32];
/// Canonical EVM address value.
pub type Address = [u8; 20];
/// EVM transaction hash.
pub type TxHash = B256;
/// Raw byte vector.
pub type Bytes = Vec<u8>;

/// Canonical EVM call request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayyEvmCallRequest {
    /// Contract address.
    pub to: Address,
    /// Calldata.
    pub data: Bytes,
    /// Optional block number.
    pub block_number: Option<u64>,
}

/// Canonical EVM log filter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayyEvmLogFilter {
    /// Contract address.
    pub address: Address,
    /// Inclusive start block.
    pub from_block: u64,
    /// Inclusive end block.
    pub to_block: u64,
    /// Positional topic filters. `None` leaves that topic position unconstrained.
    pub topics: Vec<Option<B256>>,
}

/// Canonical EVM log shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayyEvmLog {
    /// Emitting contract address.
    pub address: Address,
    /// Log data.
    pub data: Bytes,
    /// Log topics.
    pub topics: Vec<B256>,
    /// Source block number.
    pub block_number: u64,
    /// Source transaction index.
    pub transaction_index: u64,
    /// Source log index.
    pub log_index: u64,
    /// Source outer transaction hash.
    pub transaction_hash: TxHash,
}

/// Canonical transaction receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayyTransactionReceipt {
    /// Outer transaction hash.
    pub transaction_hash: TxHash,
    /// Inclusion block.
    pub block_number: u64,
    /// Receipt status.
    pub status: PayyTransactionStatus,
}

/// Canonical receipt status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayyTransactionStatus {
    /// Transaction succeeded.
    Success,
    /// Transaction reverted.
    Reverted,
}

/// Receipt wait args.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayyWaitForTransactionReceiptArgs {
    /// Transaction hash.
    pub hash: TxHash,
    /// Optional confirmation count.
    pub confirmations: Option<u64>,
    /// Optional timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Optional polling interval in milliseconds.
    pub poll_interval_ms: Option<u64>,
}

/// Transaction count args.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayyTransactionCountArgs {
    /// Account address.
    pub address: Address,
    /// Block tag.
    pub block_tag: PayyBlockTag,
}

/// Minimal block tag model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayyBlockTag {
    /// Latest block.
    Latest,
    /// Pending block.
    Pending,
}

/// EIP-1559 fee data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayyEvmFeeData {
    /// Max fee per gas.
    pub max_fee_per_gas: u128,
    /// Max priority fee per gas.
    pub max_priority_fee_per_gas: u128,
}

/// Canonical EVM transaction request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayyEvmTransactionRequest {
    /// Optional sender address.
    pub from: Option<Address>,
    /// Target contract.
    pub to: Address,
    /// Calldata.
    pub data: Bytes,
    /// ETH value.
    pub value: u128,
    /// Optional gas limit.
    pub gas_limit: Option<u64>,
}

/// Read-only EVM adapter.
#[async_trait]
pub trait PayyEvmReadClient: Send + Sync {
    /// Fetch chain ID.
    async fn get_chain_id(&self) -> Result<u64>;
    /// Fetch current block number.
    async fn get_block_number(&self) -> Result<u64>;
    /// Execute an eth_call style read.
    async fn read_contract(&self, request: PayyEvmCallRequest) -> Result<Bytes>;
    /// Fetch logs.
    async fn get_logs(&self, filter: PayyEvmLogFilter) -> Result<Vec<PayyEvmLog>>;
    /// Fetch transaction receipt, if available.
    async fn get_transaction_receipt(&self, hash: TxHash)
    -> Result<Option<PayyTransactionReceipt>>;
    /// Wait for a transaction receipt.
    async fn wait_for_transaction_receipt(
        &self,
        args: PayyWaitForTransactionReceiptArgs,
    ) -> Result<PayyTransactionReceipt>;
}

/// Delegated transaction submitter.
#[async_trait]
pub trait PayyEvmSubmitter: Send + Sync {
    /// Fetch chain ID.
    async fn get_chain_id(&self) -> Result<u64>;
    /// Fetch default or bound address.
    async fn get_address(&self) -> Result<Option<Address>>;
    /// Send a transaction.
    async fn send_transaction(&self, request: PayyEvmTransactionRequest) -> Result<TxHash>;
}

/// Raw transaction submitter for local signing.
#[async_trait]
pub trait PayyRawTransactionSubmitter: Send + Sync {
    /// Fetch chain ID.
    async fn get_chain_id(&self) -> Result<u64>;
    /// Fetch nonce.
    async fn get_transaction_count(&self, args: PayyTransactionCountArgs) -> Result<u64>;
    /// Estimate gas.
    async fn estimate_gas(&self, request: PayyEvmTransactionRequest) -> Result<u64>;
    /// Fetch fee data.
    async fn get_fee_data(&self) -> Result<PayyEvmFeeData>;
    /// Broadcast signed raw transaction.
    async fn send_raw_transaction(&self, raw_transaction: Bytes) -> Result<TxHash>;
}
