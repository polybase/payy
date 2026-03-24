// lint-long-file-override allow-max-lines=300
use std::{ops::Range, path::Path};

use borsh::BorshDeserialize;
use element::Element;
use prover::RollupInput;
use wire_message::WireMessage;

use crate::types::BlockHeight;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid key")]
    InvalidKey,

    #[error("invalid value")]
    InvalidValue,

    #[error("rocksdb error: {0}")]
    RocksDB(#[from] rocksdb::Error),

    #[error("WireMessage error")]
    WireMessage(#[from] wire_message::Error),

    #[error("io error")]
    Io(#[from] std::io::Error),
}

type Result<T, E = Error> = std::result::Result<T, E>;

pub(crate) struct ProverDb {
    db: rocksdb::DB,
}

pub(crate) enum Key {
    LastSeenBlock,
    Rollup { height: BlockHeight },
    ProverVersion,
}

impl Key {
    fn kind(&self) -> u8 {
        match self {
            Self::LastSeenBlock => 0,
            Self::Rollup { .. } => 1,
            Self::ProverVersion => 2,
        }
    }

    fn serialize(&self) -> Vec<u8> {
        let mut out = vec![self.kind()];

        match self {
            Self::LastSeenBlock => {}
            Self::Rollup { height } => {
                out.extend_from_slice(&height.to_be_bytes());
            }
            Self::ProverVersion => {}
        }

        out
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() {
            return Err(Error::InvalidKey);
        }

        match bytes[0] {
            0 => Ok(Self::LastSeenBlock),
            1 => {
                let height = u64::from_be_bytes(bytes[1..9].try_into().unwrap());
                let height = BlockHeight(height);
                Ok(Self::Rollup { height })
            }
            2 => Ok(Self::ProverVersion),
            _ => Err(Error::InvalidKey),
        }
    }
}

#[derive(Debug, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub(crate) struct LastSeenBlock {
    pub(crate) height: BlockHeight,
    pub(crate) root_hash: Element,
}

#[derive(Debug, borsh::BorshSerialize, borsh::BorshDeserialize)]
#[allow(clippy::large_enum_variant)]
enum ValueV1 {
    LastSeenBlock(LastSeenBlock),
    Rollup(RollupInput),
    ProverVersion(u64),
}

#[wire_message::wire_message]
enum Value {
    V1(ValueV1),
}

impl WireMessage for Value {
    type Ctx = ();
    type Err = core::convert::Infallible;

    fn version(&self) -> u64 {
        match self {
            Self::V1(_) => 1,
        }
    }

    fn upgrade_once(self, _ctx: &mut Self::Ctx) -> Result<Self, wire_message::Error> {
        match self {
            Self::V1(_) => Err(Self::max_version_error()),
        }
    }
}

pub(crate) const LATEST_VERSION: u64 = 1;

impl ProverDb {
    pub(crate) fn create_or_load(path: &Path) -> Result<Self> {
        let new_db = !(path.exists() && std::fs::read_dir(path)?.next().is_some());
        let db = rocksdb::DB::open_default(path)?;
        let db = Self { db };
        if new_db {
            db.set_version(LATEST_VERSION)?;
        }
        Ok(db)
    }

    fn get(&self, key: Key) -> Result<Option<Vec<u8>>> {
        let bytes = self.db.get(key.serialize())?;
        Ok(bytes)
    }

    pub(crate) fn get_last_seen_block(&self) -> Result<Option<LastSeenBlock>> {
        let Some(bytes) = self.get(Key::LastSeenBlock)? else {
            return Ok(None);
        };

        let value = Value::deserialize(&mut &*bytes)?;

        match value {
            Value::V1(ValueV1::LastSeenBlock(value)) => Ok(Some(value)),
            Value::V1(_) => Err(Error::InvalidValue),
        }
    }

    fn set(&self, key: Key, value: Value) -> Result<()> {
        self.db.put(key.serialize(), value.to_bytes()?)?;
        Ok(())
    }

    pub(crate) fn set_last_seen_block(&self, last_seen_block: LastSeenBlock) -> Result<()> {
        self.set(
            Key::LastSeenBlock,
            Value::V1(ValueV1::LastSeenBlock(last_seen_block)),
        )?;
        Ok(())
    }

    pub(crate) fn set_rollup(&self, height: BlockHeight, value: RollupInput) -> Result<()> {
        self.set(Key::Rollup { height }, Value::V1(ValueV1::Rollup(value)))?;
        Ok(())
    }

    pub fn list_rollups(
        &self,
        height_range: Range<BlockHeight>,
    ) -> impl Iterator<Item = Result<(BlockHeight, RollupInput)>> + '_ {
        let mut read_opts = rocksdb::ReadOptions::default();

        read_opts.set_iterate_lower_bound(
            Key::Rollup {
                height: height_range.start,
            }
            .serialize(),
        );
        read_opts.set_iterate_upper_bound(
            Key::Rollup {
                height: height_range.end,
            }
            .serialize(),
        );

        let iter = self
            .db
            .iterator_opt(rocksdb::IteratorMode::Start, read_opts);
        iter.map(|r| {
            let (key, value) = r?;

            let key = Key::deserialize(key.as_ref()).map_err(|_| Error::InvalidKey)?;

            let Key::Rollup { height } = key else {
                return Err(Error::InvalidKey);
            };

            let value = Value::deserialize(&mut &*value)?;
            let rollup = match value {
                Value::V1(ValueV1::Rollup(rollup)) => rollup,
                Value::V1(_) => return Err(Error::InvalidValue),
            };

            Ok((height, rollup))
        })
    }

    pub(crate) fn get_version(&self) -> Result<Option<u64>> {
        let Some(bytes) = self.get(Key::ProverVersion)? else {
            return Ok(None);
        };

        let value = Value::deserialize(&mut &*bytes)?;

        match value {
            Value::V1(ValueV1::ProverVersion(value)) => Ok(Some(value)),
            Value::V1(_) => Err(Error::InvalidValue),
        }
    }

    pub(crate) fn set_version(&self, version: u64) -> Result<()> {
        self.set(
            Key::ProverVersion,
            Value::V1(ValueV1::ProverVersion(version)),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_rollups() {
        let tmpdir = tempdir::TempDir::new("list_rollups").unwrap();

        let db = ProverDb::create_or_load(tmpdir.path()).unwrap();

        db.set_rollup(1.into(), RollupInput::default()).unwrap();
        db.set_rollup(2.into(), RollupInput::default()).unwrap();

        let rollups: Vec<_> = db.list_rollups(BlockHeight(1)..BlockHeight(3)).collect();
        assert_eq!(rollups.len(), 2);
        assert_eq!(rollups[0].as_ref().unwrap().0, BlockHeight(1));
        assert_eq!(rollups[1].as_ref().unwrap().0, BlockHeight(2));

        let rollups: Vec<_> = db.list_rollups(BlockHeight(2)..BlockHeight(3)).collect();
        assert_eq!(rollups.len(), 1);
        assert_eq!(rollups[0].as_ref().unwrap().0, BlockHeight(2));
    }
}
