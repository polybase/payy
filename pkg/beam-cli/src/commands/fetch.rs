// lint-long-file-override allow-max-lines=600
mod logging;
pub(crate) mod payment;
pub(crate) mod protocol;
mod retry;

use std::{net::IpAddr, path::PathBuf, time::Duration};

use contextful::ResultContextExt;
use futures::stream;
use reqwest::{
    Body, Client, Method, RequestBuilder, Response, StatusCode, Url,
    header::{AUTHORIZATION, CONTENT_LENGTH, HeaderMap, HeaderName, HeaderValue},
    redirect::Policy,
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
};

use crate::{
    cli::FetchArgs,
    error::{Error, Result},
    output::OutputMode,
    runtime::BeamApp,
};

use self::{
    super::interactive_interrupt::delegate_current_interrupt_to_command,
    logging::{print_request, print_response},
    payment::{approve_payment, execute_payment, prepare_mpp_payment, prepare_x402_payment},
    protocol::{PaymentChallenge, RetryHeader, parse_payment_challenge},
    retry::{RedirectTracker, origin_locked_redirect_policy, retry_spec_for_challenge},
};

const USER_AGENT: &str = concat!("beam/", env!("CARGO_PKG_VERSION"));

#[cfg(test)]
pub(crate) use self::logging::printable_request_header_value_for_test;

#[derive(Clone, Debug)]
struct RequestSpec {
    body: Option<RequestBodySpec>,
    headers: HeaderMap,
    method: Method,
    url: Url,
}

struct FetchClient {
    client: Client,
    redirect_tracker: RedirectTracker,
}

struct SentRequest {
    effective_spec: RequestSpec,
    response: Response,
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) struct RequestSpecForTest {
    pub(crate) body: Option<Vec<u8>>,
    pub(crate) headers: HeaderMap,
    pub(crate) method: Method,
    pub(crate) url: Url,
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) struct SentRequestForTest {
    pub(crate) effective_spec: RequestSpecForTest,
    pub(crate) status: StatusCode,
}

#[derive(Clone, Debug)]
enum RequestBodySpec {
    Inline(Vec<u8>),
    File { length: u64, path: PathBuf },
}

pub async fn run(app: &BeamApp, args: FetchArgs) -> Result<()> {
    let spec = request_spec_from_args(&args).await?;
    let client = build_initial_request_client(&args, &spec.url)?;
    let sent = send_request(&client, &spec, None, args.verbose).await?;

    if sent.response.status() != StatusCode::PAYMENT_REQUIRED {
        return write_response(sent.response, args.output_path.as_deref(), app.output_mode).await;
    }

    handle_payment_required(app, &args, &client, &sent.effective_spec, sent.response).await
}

async fn handle_payment_required(
    app: &BeamApp,
    args: &FetchArgs,
    client: &FetchClient,
    spec: &RequestSpec,
    response: Response,
) -> Result<()> {
    let challenged_url = response.url().clone();
    ensure_payment_challenge_transport(args, &challenged_url)?;
    let headers = response.headers().clone();
    let body = response
        .bytes()
        .await
        .map_err(|_| Error::FetchRequestFailed)?;
    let challenge =
        parse_payment_challenge(&headers, &body)?.ok_or(Error::FetchInvalidPaymentResponse)?;

    if args.no_pay {
        eprintln!("{}", challenge.describe());
        return Err(Error::FetchPaymentRequired);
    }

    let retry_spec = retry_spec_for_challenge(spec, challenged_url);
    if matches!(challenge, PaymentChallenge::Mpp(_)) {
        ensure_retry_header_can_merge(&retry_spec.headers, &AUTHORIZATION)?;
    }

    let payment = match &challenge {
        PaymentChallenge::X402(x402) => prepare_x402_payment(app, args, x402).await?,
        PaymentChallenge::Mpp(mpp) => prepare_mpp_payment(app, mpp).await?,
    };

    eprintln!(
        "{}",
        payment.confirmation_message(challenge.protocol_name())
    );
    let chain_store = app.chain_store.get().await;
    approve_payment(args, &payment, &chain_store)?;
    // Interactive fetch starts under the REPL ctrl-c wrapper, but the payment transaction owns
    // ctrl-c while it is being submitted so Beam can surface the tx hash/status. Once execution
    // returns, the paid retry/download path falls back to REPL interrupt handling.
    let executed = {
        let _interrupt_guard = delegate_current_interrupt_to_command();
        execute_payment(app, &payment).await?
    };
    let retry_header = challenge.retry_header(&executed)?;
    let retry_response =
        send_retry_request(args, client, &retry_spec, &retry_header, args.verbose).await?;
    if retry_response.status() == StatusCode::PAYMENT_REQUIRED {
        let retry_headers = retry_response.headers().clone();
        let retry_body = retry_response
            .bytes()
            .await
            .map_err(|_| Error::FetchRequestFailed)?;

        if let Some(retry_challenge) = parse_payment_challenge(&retry_headers, &retry_body)? {
            eprintln!("{}", retry_challenge.describe());
        }

        return Err(Error::FetchPaymentRequired);
    }

    write_response(retry_response, args.output_path.as_deref(), app.output_mode).await
}

