// These features have been stabilized in recent Rust versions
#![allow(incomplete_features)]
// #![feature(return_position_impl_trait_in_trait)]
// #![feature(associated_type_defaults)]
// #![feature(bound_map)]

// lint-long-file-override allow-max-lines=700
mod element_history;
mod keys;
mod list;
mod migration;
mod values;

use std::{marker::PhantomData, ops::RangeBounds, path::Path};

use borsh::{BorshDeserialize, BorshSerialize};
use element_history::ElementHistoryIndex;
use keys::{Key, KeyBlock, StoreKey};
use migration::LATEST_VERSION;
use primitives::{block_height::BlockHeight, hash::CryptoHash};
use rocksdb::DB;
use values::{ElementHistoryData, ElementHistoryValue, MintHashData, MintHashValue};
use wire_message::WireMessage;

pub use element_history::ElementHistoryIndexEntry;
pub use keys::{BlockListOrder, ElementHistoryKind};
pub use list::StoreList;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid key")]
    InvalidKey,

    #[error("invalid value {0}")]
    InvalidValue(u64),

    #[error("invalid version '{0}'")]
    InvalidVersion(u32),

    #[error("rocksdb error: {0}")]
    RocksDB(#[from] rocksdb::Error),

    #[error("wire message error: {0}")]
    WireMessage(#[from] wire_message::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

type Result<T, E = Error> = std::result::Result<T, E>;

pub struct BlockStore<B> {
    db: DB,
    element_history_index: ElementHistoryIndex,
    _marker: PhantomData<B>,
}

pub trait Block {
    type Txn: Transaction;

    fn block_height(&self) -> BlockHeight;
    fn block_hash(&self) -> [u8; 32];

    fn txns(&self) -> Vec<Self::Txn>;
}

pub trait Transaction {
    fn txn_hash(&self) -> [u8; 32];
    fn input_elements(&self) -> Vec<element::Element>;
    fn output_elements(&self) -> Vec<element::Element>;
    fn mint_hash(&self) -> Option<element::Element>;
}

impl<B> BlockStore<B>
where
    B: Block + WireMessage,
    B::Txn: WireMessage + Transaction,
{
    fn db_options(create_if_missing: bool) -> rocksdb::Options {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(create_if_missing);
        opts
    }

    #[tracing::instrument(skip_all)]
    pub fn create_or_load(path: &Path) -> Result<Self> {
        if path.exists() && std::fs::read_dir(path)?.next().is_some() {
            Self::load_existing(path)
        } else {
            Self::create(path)
        }
    }

    fn create(path: &Path) -> Result<Self> {
        let db = DB::open(&Self::db_options(true), path)?;

        let store = Self::from_db(db)?;
        store.set_store_version(LATEST_VERSION)?;

        Ok(store)
    }

    #[tracing::instrument(skip_all)]
    fn load_existing(path: &Path) -> Result<Self> {
        let db = DB::open(&Self::db_options(false), path)?;

        Self::from_db(db)
    }

    #[tracing::instrument(skip_all)]
    fn from_db(db: DB) -> Result<Self> {
        let element_history_index = ElementHistoryIndex::load(&db)?;

        Ok(Self {
            db,
            element_history_index,
            _marker: PhantomData,
        })
    }

    pub fn set(&self, block: &B) -> Result<()> {
        let mut batch = rocksdb::WriteBatchWithTransaction::<false>::default();

        let height = block.block_height();
        let block_hash_arr = block.block_hash();

        batch.put(Key::Block(KeyBlock(height)).serialize(), block.to_bytes()?);

        let max_height = self.get_max_height()?;
        if max_height.is_none_or(|max_height| height > max_height) {
            batch.put(Key::MaxHeight.serialize(), height.to_be_bytes());
        }

        batch.put(
            Key::BlockHashToHeight(block_hash_arr).serialize(),
            height.to_be_bytes(),
        );

        for e in Self::txn_entries(block) {
            let (k, v) = e?;
            batch.put(k.serialize(), v);
        }

        if let Some(key) = keys::KeyNonEmptyBlock::from_block(block) {
            batch.put(key.to_key().serialize(), block.to_bytes()?);
        }

        let mut pending_history_entries = Vec::new();

        for txn in block.txns() {
            for (leaf, kind) in txn
                .output_elements()
                .into_iter()
                .map(|e| (e, ElementHistoryKind::Output))
                .chain(
                    txn.input_elements()
                        .into_iter()
                        .map(|e| (e, ElementHistoryKind::Input)),
                )
            {
                let history_key = Key::ElementHistory((leaf, kind));
                let history_data = ElementHistoryData {
                    block_hash: CryptoHash(block_hash_arr),
                    block_height: height,
                };
                let history_value = ElementHistoryValue::V1(history_data);

                let mut history_value_bytes = Vec::new();
                history_value.serialize(&mut history_value_bytes)?;

                batch.put(history_key.serialize(), &history_value_bytes);

                pending_history_entries.push(ElementHistoryIndexEntry {
                    block_height: height,
                    element: leaf,
                    kind,
                });
            }

            if let Some(mint_hash) = txn.mint_hash() {
                let mint_hash_key = Key::MintHash(mint_hash);

                let mint_hash_value = MintHashValue::V1(MintHashData {
                    block_hash: CryptoHash(block_hash_arr),
                    block_height: height,
                });
                let mut mint_hash_value_bytes = Vec::new();
                mint_hash_value.serialize(&mut mint_hash_value_bytes)?;

                batch.put(mint_hash_key.serialize(), &mint_hash_value_bytes);
            }
        }

        self.db.write(batch)?;
        self.element_history_index.append(pending_history_entries);

        Ok(())
    }

    fn txn_entries(block: &B) -> impl Iterator<Item = Result<(Key, Vec<u8>)>> + '_ {
        block
            .txns()
            .into_iter()
            .map(move |tx| Ok((Key::TxnByHash(tx.txn_hash()), tx.to_bytes()?)))
    }

    pub fn get(&self, block_number: BlockHeight) -> Result<Option<B>> {
        let key = Key::Block(KeyBlock(block_number)).serialize();

        let block_bytes = self.db.get(key)?;
        let block = block_bytes.map(|bytes| B::from_bytes(&bytes)).transpose()?;

        Ok(block)
    }

    pub fn get_max_height(&self) -> Result<Option<BlockHeight>> {
        if let Some(max_block) = self.db.get(Key::MaxHeight.serialize())? {
            Ok(Some(BlockHeight(u64::from_be_bytes(
                max_block.try_into().unwrap(),
            ))))
        } else {
            Ok(None)
        }
    }

    pub fn get_block_height_by_hash(&self, block_hash: [u8; 32]) -> Result<Option<BlockHeight>> {
        let key_bytes = Key::BlockHashToHeight(block_hash).serialize();

        if let Some(block_height) = self.db.get(key_bytes)? {
            Ok(Some(BlockHeight(u64::from_be_bytes(
                block_height.try_into().unwrap(),
            ))))
        } else {
            Ok(None)
        }
    }

    pub fn get_pending_block(&self) -> Result<Option<B>> {
        let key = Key::PendingBlock;
        let bytes = self.db.get(key.serialize())?;
        let block = bytes.map(|bytes| B::from_bytes(&bytes)).transpose()?;

        Ok(block)
    }

    pub fn get_txn_by_hash(&self, txn_hash: [u8; 32]) -> Result<Option<B::Txn>> {
        let key = Key::TxnByHash(txn_hash);
        let bytes = self.db.get(key.serialize())?;

        if let Some(bytes) = bytes {
            Ok(Some(B::Txn::from_bytes(&bytes)?))
        } else {
            Ok(None)
        }
    }

    fn store_version(&self) -> Result<u32> {
        if let Some(version) = self.db.get(Key::StoreVersion.serialize())? {
            Ok(u32::from_be_bytes(version.try_into().unwrap()))
        } else {
            Ok(0)
        }
    }

    fn set_store_version(&self, version: u32) -> Result<()> {
        self.db
            .put(Key::StoreVersion.serialize(), version.to_be_bytes())?;
        Ok(())
    }

    fn get_element_history_with_kind(
        &self,
        element: element::Element,
        kind: ElementHistoryKind,
    ) -> Result<Option<ElementHistoryData>> {
        let key = Key::ElementHistory((element, kind));
        let Some(bytes) = self.db.get(key.serialize())? else {
            return Ok(None);
        };
        let value = ElementHistoryValue::deserialize(&mut &bytes[..])?;
        match value {
            ElementHistoryValue::V1(data) => Ok(Some(data)),
        }
    }

    /// Returns (Input History, Output History)
    pub fn get_element_history(
        &self,
        element: element::Element,
    ) -> Result<(Option<ElementHistoryData>, Option<ElementHistoryData>)> {
        Ok((
            self.get_element_history_with_kind(element, ElementHistoryKind::Input)?,
            self.get_element_history_with_kind(element, ElementHistoryKind::Output)?,
        ))
    }

    pub fn element_history_range(
        &self,
        range: impl RangeBounds<BlockHeight>,
    ) -> Vec<ElementHistoryIndexEntry> {
        self.element_history_index.range(range)
    }

    /// Returns mint hash data
    pub fn get_mint_hash(&self, mint_hash: element::Element) -> Result<Option<MintHashData>> {
        let key = Key::MintHash(mint_hash);
        let Some(bytes) = self.db.get(key.serialize())? else {
            return Ok(None);
        };
        let value = MintHashValue::deserialize(&mut &bytes[..])?;
        match value {
            MintHashValue::V1(data) => Ok(Some(data)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use element::Element;
    use tempdir::TempDir;
    use wire_message::test_api::DummyMsg;

    pub(crate) type DummyBlock = DummyMsg<(BlockHeight, [u8; 32], Vec<DummyTxn>)>;
    pub(crate) type DummyTxn =
        wire_message::test_api::DummyMsg<([u8; 32], (Vec<Element>, Vec<Element>), Option<Element>)>;

    impl Block for DummyBlock {
        type Txn = DummyTxn;

        fn block_height(&self) -> BlockHeight {
            self.inner().0
        }

        fn block_hash(&self) -> [u8; 32] {
            self.inner().1
        }

        fn txns(&self) -> Vec<Self::Txn> {
            self.inner().2.clone()
        }
    }

    impl Transaction for DummyTxn {
        fn txn_hash(&self) -> [u8; 32] {
            self.inner().0
        }

        fn input_elements(&self) -> Vec<element::Element> {
            self.inner().1.0.clone()
        }

        fn output_elements(&self) -> Vec<element::Element> {
            self.inner().1.1.clone()
        }

        fn mint_hash(&self) -> Option<element::Element> {
            self.inner().2
        }
    }

    fn temp_dir() -> TempDir {
        TempDir::new("block-store").unwrap()
    }

    #[test]
    fn test_create_or_load() {
        let temp_dir = temp_dir();
        BlockStore::<DummyBlock>::create_or_load(temp_dir.path()).unwrap();
    }

    #[test]
    fn test_set_and_get() {
        let temp_dir = temp_dir();
        let block_store = BlockStore::<DummyBlock>::create_or_load(temp_dir.path()).unwrap();

        let block_number = BlockHeight(1);
        let element1 = Element::from(100u64);
        let element2 = Element::from(101u64);
        let mint_hash = Element::from(102u64);
        let mint_hash_2 = Element::from(103u64);
        let txns = vec![
            DummyTxn::V1((
                [123; 32],
                (vec![], vec![element1, element2]),
                Some(mint_hash),
            )),
            DummyTxn::V1((
                [124; 32],
                (vec![element1, element2], vec![]),
                Some(mint_hash_2),
            )),
        ];
        let block_data = DummyBlock::V1((block_number, [0; 32], txns.clone()));

        block_store.set(&block_data).unwrap();
        let retrieved_data = block_store.get(block_number).unwrap().unwrap();
        assert_eq!(&retrieved_data, &block_data);

        let max_height = block_store.get_max_height().unwrap().unwrap();
        assert_eq!(max_height, block_number);

        let block_height_by_hash = block_store
            .get_block_height_by_hash([0u8; 32])
            .unwrap()
            .unwrap();

        assert_eq!(block_height_by_hash, block_number);

        assert_eq!(
            block_store
                .get_txn_by_hash(txns[0].txn_hash())
                .unwrap()
                .unwrap(),
            txns[0]
        );

        assert_eq!(
            block_store
                .get_txn_by_hash(txns[1].txn_hash())
                .unwrap()
                .unwrap(),
            txns[1]
        );

        assert_eq!(block_store.get_txn_by_hash([125; 32]).unwrap(), None);

        let listed_txns = block_store.list_txns().collect::<Result<Vec<_>>>().unwrap();
        assert_eq!(listed_txns, txns);

        // Test get_element_history
        // TODO: write better tests for this
        let (history1_input, history1_output) = block_store.get_element_history(element1).unwrap();
        let history1_input = history1_input.unwrap();
        let history1_output = history1_output.unwrap();
        assert_eq!(history1_output.block_height, block_number);
        assert_eq!(
            history1_input.block_hash,
            primitives::hash::CryptoHash::new([0u8; 32]),
        );

        let history2 = block_store
            .get_element_history(element2)
            .unwrap()
            .1
            .unwrap();
        assert_eq!(history2.block_height, block_number);
        assert_eq!(
            history2.block_hash,
            primitives::hash::CryptoHash::new([0u8; 32]),
        );

        let non_existent_element = element::Element::from(999u64);
        assert!(
            block_store
                .get_element_history(non_existent_element)
                .unwrap()
                .1
                .is_none()
        );

        // Test get_mint_hash
        let mint_hash_data = block_store.get_mint_hash(mint_hash).unwrap().unwrap();
        assert_eq!(mint_hash_data.block_height, block_number);
        assert_eq!(
            mint_hash_data.block_hash,
            primitives::hash::CryptoHash::new([0u8; 32]),
        );

        let mint_hash_data_2 = block_store.get_mint_hash(mint_hash_2).unwrap().unwrap();
        assert_eq!(mint_hash_data_2.block_height, block_number);
        assert_eq!(
            mint_hash_data_2.block_hash,
            primitives::hash::CryptoHash::new([0u8; 32]),
        );
    }

    #[test]
    fn test_element_history_range() {
        let temp_dir = temp_dir();
        let block_store = BlockStore::<DummyBlock>::create_or_load(temp_dir.path()).unwrap();

        let block_number = BlockHeight(1);
        let element = Element::from(100u64);

        let txns = vec![
            DummyTxn::V1(([1; 32], (vec![], vec![element]), None)),
            DummyTxn::V1(([2; 32], (vec![element], vec![]), None)),
        ];

        let block_data = DummyBlock::V1((block_number, [0; 32], txns));
        block_store.set(&block_data).unwrap();

        let entries = block_store.element_history_range(..=block_number);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, ElementHistoryKind::Output);
        assert_eq!(entries[0].element, element);
        assert_eq!(entries[1].kind, ElementHistoryKind::Input);

        let exact_entries = block_store.element_history_range(block_number..=block_number);
        assert_eq!(exact_entries.len(), 2);

        let later_entries = block_store.element_history_range(block_number.next()..);
        assert!(later_entries.is_empty());
    }

    #[test]
    fn test_list_blocks() {
        let temp_dir = temp_dir();
        let block_store = BlockStore::<DummyBlock>::create_or_load(temp_dir.path()).unwrap();

        for i in 0..10_000 {
            block_store
                .set(&DummyBlock::V1((BlockHeight(i as u64), [0; 32], vec![])))
                .unwrap();
        }

        let blocks: Vec<_> = block_store
            .list(
                BlockHeight(3)..BlockHeight(8),
                BlockListOrder::LowestToHighest,
            )
            .into_iterator()
            .collect::<Result<_>>()
            .unwrap();

        assert_eq!(blocks.len(), 5);
        for (i, (block_number, data)) in blocks.into_iter().enumerate() {
            assert_eq!(block_number, KeyBlock(BlockHeight((i + 3) as u64)));
            assert_eq!(
                &data,
                &DummyBlock::V1((BlockHeight(i as u64 + 3), [0; 32], vec![]))
            );
        }

        let blocks = block_store
            .list_paginated(&None, BlockListOrder::LowestToHighest, usize::MAX)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        let before_blocks = block_store
            .list_paginated(
                &Some(primitives::pagination::CursorChoice::Before(
                    primitives::pagination::CursorChoiceBefore::BeforeInclusive(
                        blocks.last().unwrap().1.block_height(),
                    ),
                )),
                BlockListOrder::LowestToHighest,
                blocks.len(),
            )
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(blocks, before_blocks);

        let before_blocks_except_first = block_store
            .list_paginated(
                &Some(primitives::pagination::CursorChoice::Before(
                    primitives::pagination::CursorChoiceBefore::BeforeInclusive(
                        blocks.last().unwrap().1.block_height(),
                    ),
                )),
                BlockListOrder::LowestToHighest,
                blocks.len() - 1,
            )
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(blocks[1..], before_blocks_except_first);
    }

    #[test]
    fn successor() {
        let temp_dir = temp_dir();
        let db = BlockStore::<DummyBlock>::create_or_load(temp_dir.path()).unwrap();

        let height = BlockHeight(u64::MAX);
        db.set(&DummyBlock::V1((height, [0; 32], vec![]))).unwrap();
        db.db
            .put(
                Key::Block(KeyBlock(BlockHeight(u64::MAX))).serialize_immediate_successor(),
                b"test",
            )
            .unwrap();

        assert_eq!(
            db.list(.., BlockListOrder::LowestToHighest)
                .into_iterator()
                .count(),
            1
        );
    }
}
