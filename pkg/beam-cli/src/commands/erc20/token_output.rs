use serde_json::{Value, json};

use crate::{
    error::Result,
    output::{
        CommandOutput, OutputMode, confirmed_transaction_message, dropped_transaction_message,
        pending_transaction_message,
    },
    transaction::TransactionExecution,
};

pub(super) struct TokenWriteOutputConfig {
    pub amount: String,
    pub chain_key: String,
    pub confirmed_summary: String,
    pub dropped_summary: String,
    pub pending_summary: String,
    pub target_key: &'static str,
    pub target_value: String,
    pub token_address: String,
    pub token_label: String,
}

pub(super) fn print_token_write_output(
    output_mode: OutputMode,
    execution: TransactionExecution,
    config: TokenWriteOutputConfig,
) -> Result<()> {
    let (default, state, block_number, status, tx_hash) = match execution {
        TransactionExecution::Confirmed(outcome) => (
            confirmed_transaction_message(
                config.confirmed_summary,
                &outcome.tx_hash,
                outcome.block_number,
            ),
            "confirmed",
            outcome.block_number,
            outcome.status,
            outcome.tx_hash,
        ),
        TransactionExecution::Pending(pending) => (
            pending_transaction_message(
                config.pending_summary,
                &pending.tx_hash,
                pending.block_number,
            ),
            "pending",
            pending.block_number,
            None,
            pending.tx_hash,
        ),
        TransactionExecution::Dropped(dropped) => (
            dropped_transaction_message(
                config.dropped_summary,
                &dropped.tx_hash,
                dropped.block_number,
            ),
            "dropped",
            dropped.block_number,
            None,
            dropped.tx_hash,
        ),
    };

    let mut value = json!({
        "amount": config.amount,
        "block_number": block_number,
        "chain": config.chain_key,
        "state": state,
        "status": status,
        "token": config.token_label,
        "token_address": config.token_address,
        "tx_hash": tx_hash.clone(),
    });
    if let Some(object) = value.as_object_mut() {
        object.insert(
            config.target_key.to_string(),
            Value::String(config.target_value),
        );
    }

    CommandOutput::new(default, value)
        .compact(tx_hash)
        .print(output_mode)
}