fn ensure_payment_challenge_transport(args: &FetchArgs, challenged_url: &Url) -> Result<()> {
    if challenged_url.scheme() == "https" {
        return Ok(());
    }

    if args.dev && challenged_url.scheme() == "http" && is_loopback_http_host(challenged_url) {
        return Ok(());
    }

    Err(Error::FetchPaymentRequiresHttps {
        url: challenged_url.to_string(),
    })
}

fn is_loopback_http_host(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };

    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return true;
    }

    host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

#[cfg(test)]
pub(crate) fn ensure_payment_challenge_transport_for_test(
    args: &FetchArgs,
    challenged_url: &Url,
) -> Result<()> {
    ensure_payment_challenge_transport(args, challenged_url)
}

async fn request_spec_from_args(args: &FetchArgs) -> Result<RequestSpec> {
    Ok(RequestSpec {
        body: request_body(args).await?,
        headers: request_headers(&args.headers)?,
        method: request_method(args)?,
        url: Url::parse(&args.url).map_err(|_| Error::FetchRequestFailed)?,
    })
}

async fn request_body(args: &FetchArgs) -> Result<Option<RequestBodySpec>> {
    if let Some(data) = args.data.as_ref() {
        return Ok(Some(RequestBodySpec::Inline(data.as_bytes().to_vec())));
    }

    match args.data_file.as_ref() {
        Some(path) => {
            let path = PathBuf::from(path);
            let length = tokio::fs::metadata(&path)
                .await
                .context("stat beam fetch data file")?
                .len();

            Ok(Some(RequestBodySpec::File { length, path }))
        }
        None => Ok(None),
    }
}

fn request_headers(values: &[String]) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    for value in values {
        let (name, value) = value
            .split_once(':')
            .ok_or_else(|| Error::FetchInvalidHeader {
                value: value.clone(),
            })?;
        let name = HeaderName::from_bytes(name.trim().as_bytes()).map_err(|_| {
            Error::FetchInvalidHeader {
                value: value.to_string(),
            }
        })?;
        let value = HeaderValue::from_str(value.trim()).map_err(|_| Error::FetchInvalidHeader {
            value: value.to_string(),
        })?;
        headers.append(name, value);
    }

    Ok(headers)
}

fn parse_method(value: &str) -> Result<Method> {
    Method::from_bytes(value.trim().as_bytes()).map_err(|_| Error::FetchInvalidMethod {
        value: value.to_string(),
    })
}

fn request_method(args: &FetchArgs) -> Result<Method> {
    match args.method.as_deref() {
        Some(method) => parse_method(method),
        None if args.data.is_some() || args.data_file.is_some() => Ok(Method::POST),
        None => Ok(Method::GET),
    }
}

fn build_initial_request_client(args: &FetchArgs, request_url: &Url) -> Result<FetchClient> {
    let redirect_tracker = RedirectTracker::default();
    let policy = redirect_policy(
        args.follow_redirects,
        args.max_redirects,
        request_url,
        redirect_tracker.clone(),
    );

    build_client_with_policy(args, policy, redirect_tracker)
}

#[cfg(test)]
pub(crate) fn build_initial_request_client_for_test(
    args: &FetchArgs,
    request_url: &Url,
) -> Result<Client> {
    Ok(build_initial_request_client(args, request_url)?
        .client
        .clone())
}

