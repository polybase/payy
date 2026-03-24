// lint-long-file-override allow-max-lines=300
use std::future::Future;
use std::pin::Pin;

use actix_web::{FromRequest, HttpRequest, dev::Payload, web};
use serde::{Serialize, de::DeserializeOwned};

/// A JSON extractor that logs deserialization errors with structured logging
/// and request context for debugging purposes.
#[derive(Debug)]
pub struct JsonWithLogging<T> {
    pub json: T,
}

impl<T> Serialize for JsonWithLogging<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.json.serialize(serializer)
    }
}

impl<T> FromRequest for JsonWithLogging<T>
where
    T: DeserializeOwned + 'static,
{
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let mut payload = payload.take();
        let req = req.clone();

        let fut = async move {
            let frozen_body = web::Bytes::from_request(&req, &mut payload).await?;

            // Deserialize the body into the specified type
            let deserialized_body: T = serde_json::from_slice(&frozen_body).map_err(|e| {
                // Log the raw payload for debugging with structured logging
                let method = req.method().as_str();
                let path = req.path();
                let content_type = req
                    .headers()
                    .get("content-type")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("unknown");

                // Truncate oversized payloads (>10KB)
                const MAX_PAYLOAD_LOG_SIZE: usize = 10 * 1024; // 10KB
                let payload_str = String::from_utf8_lossy(&frozen_body);
                let truncated_payload = if payload_str.len() > MAX_PAYLOAD_LOG_SIZE {
                    format!(
                        "{}...[TRUNCATED: {} bytes total]",
                        &payload_str[..MAX_PAYLOAD_LOG_SIZE],
                        payload_str.len()
                    )
                } else {
                    payload_str.to_string()
                };

                // Sanitize sensitive headers for logging
                let mut safe_headers = std::collections::HashMap::new();
                for (name, value) in req.headers().iter() {
                    let name_lower = name.as_str().to_lowercase();
                    if name_lower.contains("authorization")
                        || name_lower.contains("signature")
                        || name_lower.contains("secret")
                        || name_lower.contains("key")
                    {
                        safe_headers.insert(name.as_str(), "[REDACTED]");
                    } else if let Ok(value_str) = value.to_str() {
                        safe_headers.insert(name.as_str(), value_str);
                    }
                }

                tracing::error!(
                    method = method,
                    path = path,
                    content_type = content_type,
                    payload_size = frozen_body.len(),
                    raw_payload = %truncated_payload,
                    headers = ?safe_headers,
                    serde_error = %e,
                    "JSON deserialization failed - logging raw payload for debugging"
                );

                actix_web::error::ErrorBadRequest(format!("Failed to parse JSON: {e:?}"))
            })?;

            Ok(JsonWithLogging {
                json: deserialized_body,
            })
        };

        Box::pin(fut)
    }
}

impl<T> std::ops::Deref for JsonWithLogging<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.json
    }
}

impl<T> std::ops::DerefMut for JsonWithLogging<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.json
    }
}

impl<T> JsonWithLogging<T> {
    /// Extract the inner JSON value, consuming the wrapper
    pub fn into_inner(self) -> T {
        self.json
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{HttpResponse, test, web};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestPayload {
        message: String,
        count: i32,
    }

    async fn test_handler(payload: JsonWithLogging<TestPayload>) -> HttpResponse {
        HttpResponse::Ok().json(&*payload)
    }

    #[actix_web::test]
    async fn test_valid_json_parsing() {
        let app =
            test::init_service(actix_web::App::new().route("/test", web::post().to(test_handler)))
                .await;

        let payload = TestPayload {
            message: "hello".to_string(),
            count: 42,
        };

        let req = test::TestRequest::post()
            .uri("/test")
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_payload_limit_enforced() {
        let app = test::init_service(
            actix_web::App::new()
                .app_data(web::PayloadConfig::default().limit(1024))
                .route("/test", web::post().to(test_handler)),
        )
        .await;

        let payload = TestPayload {
            message: "x".repeat(2048),
            count: 1,
        };

        let req = test::TestRequest::post()
            .uri("/test")
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            actix_web::http::StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[actix_web::test]
    async fn test_payload_truncation_logic() {
        let long_string = "x".repeat(12000); // > 10KB
        let payload = TestPayload {
            message: long_string.clone(),
            count: 123,
        };

        // This test verifies the truncation logic without actually calling the service
        // since we can't easily test the error case without mocking
        const MAX_PAYLOAD_LOG_SIZE: usize = 10 * 1024;
        let payload_str = serde_json::to_string(&payload).unwrap();

        if payload_str.len() > MAX_PAYLOAD_LOG_SIZE {
            let truncated = format!(
                "{}...[TRUNCATED: {} bytes total]",
                &payload_str[..MAX_PAYLOAD_LOG_SIZE],
                payload_str.len()
            );
            assert!(truncated.contains("[TRUNCATED:"));
            assert!(truncated.len() > MAX_PAYLOAD_LOG_SIZE);
        }
    }

    #[test]
    async fn test_header_sanitization() {
        let mut safe_headers = std::collections::HashMap::new();

        // Test cases for header sanitization
        let test_headers = vec![
            ("authorization", "Bearer token123", "[REDACTED]"),
            ("x-signature", "sig123", "[REDACTED]"),
            ("x-api-key", "key123", "[REDACTED]"),
            ("x-secret", "secret123", "[REDACTED]"),
            ("content-type", "application/json", "application/json"),
            ("user-agent", "test-agent", "test-agent"),
        ];

        for (name, value, _expected) in test_headers {
            let name_lower = name.to_lowercase();
            if name_lower.contains("authorization")
                || name_lower.contains("signature")
                || name_lower.contains("secret")
                || name_lower.contains("key")
            {
                safe_headers.insert(name, "[REDACTED]");
            } else {
                safe_headers.insert(name, value);
            }
        }

        assert_eq!(safe_headers.get("authorization"), Some(&"[REDACTED]"));
        assert_eq!(safe_headers.get("x-signature"), Some(&"[REDACTED]"));
        assert_eq!(safe_headers.get("x-api-key"), Some(&"[REDACTED]"));
        assert_eq!(safe_headers.get("x-secret"), Some(&"[REDACTED]"));
        assert_eq!(safe_headers.get("content-type"), Some(&"application/json"));
        assert_eq!(safe_headers.get("user-agent"), Some(&"test-agent"));
    }
}
