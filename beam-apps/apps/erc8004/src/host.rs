// lint-long-file-override allow-max-lines=500
use serde_json::Value;

use crate::{Error, Result};

const HOST_API_VERSION: u32 = 1;
const HOST_RESPONSE_CAPACITY: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanContext {
    pub app_id: String,
    pub app_version: String,
    pub chain: String,
    pub manifest_sha256: String,
    pub wallet: String,
    pub wasm_sha256: String,
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
    #[serde(default)]
    pub dynamic_contracts: Vec<DynamicContractScope>,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DynamicContractScope {
    pub chain: String,
    pub contract: String,
    #[serde(default)]
    pub reason: String,
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
    SignTypedData(TypedDataSignRequest),
    Diagnostic { level: String, message: String },
    ResolveAddress { value: Option<String> },
    AppStorageGet { key: String },
    AppStorageSet { key: String, value: String },
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
pub struct HttpFetchResponse {
    pub body: Vec<u8>,
    pub status: u16,
    pub url: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct ChainReadRequest {
    chain: String,
    operation: ChainReadOperation,
    address: Option<String>,
    data: Option<String>,
    dynamic_contracts: Vec<DynamicContractScope>,
    from_block: Option<u64>,
    owner: Option<String>,
    spender: Option<String>,
    target: Option<String>,
    token: Option<String>,
    topics: Vec<Option<Vec<String>>>,
    to_block: Option<u64>,
    value: Option<String>,
    selector: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum ChainReadOperation {
    Call,
    Logs,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct TypedDataSignRequest {
    chain: String,
    dynamic_contracts: Vec<DynamicContractScope>,
    domain_separator: String,
    fields: Vec<TypedDataDisplayField>,
    primary_type: String,
    struct_hash: String,
    verifying_contract: String,
    wallet: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct TypedDataDisplayField {
    name: String,
    kind: String,
    value: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct HostCallResponse {
    ok: bool,
    value: Option<Value>,
    error: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct StorageGetResponse {
    exists: bool,
    value: Option<Value>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct CallResponse {
    pub raw: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct LogsResponse {
    pub logs: Vec<LogEntry>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct LogEntry {
    pub address: String,
    pub data: String,
    pub topics: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct SignatureResponse {
    pub digest: String,
    pub signature: String,
    pub signer: String,
}

pub fn ensure_host_abi(invocation: &GuestInvocation) -> Result<()> {
    if invocation.host_api_version < HOST_API_VERSION
        || invocation.metadata.host_api_version < HOST_API_VERSION
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

pub fn eth_call(
    chain: &str,
    target: &str,
    data: &str,
    dynamic_contracts: &[DynamicContractScope],
) -> Result<String> {
    let response = chain_read(ChainReadRequest {
        address: None,
        chain: chain.to_string(),
        data: Some(data.to_string()),
        dynamic_contracts: dynamic_contracts.to_vec(),
        from_block: None,
        operation: ChainReadOperation::Call,
        owner: None,
        selector: selector_from_calldata(data),
        spender: None,
        target: Some(target.to_string()),
        token: None,
        topics: Vec::new(),
        to_block: None,
        value: None,
    })?;
    Ok(parse_host_value::<CallResponse>(response)?.raw)
}

pub fn logs(
    chain: &str,
    target: &str,
    topics: Vec<Option<Vec<String>>>,
    from_block: Option<u64>,
    to_block: Option<u64>,
    dynamic_contracts: &[DynamicContractScope],
) -> Result<LogsResponse> {
    let response = chain_read(ChainReadRequest {
        address: None,
        chain: chain.to_string(),
        data: None,
        dynamic_contracts: dynamic_contracts.to_vec(),
        from_block,
        operation: ChainReadOperation::Logs,
        owner: None,
        selector: None,
        spender: None,
        target: Some(target.to_string()),
        token: None,
        topics,
        to_block,
        value: None,
    })?;
    parse_host_value(response)
}

pub fn sign_typed_data(
    chain: &str,
    wallet: &str,
    verifying_contract: &str,
    domain_separator: &str,
    struct_hash: &str,
    fields: Vec<(&str, &str, String)>,
    dynamic_contracts: &[DynamicContractScope],
) -> Result<SignatureResponse> {
    let response = host_call(HostRequest::SignTypedData(TypedDataSignRequest {
        chain: chain.to_string(),
        dynamic_contracts: dynamic_contracts.to_vec(),
        domain_separator: domain_separator.to_string(),
        fields: fields
            .into_iter()
            .map(|(kind, name, value)| TypedDataDisplayField {
                kind: kind.to_string(),
                name: name.to_string(),
                value,
            })
            .collect(),
        primary_type: "AgentWalletSet".to_string(),
        struct_hash: struct_hash.to_string(),
        verifying_contract: verifying_contract.to_string(),
        wallet: wallet.to_string(),
    }))?;
    parse_host_value(response)
}

pub fn http_get(url: &str) -> Result<HttpFetchResponse> {
    let response = host_call(HostRequest::HttpFetch(HttpFetchRequest {
        method: "GET".to_string(),
        url: url.to_string(),
        headers: Vec::new(),
        body: Vec::new(),
    }))?;
    parse_host_value(response)
}

pub fn resolve_address(value: Option<&str>) -> Result<String> {
    let response = host_call(HostRequest::ResolveAddress {
        value: value.map(str::to_string),
    })?;
    Ok(response
        .get("address")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::InvalidHostResponse {
            reason: "resolve address response missing address".to_string(),
        })?
        .to_string())
}

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

pub fn storage_set(key: &str, value: &str) -> Result<()> {
    host_call(HostRequest::AppStorageSet {
        key: key.to_string(),
        value: value.to_string(),
    })?;
    Ok(())
}

fn chain_read(request: ChainReadRequest) -> Result<Value> {
    host_call(HostRequest::ChainRead(request))
}

fn selector_from_calldata(data: &str) -> Option<String> {
    let data = data.strip_prefix("0x").unwrap_or(data);
    (data.len() >= 8).then(|| format!("0x{}", &data[..8]))
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

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn beam_host_call(
        request_ptr: *const u8,
        request_len: usize,
        response_ptr: *mut u8,
        response_capacity: usize,
    ) -> i32;
}
