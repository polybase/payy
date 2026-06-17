// lint-long-file-override allow-max-lines=450
use serde_json::Value;

use crate::{Error, Result, selector};

const HOST_API_VERSION: u32 = 1;
const HOST_RESPONSE_CAPACITY: usize = 2 * 1024 * 1024;
const MAX_ERROR_BODY_CHARS: usize = 500;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanContext {
    pub app_id: String,
    pub app_version: String,
    pub chain: String,
    pub manifest_sha256: String,
    pub wallet: String,
    pub wasm_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapToken {
    pub address: String,
    pub decimals: u8,
    pub is_native: bool,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GuestInvocation {
    pub args: Vec<String>,
    pub host_api_version: u32,
    pub metadata: HostMetadata,
    pub output_mode: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct HostMetadata {
    pub app_id: String,
    pub app_version: String,
    pub chain: String,
    pub chain_id: u64,
    #[serde(default)]
    pub debug: bool,
    pub host_api_version: u32,
    pub manifest_sha256: String,
    pub now: u64,
    pub wallet: String,
    pub wasm_sha256: String,
}

impl HostMetadata {
    pub fn plan_context(&self) -> PlanContext {
        PlanContext {
            app_id: self.app_id.clone(),
            app_version: self.app_version.clone(),
            chain: self.chain.clone(),
            manifest_sha256: self.manifest_sha256.clone(),
            wallet: self.wallet.clone(),
            wasm_sha256: self.wasm_sha256.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ActionPlan {
    pub app_id: String,
    pub app_version: String,
    pub wasm_sha256: String,
    pub manifest_sha256: String,
    pub command: String,
    pub wallet: Option<String>,
    pub chain: String,
    #[serde(default)]
    pub steps: Vec<ActionStep>,
    #[serde(default)]
    pub bindings: Vec<ActionBinding>,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ActionStep {
    pub kind: String,
    pub summary: String,
    pub target: Option<String>,
    pub selector: Option<String>,
    pub spender: Option<String>,
    pub value: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ActionBinding {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum HostRequest {
    HttpFetch(HttpFetchRequest),
    ChainRead(ChainReadRequest),
    SimulateTransaction(HostTransaction),
    StructuredOutput { value: Value },
    Diagnostic { level: String, message: String },
    ResolveAddress { value: Option<String> },
    AppStorageGet { key: String },
    AppStorageSet { key: String, value: Value },
    AppStorageRemove { key: String },
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct HttpFetchRequest {
    method: String,
    url: String,
    headers: Vec<HttpHeader>,
    body: Vec<u8>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct HttpHeader {
    name: String,
    value: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct HttpFetchResponse {
    body: Vec<u8>,
    status: u16,
    url: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct ChainReadRequest {
    chain: String,
    operation: ChainReadOperation,
    address: Option<String>,
    data: Option<String>,
    owner: Option<String>,
    spender: Option<String>,
    target: Option<String>,
    token: Option<String>,
    value: Option<String>,
    selector: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum ChainReadOperation {
    TokenMetadata,
    Balance,
    Allowance,
    Gas,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct HostTransaction {
    chain: String,
    data: String,
    target: String,
    value: String,
    selector: Option<String>,
    spender: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct HostCallResponse {
    ok: bool,
    value: Option<Value>,
    error: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct TokenMetadataResponse {
    address: String,
    decimals: Option<u8>,
    label: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct BalanceResponse {
    balance: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct AllowanceResponse {
    allowance: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct GasResponse {
    gas_estimate: Option<String>,
    gas_price: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ResolveAddressResponse {
    address: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct StorageGetResponse {
    exists: bool,
    value: Option<Value>,
}

pub fn ensure_host_abi(invocation: &GuestInvocation) -> Result<()> {
    if invocation.host_api_version != HOST_API_VERSION
        || invocation.metadata.host_api_version != HOST_API_VERSION
    {
        return Err(Error::InvalidHostResponse {
            reason: format!(
                "unsupported host abi version {}",
                invocation.host_api_version
            ),
        });
    }

    Ok(())
}

pub fn http_json(method: &str, url: &str, value: &Value) -> Result<Value> {
    let body = serde_json::to_vec(value).map_err(|err| Error::Serialization {
        reason: err.to_string(),
    })?;
    let response = host_call(HostRequest::HttpFetch(HttpFetchRequest {
        method: method.to_string(),
        url: url.to_string(),
        headers: vec![
            HttpHeader {
                name: "content-type".to_string(),
                value: "application/json".to_string(),
            },
            HttpHeader {
                name: "accept".to_string(),
                value: "application/json".to_string(),
            },
            HttpHeader {
                name: "x-api-key".to_string(),
                value: crate::public_api_key().to_string(),
            },
            HttpHeader {
                name: "x-permit2-disabled".to_string(),
                value: "true".to_string(),
            },
        ],
        body,
    }))?;
    let response = serde_json::from_value::<HttpFetchResponse>(response).map_err(|err| {
        Error::InvalidHostResponse {
            reason: err.to_string(),
        }
    })?;
    if !(200..300).contains(&response.status) {
        let detail = error_body(&response.body)
            .map(|body| format!(": {body}"))
            .unwrap_or_default();
        return Err(Error::HostCallFailed {
            message: format!(
                "{} returned status {}{}",
                response.url, response.status, detail
            ),
        });
    }
    serde_json::from_slice(&response.body).map_err(|err| Error::InvalidHostResponse {
        reason: err.to_string(),
    })
}

fn error_body(body: &[u8]) -> Option<String> {
    let body = core::str::from_utf8(body).ok()?.trim();
    if body.is_empty() {
        return None;
    }
    Some(body.chars().take(MAX_ERROR_BODY_CHARS).collect())
}

pub fn resolve_address(value: Option<&str>) -> Result<String> {
    let response = host_call(HostRequest::ResolveAddress {
        value: value.map(str::to_string),
    })?;
    Ok(parse_host_value::<ResolveAddressResponse>(response)?.address)
}

pub fn token_metadata(chain: &str, token: &str) -> Result<SwapToken> {
    let response = chain_read(ChainReadRequest {
        address: None,
        chain: chain.to_string(),
        data: None,
        operation: ChainReadOperation::TokenMetadata,
        owner: None,
        selector: None,
        spender: None,
        target: Some(token.to_string()),
        token: Some(token.to_string()),
        value: None,
    })?;
    let response = parse_host_value::<TokenMetadataResponse>(response)?;
    let decimals = response
        .decimals
        .ok_or_else(|| Error::InvalidHostResponse {
            reason: format!("token {token} missing decimals"),
        })?;
    Ok(SwapToken {
        is_native: is_native_token(&response.address, &response.label),
        address: response.address,
        decimals,
        label: response.label,
    })
}

pub fn balance(chain: &str, token: &str) -> Result<String> {
    let response = chain_read(ChainReadRequest {
        address: None,
        chain: chain.to_string(),
        data: None,
        operation: ChainReadOperation::Balance,
        owner: None,
        selector: None,
        spender: None,
        target: None,
        token: Some(token.to_string()),
        value: None,
    })?;
    Ok(parse_host_value::<BalanceResponse>(response)?.balance)
}

pub fn allowance(chain: &str, token: &str, spender: &str) -> Result<String> {
    let response = chain_read(ChainReadRequest {
        address: None,
        chain: chain.to_string(),
        data: None,
        operation: ChainReadOperation::Allowance,
        owner: None,
        selector: None,
        spender: Some(spender.to_string()),
        target: Some(token.to_string()),
        token: Some(token.to_string()),
        value: None,
    })?;
    Ok(parse_host_value::<AllowanceResponse>(response)?.allowance)
}

pub fn gas(chain: &str, target: &str, data: &str, value: &str) -> Result<(Option<String>, String)> {
    let response = chain_read(ChainReadRequest {
        address: None,
        chain: chain.to_string(),
        data: Some(data.to_string()),
        operation: ChainReadOperation::Gas,
        owner: None,
        selector: selector(data),
        spender: None,
        target: Some(target.to_string()),
        token: None,
        value: Some(value.to_string()),
    })?;
    let response = parse_host_value::<GasResponse>(response)?;
    Ok((response.gas_estimate, response.gas_price))
}

pub fn simulate(
    chain: &str,
    target: &str,
    data: &str,
    value: &str,
    spender: Option<&str>,
) -> Result<()> {
    host_call(HostRequest::SimulateTransaction(HostTransaction {
        chain: chain.to_string(),
        data: data.to_string(),
        selector: selector(data),
        spender: spender.map(str::to_string),
        target: target.to_string(),
        value: value.to_string(),
    }))?;
    Ok(())
}

pub fn diagnostic(level: &str, message: &str) -> Result<()> {
    host_call(HostRequest::Diagnostic {
        level: level.to_string(),
        message: message.to_string(),
    })?;
    Ok(())
}

#[expect(
    dead_code,
    reason = "storage is part of the app SDK even when Uniswap v1 does not persist data"
)]
pub fn storage_get(key: &str) -> Result<Option<Value>> {
    let response = host_call(HostRequest::AppStorageGet {
        key: key.to_string(),
    })?;
    let response = parse_host_value::<StorageGetResponse>(response)?;
    if response.exists {
        Ok(response.value)
    } else {
        Ok(None)
    }
}

#[expect(
    dead_code,
    reason = "storage is part of the app SDK even when Uniswap v1 does not persist data"
)]
pub fn storage_set(key: &str, value: Value) -> Result<()> {
    host_call(HostRequest::AppStorageSet {
        key: key.to_string(),
        value,
    })?;
    Ok(())
}

#[expect(
    dead_code,
    reason = "storage is part of the app SDK even when Uniswap v1 does not persist data"
)]
pub fn storage_remove(key: &str) -> Result<()> {
    host_call(HostRequest::AppStorageRemove {
        key: key.to_string(),
    })?;
    Ok(())
}

fn chain_read(request: ChainReadRequest) -> Result<Value> {
    host_call(HostRequest::ChainRead(request))
}

fn host_call(request: HostRequest) -> Result<Value> {
    let request = serde_json::to_vec(&request).map_err(|err| Error::Serialization {
        reason: err.to_string(),
    })?;
    let mut response = vec![0_u8; HOST_RESPONSE_CAPACITY];
    let len = beam_host_call_wrapper(&request, &mut response)?;
    let response = serde_json::from_slice::<HostCallResponse>(&response[..len]).map_err(|err| {
        Error::InvalidHostResponse {
            reason: err.to_string(),
        }
    })?;
    if !response.ok {
        return Err(Error::HostCallFailed {
            message: response
                .error
                .unwrap_or_else(|| "host call failed without message".to_string()),
        });
    }
    response.value.ok_or_else(|| Error::InvalidHostResponse {
        reason: "successful host response missing value".to_string(),
    })
}

fn beam_host_call_wrapper(request: &[u8], response: &mut [u8]) -> Result<usize> {
    #[cfg(target_arch = "wasm32")]
    {
        let len = unsafe {
            beam_host_call(
                request.as_ptr(),
                request.len(),
                response.as_mut_ptr(),
                response.len(),
            )
        };
        if len < 0 {
            return Err(Error::HostCallFailed {
                message: format!("host response exceeded buffer: {} bytes", -len),
            });
        }
        usize::try_from(len).map_err(|_| Error::InvalidHostResponse {
            reason: format!("invalid host response length {len}"),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = request;
        let _ = response;
        Err(Error::HostCallFailed {
            message: "host calls are only available in wasm guest execution".to_string(),
        })
    }
}

fn parse_host_value<T>(value: Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value::<T>(value).map_err(|err| Error::InvalidHostResponse {
        reason: err.to_string(),
    })
}

fn is_native_token(address: &str, label: &str) -> bool {
    address.eq_ignore_ascii_case("0x0000000000000000000000000000000000000000")
        || label.eq_ignore_ascii_case("native")
        || label.eq_ignore_ascii_case("eth")
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn beam_host_call(
        request_ptr: *const u8,
        request_len: usize,
        response_ptr: *mut u8,
        response_capacity: usize,
    ) -> i32;
}
