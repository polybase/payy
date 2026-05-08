use std::time::Duration;

use payy_evm_client_interface::{
    Error, IncomingNote, IncomingNoteStatus, PayyEvmLogFilter, PrivacyAccount,
    PrivacyAddressPrefix, Result, SourceChainPosition, ValidationErrorKind,
};

use crate::client::PrivacyNamespace;
use crate::util::{element_to_b256, keccak32};

/// Incoming query params.
#[derive(Debug, Clone)]
pub struct IncomingListParams {
    /// Recipient privacy account.
    pub privacy_account: PrivacyAccount,
    /// Optional prefix override.
    pub privacy_address_prefix: Option<PrivacyAddressPrefix>,
    /// Inclusive start block.
    pub from_block: u64,
    /// Inclusive end block.
    pub to_block: Option<u64>,
    /// Include spent notes.
    pub include_spent: bool,
    /// Poll interval for `watch`. Defaults to 3000ms.
    pub poll_interval_ms: Option<u64>,
}

/// Incoming watch result.
#[derive(Debug, Clone)]
pub struct IncomingWatchResult {
    /// Next inclusive block to pass when resuming.
    pub next_from_block: u64,
    /// Error that stopped the watcher, if any.
    pub error: Option<Error>,
}

/// Incoming-note namespace.
pub struct IncomingClient {
    pub(super) client: PrivacyNamespace,
}

impl IncomingClient {
    /// List incoming notes.
    pub async fn list(&self, params: IncomingListParams) -> Result<Vec<IncomingNote>> {
        let (address, account) = self
            .client
            .resolve_privacy_account(params.privacy_account)?;
        let prefix = address.prefix()?;
        if params
            .privacy_address_prefix
            .is_some_and(|candidate| candidate != prefix)
        {
            return Err(payy_evm_client_interface::Error::Validation {
                kind: ValidationErrorKind::PrefixMismatch,
            });
        }
        self.client.inner.validate_read_chain().await?;
        let to_block = match params.to_block {
            Some(block) => block,
            None => self.client.inner.read_client.get_block_number().await?,
        };
        let owner = address.owner()?;
        let logs = self
            .client
            .inner
            .read_client
            .get_logs(PayyEvmLogFilter {
                address: self.client.inner.network.privacy_bridge,
                from_block: params.from_block,
                to_block,
                topics: vec![
                    Some(external_transfer_topic()),
                    Some(prefix_to_topic(prefix)),
                ],
            })
            .await?;
        let mut notes = Vec::new();
        for log in logs {
            let tx_hash = log.topics.get(2).copied().unwrap_or(log.transaction_hash);
            let txn_data = self.client.bridge().get_txn_data(tx_hash).await?;
            if let Some(note) = account
                .signer()
                .decrypt_recipient_note(account.clone().into(), txn_data)?
            {
                if note.owner != owner {
                    continue;
                }
                let commitment = element_to_b256(note.commitment());
                let nullifier = element_to_b256(note.nullifier());
                let spent = self.client.bridge().element_exists(nullifier).await?;
                if spent && !params.include_spent {
                    continue;
                }
                notes.push(IncomingNote {
                    note,
                    commitment,
                    nullifier,
                    source_position: SourceChainPosition {
                        block_number: log.block_number,
                        transaction_index: log.transaction_index,
                        log_index: log.log_index,
                    },
                    source_tx_hash: log.transaction_hash,
                    source_bridge_tx_hash: tx_hash,
                    status: if spent {
                        IncomingNoteStatus::Spent
                    } else {
                        IncomingNoteStatus::Claimable
                    },
                });
            }
        }
        notes.sort_by_key(|note| note.source_position);
        Ok(notes)
    }

    /// Watch the requested range and call the callback for each note.
    pub async fn watch<F>(
        &self,
        params: IncomingListParams,
        mut callback: F,
    ) -> Result<IncomingWatchResult>
    where
        F: FnMut(IncomingNote) -> Result<()>,
    {
        let mut fully_processed = params.from_block.saturating_sub(1);
        let mut from_block = params.from_block;
        let poll_interval = Duration::from_millis(params.poll_interval_ms.unwrap_or(3000));
        loop {
            let head = self.client.inner.read_client.get_block_number().await?;
            let to_block = params.to_block.map_or(head, |block| block.min(head));
            if from_block <= to_block {
                let mut batch = params.clone();
                batch.from_block = from_block;
                batch.to_block = Some(to_block);
                let notes = match self.list(batch).await {
                    Ok(notes) => notes,
                    Err(err) => {
                        return Ok(IncomingWatchResult {
                            next_from_block: from_block,
                            error: Some(err),
                        });
                    }
                };
                for note in notes {
                    if let Err(err) = callback(note.clone()) {
                        return Ok(IncomingWatchResult {
                            next_from_block: note.source_position.block_number,
                            error: Some(err),
                        });
                    }
                }
                fully_processed = to_block;
                from_block = to_block.saturating_add(1);
            }
            if let Some(stop_block) = params.to_block
                && from_block > stop_block
            {
                return Ok(IncomingWatchResult {
                    next_from_block: fully_processed.saturating_add(1),
                    error: None,
                });
            }
            tokio::time::sleep(poll_interval).await;
        }
    }
}

fn prefix_to_topic(prefix: PrivacyAddressPrefix) -> [u8; 32] {
    let mut topic = [0u8; 32];
    topic[..6].copy_from_slice(&prefix.bytes);
    topic
}

fn external_transfer_topic() -> [u8; 32] {
    keccak32(&[b"ExternalTransfer(bytes6,bytes32)"])
}
