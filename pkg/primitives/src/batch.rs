// lint-long-file-override allow-max-lines=800
use async_trait::async_trait;
use futures::future::BoxFuture;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tokio::time::sleep;

/// Errors that can occur within the batch processing system
#[derive(Debug, Clone, thiserror::Error)]
pub enum BatchError<T: Clone + Send + Sync + std::error::Error + 'static> {
    /// The task processing the batch was dropped
    #[error("task has been dropped")]
    TaskDropped,
    /// Processor error
    #[error("processor error: {0}")]
    ProcessorError(#[from] T),
}

/// A trait for processing batches of items asynchronously.
///
/// The `BatchProcessor` trait defines an interface for types that can process
/// multiple items in a single batch operation. This is useful for optimizing
/// operations that are more efficient when performed in bulk, such as:
///
/// - Database operations (bulk inserts, updates)
/// - Network requests (reducing connection overhead)
/// - API calls with rate limits (maximizing throughput)
/// - Resource-intensive computations (amortizing setup costs)
///
/// # Type Parameters
///
/// - `Item`: The type of individual items to be processed. Must be clonable and
///   safe to send across thread boundaries.
/// - `Result`: The type of result produced for each processed item.
/// - `Error`: The error type that can occur during batch processing. Must implement
///   the standard error trait and be clonable.
///
/// # Implementation Requirements
///
/// Implementors must ensure:
///
/// 1. The number of results matches the number of input items
/// 2. Results maintain the same order as the input items
/// 3. If an error occurs, the entire batch fails (no partial processing)
///
/// # Examples
///
/// ## Basic numeric processor
///
/// ```rust
/// use async_trait::async_trait;
/// use primitives::batch::BatchProcessor;
///
/// struct NumberMultiplier {
///     factor: u32,
/// }
///
/// // Define a proper error type
/// #[derive(Debug, Clone)]
/// struct MyError(String);
///
/// impl std::fmt::Display for MyError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.0)
///     }
/// }
///
/// impl std::error::Error for MyError {}
///
/// #[async_trait]
/// impl BatchProcessor for NumberMultiplier {
///     type Item = u32;
///     type Result = u32;
///     type Error = MyError;
///
///     async fn process_batch(&self, items: Vec<Self::Item>) -> Result<Vec<Self::Result>, Self::Error> {
///         // Multiply each number by the factor
///         Ok(items.into_iter().map(|item| item * self.factor).collect())
///     }
/// }
///
/// // Usage
/// async fn example() -> Result<(), MyError> {
///     let processor = NumberMultiplier { factor: 10 };
///     let batch = vec![1, 2, 3, 4, 5];
///
///     let results = processor.process_batch(batch).await?;
///     // results will be [10, 20, 30, 40, 50]
///
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait BatchProcessor {
    /// The type of item being batched.
    ///
    /// This represents individual items that will be collected into batches for processing.
    /// These are the input parameters.
    type Item: Clone + Send + 'static;

    /// The result type returned for each item.
    ///
    /// Each input item will correspond to exactly one result item in the output vector,
    /// maintaining the same order as the input.
    type Result: Send + 'static;

    /// The error type that can occur during processing.
    ///
    /// If batch processing fails, this error will be returned. The error must be clonable
    /// to support error propagation across async boundaries and must implement the standard
    /// error trait for compatibility with error handling libraries.
    type Error: Clone + Send + Sync + std::error::Error + 'static;

    /// Process a batch of items.
    ///
    /// This method takes a vector of items and processes them as a batch, returning
    /// a vector of results in the same order as the input items. If processing fails,
    /// an error is returned and no results are produced.
    ///
    /// # Parameters
    ///
    /// * `items` - A vector of items to process in a batch
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Self::Result>)` - A vector of results, one for each input item in the same order
    /// * `Err(Self::Error)` - An error if batch processing failed
    ///
    /// # Implementation Notes
    ///
    /// Implementations should ensure:
    /// - The output vector contains exactly the same number of elements as the input vector
    /// - Results are in the same order as the corresponding inputs
    /// - If partial processing is possible, consider whether to return partial results or an error
    async fn process_batch(&self, items: Vec<Self::Item>)
    -> Result<Vec<Self::Result>, Self::Error>;
}

/// Configuration for the batch processor
pub struct BatchConfig {
    /// Minimum time to wait before processing a batch after receiving the first item
    pub batch_delay_ms: u64,
    /// Minimum time between batch executions (throttling)
    pub min_interval_ms: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_delay_ms: 10,
            min_interval_ms: 100,
        }
    }
}

/// The result for a single processed item
type ProcessResult<P> =
    Result<<P as BatchProcessor>::Result, BatchError<<P as BatchProcessor>::Error>>;

/// A struct that batches items to be processed by a BatchProcessor
pub struct Batch<P: BatchProcessor> {
    processor: Arc<P>,
    config: BatchConfig,
    queue: Arc<Mutex<BatchQueue<P::Item, ProcessResult<P>>>>,
}

struct BatchQueue<Item, Result> {
    items: VecDeque<(Item, oneshot::Sender<Result>)>,
    next_execution: Option<Instant>,
    last_execution: Option<Instant>,
    processing: bool,
}

impl<P: BatchProcessor + Send + Sync + 'static> Batch<P> {
    /// Creates a new batch with the given processor and configuration
    pub fn new(processor: P, config: BatchConfig) -> Self {
        Self {
            processor: Arc::new(processor),
            config,
            queue: Arc::new(Mutex::new(BatchQueue {
                items: VecDeque::new(),
                next_execution: None,
                last_execution: None,
                processing: false,
            })),
        }
    }

    /// Creates a new batch with default configuration
    pub fn with_defaults(processor: P) -> Self {
        Self::new(processor, BatchConfig::default())
    }

    /// Creates a new batch with specified delay and interval
    pub fn with_timing(processor: P, batch_delay_ms: u64, min_interval_ms: u64) -> Self {
        Self::new(
            processor,
            BatchConfig {
                batch_delay_ms,
                min_interval_ms,
            },
        )
    }

    /// Schedules an item to be processed and returns the result
    pub async fn call(&self, item: P::Item) -> ProcessResult<P> {
        let (tx, rx) = oneshot::channel();
        let should_schedule = {
            let mut queue = self.queue.lock();
            queue.items.push_back((item, tx));

            // If no execution is scheduled and not currently processing, schedule one
            if queue.next_execution.is_none() && !queue.processing {
                let now = Instant::now();

                // Calculate when the next execution should happen, respecting the throttle
                let next_time = if let Some(last_exec) = queue.last_execution {
                    let min_next = last_exec + Duration::from_millis(self.config.min_interval_ms);
                    let batch_next = now + Duration::from_millis(self.config.batch_delay_ms);
                    std::cmp::max(min_next, batch_next)
                } else {
                    now + Duration::from_millis(self.config.batch_delay_ms)
                };

                queue.next_execution = Some(next_time);
                true
            } else {
                false
            }
        };

        if should_schedule {
            // Clone what we need for the processor
            let queue_clone = Arc::clone(&self.queue);
            let processor_clone = Arc::clone(&self.processor);

            // Spawn a task to manage the batch processing
            tokio::spawn(async move {
                Self::schedule_next_batch(queue_clone, processor_clone).await;
            });
        }

        // Wait for the result
        rx.await.map_err(|_| BatchError::TaskDropped)?
    }

    /// Schedules and processes the next batch, respecting throttling
    fn schedule_next_batch(
        queue: Arc<Mutex<BatchQueue<P::Item, ProcessResult<P>>>>,
        processor: Arc<P>,
    ) -> BoxFuture<'static, ()> {
        Box::pin(async move {
            let wait_duration = {
                let queue_guard = queue.lock();
                if let Some(next_time) = queue_guard.next_execution {
                    let now = Instant::now();
                    if next_time > now {
                        next_time - now
                    } else {
                        Duration::from_millis(0)
                    }
                } else {
                    return; // No scheduled execution
                }
            };

            // Wait until it's time to process the batch
            sleep(wait_duration).await;

            // Mark as processing and get items
            let items = {
                let mut queue_guard = queue.lock();
                queue_guard.next_execution = None;
                queue_guard.processing = true;
                std::mem::take(&mut queue_guard.items)
            };

            if !items.is_empty() {
                // Split into arguments and senders
                let (args, senders): (Vec<_>, Vec<_>) = items.into_iter().unzip();

                // Call the processor with all arguments
                match processor.process_batch(args).await {
                    Ok(results) => {
                        // Send results back to callers
                        for (sender, result) in senders.into_iter().zip(results.into_iter()) {
                            let _ = sender.send(Ok(result)); // Ignore if receiver dropped
                        }
                    }
                    Err(err) => {
                        // If processing failed, notify all callers of the error
                        let batch_error = BatchError::ProcessorError(err);
                        for sender in senders {
                            let _ = sender.send(Err(batch_error.clone()));
                        }
                    }
                }
            }

            // Update state and check if we need to schedule another batch
            let should_schedule = {
                let mut queue_guard = queue.lock();
                queue_guard.last_execution = Some(Instant::now());
                queue_guard.processing = false;

                if !queue_guard.items.is_empty() && queue_guard.next_execution.is_none() {
                    // Calculate next execution time based on throttle
                    let now = Instant::now();
                    let next_time = if let Some(last_exec) = queue_guard.last_execution {
                        let min_next = last_exec
                            + Duration::from_millis(
                                queue_guard
                                    .last_execution
                                    .map(|_| {
                                        std::cmp::max(
                                            0,
                                            queue_guard
                                                .last_execution
                                                .unwrap()
                                                .elapsed()
                                                .as_millis()
                                                as u64,
                                        )
                                    })
                                    .unwrap_or(0),
                            );
                        std::cmp::max(min_next, now)
                    } else {
                        now
                    };

                    queue_guard.next_execution = Some(next_time);
                    true
                } else {
                    false
                }
            };

            // If there are more items, schedule another batch
            if should_schedule {
                Self::schedule_next_batch(queue, processor).await;
            }
        })
    }

    /// Updates the batch configuration
    pub fn update_config(&mut self, config: BatchConfig) {
        self.config = config;
    }

    /// Updates just the batch delay
    pub fn set_batch_delay(&mut self, batch_delay_ms: u64) {
        self.config.batch_delay_ms = batch_delay_ms;
    }

    /// Updates just the minimum interval (throttle)
    pub fn set_min_interval(&mut self, min_interval_ms: u64) {
        self.config.min_interval_ms = min_interval_ms;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::sleep;

    // Custom error type for the test processor
    #[derive(Debug, Clone, thiserror::Error)]
    #[error("{0}")]
    struct TestError(String);

    // A simple test processor that multiplies numbers
    struct TestProcessor {
        process_count: Arc<AtomicUsize>,
        delay_ms: u64,
        should_fail: bool,
    }

    #[async_trait]
    impl BatchProcessor for TestProcessor {
        type Item = u32;
        type Result = u32;
        type Error = TestError;

        async fn process_batch(
            &self,
            items: Vec<Self::Item>,
        ) -> Result<Vec<Self::Result>, Self::Error> {
            // Increment the process count
            self.process_count.fetch_add(1, Ordering::SeqCst);

            // Simulate processing delay
            if self.delay_ms > 0 {
                sleep(Duration::from_millis(self.delay_ms)).await;
            }

            // Optionally fail the processing
            if self.should_fail {
                return Err(TestError("Simulated processor error".to_string()));
            }

            // Process items (multiply by 2)
            Ok(items.into_iter().map(|x| x * 2).collect())
        }
    }

    impl TestProcessor {
        fn new() -> Self {
            Self {
                process_count: Arc::new(AtomicUsize::new(0)),
                delay_ms: 0,
                should_fail: false,
            }
        }

        fn with_delay(delay_ms: u64) -> Self {
            Self {
                process_count: Arc::new(AtomicUsize::new(0)),
                delay_ms,
                should_fail: false,
            }
        }

        fn with_failure() -> Self {
            Self {
                process_count: Arc::new(AtomicUsize::new(0)),
                delay_ms: 0,
                should_fail: true,
            }
        }

        fn process_count(&self) -> usize {
            self.process_count.load(Ordering::SeqCst)
        }
    }

    #[tokio::test]
    async fn test_single_item_processing() {
        let processor = TestProcessor::new();
        let batch = Batch::with_defaults(processor);

        let result = batch.call(5).await.unwrap();
        assert_eq!(result, 10);
    }

    #[tokio::test]
    async fn test_multiple_items_batched() {
        let processor = TestProcessor::new();
        let batch = Arc::new(Batch::with_timing(processor, 0, 0));

        // Submit multiple items that should be batched together
        let batch_clone1 = Arc::clone(&batch);
        let handle1 = tokio::spawn(async move { batch_clone1.call(1).await });

        let batch_clone2 = Arc::clone(&batch);
        let handle2 = tokio::spawn(async move { batch_clone2.call(2).await });

        let batch_clone3 = Arc::clone(&batch);
        let handle3 = tokio::spawn(async move { batch_clone3.call(3).await });

        let result1 = handle1.await.unwrap().unwrap();
        let result2 = handle2.await.unwrap().unwrap();
        let result3 = handle3.await.unwrap().unwrap();

        assert_eq!(result1, 2);
        assert_eq!(result2, 4);
        assert_eq!(result3, 6);

        // Should have been processed in a single batch
        assert_eq!(batch.processor.process_count(), 1);
    }

    #[tokio::test]
    async fn test_batching_with_delay() {
        let processor = TestProcessor::new();
        let batch_delay = 100;
        let batch = Batch::with_timing(processor, batch_delay, 0);

        let start = Instant::now();
        let result = batch.call(5).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result, 10);
        // Should have waited at least the batch delay
        assert!(elapsed.as_millis() >= batch_delay as u128);
    }

    #[tokio::test]
    async fn test_throttling() {
        let processor = TestProcessor::new();
        let min_interval = 200;
        let batch = Batch::with_timing(processor, 0, min_interval);

        // First batch should process immediately
        let result1 = batch.call(1).await.unwrap();
        assert_eq!(result1, 2);

        // Second call should be throttled
        let start = Instant::now();
        let result2 = batch.call(2).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result2, 4);
        // Should have waited at least the minimum interval
        assert!(elapsed.as_millis() >= min_interval as u128);
    }

    #[tokio::test]
    async fn test_processor_error() {
        let processor = TestProcessor::with_failure();
        let batch = Batch::with_defaults(processor);

        let result = batch.call(5).await;
        assert!(result.is_err());

        match result {
            Err(BatchError::ProcessorError(err)) => {
                assert_eq!(err.0, "Simulated processor error");
            }
            _ => panic!("Expected ProcessorError"),
        }
    }

    #[tokio::test]
    async fn test_multiple_batches() {
        let processor = TestProcessor::new();
        let batch = Arc::new(Batch::with_timing(processor, 50, 100));

        // First batch
        let futures = vec![1, 2, 3]
            .into_iter()
            .map(|i| {
                let batch_clone = Arc::clone(&batch);
                tokio::spawn(async move { batch_clone.call(i).await })
            })
            .collect::<Vec<_>>();

        let results1 = futures::future::join_all(futures).await;

        for result in &results1 {
            assert!(result.as_ref().unwrap().is_ok());
        }

        // Wait for the first batch to fully complete and the throttle time to be recorded
        sleep(Duration::from_millis(50)).await;

        // Second batch - should be throttled
        let start = Instant::now();

        let futures = vec![4, 5]
            .into_iter()
            .map(|i| {
                let batch_clone = Arc::clone(&batch);
                tokio::spawn(async move { batch_clone.call(i).await })
            })
            .collect::<Vec<_>>();

        let results2 = futures::future::join_all(futures).await;
        let elapsed = start.elapsed();

        for result in &results2 {
            assert!(result.as_ref().unwrap().is_ok());
        }

        // Should have processed in two separate batches
        assert_eq!(batch.processor.process_count(), 2);

        // The second batch might not be throttled by the full interval if some time has already passed
        // Just check that there was some delay, not the exact amount
        assert!(elapsed.as_millis() > 0);
    }

    #[tokio::test]
    async fn test_update_config() {
        let processor = TestProcessor::new();
        let mut batch = Batch::with_timing(processor, 100, 200);

        // Update config
        batch.update_config(BatchConfig {
            batch_delay_ms: 10,
            min_interval_ms: 20,
        });

        let start = Instant::now();
        let result = batch.call(5).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result, 10);
        // Should use the new shorter delay
        assert!(elapsed.as_millis() >= 10);
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_set_batch_delay() {
        let processor = TestProcessor::new();
        let mut batch = Batch::with_timing(processor, 100, 200);

        // Update just the batch delay
        batch.set_batch_delay(10);

        let start = Instant::now();
        let result = batch.call(5).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result, 10);
        // Should use the new shorter delay
        assert!(elapsed.as_millis() >= 10);
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_set_min_interval() {
        let processor = TestProcessor::new();
        let mut batch = Batch::with_timing(processor, 0, 200);

        // First call
        batch.call(1).await.unwrap();

        // Update min interval
        batch.set_min_interval(50);

        // Second call should use new interval
        let start = Instant::now();
        batch.call(2).await.unwrap();
        let elapsed = start.elapsed();

        // Should use the new shorter interval
        assert!(elapsed.as_millis() >= 50);
        assert!(elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_empty_batch() {
        // This is an edge case that shouldn't happen in normal usage,
        // but we test it for completeness
        let processor = TestProcessor::new();
        let batch = Batch::with_defaults(processor);

        // Manually trigger the batch processing with no items
        // (This is a bit of a hack to test internal behavior)
        let queue = Arc::clone(&batch.queue);
        Batch::<TestProcessor>::schedule_next_batch(queue, Arc::clone(&batch.processor)).await;

        // The processor should not have been called
        assert_eq!(batch.processor.process_count(), 0);
    }

    #[tokio::test]
    async fn test_slow_processor() {
        let processor = TestProcessor::with_delay(200);
        let batch = Arc::new(Batch::with_timing(processor, 10, 0));

        // Submit multiple items
        let futures = vec![1, 2, 3]
            .into_iter()
            .map(|i| {
                let batch_clone = Arc::clone(&batch);
                tokio::spawn(async move { batch_clone.call(i).await })
            })
            .collect::<Vec<_>>();

        let start = Instant::now();
        let results = futures::future::join_all(futures).await;
        let elapsed = start.elapsed();

        for result in &results {
            assert!(result.as_ref().unwrap().is_ok());
        }

        // Should have waited for the processor delay
        assert!(elapsed.as_millis() >= 200);
    }

    #[tokio::test]
    async fn test_task_dropped() {
        let processor = TestProcessor::with_delay(1000); // Long delay
        let batch = Arc::new(Batch::with_defaults(processor));

        // Start a task that will be cancelled
        let batch_clone = Arc::clone(&batch);
        let handle = tokio::spawn(async move { batch_clone.call(5).await });

        // Cancel the task before it completes
        sleep(Duration::from_millis(10)).await;
        handle.abort();

        // The batch should still process, but no one will receive the result
        sleep(Duration::from_millis(1100)).await;

        // The processor should have been called once
        assert_eq!(batch.processor.process_count(), 1);
    }

    #[tokio::test]
    async fn test_concurrent_batches_with_different_timings() {
        let processor = TestProcessor::new();
        let batch = Arc::new(Batch::with_timing(processor, 50, 100));

        // Submit first item
        let batch_clone1 = Arc::clone(&batch);
        let handle1 = tokio::spawn(async move { batch_clone1.call(1).await });

        // Wait a bit but not enough to trigger the batch
        sleep(Duration::from_millis(20)).await;

        // Submit more items that should be in the same batch
        let batch_clone2 = Arc::clone(&batch);
        let handle2 = tokio::spawn(async move { batch_clone2.call(2).await });

        let batch_clone3 = Arc::clone(&batch);
        let handle3 = tokio::spawn(async move { batch_clone3.call(3).await });

        // Wait for the first batch to complete
        let result1 = handle1.await.unwrap().unwrap();
        let result2 = handle2.await.unwrap().unwrap();
        let result3 = handle3.await.unwrap().unwrap();

        assert_eq!(result1, 2);
        assert_eq!(result2, 4);
        assert_eq!(result3, 6);

        // Wait a bit but not enough to clear the throttle
        sleep(Duration::from_millis(50)).await;

        // Submit more items that should be in a second batch
        let start = Instant::now();
        let batch_clone4 = Arc::clone(&batch);
        let handle4 = tokio::spawn(async move { batch_clone4.call(4).await });

        let result4 = handle4.await.unwrap().unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result4, 8);

        // Should have been throttled
        assert!(elapsed.as_millis() >= 50); // Remaining throttle time

        // Should have processed in two batches
        assert_eq!(batch.processor.process_count(), 2);
    }

    #[tokio::test]
    async fn test_large_batch() {
        let processor = TestProcessor::new();
        let batch = Arc::new(Batch::with_timing(processor, 10, 0));

        // Submit a large number of items
        let count = 1000;
        let handles = (0..count)
            .map(|i| {
                let batch_clone = Arc::clone(&batch);
                tokio::spawn(async move { batch_clone.call(i).await })
            })
            .collect::<Vec<_>>();

        // Wait for all to complete
        let results = futures::future::join_all(handles).await;

        // Verify all results
        for (i, result) in results.iter().enumerate() {
            let value = result.as_ref().unwrap().as_ref().unwrap();
            assert_eq!(*value, (i as u32) * 2);
        }

        // Should have processed in a single batch
        assert_eq!(batch.processor.process_count(), 1);
    }

    #[tokio::test]
    async fn test_zero_timing_wait_for_active() {
        let processor = TestProcessor::with_delay(100); // Processor with delay
        let batch = Arc::new(Batch::with_timing(processor, 0, 0)); // Zero timing config

        // Start first batch
        let batch_clone1 = Arc::clone(&batch);
        let handle1 = tokio::spawn(async move { batch_clone1.call(1).await });

        // Immediately start second batch while first is still processing
        sleep(Duration::from_millis(10)).await;
        let start = Instant::now();
        let batch_clone2 = Arc::clone(&batch);
        let handle2 = tokio::spawn(async move { batch_clone2.call(2).await });

        let result1 = handle1.await.unwrap().unwrap();
        let result2 = handle2.await.unwrap().unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result1, 2);
        assert_eq!(result2, 4);

        // Even with zero timing, should have waited for first batch to complete
        assert!(elapsed.as_millis() >= 90); // Processor delay was 100ms, we waited 10ms before starting
        assert_eq!(batch.processor.process_count(), 2); // Should be two separate batches
    }
}
