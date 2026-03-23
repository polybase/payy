#![allow(clippy::unwrap_used)]
// lint-long-file-override allow-max-lines=300
pub mod core;
pub mod event;
mod shared;
mod stream;
use std::collections::VecDeque;
use std::sync::Arc;

use self::core::SolidCore;
use self::event::SolidEvent;
pub use self::shared::*;

use crate::config::SolidConfig;
use crate::errors::Result;
use crate::proposal::{Manifest, ProposalAccept, ProposalHash};
use crate::traits::{App, PeerSigner};
use crate::util::tick_worker::TickWorker;
use crate::util::time::AtomicTimestamp;
use chrono::Utc;
use parking_lot::Mutex;
use tokio::time::Instant;

/**
 * Solid is responsible creating an event stream of SolidEvent events, and
 * triggering skip messages if interval expires.
 */

#[derive(Clone)]
pub struct Solid<A: App> {
    shared: Arc<SolidShared<A>>,
    ticker: Arc<TickWorker<Arc<SolidShared<A>>>>,
}

impl<A: App> Solid<A> {
    pub fn with_last_confirmed(
        local_peer_signer: A::PS,
        validators: Vec<A::P>,
        manifest: Manifest<A::P, A::State>,
        app: A,
        config: SolidConfig,
    ) -> Self {
        assert!(!validators.is_empty(), "Must have at least one validator");

        let shared = Arc::new(SolidShared {
            local_peer_signer: local_peer_signer.clone(),
            events: Mutex::new(VecDeque::new()),
            core: Mutex::new(SolidCore::with_last_confirmed(
                local_peer_signer,
                manifest,
                app,
                config.clone(),
            )),
            skip_timeout: AtomicTimestamp::new(None),
            out_of_sync_timeout: Mutex::new(None),
            state: Mutex::new(SolidState { waker: None }),
            config,
        });

        Self {
            shared,
            ticker: Arc::new(TickWorker::new()),
        }
    }

    pub fn genesis(
        local_peer_signer: A::PS,
        validators: Vec<A::P>,
        app: A,
        config: SolidConfig,
    ) -> Self {
        Self::with_last_confirmed(
            local_peer_signer,
            validators.clone(),
            Manifest::genesis(validators),
            app,
            config,
        )
    }

    pub fn reset(&self, manifest: Manifest<A::P, A::State>, app: A) {
        let mut store = self.shared.core.lock();
        *store = SolidCore::with_last_confirmed(
            self.shared.local_peer_signer.clone(),
            manifest,
            app,
            self.shared.config.clone(),
        );
        let mut events = self.shared.events.lock();
        events.clear();
        self.reset_skip_timeout();
    }

    pub fn run(&self) -> tokio::task::JoinHandle<()> {
        // Create background worker, this is mostly responsible for sending skips
        // when a new proposal has not been created by the next responsible leader
        // *self.shared.skip_timeout.lock().unwrap() = Some(Instant::now());
        self.next_event();
        self.ticker.run(Arc::clone(&self.shared))
    }

    /// Gets the highest confirmed height for this register
    pub fn height(&self) -> u64 {
        self.shared.core.lock().height()
    }

    pub fn hash(&self) -> ProposalHash {
        self.shared.core.lock().hash().clone()
    }

    pub fn max_height(&self) -> u64 {
        self.shared.core.lock().max_height()
    }

    pub fn is_out_of_sync(&self) -> bool {
        self.shared.core.lock().is_out_of_sync()
    }

    /// Whether a proposal hash exists in the data
    pub fn exists(&self, hash: &ProposalHash) -> bool {
        self.shared.core.lock().exists(hash)
    }

    /// Get a list of confirmed proposals from a given height
    pub fn confirmed_proposals_from(&self, i: u64) -> Vec<Manifest<A::P, A::State>> {
        self.shared
            .core
            .lock()
            .proposals()
            .confirmed_proposals_from(i)
            .iter()
            .map(|p| p.manifest().clone())
            .collect()
    }

    pub fn get_proposal(&self, hash: &ProposalHash) -> Option<Manifest<A::P, A::State>> {
        self.shared
            .core
            .lock()
            .proposals()
            .get(hash)
            .map(|p| p.manifest().clone())
    }

