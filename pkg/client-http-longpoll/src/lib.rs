// lint-long-file-override allow-max-lines=300
use chrono::{DateTime, Utc};
use futures::Stream;
use parking_lot::Mutex;
use rpc::longpoll::PollData;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Data returned by LongPoll stream, including the user data and last modified timestamp
#[derive(Debug, Clone)]
pub struct LongPollResult<T> {
    pub data: T,
    pub last_modified: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct LongPoll<LP: LongPollPoller> {
    poller: Arc<LP>,
    state: Arc<Mutex<LongPollState>>,
}

#[derive(Clone, Default)]
pub struct LongPollState {
    error_count: u64,
    last_modified: Option<DateTime<Utc>>,
}

impl LongPollState {
    fn register_error(&mut self, start: Instant) -> Instant {
        self.error_count += 1;
        // Maximum backoff 5 seconds
        start + Duration::from_secs(self.error_count.min(5))
    }

    fn register_modified(&mut self, date: DateTime<Utc>) {
        self.last_modified = Some(date);
        self.reset_error();
    }

    fn reset_error(&mut self) {
        self.error_count = 0;
    }
}

impl<LP: LongPollPoller> LongPoll<LP> {
    pub fn new(poller: LP) -> Self {
        Self {
            poller: Arc::new(poller),
            state: Arc::new(Mutex::new(LongPollState::default())),
        }
    }

    pub fn stream(&self) -> impl Stream<Item = Result<LongPollResult<LP::T>, LP::Error>> {
        let poller = self.poller.clone();
        let state = self.state.clone();

        async_stream::stream! {
            loop {
                let start = Instant::now();

                let last_modified = state.lock().last_modified;

                match poller.poll(last_modified).await {
                    Ok(poll_data) => {
                        match poll_data {
                            PollData::Modified { data, modified_at } => {
                                // Reset error count and update last_modified
                                if let Some(modified_at) = modified_at {
                                    state.lock().register_modified(modified_at);
                                }
                                yield Ok(LongPollResult {
                                    data,
                                    last_modified: modified_at,
                                });
                            }
                            PollData::NotModified => {
                                // Reset error count on successful poll
                                state.lock().reset_error();
                            }
                        }

                        // Don't run poll more than once per second
                        let next_poll = start + Duration::from_secs(1);
                        tokio::time::sleep_until(next_poll.into()).await;
                    }
                    Err(err) => {
                        yield Err(err);

                        // Calculate next retry time based on error count
                        let next_retry = state.lock().register_error(start);
                        tokio::time::sleep_until(next_retry.into()).await;
                    }
                }
            }
        }
    }
}

#[async_trait::async_trait]
pub trait LongPollPoller: Send + Sync + 'static {
    type T: Clone + Send + Sync + 'static;
    type Error: Clone + Send + Sync + 'static;

    async fn poll(
        &self,
        last_modified: Option<DateTime<Utc>>,
    ) -> Result<PollData<Self::T>, Self::Error>;
}

// use chrono::{DateTime, Utc};
// use parking_lot::Mutex;
// use primitives::tick_worker::{TickWorker, TickWorkerTick};
// use rpc::longpoll::PollData;
// use std::{
//     sync::Arc,
//     time::{Duration, Instant},
// };
// use tokio::sync::broadcast::{self, Receiver, Sender};

// #[derive(Clone)]
// pub struct LongPoll<LP: LongPollPoller> {
//     ticker: Arc<TickWorker<LongPollSharedArc<LP>>>,
//     shared: LongPollSharedArc<LP>,
// }

// impl<LP: LongPollPoller> LongPoll<LP> {
//     pub fn new(lp: LP) -> Self {
//         let shared = LongPollSharedArc(Arc::new(LongPollShared {
//             poller: lp,
//             state: Mutex::new(LongPollState::default()),
//         }));
//         let ticker = Arc::new(TickWorker::new());
//         Self { ticker, shared }
//     }

//     pub fn subscribe(&self) {
//         self.ticker.run(self.shared.clone());
//     }
// }

// pub struct LongPollSharedArc<LP: LongPollPoller>(Arc<LongPollShared<LP>>);

// impl<LP: LongPollPoller> Clone for LongPollSharedArc<LP> {
//     fn clone(&self) -> Self {
//         Self(self.0.clone())
//     }
// }

// pub struct LongPollShared<LP: LongPollPoller> {
//     poller: LP,
//     state: Mutex<LongPollState>,
// }

// #[async_trait::async_trait]
// pub trait LongPollPoller: Send + Sync + 'static {
//     type T: Clone + Send + Sync + 'static;
//     type Error: Clone + Send + Sync + 'static;

//     async fn poll(
//         &self,
//         last_modified: Option<DateTime<Utc>>,
//     ) -> Result<PollData<Self::T>, Self::Error>;
// }

// #[derive(Clone)]
// pub struct LongPollState {
//     error_count: u64,
//     last_modified: Option<DateTime<Utc>>,
// }

// impl Default for LongPollState {
//     fn default() -> Self {
//         Self {
//             error_count: 0,
//             last_modified: None,
//         }
//     }
// }

// impl LongPollState {
//     fn register_error(&mut self) -> Instant {
//         self.error_count += 1;
//         // Maximum backoff 5 seconds
//         Instant::now() + Duration::from_secs(self.error_count.min(5))
//     }

//     fn register_modified(&mut self, date: DateTime<Utc>) {
//         self.last_modified = Some(date);
//         self.reset_error();
//     }

//     fn reset_error(&mut self) {
//         self.error_count = 0;
//     }
// }

// #[async_trait::async_trait]
// impl<LP: LongPollPoller> TickWorkerTick for LongPollSharedArc<LP> {
//     async fn tick(&self) -> Option<Instant> {
//         let start = Instant::now();
//         let last_modified = self.0.state.lock().last_modified.clone();
//         match self.0.poller.poll(last_modified).await {
//             Ok(poll_data) => {
//                 match poll_data {
//                     PollData::Modified { data, modified_at } => {
//                         if let Err(_) = self.0.tx.send(Ok(data)) {
//                             // Only possible error is no listeners so we can stop polling
//                             self.0.state.lock().reset_error();
//                             return None;
//                         };
//                         if let Some(modified_at) = modified_at {
//                             self.0.state.lock().register_modified(modified_at);
//                         }
//                     }
//                     PollData::NotModified => {
//                         self.0.state.lock().reset_error();
//                     }
//                 }
//                 // Don't run poll more than once per second
//                 Some(Instant::now().min(start + Duration::from_secs(1)))
//             }
//             Err(err) => {
//                 if let Err(_) = self.0.tx.send(Err(err)) {
//                     // Only possible error is no listeners so we can stop polling
//                     self.0.state.lock().reset_error();
//                     return None;
//                 };
//                 Some(self.0.state.lock().register_error())
//             }
//         }
//     }
// }
