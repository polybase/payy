use std::time::Duration;
use tokio::time::sleep;

pub fn retry<F, T, E, Fut>(operation: F) -> Retry<F, T, E, Fut>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    Retry {
        operation,
        retry_delay: Duration::from_millis(1000),
        error_handler: None,
    }
}

pub async fn retry_forever<T, E, F, Fut>(operation: F, retry_delay: Duration) -> T
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    retry(operation)
        .retry_delay(retry_delay)
        .exec_forever()
        .await
}

pub async fn retry_with_exponential_backoff<T, E, F, Fut, H>(
    mut operation: F,
    attempts: usize,
    initial_delay: Duration,
    max_delay: Duration,
    mut on_error: H,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    H: FnMut(usize, &E, Duration) + Send,
{
    let attempts = attempts.max(1);
    let mut delay = initial_delay;
    let max_delay = max_delay.max(initial_delay);
    let mut attempt = 1;

    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                if attempt >= attempts {
                    return Err(err);
                }

                on_error(attempt, &err, delay);
                sleep(delay).await;
                delay = delay.saturating_mul(2).min(max_delay);
                attempt += 1;
            }
        }
    }
}

pub struct Retry<F, T, E, Fut>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    operation: F,
    retry_delay: Duration,
    error_handler: Option<ErrorHandler<E>>,
}

// Type alias to reduce type complexity warning
type ErrorHandler<E> = Box<dyn Fn(&E) + Send>;

impl<F, T, E, Fut> Retry<F, T, E, Fut>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    pub fn on_error<H>(mut self, handler: H) -> Self
    where
        H: Fn(&E) + Send + 'static,
    {
        self.error_handler = Some(Box::new(handler));
        self
    }

    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    pub async fn exec_forever(self) -> T {
        let mut operation = self.operation;
        loop {
            match operation().await {
                Ok(value) => return value,
                Err(err) => {
                    if let Some(ref handler) = self.error_handler {
                        handler(&err);
                    }
                    sleep(self.retry_delay).await;
                }
            }
        }
    }

    pub async fn exec(self, attempts: usize) -> Result<T, E> {
        let mut operation = self.operation;
        let mut last_error: Option<E> = None;

        for _ in 0..attempts {
            match operation().await {
                Ok(value) => return Ok(value),
                Err(err) => {
                    if let Some(ref handler) = self.error_handler {
                        handler(&err);
                    }
                    last_error = Some(err);
                    sleep(self.retry_delay).await;
                }
            }
        }

        Err(last_error.unwrap())
    }
}
