use crate::error::{Error, Result};
use reqwest::header::LAST_MODIFIED;
use reqwest::{Response, StatusCode};
use rpc::{
    error::{ErrorOutput, TryFromHTTPError},
    longpoll::PollData,
};
use serde::Serialize;
use std::fmt::{self, Debug};

/// Metadata describing the HTTP request associated with a response.
#[derive(Debug, Clone)]
pub struct HttpMetadata {
    /// HTTP method used for the request.
    pub method: reqwest::Method,
    /// Path the request was sent to.
    pub path: String,
}

impl fmt::Display for HttpMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.method, self.path)
    }
}

enum ResponseInner {
    Http(Response),
    Inline {
        body: String,
        status: StatusCode,
        last_modified: Option<chrono::DateTime<chrono::Utc>>,
    },
}

/// Wrapper around a response for extracting values and metadata.
pub struct ClientResponse {
    response: ResponseInner,
    http_metadata: HttpMetadata,
}

impl ClientResponse {
    #[must_use]
    /// Create a new client response wrapper.
    pub fn new(response: Response, http_metadata: HttpMetadata) -> Self {
        Self {
            response: ResponseInner::Http(response),
            http_metadata,
        }
    }

    /// Create a response from a serializable value, primarily for tests.
    #[must_use]
    pub fn from_serializable<T: Serialize>(value: &T, http_metadata: HttpMetadata) -> Self {
        let body =
            serde_json::to_string(value).expect("serializing inline HTTP response should succeed");
        Self {
            response: ResponseInner::Inline {
                body,
                status: StatusCode::OK,
                last_modified: None,
            },
            http_metadata,
        }
    }

    /// Parse client response into the target type.
    pub async fn to_value<
        R: serde::de::DeserializeOwned,
        E: TryFrom<ErrorOutput, Error = TryFromHTTPError> + Debug,
    >(
        self,
    ) -> Result<R, E> {
        let metadata = self.http_metadata.clone();
        match self.response {
            ResponseInner::Http(response) => {
                let text = response
                    .text()
                    .await
                    .map_err(|err| Error::Reqwest(err.to_string(), self.http_metadata.clone()))?;
                Self::parse_body(&text, self.http_metadata)
            }
            ResponseInner::Inline { body, .. } => Self::parse_body(&body, metadata),
        }
    }

    /// Parse client response with long poll info.
    pub async fn to_long_poll<
        R: serde::de::DeserializeOwned,
        E: TryFrom<ErrorOutput, Error = TryFromHTTPError> + Debug,
    >(
        self,
    ) -> Result<PollData<R>, E> {
        match self.response {
            ResponseInner::Http(response) => {
                if response.status() == StatusCode::NOT_MODIFIED {
                    return Ok(PollData::NotModified);
                }

                // Get header
                let last_modified = response
                    .headers()
                    .get(LAST_MODIFIED)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|time| chrono::DateTime::parse_from_rfc3339(time).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc));

                let text = response
                    .text()
                    .await
                    .map_err(|err| Error::Reqwest(err.to_string(), self.http_metadata.clone()))?;

                let data = Self::parse_body::<R, E>(&text, self.http_metadata)?;

                Ok(PollData::Modified {
                    data,
                    modified_at: last_modified,
                })
            }
            ResponseInner::Inline {
                body,
                status,
                last_modified,
            } => {
                if status == StatusCode::NOT_MODIFIED {
                    return Ok(PollData::NotModified);
                }

                let data = Self::parse_body::<R, E>(&body, self.http_metadata)?;

                Ok(PollData::Modified {
                    data,
                    modified_at: last_modified,
                })
            }
        }
    }

    #[allow(clippy::result_large_err)]
    fn parse_body<R, E>(body: &str, metadata: HttpMetadata) -> Result<R, E>
    where
        R: serde::de::DeserializeOwned,
        E: TryFrom<ErrorOutput, Error = TryFromHTTPError> + Debug,
    {
        serde_json::from_str::<R>(body).map_err(|err| Error::SerdeJson(err.to_string(), metadata))
    }
}
