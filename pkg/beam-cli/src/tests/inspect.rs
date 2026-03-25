use web3::types::{BlockId, BlockNumber, H256};

use crate::commands::{block::parse_block_id, txn::parse_tx_hash};

#[test]
fn parses_transaction_hash_for_inspection_commands() {
    let hash = parse_tx_hash("0x00000000000000000000000000000000000000000000000000000000000000aa")
        .expect("parse transaction hash");

    assert_eq!(hash, H256::from_low_u64_be(0xaa));
}

#[test]
fn parses_latest_block_when_selector_is_omitted_in_docs_examples() {
    let block_id = parse_block_id("latest").expect("parse latest block");

    assert!(matches!(block_id, BlockId::Number(BlockNumber::Latest)));
}

#[test]
fn parses_numeric_and_hash_block_selectors() {
    let block_id = parse_block_id("42").expect("parse decimal block");
    assert!(matches!(
        block_id,
        BlockId::Number(BlockNumber::Number(number)) if number.as_u64() == 42
    ));

    let block_id = parse_block_id("0x2a").expect("parse hex block");
    assert!(matches!(
        block_id,
        BlockId::Number(BlockNumber::Number(number)) if number.as_u64() == 42
    ));

    let block_id =
        parse_block_id("0x00000000000000000000000000000000000000000000000000000000000000aa")
            .expect("parse block hash");
    assert!(matches!(
        block_id,
        BlockId::Hash(hash) if hash == H256::from_low_u64_be(0xaa)
    ));
}
