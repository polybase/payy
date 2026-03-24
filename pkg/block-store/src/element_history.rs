use std::{
    ops::{Bound, RangeBounds},
    sync::RwLock,
};

use borsh::BorshDeserialize;
use element::Element;
use primitives::block_height::BlockHeight;
use rocksdb::{DB, ReadOptions};

use crate::{
    Error, Result,
    keys::{ElementHistoryKind, Key},
    values::ElementHistoryValue,
};

#[derive(Debug, Clone, Copy)]
pub struct ElementHistoryIndexEntry {
    pub block_height: BlockHeight,
    pub element: Element,
    pub kind: ElementHistoryKind,
}

impl ElementHistoryIndexEntry {
    fn order_key(&self) -> (BlockHeight, u8) {
        (self.block_height, self.kind.ordering_weight())
    }
}

#[derive(Debug)]
pub struct ElementHistoryIndex {
    entries: RwLock<Vec<ElementHistoryIndexEntry>>,
}

impl ElementHistoryIndex {
    pub fn load(db: &DB) -> Result<Self> {
        let mut entries = Vec::new();

        let mut read_opts = ReadOptions::default();
        read_opts.set_iterate_lower_bound(
            Key::ElementHistory((Element::from_be_bytes([0; 32]), ElementHistoryKind::Input))
                .serialize(),
        );
        read_opts.set_iterate_upper_bound(
            Key::ElementHistory((
                Element::from_be_bytes([255; 32]),
                ElementHistoryKind::Output,
            ))
            .serialize_immediate_successor(),
        );

        for row in db.iterator_opt(rocksdb::IteratorMode::Start, read_opts) {
            let (key_bytes, value_bytes) = row?;
            let key = Key::deserialize(key_bytes.as_ref())?;

            let Key::ElementHistory((element, kind)) = key else {
                return Err(Error::InvalidKey);
            };

            let value = ElementHistoryValue::deserialize(&mut &value_bytes.as_ref()[..])?;
            let ElementHistoryValue::V1(data) = value;

            entries.push(ElementHistoryIndexEntry {
                block_height: data.block_height,
                element,
                kind,
            });
        }

        entries.sort_by_key(|entry| entry.order_key());

        Ok(Self {
            entries: RwLock::new(entries),
        })
    }

    pub fn append(&self, mut new_entries: Vec<ElementHistoryIndexEntry>) {
        if new_entries.is_empty() {
            return;
        }

        new_entries.sort_by_key(|entry| entry.order_key());

        let mut entries = self.entries.write().unwrap();

        if let Some(last) = entries.last().copied()
            && last.order_key() > new_entries[0].order_key()
        {
            entries.extend(new_entries);
            entries.sort_by_key(|entry| entry.order_key());
            return;
        }

        entries.extend(new_entries);
    }

    pub fn range(&self, range: impl RangeBounds<BlockHeight>) -> Vec<ElementHistoryIndexEntry> {
        let entries = self.entries.read().unwrap();
        if entries.is_empty() {
            return Vec::new();
        }

        let len = entries.len();

        let mut start_idx = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(height) => {
                entries.partition_point(|entry| entry.block_height < *height)
            }
            Bound::Excluded(height) => {
                entries.partition_point(|entry| entry.block_height <= *height)
            }
        };

        let mut end_idx = match range.end_bound() {
            Bound::Unbounded => len,
            Bound::Included(height) => {
                entries.partition_point(|entry| entry.block_height <= *height)
            }
            Bound::Excluded(height) => {
                entries.partition_point(|entry| entry.block_height < *height)
            }
        };

        start_idx = start_idx.min(len);
        end_idx = end_idx.min(len);

        if start_idx >= end_idx {
            return Vec::new();
        }

        entries[start_idx..end_idx].to_vec()
    }
}
