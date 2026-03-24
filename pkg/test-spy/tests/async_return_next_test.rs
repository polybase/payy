use test_spy::spy_mock;

#[derive(Debug, PartialEq)]
pub struct FetchError {
    message: String,
}

impl FetchError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

#[spy_mock]
#[async_trait::async_trait]
trait RemoteClient {
    async fn fetch(&self, url: String) -> Result<String, FetchError>;
    async fn publish(&self, body: String);
}

#[tokio::test]
async fn test_async_return_next() {
    let mock = RemoteClientMock::new();

    // Test return_next with a closure that returns a pinned future
    mock.fetch.return_next(|url| {
        let url = url.clone(); // Clone the parameter to avoid lifetime issues
        Box::pin(async move { Ok(format!("{{\"url\":\"{url}\"}}")) })
    });

    let response = mock.fetch("https://api".to_string()).await;
    assert!(response.unwrap().contains("https://api"));

    // Verify calls were recorded
    assert_eq!(mock.fetch.calls().len(), 1);
    assert_eq!(mock.publish.calls().len(), 0);

    // Test multiple return_next calls
    mock.fetch
        .return_next(|_url| Box::pin(async move { Err(FetchError::new("network error")) }));

    let error_response = mock.fetch("https://api2".to_string()).await;
    assert!(error_response.is_err());

    assert_eq!(mock.fetch.calls().len(), 2);
}

#[tokio::test]
async fn test_async_default_behavior() {
    let mock = RemoteClientMock::new();

    // Test default behavior (should return Ok(String::default()) for Result<String, FetchError>)
    let response = mock.fetch("https://default".to_string()).await;
    assert_eq!(response, Ok(String::default()));

    // Test publish default (should return ())
    mock.publish("test".to_string()).await;

    assert_eq!(mock.fetch.calls().len(), 1);
    assert_eq!(mock.publish.calls().len(), 1);
}
