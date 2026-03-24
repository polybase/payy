use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use borsh::{BorshDeserialize, BorshSerialize};
use rocksdb::WriteBatch;
use wire_message::WireMessage;

use crate::{
    Batch,
    storage::format::{ValueFormat, ValueV2},
};

use super::{
    Error, Persistent,
    format::{KeyFormat, KeyV2},
};

impl<const DEPTH: usize, V> Persistent<DEPTH, V> {
    /// Insert a [`Batch`] into this [`Persistent`] tree
    ///
    /// ```rust
    /// # use smirk::*;
    /// # use element::Element;
    /// # use smirk::storage::*;
    /// # let dir = tempdir::TempDir::new("smirk_doctest").unwrap();
    /// # let path = dir.path().join("db");
    /// let mut persistent = Persistent::<64, ()>::new(&path).unwrap();
    /// let batch = batch! { 1, 2, 3 };
    ///
    /// persistent.insert_batch(batch).unwrap();
    ///
    /// assert!(persistent.tree().contains_element(&Element::new(1)));
    /// assert!(persistent.tree().contains_element(&Element::new(2)));
    /// assert!(persistent.tree().contains_element(&Element::new(3)));
    /// ```
    pub fn insert_batch(&mut self, batch: Batch<DEPTH, V>) -> Result<(), Error>
    where
        V: BorshSerialize + BorshDeserialize + Send + Sync + 'static + Clone,
    {
        if batch.is_empty() {
            return Ok(());
        }

        let new_kv_pairs: HashMap<_, _> = batch.insert_entries().iter().cloned().collect();
        let removed_elements = batch.remove_elements().collect::<Vec<_>>();

        let hash_changes = Arc::new(Mutex::new(HashMap::new()));
        self.tree.insert_batch(
            batch,
            |(left, right)| {
                hash_changes.lock().unwrap().insert((*left, *right), None);
            },
            |(left, right, result)| {
                hash_changes
                    .lock()
                    .unwrap()
                    .insert((*left, *right), Some(*result));
            },
        )?;
        let (hashes_to_insert, hashes_to_remove): (Vec<_>, Vec<_>) = Arc::try_unwrap(hash_changes)
            .unwrap()
            .into_inner()
            .unwrap()
            .into_iter()
            .partition(|(_hash, result)| result.is_some());

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

        for key in removed_elements {
            for k in [KeyFormat::V2(KeyV2::Element(key)), KeyFormat::V1(key)] {
                write_batch.delete(k.to_bytes().unwrap());
            }
        }

        for ((left, right), _) in hashes_to_remove {
            let key = KeyFormat::V2(KeyV2::KnownHash { left, right });
            write_batch.delete(key.to_bytes().unwrap());
        }

        for ((left, right), result) in hashes_to_insert {
            let key = KeyFormat::V2(KeyV2::KnownHash { left, right });
            let value = ValueFormat::<V>::V2(ValueV2::KnownHash(result.unwrap()));
            write_batch.put(key.to_bytes().unwrap(), value.to_bytes().unwrap());
        }

        self.db.write(write_batch)?;

        // TODO: handle case where rocksdb fails with pending list

        Ok(())
    }
}
