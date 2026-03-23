# Guild Client HTTP

HTTP client for interacting with guild services.

## Overview

This package provides a specialized HTTP client for communicating with the Guild application server.

## Features

- Guild service API client
- Authentication handling
- Wallet operations
- Note management
- Ramps integration

## Dependency Injection

The client now accepts any implementation of the shared
[`HttpClient`](../http-interface/src/client.rs) trait. This enables
swapping the underlying HTTP behaviour for tests or alternative runtimes.

```rust
use guild_client_http::GuildClientHttp;

#[derive(Clone)]
struct RecordingHttp { /* implements HttpClient */ }

let http = RecordingHttp::new();
let client = GuildClientHttp::with_http_client(http.clone());

// Exercise client methods with the injected HTTP implementation.
```

When working against the dynamic `HttpClient` trait, payloads are wrapped in
`client_http::HttpBody` so they can be cloned for retries. For example:

```rust
use client_http::HttpBody;

guild_client
    .http_client
    .post("/wallets/me/activity", Some(HttpBody::json(&payload)))
    .auth()
    .exec()
    .await?;
```

See `tests/http_client_swapping.rs` for a fully worked example that records
requests without performing network calls using a lightweight test double.
