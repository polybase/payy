// lint-long-file-override allow-max-lines=300
use std::{io, time::Duration};

use barretenberg_api_interface::{PERMIT_TIMEOUT_HEADER, ServerError};
use contextful::{ErrorContextExt, ResultContextExt};
use serde::de::DeserializeOwned;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

use super::error::TransportError;
use super::stream::MaybeTlsStream;
use crate::error::ClientError;

pub(crate) async fn perform_handshake_and_write_body(
    stream: &mut MaybeTlsStream,
    base_url: &Url,
    path: &str,
    body: &[u8],
    permit_timeout: Option<Duration>,
    expect_continue_timeout_buffer: Duration,
) -> std::result::Result<Vec<u8>, ClientError> {
    let url = base_url
        .join(path)
        .context("join request url")
        .map_err(TransportError::from)?;
    let host = url
        .host_str()
        .ok_or_else(|| TransportError::MissingRequestUrlHost)?;

    let path_query = if let Some(q) = url.query() {
        format!("{}?{}", url.path(), q)
    } else {
        url.path().to_owned()
    };

    let mut headers = format!(
        "POST {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Connection: close\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Expect: 100-continue\r\n",
        path_query,
        host,
        body.len()
    );

    if let Some(pt) = permit_timeout {
        headers.push_str(&format!(
            "{}: {}\r\n",
            PERMIT_TIMEOUT_HEADER,
            duration_to_ms(pt)
        ));
    }
    headers.push_str("\r\n");

    stream
        .write_all(headers.as_bytes())
        .await
        .context("write request headers")
        .map_err(TransportError::Io)?;
    stream
        .flush()
        .await
        .context("flush request headers")
        .map_err(TransportError::Io)?;

    let wait_time = permit_timeout
        .unwrap_or(Duration::ZERO)
        .checked_add(expect_continue_timeout_buffer)
        .unwrap_or(Duration::from_secs(1));

    let (should_write_body, buffer) = wait_for_continue(stream, wait_time).await?;

    if should_write_body {
        stream
            .write_all(body)
            .await
            .context("write request body")
            .map_err(TransportError::Io)?;
        stream
            .flush()
            .await
            .context("flush request body")
            .map_err(TransportError::Io)?;
    }

    Ok(buffer)
}

async fn wait_for_continue(
    stream: &mut MaybeTlsStream,
    wait_time: Duration,
) -> std::result::Result<(bool, Vec<u8>), ClientError> {
    let mut buffer = Vec::with_capacity(1024);
    let mut temp_buf = [0u8; 1024];

    let continue_result = tokio::time::timeout(wait_time, async {
        loop {
            let n =
                read_allow_unexpected_eof(stream, &mut temp_buf, "read continue response").await?;
            if n == 0 {
                return Err(ClientError::Transport(
                    TransportError::ConnectionClosedWhileWaitingForContinue,
                ));
            }
            buffer.extend_from_slice(&temp_buf[..n]);

            let mut headers = [httparse::Header {
                name: "",
                value: &[],
            }; 16];
            let mut req = httparse::Response::new(&mut headers);
            match req
                .parse(&buffer)
                .context("parse continue response")
                .map_err(TransportError::Parse)?
            {
                httparse::Status::Complete(offset) => {
                    if req.code == Some(100) {
                        buffer.drain(..offset);
                        return Ok((true, buffer));
                    } else {
                        return Ok((false, buffer));
                    }
                }
                httparse::Status::Partial => continue,
            }
        }
    })
    .await;

    match continue_result {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Transport(TransportError::ContinueTimeout)),
    }
}

pub(crate) async fn read_and_parse_response<Resp>(
    stream: &mut MaybeTlsStream,
    initial_buffer: Vec<u8>,
) -> std::result::Result<Resp, ClientError>
where
    Resp: DeserializeOwned,
{
    let mut buffer = initial_buffer;
    if buffer.capacity() < 4096 {
        buffer.reserve(4096 - buffer.len());
    }

    let mut temp_buf = [0u8; 4096];
    let mut body_start = 0;
    let mut headers_parsed = false;

    loop {
        if !headers_parsed {
            let mut headers = [httparse::Header {
                name: "",
                value: &[],
            }; 64];
            let mut resp = httparse::Response::new(&mut headers);
            match resp
                .parse(&buffer)
                .context("parse response headers")
                .map_err(TransportError::Parse)?
            {
                httparse::Status::Complete(offset) => {
                    body_start = offset;
                    headers_parsed = true;
                    break;
                }
                httparse::Status::Partial => {} // continue reading
            }
        }

        let n = read_allow_unexpected_eof(stream, &mut temp_buf, "read response").await?;
        if n == 0 {
            if buffer.is_empty() {
                return Err(ClientError::Transport(
                    TransportError::ConnectionClosedBeforeResponse,
                ));
            }
            break;
        }
        buffer.extend_from_slice(&temp_buf[..n]);
    }

    if headers_parsed
        && let Err(err) = stream.read_to_end(&mut buffer).await
        && !is_unexpected_eof(&err)
    {
        return Err(ClientError::Transport(TransportError::Io(
            err.wrap_err("read response body"),
        )));
    }

    let mut headers = [httparse::Header {
        name: "",
        value: &[],
    }; 64];
    let mut resp = httparse::Response::new(&mut headers);
    resp.parse(&buffer)
        .context("parse response headers")
        .map_err(TransportError::Parse)?;

    let status = resp.code.unwrap_or(0);
    let body_slice = if headers_parsed && body_start <= buffer.len() {
        &buffer[body_start..]
    } else {
        &[]
    };

    match status {
        200..=299 => serde_json::from_slice(body_slice)
            .context("parse response body")
            .map_err(TransportError::Json)
            .map_err(ClientError::from),
        400..=499 => {
            let body_str = String::from_utf8_lossy(body_slice).into_owned();
            Err(serde_json::from_str::<ServerError>(&body_str)
                .map(ClientError::Server)
                .unwrap_or_else(|_| {
                    TransportError::UnexpectedResponse {
                        status,
                        body: body_str,
                    }
                    .into()
                }))
        }
        500..=599 => {
            let body_str = String::from_utf8_lossy(body_slice).into_owned();
            Err(ClientError::Transport(TransportError::Infrastructure {
                status,
                body: body_str,
            }))
        }
        _ => {
            let body_str = String::from_utf8_lossy(body_slice).into_owned();
            Err(ClientError::Transport(TransportError::UnexpectedResponse {
                status,
                body: body_str,
            }))
        }
    }
}

fn duration_to_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn is_unexpected_eof(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::UnexpectedEof
}

async fn read_allow_unexpected_eof(
    stream: &mut MaybeTlsStream,
    buffer: &mut [u8],
    context: &'static str,
) -> std::result::Result<usize, ClientError> {
    match stream.read(buffer).await {
        Ok(bytes) => Ok(bytes),
        Err(err) if is_unexpected_eof(&err) => Ok(0),
        Err(err) => Err(ClientError::Transport(TransportError::Io(
            err.wrap_err(context),
        ))),
    }
}
