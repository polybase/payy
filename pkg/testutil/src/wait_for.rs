use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Polls the given function until it returns true or the timeout is reached.
///
/// # Arguments
///
/// * `f` - The function to poll. Should return true when the condition is met.
/// * `timeout` - The maximum duration to wait for the condition to be met.
pub async fn wait_for<F>(f: F, timeout: Duration)
where
    F: FnMut() -> bool,
{
    wait_for_poll_interval(f, timeout, None).await
}

/// Polls the given future-producing function until it returns true or the timeout is reached.
///
/// # Arguments
///
/// * `f` - The function to poll. Should return a Future that resolves to true when the condition is met.
/// * `timeout` - The maximum duration to wait for the condition to be met.
/// * `poll_frequency` - Optional polling frequency. If None, will poll as fast as possible.
pub async fn wait_for_poll_interval<F>(
    mut f: F,
    timeout: Duration,
    poll_frequency: Option<Duration>,
) where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    let poll_delay = poll_frequency.unwrap_or(Duration::from_millis(0));

    loop {
        if f() {
            return;
        }

        if start.elapsed() >= timeout {
            panic!("Timed out after waiting for {timeout:?}");
        }

        sleep(poll_delay).await;
    }
}
