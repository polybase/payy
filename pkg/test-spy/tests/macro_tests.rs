#![allow(dead_code)]

use test_spy::spy_mock;

// Test simple sync trait
#[spy_mock]
trait SimpleService {
    fn get_value(&self, id: u32) -> String;
    fn set_value(&self, id: u32, value: String) -> bool;
}

#[test]
fn test_simple_sync_trait() {
    let mock = SimpleServiceMock::new();

    // Test default return
    let result = mock.get_value(42);
    assert_eq!(result, String::default());

    // Verify call was recorded
    let calls = mock.get_value.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].params, 42);

    // Test return_next
    mock.get_value.return_next("custom".to_string());
    let result = mock.get_value(100);
    assert_eq!(result, "custom");

    // Verify both calls recorded
    let calls = mock.get_value.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].params, 100);
}

// Test async trait
#[spy_mock]
#[async_trait::async_trait]
trait AsyncService {
    async fn fetch_data(&self, url: &str) -> Result<String, String>;
    async fn process(&self) -> u64;
}

#[tokio::test]
async fn test_async_trait() {
    let mock = AsyncServiceMock::new();

    // Test default return
    let result = mock.fetch_data("https://example.com").await;
    assert_eq!(result, Ok(String::default()));

    // Verify call was recorded
    let calls = mock.fetch_data.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].params, "https://example.com".to_string());

    // Test process method
    let result = mock.process().await;
    assert_eq!(result, 0);
}

// Test trait with multiple parameters
#[spy_mock]
trait ComplexService {
    fn calculate(&self, a: i32, b: i32, c: f64) -> f64;
}

#[test]
fn test_multiple_params() {
    let mock = ComplexServiceMock::new();

    let result = mock.calculate(10, 20, 3.5);
    assert_eq!(result, f64::default());

    let calls = mock.calculate.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].params, (10, 20, 3.5));
}

// Test trait with no parameters
#[spy_mock]
trait NoParamsService {
    fn get_constant(&self) -> i32;
}

#[test]
fn test_no_params() {
    let mock = NoParamsServiceMock::new();

    let result = mock.get_constant();
    assert_eq!(result, i32::default());

    let calls = mock.get_constant.calls();
    assert_eq!(calls.len(), 1);
    // params is () for no-args functions
}

// Test mixed sync and async methods
#[spy_mock]
#[async_trait::async_trait]
trait MixedService {
    fn sync_method(&self, value: String) -> bool;
    async fn async_method(&self, value: String) -> bool;
}

#[tokio::test]
async fn test_mixed_methods() {
    let mock = MixedServiceMock::new();

    // Test sync method
    let result = mock.sync_method("test".to_string());
    assert_eq!(result, bool::default());

    // Test async method
    let result = mock.async_method("test".to_string()).await;
    assert_eq!(result, bool::default());

    // Verify both methods tracked their calls
    assert_eq!(mock.sync_method.calls().len(), 1);
    assert_eq!(mock.async_method.calls().len(), 1);
}
