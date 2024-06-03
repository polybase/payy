use crate::CollisionError;

/// An error that can occur when interacting with a a [`Persistent`]
///
/// [`Persistent`]: crate::storage::Persistent
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error from rocksdb
    #[error("rocksdb error: {0}")]
    Rocksdb(#[from] rocksdb::Error),

    /// A collision error
    #[error("collision: {0}")]
    #[deprecated = "use Collisions"]
    Collision(#[from] CollisionError),

    /// A collision error
    #[error("collision: {0:#?}")]
    Collisions(Vec<CollisionError>),

    /// Rocksdb contained the wrong number of bytes for an element
    #[error("deserialization error: {0}")]
    WrongLength(core::array::TryFromSliceError),

    /// An error with the binary format of the data
    #[error("wire message error: {0}")]
    WireMessage(#[from] wire_message::Error),

    /// Database consistency
    #[error("the database contained inconsistent data")]
    DatabaseConsistency,
}