fn build_client_with_policy(
    args: &FetchArgs,
    policy: Policy,
    redirect_tracker: RedirectTracker,
) -> Result<FetchClient> {
    let mut builder = Client::builder().user_agent(USER_AGENT);

    builder = builder.redirect(policy);

    if let Some(seconds) = args.connect_timeout {
        builder = builder.connect_timeout(Duration::from_secs(seconds));
    }

    if let Some(seconds) = args.timeout {
        builder = builder.timeout(Duration::from_secs(seconds));
    }

    let client = builder.build().map_err(|_| Error::FetchRequestFailed)?;

    Ok(FetchClient {
        client,
        redirect_tracker,
    })
}

fn redirect_policy(
    follow_redirects: bool,
    max_redirects: usize,
    request_url: &Url,
    redirect_tracker: RedirectTracker,
) -> Policy {
    if follow_redirects {
        origin_locked_redirect_policy(max_redirects, request_url, redirect_tracker)
    } else {
        Policy::none()
    }
}

fn build_payment_retry_client(args: &FetchArgs, original_url: &Url) -> Result<FetchClient> {
    let redirect_tracker = RedirectTracker::default();

    build_client_with_policy(
        args,
        origin_locked_redirect_policy(args.max_redirects, original_url, redirect_tracker.clone()),
        redirect_tracker,
    )
}

#[cfg(test)]
pub(crate) fn build_payment_retry_client_for_test(
    args: &FetchArgs,
    original_url: &Url,
) -> Result<Client> {
    Ok(build_payment_retry_client(args, original_url)?
        .client
        .clone())
}

async fn send_retry_request(
    args: &FetchArgs,
    client: &FetchClient,
    spec: &RequestSpec,
    retry_header: &RetryHeader,
    verbose: bool,
) -> Result<Response> {
    if !args.follow_redirects {
        return Ok(send_request(client, spec, Some(retry_header), verbose)
            .await?
            .response);
    }

    let retry_client = build_payment_retry_client(args, &spec.url)?;
    Ok(
        send_request(&retry_client, spec, Some(retry_header), verbose)
            .await?
            .response,
    )
}

#[cfg(test)]
pub(crate) async fn send_retry_request_for_test(
    args: &FetchArgs,
    request_url: &Url,
    challenged_url: &Url,
    retry_header: RetryHeader,
) -> Result<Response> {
    send_retry_request_with_spec_for_test(
        args,
        request_url,
        challenged_url,
        Method::GET,
        HeaderMap::new(),
        None,
        retry_header,
    )
    .await
}

#[cfg(test)]
pub(crate) async fn send_retry_request_with_spec_for_test(
    args: &FetchArgs,
    request_url: &Url,
    challenged_url: &Url,
    method: Method,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    retry_header: RetryHeader,
) -> Result<Response> {
    let client = build_initial_request_client(args, request_url)?;
    let spec = RequestSpec {
        body: body.map(RequestBodySpec::Inline),
        headers,
        method,
        url: request_url.clone(),
    };
    let retry_spec = retry_spec_for_challenge(&spec, challenged_url.clone());

    send_retry_request(args, &client, &retry_spec, &retry_header, false).await
}

#[cfg(test)]
pub(crate) async fn send_request_for_test(
    args: &FetchArgs,
    request_url: &Url,
    method: Method,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
) -> Result<SentRequestForTest> {
    let client = build_initial_request_client(args, request_url)?;
    let spec = RequestSpec {
        body: body.map(RequestBodySpec::Inline),
        headers,
        method,
        url: request_url.clone(),
    };
    let sent = send_request(&client, &spec, None, false).await?;

    Ok(SentRequestForTest {
        effective_spec: RequestSpecForTest::from_request_spec(&sent.effective_spec),
        status: sent.response.status(),
    })
}

async fn send_request(
    client: &FetchClient,
    spec: &RequestSpec,
    retry_header: Option<&RetryHeader>,
    verbose: bool,
) -> Result<SentRequest> {
    let headers = merged_headers(&spec.headers, retry_header, spec.body.as_ref())?;

    if verbose {
        print_request(spec, &headers);
    }

    client.redirect_tracker.clear();

    let request = client
        .client
        .request(spec.method.clone(), spec.url.clone())
        .headers(headers);

    let request = match spec.body.as_ref() {
        Some(body) => body.apply(request).await?,
        None => request,
    };

    let response = request
        .send()
        .await
        .map_err(|_| Error::FetchRequestFailed)?;

    if verbose {
        print_response(&response);
    }

    Ok(SentRequest {
        effective_spec: client.redirect_tracker.effective_request_spec(spec),
        response,
    })
}

