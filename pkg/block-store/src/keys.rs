// lint-long-file-override allow-max-lines=400
use std::{
    marker::PhantomData,
    ops::{Bound, RangeBounds},
};

use crate::list::StoreList;
use primitives::{block_height::BlockHeight, pagination::CursorChoice};
use rocksdb::DB;
use wire_message::WireMessage;

use crate::{Block, Error, Result, list::List};

pub(crate) trait StoreKey: Clone {
    fn to_key(&self) -> Key;
    fn serialize_to(&self, to: &mut Vec<u8>);
    fn deserialize(bytes: &[u8]) -> Result<Self>;
}

pub(crate) trait StoreValue {
    fn deserialize(bytes: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

impl<T> StoreValue for T
where
    T: WireMessage,
{
    fn deserialize(bytes: &[u8]) -> Result<Self> {
        Ok(Self::from_bytes(bytes)?)
    }
}

pub(crate) trait KeyOrder: Copy {
    /// If this is the default order the keys are indexed in.
    fn is_indexed_order(&self) -> bool;

    fn reverse(&self) -> Self;
}

pub(crate) trait ListableKey<Value>: StoreKey
where
    Value: StoreValue,
{
    type Order: KeyOrder;

    fn min_value() -> Self;
    fn max_value() -> Self;

    fn list<'db>(
        db: &'db DB,
        range: impl RangeBounds<Self>,
        order: &Self::Order,
    ) -> List<'db, Value>
    where
        Self: Sized,
    {
        List {
            db,
            start_key: match range.start_bound() {
                std::ops::Bound::Included(x) => x.to_key(),
                std::ops::Bound::Excluded(x) => x.to_key(),
                std::ops::Bound::Unbounded => Self::min_value().to_key(),
            },
            end_key: match range.end_bound() {
                std::ops::Bound::Included(x) => x.to_key(),
                std::ops::Bound::Excluded(x) => x.to_key(),
                std::ops::Bound::Unbounded => Self::max_value().to_key(),
            },
            lower_exclusive: match range.start_bound() {
                std::ops::Bound::Included(_) => false,
                std::ops::Bound::Excluded(_) => true,
                std::ops::Bound::Unbounded => false,
            },
            upper_inclusive: match range.end_bound() {
                std::ops::Bound::Included(_) => true,
                std::ops::Bound::Excluded(_) => false,
                std::ops::Bound::Unbounded => true,
            },
            start_to_end: order.is_indexed_order(),
            _phantom: PhantomData,
        }
    }

    fn list_paginated(
        db: &DB,
        cursor: &Option<CursorChoice<Self>>,
        order: Self::Order,
        limit: usize,
    ) -> Result<Vec<(Key, Value)>>
    where
        Self: Sized,
    {
        let (start, end) = match cursor {
            None => (Bound::Unbounded, Bound::Unbounded),
            Some(CursorChoice::Before(before)) => (Bound::Unbounded, before.to_bound().cloned()),
            Some(CursorChoice::After(after)) => (after.to_bound().cloned(), Bound::Unbounded),
        };

        let (start, end) = match order.is_indexed_order() {
            true => (start, end),
            false => (end, start),
        };

        let (reverse_results, order) = match cursor {
            None | Some(CursorChoice::After(_)) => (false, order),
            Some(CursorChoice::Before(_)) => (true, order.reverse()),
        };

        let mut results = Self::list(db, (start, end), &order)
            .into_iterator()
            .take(limit)
            .collect::<Result<Vec<(Key, Value)>>>()?;

        if reverse_results {
            results.reverse();
        }

        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub enum Key {
    Block(KeyBlock),
    MaxHeight,
    BlockHashToHeight([u8; 32]),
    PendingBlock,
    TxnByHash([u8; 32]),
    StoreVersion,
    NonEmptyBlock(KeyNonEmptyBlock),
    LockedElement([u8; 32]),
    ElementHistory((element::Element, ElementHistoryKind)),
    MintHash(element::Element),
}

// TODO: this might be confusing,
// when a note is created, it will appear as "Output" history.
// Maybe rename?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementHistoryKind {
    Input,
    Output,
}

impl ElementHistoryKind {
    fn to_byte(self) -> u8 {
        match self {
            ElementHistoryKind::Input => 0,
            ElementHistoryKind::Output => 1,
        }
    }

    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(ElementHistoryKind::Input),
            1 => Some(ElementHistoryKind::Output),
            _ => None,
        }
    }

    pub(crate) fn ordering_weight(&self) -> u8 {
        match self {
            ElementHistoryKind::Output => 0,
            ElementHistoryKind::Input => 1,
        }
    }
}

impl Key {
    fn kind(&self) -> u8 {
        match self {
            Self::Block(_) => 0,
            Self::MaxHeight => 1,
            Self::BlockHashToHeight(_) => 2,
            Self::PendingBlock => 3,
            Self::TxnByHash(_) => 4,
            Self::StoreVersion => 5,
            Self::NonEmptyBlock(_) => 6,
            Self::LockedElement(_) => 7,
            Self::ElementHistory(_) => 8,
            Self::MintHash(_) => 9,
        }
    }

