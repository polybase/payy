// lint-long-file-override allow-max-lines=250
use std::time::Duration;

use async_trait::async_trait;
use contextful::ResultContextExt;
use futures_util::StreamExt;
use reqwest::{StatusCode, redirect};
use serde_json::Value;
use sourcify_interface::{
    ContractLookup, ContractRecord, ContractResponse, Error as InterfaceError, SourcifyClient,
};
use url::Url;

use crate::error::{Error, Result};

const SOURCIFY_ENDPOINT: &str = "https://sourcify.dev/server/v2";
const USER_AGENT: &str = "beam-cli sourcify-client-reqwest";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_REDIRECTS: usize = 3;

/// Constructor-time configuration for [`SourcifyReqwestClient`].
#[derive(Clone, Debug, Default)]
pub enum SourcifyReqwestClientOptions {
    /// Public Sourcify v2 endpoint.
    #[default]
    Public,
    /// Custom endpoint for tests and development.
    Custom {
        /// Base endpoint without the `/contract/<chain>/<address>` suffix.
        endpoint: String,
    },
}

/// Reqwest-backed implementation of [`SourcifyClient`].
#[derive(Clone)]
pub struct SourcifyReqwestClient {
    client: reqwest::Client,
    endpoint: Url,
}

impl SourcifyReqwestClient {
    /// Create a client for the public Sourcify endpoint.
    pub fn new(options: SourcifyReqwestClientOptions) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .redirect(redirect_policy())
            .build()
            .context("build Sourcify reqwest client")?;
        Self::with_reqwest_client(client, options)
    }

    #[cfg(test)]
    pub(crate) fn with_reqwest_client(
        client: reqwest::Client,
        options: SourcifyReqwestClientOptions,
    ) -> Result<Self> {
        let endpoint = endpoint_url(options)?;
        Ok(Self { client, endpoint })
    }

    #[cfg(not(test))]
    fn with_reqwest_client(
        client: reqwest::Client,
        options: SourcifyReqwestClientOptions,
    ) -> Result<Self> {
        let endpoint = endpoint_url(options)?;
        Ok(Self { client, endpoint })
    }

    async fn contract_internal(&self, lookup: &ContractLookup) -> Result<ContractResponse> {
        let requested_fields = lookup
            .fields
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let query_fields = lookup
            .fields
            .iter()
            .filter_map(|field| field.as_query_str().map(str::to_owned))
            .collect::<Vec<_>>();
        let url = contract_url(
            &self.endpoint,
            lookup.chain_id,
            &lookup.address,
            &query_fields,
        )?;
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .context("send Sourcify contract request")?;
        let status = response.status();
        let body = read_capped_body(response, lookup.response_cap_bytes).await?;

        match status {
            StatusCode::OK => parse_contract_response(
                url.as_str(),
                requested_fields,
                lookup.chain_id,
                &lookup.address,
                &body,
            ),
            StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND
                if body_indicates_unsupported_chain(&body) =>
            {
                Err(Error::ChainUnsupported {
                    chain_id: lookup.chain_id,
                })
            }
            StatusCode::NOT_FOUND => Err(Error::NotVerified),
            StatusCode::TOO_MANY_REQUESTS => Err(Error::LookupFailed {
                reason: "Sourcify rate limit exceeded".to_owned(),
            }),
            status if status.is_server_error() => Err(Error::LookupFailed {
                reason: format!("Sourcify returned HTTP {status}"),
            }),
            status => Err(Error::LookupFailed {
                reason: format!("Sourcify returned HTTP {status}"),
            }),
        }
    }
}

#[async_trait]
impl SourcifyClient for SourcifyReqwestClient {
    async fn contract(
        &self,
        lookup: &ContractLookup,
    ) -> std::result::Result<ContractResponse, InterfaceError> {
        Ok(self.contract_internal(lookup).await?)
    }
}

fn endpoint_url(options: SourcifyReqwestClientOptions) -> Result<Url> {
    let endpoint = match options {
        SourcifyReqwestClientOptions::Public => SOURCIFY_ENDPOINT.to_owned(),
        SourcifyReqwestClientOptions::Custom { endpoint } => endpoint,
    };

    Url::parse(&endpoint).map_err(|_| Error::InvalidEndpoint { endpoint })
}

fn contract_url(
    endpoint: &Url,
    chain_id: u64,
    address: &str,
    requested_fields: &[String],
) -> Result<Url> {
    let mut url = endpoint.clone();
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|()| Error::InvalidEndpoint {
                endpoint: endpoint.to_string(),
            })?;
        segments
            .pop_if_empty()
            .push("contract")
            .push(&chain_id.to_string())
            .push(address);
    }
    if !requested_fields.is_empty() {
        url.query_pairs_mut()
            .append_pair("fields", &requested_fields.join(","));
    }
    Ok(url)
}

fn redirect_policy() -> redirect::Policy {
    redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            return attempt.error("too many Sourcify redirects");
        }

        if attempt.url().scheme() != "https" {
            return attempt.error("Sourcify redirected to a non-HTTPS URL");
        }

        attempt.follow()
    })
}

async fn read_capped_body(response: reqwest::Response, cap_bytes: usize) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("read Sourcify response body")?;
        if body.len().saturating_add(chunk.len()) > cap_bytes {
            return Err(Error::ResponseTooLarge { cap_bytes });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn parse_contract_response(
    endpoint: &str,
    requested_fields: Vec<String>,
    chain_id: u64,
    address: &str,
    body: &[u8],
) -> Result<ContractResponse> {
    let value = serde_json::from_slice::<Value>(body).map_err(|err| Error::MalformedResponse {
        reason: err.to_string(),
    })?;
    validate_requested_match_fields(&value, &requested_fields)?;

    let contract = serde_json::from_value::<ContractRecord>(value).map_err(|err| {
        Error::MalformedResponse {
            reason: err.to_string(),
        }
    })?;
    contract
        .validate_target(chain_id, address)
        .map_err(|reason| Error::MalformedResponse { reason })?;

    Ok(ContractResponse {
        endpoint: endpoint.to_owned(),
        requested_fields,
        contract,
    })
}

fn validate_requested_match_fields(value: &Value, requested_fields: &[String]) -> Result<()> {
    let object = value.as_object().ok_or_else(|| Error::MalformedResponse {
        reason: "response is not a JSON object".to_owned(),
    })?;
    for field in requested_fields {
        if matches!(field.as_str(), "creationMatch" | "runtimeMatch") && !object.contains_key(field)
        {
            return Err(Error::MalformedResponse {
                reason: format!("{field} field is missing"),
            });
        }
    }

    Ok(())
}

fn body_indicates_unsupported_chain(body: &[u8]) -> bool {
    let body = String::from_utf8_lossy(body).to_ascii_lowercase();
    body.contains("unsupported") && body.contains("chain")
}
