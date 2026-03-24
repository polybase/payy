// lint-long-file-override allow-max-lines=270

extern crate self as contextful;

use core::fmt::{self, Display, Formatter};
use std::ops::Deref;

pub use contextful_macros::FromContextful;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod internal_error;

pub use internal_error::InternalError;
/// Error wrapper that adds a human-friendly context while preserving the
/// original error as its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contextful<E> {
    context: Box<str>,
    source: Box<E>,
}

impl<E> Contextful<E> {
    /// Construct a contextful error directly from a context string and source error.
    pub fn new(context: impl Into<String>, source: E) -> Self {
        Self {
            context: context.into().into_boxed_str(),
            source: Box::new(source),
        }
    }

    /// Access the context string.
    pub fn context_message(&self) -> &str {
        &self.context
    }

    /// Access the original error.
    pub fn source_ref(&self) -> &E {
        &self.source
    }

    /// Consume and split into parts.
    pub fn into_parts(self) -> (Box<str>, E) {
        let Contextful { context, source } = self;
        (context, *source)
    }

    /// Consume and return the source error, discarding the context.
    pub fn into_source(self) -> E {
        *self.source
    }

    /// Map the inner error into a different type while preserving the context message.
    pub fn map_source<F>(self, f: impl FnOnce(E) -> F) -> Contextful<F> {
        let (context, source) = self.into_parts();
        Contextful {
            context,
            source: Box::new(f(source)),
        }
    }

    fn without_context(source: E) -> Self {
        Self {
            context: Box::from(""),
            source: Box::new(source),
        }
    }

    /// Clone with a new source error.
    pub fn clone_with_source<F>(&self, source: F) -> Contextful<F> {
        Contextful {
            context: self.context.clone(),
            source: Box::new(source),
        }
    }
}

impl<E> Deref for Contextful<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.source
    }
}

impl<E> From<E> for Contextful<E> {
    fn from(source: E) -> Self {
        Self::without_context(source)
    }
}

impl<E> Display for Contextful<E>
where
    E: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.context.is_empty() {
            write!(f, "{}", self.source)
        } else {
            write!(f, "{}: {}", self.context, self.source)
        }
    }
}

impl<E> std::error::Error for Contextful<E>
where
    E: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.source)
    }
}

impl<E> Serialize for Contextful<E>
where
    E: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct ContextfulSer<'a, E> {
            context: &'a str,
            source: &'a E,
        }

        let helper = ContextfulSer {
            context: &self.context,
            source: &self.source,
        };

        helper.serialize(serializer)
    }
}

impl<'de, E> Deserialize<'de> for Contextful<E>
where
    E: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ContextfulDe<E> {
            WithContext { context: Box<str>, source: E },
            Legacy(E),
        }

        match ContextfulDe::deserialize(deserializer)? {
            ContextfulDe::WithContext { context, source } => Ok(Self {
                context,
                source: Box::new(source),
            }),
            ContextfulDe::Legacy(source) => Ok(Self::without_context(source)),
        }
    }
}

/// Extension trait adding `.context(...)` to any `Result<T, E>`.
pub trait ResultContextExt<T, E> {
    /// Attach a context message that will be evaluated eagerly.
    fn context(self, msg: impl Into<String>) -> Result<T, Contextful<E>>;

    /// Contextful error without additional context
    fn without_context(self) -> Result<T, Contextful<E>>;

    /// Attach a context message computed lazily only on error.
    fn with_context<M>(self, msg: M) -> Result<T, Contextful<E>>
    where
        M: FnOnce() -> String;
}

impl<T, E> ResultContextExt<T, E> for Result<T, E> {
    fn context(self, msg: impl Into<String>) -> Result<T, Contextful<E>> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Contextful {
                context: msg.into().into_boxed_str(),
                source: Box::new(e),
            }),
        }
    }

    fn without_context(self) -> Result<T, Contextful<E>> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Contextful {
                context: Box::from(""),
                source: Box::new(e),
            }),
        }
    }

    fn with_context<M>(self, msg: M) -> Result<T, Contextful<E>>
    where
        M: FnOnce() -> String,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Contextful {
                context: msg().into_boxed_str(),
                source: Box::new(e),
            }),
        }
    }
}

/// Extension trait adding `.wrap_err(...)` to any error value.
pub trait ErrorContextExt: Sized {
    /// Wrap this error with an eagerly evaluated context message.
    fn context(self, msg: impl Into<String>) -> Contextful<Self>;

    /// Wrap this error with a lazily evaluated context message.
    fn with_context<M>(self, msg: M) -> Contextful<Self>
    where
        M: FnOnce() -> String;

    /// Wrap this error with an eagerly evaluated context message.
    fn wrap_err(self, msg: impl Into<String>) -> Contextful<Self>;

    /// Wrap this error with a lazily evaluated context message.
    fn wrap_err_with<M>(self, msg: M) -> Contextful<Self>
    where
        M: FnOnce() -> String;

    /// Wrap this error without additional context.
    fn without_context(self) -> Contextful<Self>;
}

impl<E> ErrorContextExt for E
where
    E: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
{
    fn context(self, msg: impl Into<String>) -> Contextful<Self> {
        Contextful::new(msg, self)
    }

    fn with_context<M>(self, msg: M) -> Contextful<Self>
    where
        M: FnOnce() -> String,
    {
        Contextful::new(msg(), self)
    }

    fn wrap_err(self, msg: impl Into<String>) -> Contextful<Self> {
        Contextful::new(msg, self)
    }

    fn wrap_err_with<M>(self, msg: M) -> Contextful<Self>
    where
        M: FnOnce() -> String,
    {
        Contextful::new(msg(), self)
    }

    fn without_context(self) -> Contextful<Self> {
        Contextful::from(self)
    }
}

/// A small prelude to import where you use `.context(...)`.
pub mod prelude {
    pub use crate::{Contextful, ErrorContextExt, InternalError, ResultContextExt};
    pub use contextful_macros::FromContextful;
}

#[cfg(test)]
mod tests;