    pub(crate) fn serialize(&self) -> Vec<u8> {
        let mut out = vec![self.kind()];

        match self {
            Self::Block(block_number) => {
                block_number.serialize_to(&mut out);
            }
            Self::MaxHeight => {}
            Self::BlockHashToHeight(block_hash) => {
                out.extend_from_slice(block_hash);
            }
            Self::PendingBlock => {}
            Self::TxnByHash(txn_hash) => {
                out.extend_from_slice(txn_hash);
            }
            Self::StoreVersion => {}
            Self::NonEmptyBlock(block_number) => {
                block_number.serialize_to(&mut out);
            }
            Self::LockedElement(locked_element) => {
                out.extend_from_slice(locked_element);
            }
            Self::ElementHistory((element, kind)) => {
                out.extend_from_slice(&element.to_be_bytes());
                out.extend_from_slice(&[kind.to_byte()]);
            }
            Self::MintHash(mint_hash) => {
                out.extend_from_slice(&mint_hash.to_be_bytes());
            }
        }

        out
    }

    pub(crate) fn deserialize(bytes: &[u8]) -> Result<Self> {
        let Some((kind, bytes)) = bytes.split_first() else {
            return Err(Error::InvalidKey);
        };

        match *kind {
            0 => KeyBlock::deserialize(bytes).map(Self::Block),
            1 => Ok(Self::MaxHeight),
            2 => {
                let mut block_hash = [0u8; 32];
                block_hash.copy_from_slice(&bytes[0..32]);
                Ok(Self::BlockHashToHeight(block_hash))
            }
            3 => Ok(Self::PendingBlock),
            4 => {
                let mut txn_hash = [0u8; 32];
                txn_hash.copy_from_slice(&bytes[0..32]);
                Ok(Self::TxnByHash(txn_hash))
            }
            5 => Ok(Self::StoreVersion),
            6 => KeyNonEmptyBlock::deserialize(bytes).map(Self::NonEmptyBlock),
            7 => {
                let mut locked_element = [0u8; 32];
                locked_element.copy_from_slice(&bytes[0..32]);
                Ok(Self::LockedElement(locked_element))
            }
            8 => {
                let element_arr: &[u8; 32] =
                    bytes[0..32].try_into().map_err(|_| Error::InvalidKey)?;
                let element = element::Element::from_be_bytes(*element_arr);
                let kind = ElementHistoryKind::from_byte(bytes[32]).unwrap();
                Ok(Self::ElementHistory((element, kind)))
            }
            9 => {
                let mint_hash_arr: &[u8; 32] =
                    bytes[0..32].try_into().map_err(|_| Error::InvalidKey)?;
                let mint_hash = element::Element::from_be_bytes(*mint_hash_arr);
                Ok(Self::MintHash(mint_hash))
            }
            _ => Err(Error::InvalidKey),
        }
    }

    pub(crate) fn serialize_immediate_successor(&self) -> Vec<u8> {
        let mut out = self.serialize();
        out.push(0);
        out
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyBlock(pub(crate) BlockHeight);

impl StoreKey for KeyBlock {
    fn to_key(&self) -> Key {
        Key::Block(self.clone())
    }

    fn serialize_to(&self, to: &mut Vec<u8>) {
        to.extend_from_slice(&self.0.to_be_bytes());
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        let Ok(u64_bytes) = TryInto::<[u8; 8]>::try_into(&bytes[0..8]) else {
            return Err(Error::InvalidKey);
        };

        let block_height = BlockHeight::from(u64::from_be_bytes(u64_bytes));
        Ok(KeyBlock(block_height))
    }
}

impl<V: StoreValue> ListableKey<V> for KeyBlock {
    type Order = BlockListOrder;

    fn min_value() -> Self {
        KeyBlock(BlockHeight(0))
    }

    fn max_value() -> Self {
        KeyBlock(BlockHeight(u64::MAX))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BlockListOrder {
    LowestToHighest,
    HighestToLowest,
}

impl KeyOrder for BlockListOrder {
    fn is_indexed_order(&self) -> bool {
        match self {
            Self::LowestToHighest => true,
            Self::HighestToLowest => false,
        }
    }

    fn reverse(&self) -> Self {
        match self {
            Self::LowestToHighest => Self::HighestToLowest,
            Self::HighestToLowest => Self::LowestToHighest,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyNonEmptyBlock(pub(crate) BlockHeight);

impl KeyNonEmptyBlock {
    pub(crate) fn from_block<B: Block>(block: &B) -> Option<Self> {
        if block.txns().is_empty() {
            None
        } else {
            Some(Self(block.block_height()))
        }
    }
}

impl StoreKey for KeyNonEmptyBlock {
    fn to_key(&self) -> Key {
        Key::NonEmptyBlock(KeyNonEmptyBlock(self.0))
    }

    fn serialize_to(&self, to: &mut Vec<u8>) {
        to.extend_from_slice(&self.0.to_be_bytes());
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        let Ok(u64_bytes) = TryInto::<[u8; 8]>::try_into(&bytes[0..8]) else {
            return Err(Error::InvalidKey);
        };

        Ok(KeyNonEmptyBlock(BlockHeight(u64::from_be_bytes(u64_bytes))))
    }
}

impl<V: StoreValue> ListableKey<V> for KeyNonEmptyBlock {
    type Order = BlockListOrder;

    fn min_value() -> Self {
        KeyNonEmptyBlock(BlockHeight(0))
    }

    fn max_value() -> Self {
        KeyNonEmptyBlock(BlockHeight(u64::MAX))
    }
}
