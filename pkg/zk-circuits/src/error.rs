use std::fmt::Debug;

/// An error produced by zk-circuits
///
/// It is designed to be FFI-safe
#[derive(Debug)]
pub struct Error {
    /// The debug representation of the underlying error
    debug: String,
    /// The type name of the error that caused this error
    ///
    /// Note that the exact representation of this field are unspecified, since they come from
    /// [`core::any::type_name`], so should only be used for debugging (i.e. don't try to parse
    /// this)
    type_name: String,

    /// Was this error caused by a panic (if false, it was caused by a `Result::Err`)
    was_panic: bool,
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

impl Error {
    pub(crate) fn err<T: Debug>(inner: T) -> Self {
        Self {
            debug: format!("{inner:?}"),
            type_name: core::any::type_name::<T>().to_string(),
            was_panic: false,
        }
    }

    #[allow(unused)]
    pub(crate) fn panic<T: Debug>(inner: T) -> Self {
        Self {
            debug: format!("{inner:?}"),
            type_name: core::any::type_name::<T>().to_string(),
            was_panic: true,
        }
    }

    /// Was this error caused by a panic (if false, it was caused by a `Result::Err`)
    #[inline]
    pub fn was_panic(&self) -> bool {
        self.was_panic
    }

    /// The debug representation of the underlying error
    #[inline]
    pub fn debug_repr(&self) -> &str {
        &self.debug
    }

    /// The type name of the error that caused this error
    ///
    /// Note that the exact representation of this field are unspecified, since they come from
    /// [`core::any::type_name`], so should only be used for debugging (i.e. don't try to parse
    /// this)
    #[inline]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }
}

impl std::error::Error for Error {}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Error(type = {})", self.type_name)
    }
}
