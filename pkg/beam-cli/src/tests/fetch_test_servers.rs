// lint-long-file-override allow-max-lines=400
use std::sync::{Arc, Mutex};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub(crate) type RecordedHeaders = Arc<Mutex<Vec<Option<String>>>>;
pub(crate) type RecordedPaths = Arc<Mutex<Vec<String>>>;
pub(crate) type RecordedRequests = Arc<Mutex<Vec<String>>>;
pub(crate) type ServerHandle = tokio::task::JoinHandle<()>;

pub(crate) async fn spawn_redirect_server(location: String) -> (String, ServerHandle) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind redirect listener");
    let address = listener.local_addr().expect("redirect listener address");

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _peer) = listener.accept().await.expect("accept redirect request");
            let _request = read_http_request(&mut stream).await;
            let response = format!(
                "HTTP/1.1 302 Found\r\nlocation: {location}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write redirect response");
        }
    });

    (format!("http://{address}"), server)
}

pub(crate) async fn spawn_recording_redirect_server(
    location: String,
) -> (String, RecordedPaths, ServerHandle) {
    spawn_recording_redirect_server_with_status(location, 302, "Found").await
}

pub(crate) async fn spawn_recording_redirect_server_with_status(
    location: String,
    status_code: u16,
    reason: &'static str,
) -> (String, RecordedPaths, ServerHandle) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind redirect listener");
    let address = listener.local_addr().expect("redirect listener address");
    let observed_paths = Arc::new(Mutex::new(Vec::new()));
    let server_paths = Arc::clone(&observed_paths);

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _peer) = listener.accept().await.expect("accept redirect request");
            let request = read_http_request(&mut stream).await;
            server_paths
                .lock()
                .expect("record redirect path")
                .push(request_path(&request).to_string());
            let response = format!(
                "HTTP/1.1 {status_code} {reason}\r\nlocation: {location}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write redirect response");
        }
    });

    (format!("http://{address}"), observed_paths, server)
}

pub(crate) async fn spawn_header_recording_server(
    header_name: &'static str,
) -> (String, RecordedHeaders, ServerHandle) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind recording listener");
    let address = listener.local_addr().expect("recording listener address");
    let observed_headers = Arc::new(Mutex::new(Vec::new()));
    let server_headers = Arc::clone(&observed_headers);

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _peer) = listener.accept().await.expect("accept recorded request");
            let request = read_http_request(&mut stream).await;
            server_headers
                .lock()
                .expect("record observed header")
                .push(request_header(&request, header_name));
            write_ok_response(&mut stream).await;
        }
    });

    (format!("http://{address}"), observed_headers, server)
}

pub(crate) async fn spawn_request_recording_server() -> (String, RecordedRequests, ServerHandle) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind recording listener");
    let address = listener.local_addr().expect("recording listener address");
    let observed_requests = Arc::new(Mutex::new(Vec::new()));
    let server_requests = Arc::clone(&observed_requests);

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _peer) = listener.accept().await.expect("accept recorded request");
            let request = read_http_request(&mut stream).await;
            server_requests
                .lock()
                .expect("record observed request")
                .push(request);
            write_ok_response(&mut stream).await;
        }
    });

    (format!("http://{address}"), observed_requests, server)
}

pub(crate) async fn spawn_same_origin_redirect_server(
    header_name: &'static str,
) -> (String, RecordedHeaders, ServerHandle) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind same-origin listener");
    let address = listener.local_addr().expect("same-origin listener address");
    let observed_headers = Arc::new(Mutex::new(Vec::new()));
    let server_headers = Arc::clone(&observed_headers);

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _peer) = listener.accept().await.expect("accept same-origin request");
            let request = read_http_request(&mut stream).await;
            match request_path(&request) {
                "/paid" => {
                    let response = "HTTP/1.1 302 Found\r\nlocation: /settled\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
                    stream
                        .write_all(response.as_bytes())
                        .await
                        .expect("write same-origin redirect response");
                }
                "/settled" => {
                    server_headers
                        .lock()
                        .expect("record same-origin header")
                        .push(request_header(&request, header_name));
                    write_ok_response(&mut stream).await;
                }
                path => panic!("unexpected path {path}"),
            }
        }
    });

    (format!("http://{address}/paid"), observed_headers, server)
}

pub(crate) async fn spawn_same_origin_redirect_challenge_server_with_status(
    status_code: u16,
    reason: &'static str,
) -> (String, RecordedRequests, ServerHandle) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind same-origin challenge listener");
    let address = listener
        .local_addr()
        .expect("same-origin challenge listener address");
    let observed_requests = Arc::new(Mutex::new(Vec::new()));
    let server_requests = Arc::clone(&observed_requests);
    let challenge_count = Arc::new(Mutex::new(0usize));
    let server_challenge_count = Arc::clone(&challenge_count);

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _peer) = listener
                .accept()
                .await
                .expect("accept same-origin challenge request");
            let request = read_http_request(&mut stream).await;

            match request_path(&request) {
                "/start" => {
                    let response = format!(
                        "HTTP/1.1 {status_code} {reason}\r\nlocation: /paid\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                    );
                    stream
                        .write_all(response.as_bytes())
                        .await
                        .expect("write same-origin challenge redirect response");
                }
                "/paid" => {
                    server_requests
                        .lock()
                        .expect("record same-origin challenged request")
                        .push(request);

                    let should_challenge = {
                        let mut challenge_count = server_challenge_count
                            .lock()
                            .expect("lock same-origin challenge count");
                        let should_challenge = *challenge_count == 0;
                        *challenge_count += 1;
                        should_challenge
                    };

                    if should_challenge {
                        stream
                            .write_all(
                                b"HTTP/1.1 402 Payment Required\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                            )
                            .await
                            .expect("write same-origin challenge response");
                    } else {
                        write_ok_response(&mut stream).await;
                    }
                }
                path => panic!("unexpected path {path}"),
            }
        }
    });

    (format!("http://{address}/start"), observed_requests, server)
}

pub(crate) fn request_body(request: &str) -> &str {
    request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or("")
}

pub(crate) fn request_header(request: &str, name: &str) -> Option<String> {
    request.lines().find_map(|line| {
        let (header_name, value) = line.split_once(':')?;
        header_name
            .eq_ignore_ascii_case(name)
            .then(|| value.trim().to_string())
    })
}

pub(crate) fn request_method(request: &str) -> &str {
    request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .expect("request method")
}

pub(crate) fn request_path(request: &str) -> &str {
    request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("request path")
}

async fn read_http_request(stream: &mut TcpStream) -> String {
    let mut buffer = Vec::new();

    loop {
        let mut chunk = [0u8; 1024];
        let read = stream.read(&mut chunk).await.expect("read http request");
        assert!(read > 0, "http request closed before headers");
        buffer.extend_from_slice(&chunk[..read]);

        if let Some(header_end) = header_end(&buffer) {
            let content_length = content_length(&buffer[..header_end]);
            if buffer.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }

    String::from_utf8(buffer).expect("utf8 request")
}

fn header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn content_length(headers: &[u8]) -> usize {
    String::from_utf8_lossy(headers)
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("content-length"))
        })
        .unwrap_or(0)
}

async fn write_ok_response(stream: &mut TcpStream) {
    stream
        .write_all(
            b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok",
        )
        .await
        .expect("write ok response");
}
