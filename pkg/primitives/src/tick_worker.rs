use async_trait::async_trait;
use std::marker::PhantomData;
use std::sync::{
    atomic::{AtomicBool, Ordering::Relaxed},
    Arc,
};
use std::time::Instant;
use tokio::sync::Notify;

pub struct TickWorker<T: TickWorkerTick> {
    shared: Arc<TickWorkerShared>,
    _marker: PhantomData<T>,
}

pub struct TickWorkerShared {
    shutdown: AtomicBool,
    background_worker: Notify,
}

impl<T: TickWorkerTick> Drop for TickWorker<T> {
    fn drop(&mut self) {
        self.shared.shutdown();
    }
}

#[async_trait]
pub trait TickWorkerTick: Send + Sync + 'static {
    async fn tick(&self) -> Option<Instant>;
}

impl<T: TickWorkerTick> TickWorker<T> {
    pub fn new() -> Self {
        TickWorker {
            shared: Arc::new(TickWorkerShared {
                shutdown: AtomicBool::new(false),
                background_worker: Notify::new(),
            }),
            _marker: PhantomData,
        }
    }

    /// Creates and starts the background worker
    pub fn run(&self, ticker: T) -> tokio::task::JoinHandle<()> {
        tokio::spawn(background_worker(Arc::clone(&self.shared), ticker))
    }

    /// Manually forces a tick to occur
    pub fn tick(&self) {
        self.shared.background_worker.notify_one();
    }
}

impl<T: TickWorkerTick> Default for TickWorker<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl TickWorkerShared {
    pub fn shutdown(&self) {
        // Mark as shutdown
        self.shutdown.store(true, Relaxed);

        // Notify the worker, so it wakes up and exits immediately
        self.background_worker.notify_one();
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Relaxed)
    }
}

/// Background worker that calls `tick` whenever its scheduled to run the
/// next task or worken up by the `background_worker` channel.
pub async fn background_worker<T: TickWorkerTick>(worker: Arc<TickWorkerShared>, shared: T) {
    // If the shutdown flag is set, then the task should exit.
    while !worker.is_shutdown() {
        // Check timeout
        if let Some(when) = shared.tick().await {
            let time_to_sleep = when - Instant::now();

            tokio::select! {
                _ = tokio::time::sleep(time_to_sleep) => {}
                _ = worker.background_worker.notified() => {}
            }
        } else {
            // No expiry set, so wait to be notified
            worker.background_worker.notified().await;
        }
    }
}
