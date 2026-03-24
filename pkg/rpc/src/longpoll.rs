use crate::error::HTTPError;
pub use actix_web::http::header::LAST_MODIFIED;
use actix_web::{HttpResponse, Responder};
use chrono::{DateTime, SecondsFormat, Utc};
use eyre::Result;
use serde::Serialize;
use std::future::Future;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum PollData<T> {
    Modified {
        data: T,
        modified_at: Option<DateTime<Utc>>,
    },
    NotModified,
}

impl<T> From<T> for PollData<T> {
    fn from(data: T) -> Self {
        PollData::Modified {
            data,
            modified_at: None,
        }
    }
}

pub async fn wait_for_update<'a, F, Fut, T>(
    timeout_secs: u64,
    poll_interval_secs: u64,
    mut check_fn: F,
) -> Result<PollData<T>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Option<(DateTime<Utc>, T)>>>,
    T: 'a,
{
    let timeout_secs = timeout_secs.min(120);
    let start_time = Utc::now();
    let interval = Duration::from_secs(poll_interval_secs);

    loop {
        match check_fn().await {
            Ok(Some((modified_at, data))) => {
                return Ok(PollData::Modified {
                    data,
                    modified_at: Some(modified_at),
                });
            }
            Ok(None) => {}
            Err(e) => return Err(e),
        }

        if timeout_secs == 0 || (Utc::now() - start_time).num_seconds() > timeout_secs as i64 {
            return Ok(PollData::NotModified);
        }

        tokio::time::sleep(interval).await;
    }
}

impl<T> PollData<T> {
    pub fn is_modified(&self) -> bool {
        matches!(self, PollData::Modified { .. })
    }
}

pub type HttpJsonLongPollResult<T> = Result<PollData<T>, HTTPError>;

impl<T: Serialize> Responder for PollData<T> {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &actix_web::HttpRequest) -> HttpResponse {
        match self {
            PollData::Modified { data, modified_at } => {
                let mut res = HttpResponse::Ok();
                if let Some(modified_at) = modified_at {
                    res.insert_header((
                        LAST_MODIFIED,
                        modified_at.to_rfc3339_opts(SecondsFormat::Micros, true),
                    ));
                    res.insert_header((
                        "Last-Modified-Unix",
                        modified_at.timestamp_micros().to_string(),
                    ));
                }
                res.json(data)
            }
            PollData::NotModified => HttpResponse::NotModified().finish(),
        }
    }
}
