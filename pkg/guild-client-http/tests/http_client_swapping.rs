use guild_client_http::GuildClientHttp;
use http_interface::{Error, RecordingHttpClient};
use reqwest::Method;

#[tokio::test]
async fn swapping_http_client_surfaces_errors() {
    let mock = RecordingHttpClient::new();
    let client = GuildClientHttp::with_http_client(mock.clone());

    // The mock already returns server errors by default,
    // so we can directly test the behavior
    let result = client.get_wallet().await;

    match result {
        Err(Error::ServerError(_, metadata)) => {
            assert_eq!(metadata.path, "/wallets/me");
            assert_eq!(metadata.method, Method::GET);
        }
        other => panic!("unexpected result: {other:?}"),
    }

    // Verify that the get method was called with the expected path
    let calls = mock.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, Method::GET);
    assert_eq!(calls[0].1, "/wallets/me");
}
