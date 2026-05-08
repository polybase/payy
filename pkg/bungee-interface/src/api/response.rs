use http::HeaderMap;

/// Response wrapper for calque calls.
#[derive(Debug, Clone)]
pub struct Response<T> {
    /// Raw response headers.
    pub headers: ResponseHeaders,
    /// Typed response body.
    pub body: T,
}

/// Response headers wrapper.
#[derive(Debug, Clone, Default)]
pub struct ResponseHeaders {
    /// Raw response headers.
    pub raw: HeaderMap,
}
