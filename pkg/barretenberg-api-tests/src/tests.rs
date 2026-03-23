// lint-long-file-override allow-max-lines=600

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::post,
};
use barretenberg_api_client::{
    ApiTransport, ClientBackend, ProveRequest, ProveResponse, VerifyRequest, VerifyResponse,
};
use barretenberg_api_interface::{PERMIT_TIMEOUT_HEADER, ServerError};
use barretenberg_api_server::server::build_app;
use barretenberg_interface::{BbBackend, BbBackendMock, Error, Result};
use contextful::prelude::*;
use http_body_util::BodyExt;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use tokio::net::TcpListener;
use tower::ServiceExt;
use unimock::{MockFn, Unimock, matching};
use url::Url;

use barretenberg_api_client::error::{ClientError, TransportError};

const PROGRAM: &[u8] = b"test-program";
const BYTECODE: &[u8] = b"bytecode";
const KEY: &[u8] = b"key";
const WITNESS: &[u8] = b"witness";
const PROOF: &[u8] = b"proof";
const PUBLIC_INPUTS: &[u8] = b"public-inputs";

#[tokio::test(flavor = "multi_thread")]
async fn client_prove_roundtrip() {
    let backend: Arc<dyn BbBackend> = Arc::new(Unimock::new(
        BbBackendMock::prove
            .next_call(matching!((
                program,
                bytecode,
                key,
                witness,
                oracle
            ) if *program == PROGRAM
                && *bytecode == BYTECODE
                && *key == KEY
                && *witness == WITNESS
                && *oracle))
            .returns(Ok(vec![3u8, 5, 7])),
    ));
    let client = client_with_backend(Arc::clone(&backend));

    let proof = client
        .prove(PROGRAM, BYTECODE, KEY, WITNESS, true)
        .await
        .expect("proof returned");
    assert_eq!(proof, vec![3u8, 5, 7]);
}

#[tokio::test(flavor = "multi_thread")]
async fn client_verify_roundtrip() {
    let backend: Arc<dyn BbBackend> = Arc::new(Unimock::new(
        BbBackendMock::verify
            .next_call(matching!((
                proof,
                public_inputs,
                key,
                oracle
            ) if *proof == PROOF
                && *public_inputs == PUBLIC_INPUTS
                && *key == KEY
                && !*oracle))
            .returns(Ok(())),
    ));
    let client = client_with_backend(Arc::clone(&backend));

    client
        .verify(PROOF, PUBLIC_INPUTS, KEY, false)
        .await
        .expect("verification succeeded");
}

