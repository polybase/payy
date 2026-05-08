use alloy::{
    network::ReceiptResponse,
    primitives::{Address as AlloyAddress, B256 as AlloyB256},
    rpc::types::Filter,
};
use payy_evm_client_interface::{
    PayyEvmLog, PayyEvmLogFilter, PayyTransactionReceipt, PayyTransactionStatus, Result,
};

pub(crate) fn alloy_filter(filter: &PayyEvmLogFilter) -> Filter {
    let mut query = Filter::new()
        .address(AlloyAddress::from(filter.address))
        .from_block(filter.from_block)
        .to_block(filter.to_block);
    for (index, topic) in filter.topics.iter().take(4).enumerate() {
        let Some(topic) = topic else {
            continue;
        };
        let topic = AlloyB256::from(*topic);
        query = match index {
            0 => query.event_signature(topic),
            1 => query.topic1(topic),
            2 => query.topic2(topic),
            3 => query.topic3(topic),
            _ => query,
        };
    }
    query
}

pub(crate) fn interface_log(log: &alloy::rpc::types::Log) -> Result<PayyEvmLog> {
    Ok(PayyEvmLog {
        address: *log.address().as_ref(),
        data: log.data().data.to_vec(),
        topics: log.topics().iter().map(|topic| *topic.as_ref()).collect(),
        block_number: require(log.block_number, "log_block_number")?,
        transaction_index: require(log.transaction_index, "log_transaction_index")?,
        log_index: require(log.log_index, "log_index")?,
        transaction_hash: *require(log.transaction_hash, "log_transaction_hash")?.as_ref(),
    })
}

pub(crate) fn interface_receipt(
    receipt: &alloy::rpc::types::TransactionReceipt,
) -> Result<PayyTransactionReceipt> {
    Ok(PayyTransactionReceipt {
        transaction_hash: *receipt.transaction_hash().as_ref(),
        block_number: require(receipt.block_number(), "receipt_block_number")?,
        status: if receipt.status() {
            PayyTransactionStatus::Success
        } else {
            PayyTransactionStatus::Reverted
        },
    })
}

fn require<T>(value: Option<T>, capability: &'static str) -> Result<T> {
    value.ok_or(payy_evm_client_interface::Error::MissingCapability { capability })
}
