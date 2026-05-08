use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use payy_evm_client_interface::{
    Address, Error, PayyEvmCallRequest, PayyEvmLog, PayyEvmLogFilter, PayyEvmReadClient,
    PayyEvmSubmitter, PayyEvmTransactionRequest, PayyNetworkPreset, PayyTransactionReceipt,
    PayyWaitForTransactionReceiptArgs, PreparedOperationResult, PreparedPrivacyCall,
    PrivacyAddress, PrivacyOperationKind, PrivacyStatePreview, Result, TxHash,
};

use super::prepared::Prepared;
use crate::BaseClient;
use crate::client::{PrivacyClient, PrivacyNamespace};

#[cfg(any(feature = "bb-bindings", feature = "bb-cli"))]
mod ephemeral;
mod submit;
mod validate;

struct MockReadClient {
    chain_id: u64,
}

#[async_trait]
impl PayyEvmReadClient for MockReadClient {
    async fn get_chain_id(&self) -> Result<u64> {
        Ok(self.chain_id)
    }

    async fn get_block_number(&self) -> Result<u64> {
        Ok(0)
    }

    async fn read_contract(&self, _request: PayyEvmCallRequest) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    async fn get_logs(&self, _filter: PayyEvmLogFilter) -> Result<Vec<PayyEvmLog>> {
        Ok(Vec::new())
    }

    async fn get_transaction_receipt(
        &self,
        _hash: TxHash,
    ) -> Result<Option<PayyTransactionReceipt>> {
        Ok(None)
    }

    async fn wait_for_transaction_receipt(
        &self,
        _args: PayyWaitForTransactionReceiptArgs,
    ) -> Result<PayyTransactionReceipt> {
        Err(Error::MissingCapability {
            capability: "receipt",
        })
    }
}

struct MockSubmitter {
    chain_id: u64,
    address: Option<Address>,
    sent: AtomicBool,
}

impl MockSubmitter {
    fn new(chain_id: u64, address: Option<Address>) -> Self {
        Self {
            chain_id,
            address,
            sent: AtomicBool::new(false),
        }
    }

    fn sent(&self) -> bool {
        self.sent.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl PayyEvmSubmitter for MockSubmitter {
    async fn get_chain_id(&self) -> Result<u64> {
        Ok(self.chain_id)
    }

    async fn get_address(&self) -> Result<Option<Address>> {
        Ok(self.address)
    }

    async fn send_transaction(&self, _request: PayyEvmTransactionRequest) -> Result<TxHash> {
        self.sent.store(true, Ordering::SeqCst);
        Ok([0x99; 32])
    }
}

fn prepared_with_submitter(
    chain_id: u64,
    from: Option<Address>,
    submitter: Option<Arc<dyn PayyEvmSubmitter>>,
) -> Prepared<()> {
    prepared_with_payload(chain_id, from, submitter, ())
}

fn prepared_with_payload<TPayload>(
    chain_id: u64,
    from: Option<Address>,
    submitter: Option<Arc<dyn PayyEvmSubmitter>>,
    payload: TPayload,
) -> Prepared<TPayload> {
    Prepared {
        result: PreparedOperationResult {
            prepared_call: prepared_call(chain_id, from),
            payload,
        },
        client: privacy_client(submitter.clone()),
        submitter,
    }
}

fn privacy_client(submitter: Option<Arc<dyn PayyEvmSubmitter>>) -> PrivacyNamespace {
    let network = PayyNetworkPreset::Dev.config();
    let read_client = Arc::new(MockReadClient {
        chain_id: network.chain_id,
    });
    let base = BaseClient::builder(network, read_client).build();
    PrivacyClient::from_parts(&base.inner, None, submitter).privacy()
}

fn prepared_call(chain_id: u64, from: Option<Address>) -> PreparedPrivacyCall {
    PreparedPrivacyCall {
        operation: PrivacyOperationKind::Mint,
        chain_id,
        bridge_request: PayyEvmTransactionRequest {
            from,
            to: [0x31; 20],
            data: vec![0x12, 0x34],
            value: 0,
            gas_limit: None,
        },
        verification_key_hash: [0x01; 32],
        proof: vec![0x02],
        public_inputs: vec![[0x03; 32]],
        tx_commitment: [0x04; 32],
        state_preview: PrivacyStatePreview {
            privacy_account: PrivacyAddress::new([0x05; 32]),
            token: [0x06; 20],
            recent_root: [0x07; 32],
            input_commitments: Vec::new(),
            input_nullifiers: Vec::new(),
            output_commitments: vec![[0x08; 32]],
        },
    }
}