#[tokio::test(flavor = "multi_thread")]
async fn http_client_surfaces_backend_errors() {
    let test_cases = vec![
        (
            Error::VerificationFailed,
            Box::new(|err: &Error| matches!(err, Error::VerificationFailed))
                as Box<dyn Fn(&Error) -> bool>,
            "VerificationFailed",
        ),
        (
            Error::Backend("backend failure message".to_owned()),
            Box::new(|err: &Error| {
                if let Error::Backend(msg) = err {
                    msg == "backend failure message"
                } else {
                    false
                }
            }),
            "Backend",
        ),
        (
            Error::Backend("symbols: !@#$%^&*()_+-=[]{}|;':\",./<>?".to_owned()),
            Box::new(|err: &Error| {
                if let Error::Backend(msg) = err {
                    msg == "symbols: !@#$%^&*()_+-=[]{}|;':\",./<>?"
                } else {
                    false
                }
            }),
            "BackendWithSymbols",
        ),
        (
            Error::Backend("".to_owned()),
            Box::new(|err: &Error| {
                if let Error::Backend(msg) = err {
                    msg.is_empty()
                } else {
                    false
                }
            }),
            "BackendEmptyMessage",
        ),
        (
            Error::ImplementationSpecific(Box::new(std::io::Error::other("internal crash"))),
            Box::new(|err: &Error| matches!(err, Error::ImplementationSpecific(_))),
            "ImplementationSpecific",
        ),
    ];

    for (input_err, check, label) in test_cases {
        let backend: Arc<dyn BbBackend> = Arc::new(Unimock::new(
            BbBackendMock::verify
                .next_call(matching!((_, _, _, _)))
                .returns(Err(input_err)),
        ));
        let app = build_app(backend);

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("listener address");
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, app.into_make_service()).await;
        });

        let client = ClientBackend::with_retry_policy(
            Url::parse(&format!("http://{}/", addr)).expect("base url"),
            Duration::from_secs(1),
            Duration::from_secs(1),
            None,
            Duration::from_millis(1),
            Duration::ZERO,
            Duration::from_millis(500),
        )
        .expect("client backend");

        let err = client
            .verify(PROOF, PUBLIC_INPUTS, KEY, true)
            .await
            .expect_err(&format!("verification should fail for case {}", label));

        assert!(
            check(&err),
            "error surfacing failed for {}: expected matching variant, got {:?}",
            label,
            err
        );

        server.abort();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn http_client_surfaces_prove_errors() {
    let backend: Arc<dyn BbBackend> = Arc::new(Unimock::new(
        BbBackendMock::prove
            .next_call(matching!((_, _, _, _, _)))
            .returns(Err(Error::Backend("prove failed".to_owned()))),
    ));
    let app = build_app(backend);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app.into_make_service()).await;
    });

    let client = ClientBackend::with_retry_policy(
        Url::parse(&format!("http://{}/", addr)).expect("base url"),
        Duration::from_secs(1),
        Duration::from_secs(1),
        None,
        Duration::from_millis(1),
        Duration::ZERO,
        Duration::from_millis(500),
    )
    .expect("client backend");

    let err = client
        .prove(PROGRAM, BYTECODE, KEY, WITNESS, true)
        .await
        .expect_err("prove should fail");

    if let Error::Backend(msg) = err {
        assert_eq!(msg, "prove failed");
    } else {
        panic!("expected Error::Backend, got {:?}", err);
    }

    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn healthcheck_returns_ok() {
    let router = build_app(Arc::new(Unimock::new(())));

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .expect("failed to build request"),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let payload: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("valid health payload");
    assert_eq!(payload, json!({ "status": "ok" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn prove_request_times_out_when_permit_wait_expires() {
    let backend: Arc<dyn BbBackend> = Arc::new(SlowBackend::new(
        Duration::from_millis(500),
        Duration::from_millis(0),
    ));
    let app = build_app(backend);
    let blocking_client = ClientBackend::with_transport(AxumTransport::new(app.clone()));
    let timeout_client = ClientBackend::with_transport(
        AxumTransport::new(app).with_permit_timeout(Some(Duration::from_millis(25))),
    );

    let first = tokio::spawn({
        let client = blocking_client.clone();
        async move { client.prove(PROGRAM, BYTECODE, KEY, WITNESS, false).await }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let err = timeout_client
        .prove(PROGRAM, BYTECODE, KEY, WITNESS, false)
        .await
        .expect_err("request should time out while waiting for permit");

    assert!(matches!(err, Error::ImplementationSpecific(_)));

    first
        .await
        .expect("first request join")
        .expect("first request should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn verify_does_not_require_permit() {
    let backend: Arc<dyn BbBackend> = Arc::new(SlowBackend::new(
        Duration::from_millis(500),
        Duration::from_millis(0),
    ));
    let app = build_app(backend);
    let client = ClientBackend::with_transport(AxumTransport::new(app));

    let prove_task = tokio::spawn({
        let client = client.clone();
        async move { client.prove(PROGRAM, BYTECODE, KEY, WITNESS, false).await }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let start = std::time::Instant::now();
    client
        .verify(PROOF, PUBLIC_INPUTS, KEY, false)
        .await
        .expect("verify should succeed");

    assert!(
        start.elapsed() < Duration::from_millis(200),
        "verify should not wait for permit, took {:?}",
        start.elapsed()
    );

    prove_task
        .await
        .expect("prove task join")
        .expect("prove should succeed");
}

#[derive(Clone)]
struct RetryState {
    attempts: Arc<AtomicUsize>,
}

async fn retry_prove_handler(
    State(state): State<RetryState>,
    Json(_request): Json<ProveRequest>,
) -> impl IntoResponse {
    let attempt = state.attempts.fetch_add(1, Ordering::SeqCst);
    if attempt == 0 {
        let err = ServerError::ServiceUnavailable {
            message: "permit timeout".to_owned(),
        };
        (err.status_code(), Json(err)).into_response()
    } else {
        Json(ProveResponse {
            proof: vec![9u8, 8, 7].into(),
        })
        .into_response()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn http_client_retries_on_permit_timeout() {
    let state = RetryState {
        attempts: Arc::new(AtomicUsize::new(0)),
    };
    let router = Router::new()
        .route("/prove", post(retry_prove_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, router.into_make_service()).await;
    });

    let client = ClientBackend::new(Url::parse(&format!("http://{}/", addr)).expect("base url"))
        .expect("client backend");

    let proof = client
        .prove(PROGRAM, BYTECODE, KEY, WITNESS, false)
        .await
        .expect("client succeed");

    assert_eq!(proof, vec![9u8, 8, 7]);
    assert!(state.attempts.load(Ordering::SeqCst) >= 2);

    server.abort();
}

fn client_with_backend(backend: Arc<dyn BbBackend>) -> ClientBackend {
    ClientBackend::with_transport(AxumTransport::new(build_app(backend)))
}

struct SlowBackend {
    prove_delay: Duration,
    verify_delay: Duration,
}

impl SlowBackend {
    fn new(prove_delay: Duration, verify_delay: Duration) -> Self {
        Self {
            prove_delay,
            verify_delay,
        }
    }
}

#[async_trait::async_trait]
impl BbBackend for SlowBackend {
    async fn prove(
        &self,
        _program: &[u8],
        _bytecode: &[u8],
        _key: &[u8],
        _witness: &[u8],
        _oracle: bool,
    ) -> Result<Vec<u8>> {
        tokio::time::sleep(self.prove_delay).await;
        Ok(vec![1, 2, 3])
    }

    async fn verify(
        &self,
        _proof: &[u8],
        _public_inputs: &[u8],
        _key: &[u8],
        _oracle: bool,
    ) -> Result<()> {
        tokio::time::sleep(self.verify_delay).await;
        Ok(())
    }
}

#[derive(Clone)]
struct AxumTransport {
    app: Router,
    permit_timeout: Option<Duration>,
}

impl AxumTransport {
    fn new(app: Router) -> Self {
        Self {
            app,
            permit_timeout: None,
        }
    }

    fn with_permit_timeout(mut self, permit_timeout: Option<Duration>) -> Self {
        self.permit_timeout = permit_timeout;
        self
    }

    async fn call<Req, Resp>(
        &self,
        path: &str,
        payload: Req,
    ) -> std::result::Result<Resp, ClientError>
    where
        Req: Serialize + Send + 'static,
        Resp: DeserializeOwned + Send + 'static,
    {
        let uri = path.to_owned();
        let body = serde_json::to_vec(&payload).map_err(|e| {
            ClientError::Transport(TransportError::Json(contextful::Contextful::new(
                "serialize body",
                e,
            )))
        })?;

        let mut builder = Request::builder()
            .method("POST")
            .uri(&uri)
            .header("content-type", "application/json")
            .header("Expect", "100-continue");

        if let Some(timeout) = self.permit_timeout {
            builder = builder.header(PERMIT_TIMEOUT_HEADER, duration_to_ms(timeout).to_string());
        }

        let req = builder.body(Body::from(body)).map_err(|e| {
            ClientError::Transport(TransportError::Io(
                std::io::Error::other(e.to_string()).wrap_err("build request"),
            ))
        })?;

        let response = self
            .app
            .clone()
            .oneshot(req)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()).wrap_err("oneshot request"))
            .map_err(TransportError::Io)
            .map_err(ClientError::Transport)?;

        let status = response.status();
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| {
                ClientError::Transport(TransportError::Io(
                    std::io::Error::other(e.to_string()).wrap_err("collect body"),
                ))
            })?
            .to_bytes();

        if !status.is_success() {
            let body_text = String::from_utf8_lossy(&body_bytes).into_owned();
            return Err(map_http_error(status, body_text));
        }

        let result: Resp = serde_json::from_slice(&body_bytes)
            .context("deserialize body")
            .map_err(TransportError::Json)
            .map_err(ClientError::Transport)?;
        Ok(result)
    }
}

#[async_trait::async_trait]
impl ApiTransport for AxumTransport {
    async fn prove(
        &self,
        request: ProveRequest,
    ) -> std::result::Result<ProveResponse, ClientError> {
        self.call("/prove", request).await
    }

    async fn verify(
        &self,
        request: VerifyRequest,
    ) -> std::result::Result<VerifyResponse, ClientError> {
        self.call("/verify", request).await
    }
}

fn map_http_error(status: StatusCode, body: String) -> ClientError {
    let server_error: ServerError =
        serde_json::from_str(&body).unwrap_or_else(|_| ServerError::Internal {
            message: format!("unexpected response ({status}): {body}"),
        });
    ClientError::Server(server_error)
}

fn duration_to_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}
