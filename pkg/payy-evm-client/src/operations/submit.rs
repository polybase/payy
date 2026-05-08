use payy_evm_client_interface::{
    ClaimLink, ConfirmedOperationResult, DirectSendDelivery, Error, IncomingNote, IncomingTransfer,
    ParsedClaimLink, PayyTransactionStatus, PayyWaitForTransactionReceiptArgs,
    PreparedOperationResult, PrivacyOperationKind, Result, SubmittedOperationResult, TxHash,
    ValidationErrorKind,
};

use super::prepared::Prepared;

/// Payload metadata hydration for submitted send artifacts.
pub trait SourceMetadataPayload: Clone {
    /// Return this payload with the source transaction hash populated when applicable.
    fn with_source_tx_hash(&self, _source_tx_hash: TxHash) -> Self {
        self.clone()
    }
}

impl SourceMetadataPayload for () {}

impl SourceMetadataPayload for IncomingNote {}

impl SourceMetadataPayload for ParsedClaimLink {}

impl SourceMetadataPayload for DirectSendDelivery {
    fn with_source_tx_hash(&self, source_tx_hash: TxHash) -> Self {
        Self {
            source_tx_hash: Some(source_tx_hash),
            ..self.clone()
        }
    }
}

impl SourceMetadataPayload for IncomingTransfer {
    fn with_source_tx_hash(&self, source_tx_hash: TxHash) -> Self {
        Self {
            source_tx_hash: Some(source_tx_hash),
            ..self.clone()
        }
    }
}

impl SourceMetadataPayload for (DirectSendDelivery, ClaimLink) {
    fn with_source_tx_hash(&self, source_tx_hash: TxHash) -> Self {
        (self.0.with_source_tx_hash(source_tx_hash), self.1.clone())
    }
}

impl SourceMetadataPayload for (IncomingTransfer, ClaimLink) {
    fn with_source_tx_hash(&self, source_tx_hash: TxHash) -> Self {
        (self.0.with_source_tx_hash(source_tx_hash), self.1.clone())
    }
}

impl<TPayload: SourceMetadataPayload> Prepared<TPayload> {
    /// Submit the prepared operation.
    pub async fn submit(&self) -> Result<SubmittedOperationResult<TPayload>> {
        let submitter = self
            .submitter
            .clone()
            .unwrap_or(self.client.inner.evm_submitter()?);
        let chain_id = submitter.get_chain_id().await?;
        if chain_id != self.result.prepared_call.chain_id {
            return Err(Error::ChainIdMismatch {
                expected: self.result.prepared_call.chain_id,
                actual: chain_id,
            });
        }
        if let Some(from) = self.result.prepared_call.bridge_request.from {
            let submitter_address = submitter.get_address().await?;
            if submitter_address != Some(from) {
                return Err(Error::Validation {
                    kind: ValidationErrorKind::EvmAccountMismatch,
                });
            }
        }
        let source_tx_hash = submitter
            .send_transaction(self.result.prepared_call.bridge_request.clone())
            .await?;
        let payload = if self.result.prepared_call.operation == PrivacyOperationKind::TransferSend {
            self.result.payload.with_source_tx_hash(source_tx_hash)
        } else {
            self.result.payload.clone()
        };
        Ok(SubmittedOperationResult {
            prepared_call: self.result.prepared_call.clone(),
            payload,
            source_tx_hash,
        })
    }

    /// Submit and wait for a successful receipt.
    pub async fn submit_and_wait(&self) -> Result<ConfirmedOperationResult<TPayload>> {
        let submitted = self.submit().await?;
        let receipt = self
            .client
            .inner
            .read_client
            .wait_for_transaction_receipt(PayyWaitForTransactionReceiptArgs {
                hash: submitted.source_tx_hash,
                confirmations: None,
                timeout_ms: None,
                poll_interval_ms: None,
            })
            .await?;
        if receipt.status == PayyTransactionStatus::Reverted {
            return Err(Error::TransactionReverted);
        }
        Ok(ConfirmedOperationResult {
            prepared_call: submitted.prepared_call,
            payload: submitted.payload,
            source_tx_hash: submitted.source_tx_hash,
            receipt,
        })
    }
}

impl<TPayload> Prepared<TPayload> {
    pub(super) fn map_payload<TNext>(self, map: impl FnOnce(TPayload) -> TNext) -> Prepared<TNext> {
        Prepared {
            result: PreparedOperationResult {
                prepared_call: self.result.prepared_call,
                payload: map(self.result.payload),
            },
            client: self.client,
            submitter: self.submitter,
        }
    }
}
