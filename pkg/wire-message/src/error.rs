use std::{backtrace::Backtrace, fmt};

pub struct Error<T = core::convert::Infallible> {
    pub(crate) backtrace: Backtrace,
    pub(crate) kind: ErrorKind<T>,
    pub(crate) source: Option<std::io::Error>,
}

impl<T: fmt::Debug> fmt::Debug for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind<T = core::convert::Infallible> {
    Serialize,
    Deserialize,
    Upgrade(T),
    MaxVersion { version: u64 },
}

impl<T> Error<T> {
    #[must_use]
    pub fn kind(&self) -> &ErrorKind<T> {
        &self.kind
    }

    #[inline]
    pub fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    /// Is this a serialization error
    #[inline]
    #[must_use]
    pub fn is_serialize(&self) -> bool {
        matches!(&self.kind, ErrorKind::Serialize)
    }

    /// Is this a deserialization error
    #[inline]
    #[must_use]
    pub fn is_deserialize(&self) -> bool {
        matches!(&self.kind, ErrorKind::Deserialize)
    }

    /// Is this a generic upgrade error
    #[inline]
    #[must_use]
    pub fn is_upgrade(&self) -> bool {
        matches!(&self.kind, ErrorKind::Upgrade(_))
    }

    /// Is this a max-version error
    #[inline]
    #[must_use]
    pub fn is_max_version(&self) -> bool {
        matches!(&self.kind, ErrorKind::MaxVersion { .. })
    }
}

impl<T> fmt::Display for Error<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::Serialize => write!(f, "serialize error: {}", self.source.as_ref().unwrap()),
            ErrorKind::Deserialize => {
                write!(f, "deserialize error: {}", self.source.as_ref().unwrap())
            }
            ErrorKind::MaxVersion { version } => write!(
                f,
                "tried to upgrade, but the version was {version}, which is the max version"
            ),
            ErrorKind::Upgrade(e) => write!(f, "failed to upgrade: {e}"),
        }
    }
}

impl<T: fmt::Debug + fmt::Display> std::error::Error for Error<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source.as_ref().map(|i| i as &dyn std::error::Error)
    }
}
