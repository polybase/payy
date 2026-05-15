use payy_evm_client::{IncomingNote, IncomingNoteStatus};
use serde_json::json;

use crate::{
    evm::format_units,
    human_output::sanitize_control_chars,
    output::CommandOutput,
    privacy::{PrivacyContext, hex20, hex32},
    table::{render_markdown_table, render_table},
};

use super::common::{element_to_u256, field_to_address};

pub(super) struct PrivateBalanceRow {
    pub decimals: u8,
    pub label: String,
    pub token_address: String,
    pub value: contracts::U256,
}

pub(super) fn render_balances(ctx: &PrivacyContext, rows: &[PrivateBalanceRow]) -> CommandOutput {
    let table_rows = rows
        .iter()
        .map(|row| {
            vec![
                sanitize_control_chars(&row.label),
                format_private_balance_value(row.value, row.decimals),
                row.token_address.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let headers = ["token", "private balance", "address"];
    CommandOutput::new(
        render_table(&headers, &table_rows),
        json!({
            "balances": rows.iter().map(|row| {
                json!({
                    "balance": format_private_balance_value(row.value, row.decimals),
                    "chain": ctx.chain.entry.key,
                    "decimals": row.decimals,
                    "private_address": ctx.privacy_address_hex(),
                    "token": row.label,
                    "token_address": row.token_address,
                    "value": row.value.to_string(),
                })
            }).collect::<Vec<_>>()
        }),
    )
    .markdown(render_markdown_table(&headers, &table_rows))
}

pub(crate) fn format_private_balance_value(value: contracts::U256, decimals: u8) -> String {
    format_units(value, decimals)
}

#[cfg(test)]
#[path = "render/tests.rs"]
mod tests;

pub(super) fn render_incoming(ctx: &PrivacyContext, notes: &[IncomingNote]) -> CommandOutput {
    let rows = notes
        .iter()
        .map(|note| {
            vec![
                hex32(&note.commitment),
                status_label(note.status).to_string(),
                hex20(&field_to_address(note.note.token)),
                element_to_u256(note.note.value).to_string(),
                note.source_position.block_number.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let headers = ["id", "status", "token", "value", "block"];
    CommandOutput::new(
        render_table(&headers, &rows),
        json!({
            "chain": ctx.chain.entry.key,
            "incoming": notes.iter().map(|note| {
                json!({
                    "block_number": note.source_position.block_number,
                    "id": hex32(&note.commitment),
                    "status": status_label(note.status),
                    "token_address": hex20(&field_to_address(note.note.token)),
                    "tx_hash": hex32(&note.source_tx_hash),
                    "value": element_to_u256(note.note.value).to_string(),
                })
            }).collect::<Vec<_>>(),
            "private_address": ctx.privacy_address_hex(),
        }),
    )
    .markdown(render_markdown_table(&headers, &rows))
}

fn status_label(status: IncomingNoteStatus) -> &'static str {
    match status {
        IncomingNoteStatus::Claimable => "claimable",
        IncomingNoteStatus::Spent => "spent",
    }
}
