use std::{backtrace::Backtrace, io::Read};

use borsh::{BorshDeserialize, BorshSerialize};

use strum::EnumCount;
// re-export the relevant crates for the macro
pub use borsh;
pub use static_assertions;
pub use strum_macros;

pub use error::{Error, ErrorKind};

/// Add required supertrait impls to a [`WireMessage`] implementer
pub use wire_message_macro::wire_message;
mod error;

#[cfg(feature = "test-api")]
pub mod test_api;

/// Types which are versioned and can be automatically upgraded
///
/// The easiest way to implement this trait is to add the `#[wire_message]` attribute, which will:
///  - implement all the required supertraits
///  - guarantee that you are implementing it on an enum
pub trait WireMessage:
    Sized + BorshSerialize + BorshDeserialize + EnumCount + Send + Sync + 'static
{
    /// The type of the context provided to `upgrade` functions (for example, a database connection)
    type Ctx;

    /// The type of custom errors produced by `upgrade` functions
    ///
    /// If your upgrade cannot fail (other than upgrading past the maximum version), consider using
    /// [`core::convert::Infallible`] to mark this case as impossible (this would be the default if
    /// Rust supported defaults on associated types)
    type Err;

    /// The maximum version of this type
    const MAX_VERSION: u64 = <Self as EnumCount>::COUNT as u64;

    /// The current version of this value
    fn version(&self) -> u64;

    /// Upgrade this message to the next highest version, or return `None` if it is already at the
    /// max version
    fn upgrade_once(self, ctx: &mut Self::Ctx) -> Result<Self, Error>;

    /// Upgrade this message until it is at [`Self::MAX_VERSION`]
    fn upgrade(mut self, ctx: &mut Self::Ctx) -> Result<Self, Error> {
        while self.version() < Self::MAX_VERSION {
            self = self.upgrade_once(ctx)?;
        }

        Ok(self)
    }

    /// Deserialize an instance of `Self` from bytes
    fn from_bytes(mut bytes: &[u8]) -> Result<Self, Error> {
        #[allow(clippy::disallowed_methods)]
        Self::deserialize(&mut bytes).map_err(|e| Error {
            kind: ErrorKind::Deserialize,
            backtrace: Backtrace::capture(),
            source: Some(e),
        })
    }

    /// Deserialize an instance of `Self` from bytes
    fn from_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        #[allow(clippy::disallowed_methods)]
        Self::deserialize_reader(reader).map_err(|e| Error {
            kind: ErrorKind::Deserialize,
            backtrace: Backtrace::capture(),
            source: Some(e),
        })
    }

    /// Serialize this instance to a [`Vec<u8>`][Vec]
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        #[allow(clippy::disallowed_methods)]
        borsh::to_vec(self).map_err(|e| Error {
            kind: ErrorKind::Serialize,
            backtrace: Backtrace::capture(),
            source: Some(e),
        })
    }

    /// Serialize this instance and write the bytes to an instance of [`Write`][std::io::Write]
    fn to_bytes_in<W: std::io::Write>(&self, writer: W) -> Result<(), Error> {
        #[allow(clippy::disallowed_methods)]
        borsh::to_writer(writer, self).map_err(|e| Error {
            kind: ErrorKind::Serialize,
            backtrace: Backtrace::capture(),
            source: Some(e),
        })
    }

    /// Construct an [`Error`] representing the case where you are trying to upgrade the maximum
    /// version of a message type
    fn max_version_error() -> Error {
        Error {
            kind: ErrorKind::MaxVersion {
                version: Self::MAX_VERSION,
            },
            backtrace: Backtrace::capture(),
            source: None,
        }
    }
}
