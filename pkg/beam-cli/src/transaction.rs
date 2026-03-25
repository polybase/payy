use std::{
    future::Future,
    time::{Duration, Instant},
};

use contextful::ResultContextExt;
use contracts::Client;
use tokio::time::{MissedTickBehavior, interval};
use web3::types::{H256, TransactionParameters};

use crate::{
    error::Result,
    evm::{TransactionOutcome, outcome_from_receipt},
    signer::Signer,
};

const RECEIPT_POLL_INTERVAL: Duration = Duration::from_millis(750);
const UNKNOWN_TRANSACTION_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingTransaction {
    pub block_number: Option<u64>,
    pub tx_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DroppedTransaction {
    pub block_number: Option<u64>,
    pub tx_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransactionExecution {
    Confirmed(TransactionOutcome),
    Pending(PendingTransaction),
    Dropped(DroppedTransaction),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransactionStatusUpdate {
    Submitted { tx_hash: String },
    Pending { tx_hash: String },
    Mined { tx_hash: String, block_number: u64 },
    Dropped { tx_hash: String },
}

pub fn loading_message(action: &str, update: &TransactionStatusUpdate) -> String {
    match update {
        TransactionStatusUpdate::Submitted { tx_hash } => {
            format!("Submitted {action}. Tx: {tx_hash}")
        }
        TransactionStatusUpdate::Pending { tx_hash } => {
            format!("Pending {action}. Tx: {tx_hash}")
        }
        TransactionStatusUpdate::Mined {
            tx_hash,
            block_number,
        } => format!("Mined {action} in block {block_number}. Tx: {tx_hash}"),
        TransactionStatusUpdate::Dropped { tx_hash } => {
            format!("Stopped waiting for {action}. Tx: {tx_hash}")
        }
    }
}

pub async fn submit_and_wait<S, F, C>(
    client: &Client,
    signer: &S,
    transaction: TransactionParameters,
    mut on_status: F,
    cancel: C,
) -> Result<TransactionExecution>
where
    S: Signer,
    F: FnMut(TransactionStatusUpdate),
    C: Future,
{
    let signed = signer
        .sign_transaction(client.client(), transaction)
        .await?;
    let tx_hash = format!("{:#x}", signed.transaction_hash);
    client
        .send_raw_transaction(signed.raw_transaction)
        .await
        .context("submit beam transaction")?;

    on_status(TransactionStatusUpdate::Submitted {
        tx_hash: tx_hash.clone(),
    });

    wait_for_completion(
        client,
        tx_hash,
        on_status,
        cancel,
        RECEIPT_POLL_INTERVAL,
        UNKNOWN_TRANSACTION_TIMEOUT,
    )
    .await
}

pub(crate) async fn wait_for_completion<F, C>(
    client: &Client,
    tx_hash: String,
    mut on_status: F,
    cancel: C,
    poll_interval: Duration,
    unknown_transaction_timeout: Duration,
) -> Result<TransactionExecution>
where
    F: FnMut(TransactionStatusUpdate),
    C: Future,
{
    let mut receipt_interval = interval(poll_interval);
    receipt_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut cancel = std::pin::pin!(cancel);
    let hash = tx_hash
        .parse::<H256>()
        .expect("tx hashes are formatted internally");
    let mut pending_reported = false;
    let mut mined_block_number = None;
    let mut missing_since = None;

    loop {
        tokio::select! {
            _ = &mut cancel => {
                return Ok(TransactionExecution::Pending(PendingTransaction {
                    block_number: mined_block_number,
                    tx_hash: tx_hash.clone(),
                }));
            }
            _ = receipt_interval.tick() => {
                if let Some(receipt) = client
                    .transaction_receipt(hash)
                    .await
                    .context("fetch beam transaction receipt")?
                {
                    return Ok(TransactionExecution::Confirmed(outcome_from_receipt(receipt)?));
                }

                let Some(transaction) = client
                    .transaction(hash)
                    .await
                    .context("fetch beam transaction status")?
                else {
                    let now = Instant::now();
                    let missing_since = missing_since.get_or_insert(now);

                    if missing_since.elapsed() >= unknown_transaction_timeout {
                        on_status(TransactionStatusUpdate::Dropped {
                            tx_hash: tx_hash.clone(),
                        });

                        return Ok(TransactionExecution::Dropped(DroppedTransaction {
                            block_number: mined_block_number,
                            tx_hash: tx_hash.clone(),
                        }));
                    }

                    continue;
                };

                missing_since = None;

                match transaction.block_number.map(|value| value.as_u64()) {
                    Some(block_number) if mined_block_number != Some(block_number) => {
                        mined_block_number = Some(block_number);
                        on_status(TransactionStatusUpdate::Mined {
                            tx_hash: tx_hash.clone(),
                            block_number,
                        });
                    }
                    None if !pending_reported => {
                        pending_reported = true;
                        on_status(TransactionStatusUpdate::Pending {
                            tx_hash: tx_hash.clone(),
                        });
                    }
                    _ => {}
                }
            }
        }
    }
}