fn merged_headers(
    base: &HeaderMap,
    retry_header: Option<&RetryHeader>,
    body: Option<&RequestBodySpec>,
) -> Result<HeaderMap> {
    let mut headers = base.clone();

    if let Some(retry_header) = retry_header {
        ensure_retry_header_can_merge(&headers, &retry_header.name)?;
        headers.insert(retry_header.name.clone(), retry_header.value.clone());
        if retry_header.name == AUTHORIZATION {
            headers.remove("payment-signature");
            headers.remove("x-payment");
        }
    }

    if let Some(body) = body {
        body.apply_headers(&mut headers)?;
    }

    Ok(headers)
}

fn ensure_retry_header_can_merge(base: &HeaderMap, retry_header_name: &HeaderName) -> Result<()> {
    if retry_header_name == AUTHORIZATION && base.contains_key(AUTHORIZATION) {
        return Err(Error::FetchPaymentAuthorizationConflict);
    }

    Ok(())
}

async fn write_response(
    response: Response,
    output_path: Option<&str>,
    output_mode: OutputMode,
) -> Result<()> {
    if let Some(output_path) = output_path {
        let mut output = File::create(output_path)
            .await
            .context("create beam fetch output file")?;
        write_response_body(
            response,
            &mut output,
            "write beam fetch output file",
            "flush beam fetch output file",
        )
        .await?;
        return Ok(());
    }

    if output_mode == OutputMode::Quiet {
        return Ok(());
    }

    let mut stdout = tokio::io::stdout();
    write_response_body(
        response,
        &mut stdout,
        "write beam fetch stdout",
        "flush beam fetch stdout",
    )
    .await
}

impl RequestBodySpec {
    fn apply_headers(&self, headers: &mut HeaderMap) -> Result<()> {
        let Self::File { length, .. } = self else {
            return Ok(());
        };

        if headers.contains_key(CONTENT_LENGTH) {
            return Ok(());
        }

        let value =
            HeaderValue::from_str(&length.to_string()).map_err(|_| Error::FetchRequestFailed)?;
        headers.insert(CONTENT_LENGTH, value);
        Ok(())
    }

    async fn apply(&self, request: RequestBuilder) -> Result<RequestBuilder> {
        match self {
            Self::Inline(body) => Ok(request.body(body.clone())),
            Self::File { path, .. } => {
                let file = File::open(path)
                    .await
                    .context("open beam fetch data file")?;
                Ok(request.body(Body::wrap_stream(request_body_stream(file))))
            }
        }
    }
}

#[cfg(test)]
impl RequestSpecForTest {
    fn from_request_spec(spec: &RequestSpec) -> Self {
        Self {
            body: match spec.body.as_ref() {
                Some(RequestBodySpec::Inline(body)) => Some(body.clone()),
                Some(RequestBodySpec::File { .. }) | None => None,
            },
            headers: spec.headers.clone(),
            method: spec.method.clone(),
            url: spec.url.clone(),
        }
    }
}

fn request_body_stream(file: File) -> impl futures::Stream<Item = std::io::Result<Vec<u8>>> {
    const CHUNK_SIZE: usize = 16 * 1024;

    stream::try_unfold(file, |mut file| async move {
        let mut chunk = vec![0; CHUNK_SIZE];
        let read = file.read(&mut chunk).await?;
        if read == 0 {
            return Ok(None);
        }

        chunk.truncate(read);
        Ok(Some((chunk, file)))
    })
}

async fn write_response_body(
    mut response: Response,
    writer: &mut (dyn AsyncWrite + Unpin),
    write_context: &'static str,
    flush_context: &'static str,
) -> Result<()> {
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| Error::FetchRequestFailed)?
    {
        writer.write_all(&chunk).await.context(write_context)?;
    }

    writer.flush().await.context(flush_context)?;
    Ok(())
}
