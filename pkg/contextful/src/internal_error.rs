use core::fmt::{self, Display, Formatter};
use std::{error::Error, io, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Contextful;

/// Wrapper for internal errors stored as contextful dynamic errors.
///
/// Uses `Arc` internally to allow cloning without copying the underlying error.
#[derive(Debug, Clone)]
pub struct InternalError {
    context: Box<str>,
    source: Arc<Box<dyn Error + Send + Sync + 'static>>,
}

impl<E> From<Contextful<E>> for InternalError
where
    E: Error + Send + Sync + 'static,
{
    fn from(error: Contextful<E>) -> Self {
        let (context, source) = error.into_parts();
        Self {
            context,
            source: Arc::new(Box::new(source) as Box<dyn Error + Send + Sync + 'static>),
        }
    }
}

impl Display for InternalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.context.is_empty() {
            write!(f, "{}", self.source)
        } else {
            write!(f, "{}: {}", self.context, self.source)
        }
    }
}

impl Error for InternalError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&**self.source)
    }
}

impl InternalError {
    /// Attempt to downcast the internal error to a specific type.
    ///
    /// # Safety
    ///
    /// The caller must ensure that this `InternalError` has not been cloned,
    /// i.e., there is only one strong reference to the inner `Arc`. This is
    /// automatically true immediately after constructing the error via
    /// `InternalError::from(contextful_error)`.
    ///
    /// If there are multiple references, the downcast will fail and return
    /// the original error unchanged.
    pub unsafe fn downcast<E>(self) -> Result<Contextful<E>, InternalError>
    where
        E: Error + Send + Sync + 'static,
    {
        let InternalError { context, source } = self;

        match Arc::try_unwrap(source) {
            Ok(boxed) => match boxed.downcast::<E>() {
                Ok(source) => Ok(Contextful { context, source }),
                Err(boxed) => Err(InternalError {
                    context,
                    source: Arc::new(boxed),
                }),
            },
            Err(arc) => Err(InternalError {
                context,
                source: arc,
            }),
        }
    }

    /// Attempt to downcast a reference to the internal error to a specific type.
    ///
    /// This is safe because it only returns a reference and doesn't require
    /// exclusive ownership of the `Arc`.
    pub fn downcast_ref<E>(&self) -> Option<&E>
    where
        E: Error + Send + Sync + 'static,
    {
        (**self.source).downcast_ref::<E>()
    }

    /// Attempt to downcast a reference by walking the error source chain.
    ///
    /// This recursively checks the current error and all its sources until
    /// it finds an error of the specified type or reaches the end of the chain.
    pub fn recursive_downcast_ref<E>(&self) -> Option<&E>
    where
        E: Error + Send + Sync + 'static,
    {
        let mut current: &(dyn Error + 'static) = &**self.source;
        loop {
            if let Some(e) = current.downcast_ref::<E>() {
                return Some(e);
            }
            current = current.source()?;
        }
    }

    /// Access the context message.
    pub fn context_message(&self) -> &str {
        &self.context
    }

    /// Access the source error.
    pub fn source_ref(&self) -> &(dyn Error + Send + Sync + 'static) {
        &**self.source
    }
}

impl Serialize for InternalError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct InternalErrorSer<'a> {
            context: &'a str,
            source: String,
        }

        let helper = InternalErrorSer {
            context: &self.context,
            source: self.source.to_string(),
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InternalError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct InternalErrorDe {
            context: String,
            source: String,
        }

        let InternalErrorDe { context, source } = InternalErrorDe::deserialize(deserializer)?;
        let source =
            Arc::new(Box::new(io::Error::other(source)) as Box<dyn Error + Send + Sync + 'static>);
        Ok(Self {
            context: context.into_boxed_str(),
            source,
        })
    }
}
