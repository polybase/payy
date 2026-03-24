use wire_message::WireMessage;

use super::Result;
use crate::{
    Block, BlockStore, Error, StoreList,
    keys::{self, BlockListOrder, StoreKey},
};

pub(crate) const LATEST_VERSION: u32 = 1;

impl<B> BlockStore<B>
where
    B: Block + WireMessage,
    B::Txn: WireMessage,
{
    #[tracing::instrument(skip(self))]
    pub fn migrate(&self) -> super::Result<()> {
        loop {
            let version = self.store_version()?;

            match version {
                0 => self.migrate_to_v1()?,
                1 => break,
                other => return Err(Error::InvalidVersion(other)),
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn migrate_to_v1(&self) -> Result<()> {
        tracing::info!("Migrating block store to version 1");

        for block in self
            .list(.., BlockListOrder::LowestToHighest)
            .into_iterator()
        {
            let (_, block) = block?;

            let mut batch = rocksdb::WriteBatchWithTransaction::<false>::default();

            let txn_indexes = Self::txn_entries(&block);
            for e in txn_indexes {
                let (k, v) = e?;

                batch.put(k.serialize(), v);
            }

            if let Some(key) = keys::KeyNonEmptyBlock::from_block(&block) {
                batch.put(key.to_key().serialize(), block.to_bytes()?);
            }

            self.db.write(batch)?;
        }

        self.set_store_version(1)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::DummyBlock;

    use super::*;
    use tempdir::TempDir;

    fn temp_dir() -> TempDir {
        TempDir::new("block-store").unwrap()
    }

    #[test]
    fn test_migrate() {
        let temp_dir = temp_dir();
        let block_store = BlockStore::<DummyBlock>::create_or_load(temp_dir.path()).unwrap();

        block_store.migrate().unwrap();

        assert_eq!(block_store.store_version().unwrap(), LATEST_VERSION);
    }
}
