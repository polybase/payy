use serde_json::{Map, Value, json};
use sourcify_interface::ContractRecord;

use super::target::checksum_address;

#[derive(Clone, Debug)]
pub(super) struct ProxyImplementation {
    pub(super) address: String,
    pub(super) name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ProxyStatus {
    Failed,
    NotPerformed,
    NotProxy,
    Resolved,
}

#[derive(Clone, Debug)]
pub(super) struct ProxyInfo {
    pub(super) implementations: Vec<ProxyImplementation>,
    pub(super) proxy_type: Option<String>,
    pub(super) status: ProxyStatus,
}

pub(super) fn parse_proxy(record: &ContractRecord, requested: bool) -> ProxyInfo {
    if !requested {
        return ProxyInfo {
            implementations: Vec::new(),
            proxy_type: None,
            status: ProxyStatus::NotPerformed,
        };
    }

    let Some(value) = record.proxy_resolution.as_ref() else {
        return not_proxy();
    };
    if value.is_null() {
        return not_proxy();
    }

    parse_proxy_value(value).unwrap_or_else(|_| ProxyInfo {
        implementations: Vec::new(),
        proxy_type: None,
        status: ProxyStatus::Failed,
    })
}

pub(super) fn proxy_value(proxy: &ProxyInfo) -> Value {
    let mut object = Map::new();
    object.insert("status".to_owned(), json!(proxy.status.as_str()));
    object.insert(
        "implementations".to_owned(),
        Value::Array(
            proxy
                .implementations
                .iter()
                .map(|implementation| {
                    let mut value = Map::new();
                    value.insert("address".to_owned(), json!(implementation.address));
                    if let Some(name) = implementation.name.as_ref() {
                        value.insert("name".to_owned(), json!(name));
                    }
                    Value::Object(value)
                })
                .collect(),
        ),
    );
    if let Some(proxy_type) = proxy.proxy_type.as_ref() {
        object.insert("proxy_type".to_owned(), json!(proxy_type));
    }

    Value::Object(object)
}

impl ProxyStatus {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Failed => "failed",
            Self::NotPerformed => "not_performed",
            Self::NotProxy => "not_proxy",
            Self::Resolved => "resolved",
        }
    }
}

fn parse_proxy_value(value: &Value) -> std::result::Result<ProxyInfo, ()> {
    let object = value.as_object().ok_or(())?;
    let is_proxy = object.get("isProxy").and_then(Value::as_bool).ok_or(())?;
    if !is_proxy {
        return Ok(not_proxy());
    }

    let implementations = object
        .get("implementations")
        .and_then(Value::as_array)
        .ok_or(())?
        .iter()
        .map(parse_proxy_implementation)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let proxy_type = object
        .get("proxyType")
        .or_else(|| object.get("type"))
        .and_then(Value::as_str)
        .map(str::to_owned);

    Ok(ProxyInfo {
        implementations,
        proxy_type,
        status: ProxyStatus::Resolved,
    })
}

fn parse_proxy_implementation(value: &Value) -> std::result::Result<ProxyImplementation, ()> {
    let object = value.as_object().ok_or(())?;
    let address = object.get("address").and_then(Value::as_str).ok_or(())?;
    let address = parse_proxy_address(address)?;
    let name = match object.get("name") {
        Some(Value::String(name)) => Some(name.clone()),
        Some(Value::Null) | None => None,
        Some(_) => return Err(()),
    };

    Ok(ProxyImplementation { address, name })
}

fn parse_proxy_address(value: &str) -> std::result::Result<String, ()> {
    if value.len() != 42
        || !value.starts_with("0x")
        || !value[2..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(());
    }
    let bytes = hex::decode(&value[2..]).map_err(|_| ())?;
    let address = contracts::Address::from_slice(&bytes);

    Ok(checksum_address(address))
}

fn not_proxy() -> ProxyInfo {
    ProxyInfo {
        implementations: Vec::new(),
        proxy_type: None,
        status: ProxyStatus::NotProxy,
    }
}
