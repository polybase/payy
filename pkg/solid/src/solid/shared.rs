#![allow(clippy::unwrap_used)]
// lint-long-file-override allow-max-lines=300
use super::core::SolidCore;
use super::event::SolidEvent;
use crate::config::SolidConfig;
use crate::traits::App;
use crate::util::tick_worker::TickWorkerTick;
use crate::util::time::AtomicTimestamp;
use chrono::{DateTime, Utc};
#[allow(unused_imports)]
use futures::stream::StreamExt;
use futures::task::Waker;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::time::Instant;

pub struct SolidShared<A: App> {
    pub(crate) local_peer_signer: A::PS,

    /// Events to be streamed
    pub(crate) events: Mutex<VecDeque<SolidEvent<A::P, A::State>>>,

    /// Timeout used for skips (aka expired accepts, no new proposal
    /// received within timeout period)
    pub(crate) skip_timeout: AtomicTimestamp,

    /// Timeout used when an out of sync message has been sent to the network,
    /// we need to wait a configurable amount of time before sending another
    pub(crate) out_of_sync_timeout: Mutex<Option<Instant>>,

    /// Shared proposal state, must be updated together
    pub(crate) state: Mutex<SolidState>,

    /// Core for processing proposals and accepts
    pub(crate) core: Mutex<SolidCore<A>>,

    /// Configuration for the proposal register
    pub(crate) config: SolidConfig,
}

#[derive(Debug)]
pub struct SolidState {
    pub(crate) waker: Option<Waker>,
}

impl<A: App> SolidShared<A> {
    pub fn send_event(&self, event: SolidEvent<A::P, A::State>) {
        {
            let mut events = self.events.lock();
            events.push_back(event);
        }
        let mut state = self.state.lock();
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

impl<A: App> TickWorkerTick for Arc<SolidShared<A>> {
    /// Checks for either: next_proposal or a skip timeout
    fn tick(&self) -> Option<DateTime<Utc>> {
        let skip_timeout = self.skip_timeout.load();

        if skip_timeout? > Utc::now() {
            return skip_timeout;
        }

        if let Some(event) = self.core.lock().skip() {
            self.send_event(event);
        }

        let add = chrono::Duration::from_std(self.config.skip_timeout).unwrap();

        // TODO: is this valid, do we really want to add the time here rather than just
        // set a new time from now?
        self.skip_timeout.fetch_add(add);

        self.skip_timeout.load()
    }
}

#[cfg(test)]
mod test {

    use super::super::*;
    use super::*;
    use crate::proposal::{ProposalAccept, ProposalHeader};
    use crate::test::util::{ManifestContent, hash};
    use crate::test::{
        app::{TestApp, UncheckedPeerId},
        util::{create_peers, peer, signer},
    };
    use chrono::{TimeZone, Timelike};

    fn solid(id: u8) -> Solid<TestApp> {
        let app = TestApp;

        Solid::<TestApp>::genesis(
            signer(id),
            create_peers().to_vec(),
            app,
            SolidConfig {
                ..SolidConfig::default()
            },
        )
    }

    fn accept(
        leader: u8,
        hash: ProposalHash,
        height: u64,
        skips: u64,
        from: u8,
    ) -> ProposalAccept<UncheckedPeerId> {
        ProposalAccept {
            leader_id: peer(leader),
            proposal: ProposalHeader {
                hash,
                height,
                skips: 0,
            },
            skips,
            from: peer(from),
            signature: vec![],
        }
    }

    #[tokio::test]
    async fn first_proposal_single_peer() {
        let config = SolidConfig::default();
        let mut solid = Solid::genesis(signer(1), vec![peer(1)], TestApp, config);

        let manifest = Manifest::new(
            ManifestContent {
                last_proposal_hash: solid.hash(),
                height: 1,
                skips: 0,
                leader_id: peer(1),
                state: 0.into(),
                validators: vec![peer(1)],
                accepts: vec![accept(1, solid.hash(), 0, 0, 1)],
            },
            vec![],
        );
        let hash = hash(&manifest);

        solid.receive_proposal(manifest.clone()).unwrap();

        let next = solid.next().await.unwrap();

        assert_eq!(
            next,
            SolidEvent::Propose {
                last_proposal_hash: hash.clone(),
                height: 2,
                skips: 0,
                accepts: vec![accept(1, hash, 1, 0, 1)],
            }
        )
    }

    #[tokio::test]
    async fn first_proposal_multi_peer() {
        let mut solid = solid(2);

        let manifest = Manifest::new(
            ManifestContent {
                last_proposal_hash: solid.hash(),
                height: 1,
                skips: 0,
                leader_id: peer(4),
                state: 0.into(),
                validators: vec![peer(1), peer(2), peer(3)],
                accepts: vec![
                    accept(4, solid.hash(), 0, 0, 1),
                    accept(4, solid.hash(), 0, 0, 2),
                    accept(4, solid.hash(), 0, 0, 3),
                ],
            },
            vec![],
        );
        let hash = hash(&manifest);

        solid.receive_proposal(manifest.clone()).unwrap();

        let next = solid.next().await.unwrap();

        assert_eq!(
            next,
            SolidEvent::Accept {
                accept: ProposalAccept {
                    proposal: ProposalHeader {
                        hash,
                        height: 1,
                        skips: 0
                    },
                    leader_id: peer(1),
                    skips: 0,
                    from: peer(2),
                    signature: vec![]
                }
            }
        )
    }

    #[tokio::test]
    async fn test_tick_no_action() {
        let solid = solid(1);

        assert_eq!(solid.shared.tick(), None);
    }

    /// round down to nearest millisecond
    fn round_millis(time: DateTime<Utc>) -> DateTime<Utc> {
        let nanos_and_micros = time.nanosecond() % 1_000_000;
        let total_nanos = time.nanosecond() - nanos_and_micros;
        time.with_nanosecond(total_nanos).unwrap()
    }

    #[test]
    fn round_millis_works() {
        let time = Utc
            .with_ymd_and_hms(2023, 1, 1, 1, 1, 1)
            .unwrap()
            .with_nanosecond(123_000_123)
            .unwrap();

        let expected = Utc
            .with_ymd_and_hms(2023, 1, 1, 1, 1, 1)
            .unwrap()
            .with_nanosecond(123_000_000)
            .unwrap();

        assert_eq!(round_millis(time), expected);
    }

    #[test]
    fn test_tick_send_skip() {
        let solid = solid(1);

        let now = Utc::now();

        // Add an expired skip_timeout instant
        solid.shared.skip_timeout.store(Some(now));

        let next_tick = solid.shared.tick();

        // Should return the next tick
        assert!(
            next_tick > Some(now + chrono::Duration::seconds(3)),
            "next tick {next_tick:?} should be more than 3 seconds away from now: {now:?}",
        );

        // Should add skip event
        assert_eq!(solid.shared.events.lock().len(), 1);
    }

    #[test]
    fn test_tick_not_ready() {
        let solid = solid(1);

        let time = Utc::now() + chrono::Duration::seconds(10);

        // Add time which is not ready
        solid.shared.skip_timeout.store(Some(time));

        // Returns time to wake up tick
        assert_eq!(solid.shared.tick().unwrap(), round_millis(time));

        // No events added
        assert_eq!(solid.shared.events.lock().len(), 0);
    }
}
