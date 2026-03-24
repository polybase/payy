use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::task::yield_now;
use tokio::time::advance;

use crate::retry::retry_with_exponential_backoff;

#[tokio::test(start_paused = true)]
async fn retry_with_exponential_backoff_retries_until_success() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = Arc::clone(&attempts);
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);

    let handle = tokio::spawn(async move {
        retry_with_exponential_backoff(
            || {
                let attempts = Arc::clone(&attempts_clone);
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                    if attempt < 2 { Err("fail") } else { Ok("ok") }
                }
            },
            5,
            Duration::from_secs(1),
            Duration::from_secs(8),
            move |attempt, _err, delay| {
                let mut guard = events_clone.lock().unwrap();
                guard.push((attempt, delay));
            },
        )
        .await
    });

    yield_now().await;
    advance(Duration::from_secs(1)).await;
    yield_now().await;
    advance(Duration::from_secs(2)).await;

    let result = handle.await.unwrap();
    assert_eq!(result.unwrap(), "ok");
    assert_eq!(attempts.load(Ordering::SeqCst), 3);

    let events = events.lock().unwrap();
    assert_eq!(
        events.as_slice(),
        &[(1, Duration::from_secs(1)), (2, Duration::from_secs(2))]
    );
}

#[tokio::test(start_paused = true)]
async fn retry_with_exponential_backoff_returns_last_error() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = Arc::clone(&attempts);
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);

    let handle = tokio::spawn(async move {
        retry_with_exponential_backoff(
            || {
                let attempts = Arc::clone(&attempts_clone);
                async move {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    Err::<(), &str>("fail")
                }
            },
            3,
            Duration::from_secs(1),
            Duration::from_secs(4),
            move |attempt, _err, delay| {
                let mut guard = events_clone.lock().unwrap();
                guard.push((attempt, delay));
            },
        )
        .await
    });

    yield_now().await;
    advance(Duration::from_secs(1)).await;
    yield_now().await;
    advance(Duration::from_secs(2)).await;

    let result = handle.await.unwrap();
    assert_eq!(result.unwrap_err(), "fail");
    assert_eq!(attempts.load(Ordering::SeqCst), 3);

    let events = events.lock().unwrap();
    assert_eq!(
        events.as_slice(),
        &[(1, Duration::from_secs(1)), (2, Duration::from_secs(2))]
    );
}
