use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Missing KYC field: {0}")]
    MissingKYCField(String),

    #[error("Invalid phone format")]
    InvalidPhoneFormat,

    #[error("[kyc] `{state}` is not a valid US state name or code")]
    InvalidState { state: String },

    #[error("JSON serialization error: {0}")]
    Json(String),
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}

impl Error {
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingKYCField(field.into())
    }
}
