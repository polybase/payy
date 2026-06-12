// lint-long-file-override allow-max-lines=700
use std::{fs, net::IpAddr, path::PathBuf, time::Duration};

use contextful::ResultContextExt;
use contracts::{Address, U256};
use reqwest::{
    Client, Method, StatusCode, Url,
    header::{HeaderMap, HeaderName, HeaderValue, LOCATION},
    redirect::Policy,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use web3::types::{Bytes, CallRequest};

use crate::{
    apps::{
        Error, Result,
        model::{AppPermissions, ChainOperation},
        permissions::{ensure_chain_scope, glob_matches},
    },
    evm::{erc20_allowance, erc20_balance, native_balance, simulate_calldata},
    runtime::{BeamApp, ResolvedToken},
};

const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_REDIRECTS: usize = 5;
const MAX_REQUEST_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostMetadata {
    pub app_id: String,
    pub app_version: String,
    pub chain: String,
    pub chain_id: u64,
    pub host_api_version: u32,
    pub manifest_sha256: String,
    pub now: u64,
    pub wallet: String,
    pub wasm_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HostRequest {
    AppMetadata,
    Args { args: Vec<String> },
    StructuredOutput { value: Value },
    Diagnostic { level: String, message: String },
    HttpFetch(HttpFetchRequest),
    ChainRead(ChainReadRequest),
    SimulateTransaction(HostTransaction),
    SubmitTransaction(HostTransaction),
    PollReceipt { tx_hash: String },
    ResolveAddress { value: Option<String> },
    AppStorageGet { key: String },
    AppStorageSet { key: String, value: Value },
    AppStorageRemove { key: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostCallResponse {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl HostCallResponse {
    pub fn ok(value: Value) -> Self {
        Self {
            ok: true,
            value: Some(value),
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            ok: false,
            value: None,
            error: Some(error),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpFetchRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<HttpHeader>,
    #[serde(default)]
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpFetchResponse {
    pub body: Vec<u8>,
    pub status: u16,
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainReadRequest {
    pub chain: String,
    pub operation: ChainReadOperation,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub spender: Option<String>,
    pub target: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    pub selector: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ChainReadOperation {
    ChainMetadata,
    TokenMetadata,
    Balance,
    Allowance,
    Call,
    Nonce,
    Gas,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostTransaction {
    pub chain: String,
    pub data: String,
    pub target: String,
    pub value: String,
    pub selector: Option<String>,
    pub spender: Option<String>,
}

pub async fn handle_host_request(
    app: &BeamApp,
    permissions: &AppPermissions,
    metadata: &HostMetadata,
    request: HostRequest,
    structured_output: &mut Option<Value>,
    diagnostics: &mut Vec<Value>,
) -> Result<Value> {
    match request {
        HostRequest::AppMetadata => Ok(json!(metadata)),
        HostRequest::Args { args } => Ok(json!({ "args": args })),
        HostRequest::StructuredOutput { value } => {
            *structured_output = Some(value.clone());
            Ok(json!({ "accepted": true }))
        }
        HostRequest::Diagnostic { level, message } => {
            diagnostics.push(json!({
                "level": level,
                "message": message,
            }));
            Ok(json!({ "accepted": true }))
        }
        HostRequest::HttpFetch(request) => Ok(json!(fetch_http(permissions, request).await?)),
        HostRequest::ChainRead(request) => chain_read(app, permissions, request).await,
        HostRequest::SimulateTransaction(transaction) => {
            let (_, client) = app
                .active_chain_client()
                .await
                .context("connect app simulation chain client")?;
            let from = app
                .active_address()
                .await
                .context("resolve app simulation wallet")?;
            simulate_transaction(&client, from, permissions, &transaction).await?;
            Ok(json!({ "ok": true }))
        }
        HostRequest::SubmitTransaction(_) => Err(Error::InvalidHostRequest {
            reason: "transaction submission is only available during approved plan execution"
                .to_string(),
        }),
        HostRequest::PollReceipt { .. } => Err(Error::InvalidHostRequest {
            reason: "receipt polling is only available during approved plan execution".to_string(),
        }),
        HostRequest::ResolveAddress { value } => {
            let address = match value.as_deref() {
                Some(value) => app
                    .resolve_wallet_or_address(value)
                    .await
                    .context("resolve beam app requested address")?,
                None => app
                    .active_address()
                    .await
                    .context("resolve beam app wallet")?,
            };
            Ok(json!({ "address": format!("{address:#x}") }))
        }
        HostRequest::AppStorageGet { key } => {
            let path = app_storage_path(app, &metadata.app_id, &key)?;
            if !path.exists() {
                return Ok(json!({ "value": null, "exists": false }));
            }
            let value = serde_json::from_slice::<Value>(
                &fs::read(path).context("read beam app storage value")?,
            )
            .context("decode beam app storage value")?;
            Ok(json!({ "value": value, "exists": true }))
        }
        HostRequest::AppStorageSet { key, value } => {
            let path = app_storage_path(app, &metadata.app_id, &key)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).context("create beam app storage directory")?;
            }
            fs::write(
                path,
                serde_json::to_vec_pretty(&value).context("encode beam app storage value")?,
            )
            .context("write beam app storage value")?;
            Ok(json!({ "written": true }))
        }
        HostRequest::AppStorageRemove { key } => {
            let path = app_storage_path(app, &metadata.app_id, &key)?;
            if path.exists() {
                fs::remove_file(path).context("remove beam app storage value")?;
            }
            Ok(json!({ "removed": true }))
        }
    }
}

fn app_storage_path(app: &BeamApp, app_id: &str, key: &str) -> Result<PathBuf> {
    if key.is_empty()
        || key.starts_with('.')
        || !key
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.'))
    {
        return Err(Error::InvalidHostRequest {
            reason: format!("invalid app storage key {key}"),
        });
    }

    Ok(app
        .paths
        .root
        .join("apps")
        .join("data")
        .join(app_id)
        .join(key))
}

pub fn ensure_http_allowed(permissions: &AppPermissions, url: &str) -> Result<Url> {
    let url = Url::parse(url).map_err(|_| Error::InvalidPermissionUrl {
        value: url.to_string(),
    })?;
    ensure_safe_https_url(&url)?;
    if !permissions
        .http
        .iter()
        .any(|permission| glob_matches(&permission.url, url.as_str()))
    {
        return Err(Error::HttpPermissionDenied {
            url: url.to_string(),
        });
    }

    Ok(url)
}

pub async fn fetch_http(
    permissions: &AppPermissions,
    request: HttpFetchRequest,
) -> Result<HttpFetchResponse> {
    if request.body.len() > MAX_REQUEST_BYTES {
        return Err(Error::HttpRequestTooLarge {
            bytes: request.body.len(),
        });
    }
    let client = Client::builder()
        .redirect(Policy::none())
        .timeout(HTTP_TIMEOUT)
        .build()
        .context("build beam app http client")?;
    let method =
        Method::from_bytes(request.method.as_bytes()).map_err(|_| Error::InvalidHostRequest {
            reason: format!("invalid http method {}", request.method),
        })?;
    let headers = headers(&request.headers)?;
    let mut current = ensure_http_allowed(permissions, &request.url)?;

    for _ in 0..=MAX_REDIRECTS {
        let mut builder = client
            .request(method.clone(), current.clone())
            .headers(headers.clone());
        if !request.body.is_empty() {
            builder = builder.body(request.body.clone());
        }
        let response = builder
            .send()
            .await
            .context("execute beam app http request")?;
        if is_redirect(response.status()) {
            let Some(location) = response.headers().get(LOCATION) else {
                return Err(Error::InvalidHostRequest {
                    reason: "redirect missing location header".to_string(),
                });
            };
            current = redirect_url(&current, location)?;
            ensure_http_allowed(permissions, current.as_str())?;
            continue;
        }

        let status = response.status().as_u16();
        let url = current.to_string();
        let bytes = response
            .bytes()
            .await
            .context("read beam app http response")?;
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(Error::HttpResponseTooLarge { bytes: bytes.len() });
        }

        return Ok(HttpFetchResponse {
            body: bytes.to_vec(),
            status,
            url,
        });
    }

    Err(Error::HttpRedirectLimitExceeded {
        url: current.to_string(),
    })
}

pub fn ensure_chain_read_allowed(
    permissions: &AppPermissions,
    request: &ChainReadRequest,
) -> Result<()> {
    ensure_chain_scope(
        permissions,
        &request.chain,
        ChainOperation::Read,
        request.target.as_deref(),
        request.selector.as_deref(),
        None,
    )
}

pub async fn chain_read(
    app: &BeamApp,
    permissions: &AppPermissions,
    request: ChainReadRequest,
) -> Result<Value> {
    ensure_chain_read_allowed(permissions, &request)?;
    let chain = app.active_chain().await.context("resolve beam app chain")?;
    if chain.entry.key != request.chain {
        return Err(Error::ChainPermissionDenied {
            chain: request.chain,
            operation: "read".to_string(),
        });
    }

    match request.operation {
        ChainReadOperation::ChainMetadata => Ok(json!({
            "chain": chain.entry.key,
            "chain_id": chain.entry.chain_id,
            "display_name": chain.entry.display_name,
            "native_symbol": chain.entry.native_symbol,
        })),
        ChainReadOperation::TokenMetadata => {
            let token = token_input(&request)?;
            let resolved = if is_native_token(token, &chain.entry.native_symbol) {
                native_token_metadata(&chain.entry.native_symbol)
            } else {
                app.token_for_chain(token, &chain.entry.key)
                    .await
                    .context("resolve beam app token metadata")?
            };
            Ok(json!({
                "address": format!("{:#x}", resolved.address),
                "decimals": resolved.decimals,
                "label": resolved.label,
            }))
        }
        ChainReadOperation::Balance => {
            let (_, client) = app
                .active_chain_client()
                .await
                .context("connect beam app chain client")?;
            let owner = owner_address(app, request.owner.as_deref()).await?;
            let balance = match token_input_optional(&request) {
                Some(token) if is_native_token(token, &chain.entry.native_symbol) => {
                    native_balance(&client, owner)
                        .await
                        .context("read beam app native balance")?
                }
                Some(token) => {
                    let token = app
                        .token_for_chain(token, &chain.entry.key)
                        .await
                        .context("resolve beam app balance token")?;
                    erc20_balance(&client, token.address, owner)
                        .await
                        .context("read beam app erc20 balance")?
                }
                None => native_balance(&client, owner)
                    .await
                    .context("read beam app native balance")?,
            };
            Ok(json!({ "balance": balance.to_string(), "owner": format!("{owner:#x}") }))
        }
        ChainReadOperation::Allowance => {
            let (_, client) = app
                .active_chain_client()
                .await
                .context("connect beam app chain client")?;
            let owner = owner_address(app, request.owner.as_deref()).await?;
            let spender = required_address("spender", request.spender.as_deref())?;
            let token = token_address(app, &chain.entry.key, &request).await?;
            let allowance = erc20_allowance(&client, token, owner, spender)
                .await
                .context("read beam app erc20 allowance")?;
            Ok(json!({
                "allowance": allowance.to_string(),
                "owner": format!("{owner:#x}"),
                "spender": format!("{spender:#x}"),
                "token": format!("{token:#x}"),
            }))
        }
        ChainReadOperation::Call => {
            let (_, client) = app
                .active_chain_client()
                .await
                .context("connect beam app chain client")?;
            let target = required_address("target", request.target.as_deref())?;
            let data = parse_hex_data(required("data", request.data.as_deref())?)?;
            let from = optional_owner_address(app, request.owner.as_deref()).await?;
            let raw = client
                .eth_call(
                    CallRequest {
                        data: Some(Bytes(data)),
                        from,
                        to: Some(target),
                        value: request.value.as_deref().map(parse_u256).transpose()?,
                        ..Default::default()
                    },
                    None,
                )
                .await
                .context("execute beam app chain read call")?;
            Ok(json!({ "raw": format!("0x{}", hex::encode(raw.0)) }))
        }
        ChainReadOperation::Nonce => {
            let (_, client) = app
                .active_chain_client()
                .await
                .context("connect beam app chain client")?;
            let owner = owner_address(app, request.owner.as_deref()).await?;
            let nonce = client.nonce(owner).await.context("fetch beam app nonce")?;
            Ok(json!({ "nonce": nonce.to_string(), "owner": format!("{owner:#x}") }))
        }
        ChainReadOperation::Gas => {
            let (_, client) = app
                .active_chain_client()
                .await
                .context("connect beam app chain client")?;
            let gas_price = client
                .fast_gas_price()
                .await
                .context("fetch beam app gas price")?;
            let estimate = if let (Some(target), Some(data)) =
                (request.target.as_deref(), request.data.as_deref())
            {
                let from = owner_address(app, request.owner.as_deref()).await?;
                Some(
                    client
                        .estimate_gas(
                            CallRequest {
                                data: Some(Bytes(parse_hex_data(data)?)),
                                from: Some(from),
                                to: Some(parse_host_address("target", target)?),
                                value: request.value.as_deref().map(parse_u256).transpose()?,
                                ..Default::default()
                            },
                            None,
                        )
                        .await
                        .context("estimate beam app gas")?,
                )
            } else {
                None
            };
            Ok(json!({
                "gas_price": gas_price.to_string(),
                "gas_estimate": estimate.map(|value| value.to_string()),
            }))
        }
    }
}

fn native_token_metadata(native_symbol: &str) -> ResolvedToken {
    ResolvedToken {
        address: Address::zero(),
        decimals: Some(18),
        label: native_symbol.to_string(),
    }
}

pub fn ensure_transaction_allowed(
    permissions: &AppPermissions,
    transaction: &HostTransaction,
    operation: ChainOperation,
) -> Result<()> {
    ensure_chain_scope(
        permissions,
        &transaction.chain,
        operation,
        Some(&transaction.target),
        transaction.selector.as_deref(),
        transaction.spender.as_deref(),
    )
}

pub async fn simulate_transaction(
    client: &contracts::Client,
    from: Address,
    permissions: &AppPermissions,
    transaction: &HostTransaction,
) -> Result<()> {
    ensure_transaction_allowed(permissions, transaction, ChainOperation::Simulate)?;
    simulate_calldata(
        client,
        from,
        parse_host_address("target", &transaction.target)?,
        parse_hex_data(&transaction.data)?,
        parse_u256(&transaction.value)?,
    )
    .await
    .context("simulate beam app transaction")?;

    Ok(())
}

pub fn ensure_safe_https_url(url: &Url) -> Result<()> {
    if url.scheme() != "https" {
        return Err(Error::InvalidPermissionUrl {
            value: url.to_string(),
        });
    }
    let host = url.host_str().unwrap_or_default();
    if is_blocked_host(host) {
        return Err(Error::HttpHostDenied {
            host: host.to_string(),
        });
    }

    Ok(())
}

fn headers(headers: &[HttpHeader]) -> Result<HeaderMap> {
    let mut output = HeaderMap::new();
    for header in headers {
        let name =
            HeaderName::from_bytes(header.name.as_bytes()).context("parse app header name")?;
        let value = HeaderValue::from_str(&header.value).context("parse app header value")?;
        output.insert(name, value);
    }

    Ok(output)
}

fn redirect_url(current: &Url, location: &HeaderValue) -> Result<Url> {
    let location = location.to_str().context("parse app redirect location")?;
    current
        .join(location)
        .map_err(|_| Error::InvalidHostRequest {
            reason: format!("invalid redirect location {location}"),
        })
}

fn is_redirect(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    )
}

fn is_blocked_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => {
            ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_unspecified()
        }
        Ok(IpAddr::V6(ip)) => ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local(),
        Err(_) => false,
    }
}

async fn owner_address(app: &BeamApp, value: Option<&str>) -> Result<Address> {
    match value {
        Some(value) => Ok(app
            .resolve_wallet_or_address(value)
            .await
            .context("resolve beam app owner address")?),
        None => Ok(app
            .active_address()
            .await
            .context("resolve beam app wallet")?),
    }
}

async fn optional_owner_address(app: &BeamApp, value: Option<&str>) -> Result<Option<Address>> {
    match value {
        Some(value) => Ok(Some(
            app.resolve_wallet_or_address(value)
                .await
                .context("resolve beam app optional owner address")?,
        )),
        None => Ok(None),
    }
}

async fn token_address(app: &BeamApp, chain: &str, request: &ChainReadRequest) -> Result<Address> {
    let token = token_input(request)?;
    Ok(app
        .token_for_chain(token, chain)
        .await
        .context("resolve beam app token address")?
        .address)
}

fn token_input(request: &ChainReadRequest) -> Result<&str> {
    token_input_optional(request).ok_or_else(|| Error::InvalidHostRequest {
        reason: "chain read missing token".to_string(),
    })
}

fn token_input_optional(request: &ChainReadRequest) -> Option<&str> {
    request.token.as_deref().or(request.target.as_deref())
}

fn required<'a>(field: &str, value: Option<&'a str>) -> Result<&'a str> {
    value.ok_or_else(|| Error::InvalidHostRequest {
        reason: format!("chain read missing {field}"),
    })
}

fn required_address(field: &str, value: Option<&str>) -> Result<Address> {
    parse_host_address(field, required(field, value)?)
}

fn is_native_token(token: &str, native_symbol: &str) -> bool {
    token.eq_ignore_ascii_case("native")
        || token.eq_ignore_ascii_case(native_symbol)
        || token.eq_ignore_ascii_case("0x0000000000000000000000000000000000000000")
}

fn parse_hex_data(value: &str) -> Result<Vec<u8>> {
    hex::decode(value.strip_prefix("0x").unwrap_or(value)).map_err(|_| Error::InvalidHostRequest {
        reason: format!("invalid hex data {value}"),
    })
}

fn parse_host_address(field: &str, value: &str) -> Result<Address> {
    value.parse().map_err(|_| Error::InvalidHostRequest {
        reason: format!("invalid {field} address {value}"),
    })
}

fn parse_u256(value: &str) -> Result<U256> {
    if let Some(value) = value.strip_prefix("0x") {
        return Ok(U256::from_str_radix(value, 16).context("parse beam app hex u256")?);
    }
    Ok(value
        .parse::<U256>()
        .context("parse beam app decimal u256")?)
}
