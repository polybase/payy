use serde::Serialize;
use serde_json::{Map, Value};

/// Converts a serializable struct into a vector of key-value pairs for use in query parameters.
/// This function skips None values in the serialized representation.
///
/// # Examples
///
/// ```
/// use serde::Serialize;
/// use client_http::serde_to_query_params;
///
/// #[derive(Serialize)]
/// struct MyQuery {
///     id: Option<u32>,
///     name: String,
///     filter: Option<String>,
/// }
///
/// let query = MyQuery {
///     id: Some(123),
///     name: "test".to_string(),
///     filter: None,
/// };
///
/// let params = serde_to_query_params(&query);
/// // Results in: [("id", "123"), ("name", "test")]
/// ```
pub fn serde_to_query_params<T: Serialize>(value: &T) -> Vec<(String, String)> {
    let json = serde_json::to_value(value).unwrap_or(Value::Null);

    match json {
        Value::Object(map) => flatten_json_object(map),
        _ => Vec::new(),
    }
}

/// Flattens a JSON object into key-value pairs, skipping null/None values
fn flatten_json_object(map: Map<String, Value>) -> Vec<(String, String)> {
    map.into_iter()
        .filter_map(|(key, value)| match value {
            Value::Null => None,
            Value::String(s) => Some((key, s)),
            Value::Number(n) => Some((key, n.to_string())),
            Value::Bool(b) => Some((key, b.to_string())),
            _ => Some((key, value.to_string())),
        })
        .collect()
}

/// Waits for a specified number of seconds.
pub(crate) async fn wait_for_secs(seconds: u64) {
    tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;
}
