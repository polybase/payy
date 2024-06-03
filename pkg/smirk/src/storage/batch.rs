use std::collections::{HashMap, HashSet};

use borsh::{BorshDeserialize, BorshSerialize};
use rocksdb::WriteBatch;
use wire_message::WireMessage;

use crate::{
    hash_cache::KnownHash,
    storage::format::{ValueFormat, ValueV2},
    Batch,
};

use super::{
    format::{KeyFormat, KeyV2},
    Error, Persistent,
};

impl<const DEPTH: usize, V> Persistent<DEPTH, V> {
    /// Insert a [`Batch`] into this [`Persistent`] tree
    ///
    /// ```rust
    /// # use smirk::*;
    /// # use smirk::storage::*;
    /// # let dir = tempdir::TempDir::new("smirk_doctest").unwrap();
    /// # let path = dir.path().join("db");
    /// let mut persistent = Persistent::<64, ()>::new(&path).unwrap();
    /// let batch = batch! { 1, 2, 3 };
    ///
    /// persistent.insert_batch(batch).unwrap();
    ///
    /// assert!(persistent.tree().contains_element(Element::new(1)));
    /// assert!(persistent.tree().contains_element(Element::new(2)));
    /// assert!(persistent.tree().contains_element(Element::new(3)));
    /// ```
    pub fn insert_batch(&mut self, batch: Batch<DEPTH, V>) -> Result<(), Error>
    where
        V: BorshSerialize + BorshDeserialize + Send + Sync + 'static + Clone,
    {
        if batch.is_empty() {
            return Ok(());
        }

        let new_kv_pairs: HashMap<_, _> = batch.entries().cloned().collect();

        let old_hashes: HashSet<_> = self.tree.known_hashes().into_iter().collect();

        self.tree.insert_batch(batch)?;

        let new_hashes: HashSet<_> = self.tree.known_hashes().into_iter().collect();

        let hashes_to_insert = new_hashes
            .iter()
            .copied()
            .filter(|h| !old_hashes.contains(h));

        let hashes_to_remove = old_hashes
            .iter()
            .copied()
            .filter(|h| !new_hashes.contains(h));

        let mut write_batch = WriteBatch::default();

        for (key, value) in new_kv_pairs {
            // insert the v2 key
            let new_key = KeyFormat::V2(KeyV2::Element(key));
            let value = ValueFormat::V2(ValueV2::Metadata(value.into()));
            write_batch.put(new_key.to_bytes().unwrap(), value.to_bytes().unwrap());

            // make sure we don't end up with the v1 and v2 key for the same element at the same
            // time
            let old_key = KeyFormat::V1(key);
            write_batch.delete(old_key.to_bytes().unwrap());
        }

        for KnownHash { left, right, .. } in hashes_to_remove {
            let key = KeyFormat::V2(KeyV2::KnownHash { left, right });
            write_batch.delete(key.to_bytes().unwrap());
        }

        for KnownHash {
            left,
            right,
            result,
        } in hashes_to_insert
        {
            let key = KeyFormat::V2(KeyV2::KnownHash { left, right });
            let value = ValueFormat::<V>::V2(ValueV2::KnownHash(result));
            write_batch.put(key.to_bytes().unwrap(), value.to_bytes().unwrap());
        }

        self.db.write(write_batch)?;

        // TODO: handle case where rocksdb fails with pending list

        Ok(())
    }
}
