use element::Element;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid decimal string: {0}")]
    InvalidDecimalString(String),
    #[error("invalid amount")]
    InvalidAmount {
        min: Option<Element>,
        max: Option<Element>,
    },
    #[error("invalid currency code")]
    InvalidCurrencyCode,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    FromUtf8(#[from] core::str::Utf8Error),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
}
