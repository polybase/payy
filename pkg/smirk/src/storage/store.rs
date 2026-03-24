use std::collections::HashSet;

use borsh::{BorshDeserialize, BorshSerialize};
use rocksdb::{DB, IteratorMode, WriteBatch};
use wire_message::WireMessage;

use crate::{
    Tree,
    hash_cache::{KnownHash, SimpleHashCache},
};

use super::format::{KeyFormat, KeyV2, ValueFormat, ValueV2};

pub(super) fn synchronize_hashes<const DEPTH: usize, V>(
    db: &DB,
    tree: &Tree<DEPTH, V, SimpleHashCache>,
) -> Result<(), super::Error>
where
    V: Clone + Send + Sync + 'static + BorshDeserialize + BorshSerialize,
{
    // we take hashes from the tree rather than the cache because the cache might have been
    // recently evicted
    let in_memory_hashes = tree.known_hashes();

    let in_db_hashes = db
        .iterator(IteratorMode::Start)
        .filter_map(|result| {
            let (key, value) = result.ok()?;

            let KeyFormat::V2(KeyV2::KnownHash { left, right }) =
                KeyFormat::from_bytes(&key).ok()?
            else {
                return None;
            };

            let ValueFormat::<V>::V2(ValueV2::KnownHash(result)) =
                ValueFormat::from_bytes(&value).ok()?
            else {
                return None;
            };

            Some(KnownHash {
                left,
                right,
                result,
            })
        })
        .collect::<HashSet<_>>();

    let hashes_to_insert = in_memory_hashes
        .into_iter()
        .filter(|hash| !in_db_hashes.contains(hash));

    let mut batch = WriteBatch::default();

    for known_hash in hashes_to_insert {
        let KnownHash {
            left,
            right,
            result,
        } = known_hash;

        let key_format = KeyFormat::V2(KeyV2::KnownHash { left, right });
        let value_format = ValueFormat::<V>::V2(ValueV2::KnownHash(result));

        let key_bytes = key_format.to_bytes()?;
        let value_bytes = value_format.to_bytes()?;

        batch.put(key_bytes, value_bytes);
    }

    db.write(batch)?;

    Ok(())
}
