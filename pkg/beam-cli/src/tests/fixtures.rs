use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{
    display::ColorMode,
    output::OutputMode,
    runtime::{BeamApp, BeamPaths, InvocationOverrides},
};

pub(super) async fn test_app(overrides: InvocationOverrides) -> (TempDir, BeamApp) {
    test_app_with_output(OutputMode::Default, overrides).await
}

pub(super) async fn test_app_with_output(
    output_mode: OutputMode,
    overrides: InvocationOverrides,
) -> (TempDir, BeamApp) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let app = BeamApp::for_root(
        BeamPaths::new(temp_dir.path().to_path_buf()),
        ColorMode::Auto,
        output_mode,
        overrides,
    )
    .await
    .expect("load beam app");

    (temp_dir, app)
}

pub(super) async fn spawn_chain_id_rpc_server(
    chain_id: u64,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind chain id rpc listener");
    let address = listener.local_addr().expect("listener address");

    let server = tokio::spawn(async move {
        loop {
            let (stream, _peer) = listener.accept().await.expect("accept rpc connection");
            serve_chain_id_connection(stream, chain_id).await;
        }
    });

    (format!("http://{address}"), server)
}

async fn serve_chain_id_connection(mut stream: TcpStream, chain_id: u64) {
    let request = read_rpc_request(&mut stream).await;
    assert_eq!(request["method"], Value::String("eth_chainId".to_string()));

    let body = chain_id_response(&request, chain_id);
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .await
        .expect("write rpc response");
}

pub(super) async fn read_rpc_request(stream: &mut TcpStream) -> Value {
    let mut buffer = Vec::new();
    let body_offset = loop {
        let mut chunk = [0u8; 1024];
        let read = stream.read(&mut chunk).await.expect("read rpc request");
        assert!(read > 0, "rpc request closed before headers");
        buffer.extend_from_slice(&chunk[..read]);

        if let Some(offset) = header_end(&buffer) {
            break offset;
        }
    };

    let headers = String::from_utf8_lossy(&buffer[..body_offset]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("parse content length"))
        })
        .expect("content-length header");

    let mut body = buffer[body_offset..].to_vec();
    while body.len() < content_length {
        let mut chunk = vec![0u8; content_length - body.len()];
        let read = stream.read(&mut chunk).await.expect("read rpc body");
        assert!(read > 0, "rpc request closed before body");
        body.extend_from_slice(&chunk[..read]);
    }

    serde_json::from_slice(&body[..content_length]).expect("parse rpc body")
}

fn header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn chain_id_response(request: &Value, chain_id: u64) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": format!("0x{chain_id:x}"),
    })
    .to_string()
}
