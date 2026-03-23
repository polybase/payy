// lint-long-file-override allow-max-lines=300
use std::{marker::PhantomData, ops::RangeBounds};

use primitives::{block_height::BlockHeight, pagination::CursorChoice};
use rocksdb::DB;
use wire_message::WireMessage;

use crate::{
    Block, BlockListOrder, BlockStore, Error, Result,
    keys::{Key, KeyBlock, KeyNonEmptyBlock, ListableKey, StoreValue},
};

pub trait StoreList {
    type Item;

    fn into_iterator(self) -> impl Iterator<Item = Self::Item> + Send;
    fn reverse(self) -> Self;
    fn map<NewMapped, F: FnMut(Self::Item) -> NewMapped + Send>(
        self,
        map: F,
    ) -> impl StoreList<Item = NewMapped>;
}

pub struct List<'db, Stored> {
    pub(crate) db: &'db DB,
    pub(crate) start_key: Key,
    pub(crate) end_key: Key,
    pub(crate) lower_exclusive: bool,
    pub(crate) upper_inclusive: bool,
    /// If true, iterates the keys in ascending order,
    /// otherwise in descending order.
    pub(crate) start_to_end: bool,
    pub(crate) _phantom: std::marker::PhantomData<Stored>,
}

impl<'db, Stored> StoreList for List<'db, Stored>
where
    Stored: StoreValue,
{
    type Item = Result<(Key, Stored)>;

    fn into_iterator(self) -> impl Iterator<Item = <List<'db, Stored> as StoreList>::Item> {
        let lower_bound = match self.lower_exclusive {
            true => self.start_key.serialize_immediate_successor(),
            false => self.start_key.serialize(),
        };
        let upper_bound = match self.upper_inclusive {
            true => self.end_key.serialize_immediate_successor(),
            false => self.end_key.serialize(),
        };

        let mut read_opts = rocksdb::ReadOptions::default();

        read_opts.set_iterate_lower_bound(lower_bound);
        read_opts.set_iterate_upper_bound(upper_bound);

        let iter = self.db.iterator_opt(
            if self.start_to_end {
                rocksdb::IteratorMode::Start
            } else {
                rocksdb::IteratorMode::End
            },
            read_opts,
        );

        iter.map(move |r| {
            let (key, value) = r?;

            let key = Key::deserialize(key.as_ref()).map_err(|_| Error::InvalidKey)?;
            let value = Stored::deserialize(&value)?;

            Ok((key, value))
        })
    }

    fn reverse(mut self) -> Self {
        self.start_to_end = !self.start_to_end;
        self
    }

    fn map<NewMapped, F: FnMut(Self::Item) -> NewMapped + Send>(
        self,
        map: F,
    ) -> impl StoreList<Item = NewMapped> {
        MappedList {
            list: self,
            map,
            _phantom: PhantomData,
        }
    }
}

pub struct MappedList<F, To, List> {
    map: F,
    list: List,
    _phantom: std::marker::PhantomData<To>,
}

impl<F, To, List> StoreList for MappedList<F, To, List>
where
    List: StoreList,
    F: FnMut(List::Item) -> To + Send,
{
    type Item = To;

    fn into_iterator(mut self) -> impl Iterator<Item = Self::Item> + Send {
        self.list.into_iterator().map(move |r| (self.map)(r))
    }

    fn reverse(mut self) -> Self {
        self.list = self.list.reverse();
        self
    }

    fn map<NewItem, NF: FnMut(Self::Item) -> NewItem + Send>(
        self,
        map: NF,
    ) -> impl StoreList<Item = NewItem> {
        MappedList {
            map,
            list: self,
            _phantom: PhantomData,
        }
    }
}

impl<B> BlockStore<B>
where
    B: Block + WireMessage,
    B::Txn: WireMessage,
{
    pub fn list(
        &self,
        block_range: impl RangeBounds<BlockHeight>,
        order: BlockListOrder,
    ) -> impl StoreList<Item = Result<(KeyBlock, B)>> + '_ {
        let start_bound = block_range.start_bound().map(|bh| KeyBlock(*bh));
        let end_bound = block_range.end_bound().map(|bh| KeyBlock(*bh));
        let key_range = (start_bound, end_bound);

        KeyBlock::list(&self.db, key_range, &order).map(|r| {
            let (k, v) = r?;

            match k {
                Key::Block(block_number) => Ok((block_number, v)),
                _ => Err(Error::InvalidKey),
            }
        })
    }

    pub fn list_paginated(
        &self,
        cursor: &Option<CursorChoice<BlockHeight>>,
        order: BlockListOrder,
        limit: usize,
    ) -> Result<impl Iterator<Item = Result<(KeyBlock, B)>>> {
        Ok(KeyBlock::list_paginated(
            &self.db,
            &cursor.map(|pag| pag.map_pos(|pos| KeyBlock(*pos))),
            order,
            limit,
        )?
        .into_iter()
        .map(|(k, v)| match k {
            Key::Block(block_number) => Ok((block_number, v)),
            _ => Err(Error::InvalidKey),
        }))
    }

    pub fn list_non_empty(
        &self,
        block_range: impl RangeBounds<BlockHeight>,
        order: BlockListOrder,
    ) -> impl StoreList<Item = Result<(KeyNonEmptyBlock, B)>> + '_ {
        let start_bound = block_range.start_bound().map(|bh| KeyNonEmptyBlock(*bh));
        let end_bound = block_range.end_bound().map(|bh| KeyNonEmptyBlock(*bh));
        let key_range = (start_bound, end_bound);

        KeyNonEmptyBlock::list(&self.db, key_range, &order).map(|r| {
            let (k, v) = r?;

            match k {
                Key::NonEmptyBlock(block_number) => Ok((block_number, v)),
                _ => Err(Error::InvalidKey),
            }
        })
    }

    pub fn list_non_empty_paginated(
        &self,
        cursor: &Option<CursorChoice<BlockHeight>>,
        order: BlockListOrder,
        limit: usize,
    ) -> Result<impl Iterator<Item = Result<(KeyNonEmptyBlock, B)>> + '_> {
        Ok(KeyNonEmptyBlock::list_paginated(
            &self.db,
            &cursor.map(|pag| pag.map_pos(|pos| KeyNonEmptyBlock(*pos))),
            order,
            limit,
        )?
        .into_iter()
        .map(|(k, v)| match k {
            Key::NonEmptyBlock(block_number) => Ok((block_number, v)),
            _ => Err(Error::InvalidKey),
        }))
    }

    pub fn list_txns(&self) -> impl Iterator<Item = Result<B::Txn>> + '_ {
        let mut read_opts = rocksdb::ReadOptions::default();

        read_opts.set_iterate_lower_bound(Key::TxnByHash([0; 32]).serialize());
        read_opts
            .set_iterate_upper_bound(Key::TxnByHash([255; 32]).serialize_immediate_successor());

        let iter = self
            .db
            .iterator_opt(rocksdb::IteratorMode::Start, read_opts);
        iter.map(|r| {
            let (_, value) = r?;
            Ok(B::Txn::from_bytes(&value)?)
        })
    }
}