    pub fn current_proposal(&self) -> Manifest<A::P, A::State> {
        self.shared
            .core
            .lock()
            .current_proposal()
            .manifest()
            .clone()
    }

    /// Get a list of decendents from a given proposal hash
    pub fn decendents(
        &self,
        decendent_hash: &ProposalHash,
        parent_hash: &ProposalHash,
    ) -> Vec<Manifest<A::P, A::State>> {
        self.shared
            .core
            .lock()
            .proposals()
            .decendents(decendent_hash, parent_hash)
            .into_iter()
            .map(|p| p.manifest().clone())
            .collect()
    }

    /// Receive a new proposal from an external source, we do some basic validation
    /// to make sure this is a valid proposal that could be confirmed.
    pub fn receive_proposal(&self, manifest: Manifest<A::P, A::State>) -> Result<()> {
        self.shared.core.lock().receive_proposal(manifest)?;

        // Process next rounds with the newly added proposal, and keep
        // processing until nothing is left
        self.next_event();

        Ok(())
    }

    /// Receive a new accept from an external source, we should only really receive accepts
    /// if we are to to be the next leader, store will determine if this is valid and send
    pub fn receive_accept(&self, accept: &ProposalAccept<A::P>) -> Result<()> {
        if let Some(event) = self.shared.core.lock().receive_accept(accept)? {
            self.shared.send_event(event)
        }

        Ok(())
    }

    /// Reset skip timeout as we received the next proposal in time
    fn reset_skip_timeout(&self) {
        // should we just make the config use `chrono::Duration` instead?
        let skip_timeout = chrono::Duration::from_std(self.shared.config.skip_timeout).unwrap();
        let new_timeout = Utc::now() + skip_timeout;
        self.shared.skip_timeout.store(Some(new_timeout));
        self.ticker.tick();
    }

    /// Clear skip timeout when we are the one proposing a new proposal, or when we are
    /// behind/out of sync with the network
    fn clear_skip_timeout(&self) {
        self.shared.skip_timeout.store(None);
        self.ticker.tick();
    }

    // Process the next proposal in the chain, this should move the proposal
    // If we don't have the next proposal in the chain, request it from the network
    fn next_event(&self) {
        let mut store = self.shared.core.lock();

        // Keep telling the store to process proposals until it returns None, signalling it cannot
        // make further process until another proposal or accept is received
        while let Some(event) = store.next_event() {
            match &event {
                // We are catching up or out of sync, so there is no need for the timeout to be active,
                // we will wake up the timer after we have caught up (i.e. when we see the next SendAccept)
                SolidEvent::OutOfSync { .. } => {
                    self.clear_skip_timeout();

                    let mut timeout = self.shared.out_of_sync_timeout.lock();
                    if let Some(timeout) = &mut *timeout
                        && *timeout > Instant::now()
                    {
                        return;
                    }

                    // Set the out of sync timeout
                    *timeout = Some(Instant::now() + self.shared.config.out_of_sync_timeout);

                    // Send the event, but cancel looping
                    self.shared.send_event(event);
                    return;
                }
                SolidEvent::Commit { .. } => {
                    self.reset_skip_timeout();
                }

                SolidEvent::Propose { .. } => {
                    // We don't want to send a skip for our own proposals, so clear the timeout
                    self.clear_skip_timeout();
                }

                // We received a new valid proposal, that we are willing to accept. This is the best
                // indication that we think we are in sync with the network. We now want to track
                // whether the proposal for this accept is also accepted by the network (in time), indicated
                // by receiving a subsequent proposal at the next height (and subsequently the store tells us
                // to send the next SendAccept)
                SolidEvent::Accept { accept } => {
                    // Cancel out of sync, if we are sending an accept
                    *self.shared.out_of_sync_timeout.lock() = None;
                    // Restart the timer for the current proposal to be accepted
                    self.reset_skip_timeout();

                    // Skip sending accept for our own proposal
                    if accept.leader_id == self.shared.local_peer_signer.peer() {
                        return;
                    }
                }

                _ => {}
            }

            // Send the event to the event stream
            self.shared.send_event(event);
        }
    }
}
