// lint-long-file-override allow-max-lines=1200
use super::event::SolidEvent;
use crate::ProposalAcceptSigData;
use crate::config::SolidConfig;
use crate::errors::{Error, Result};
use crate::proposal::cache::ProposalCache;
use crate::proposal::{Manifest, ProposalHeader};
use crate::proposal::{Proposal, ProposalAccept, ProposalHash};
use crate::traits::{App, PeerSigner};
use std::collections::HashMap;
use std::time::Instant;

/// SolidCore is responsible for processing proposals and accepts.
#[derive(Debug)]
pub struct SolidCore<A: App> {
    /// peer_id of the local node
    local_peer_signer: A::PS,

    /// Pending proposals that may or may not end up being confirmed.
    proposals: ProposalCache<A>,

    // TODO: optimisation, we should move this to the proposal cache, and have manifest be an option if we
    // haven't received the proposal yet.
    /// Orphaned accepts are when we receive an accept for a proposal before we
    /// receive the proposal itself. We can then add these as soon as the proposal arrives.
    orphan_accepts: HashMap<ProposalHash, ProposalOrphan<A>>,

    /// Application state
    app: A,

    /// Config
    config: SolidConfig,

    /// Max height
    max_height: u64,
}

#[derive(Debug)]
pub struct ProposalOrphan<A: App> {
    pub accepts: Vec<ProposalAccept<A::P>>,
    pub first_seen: Instant,
}

impl<A: App> SolidCore<A> {
    pub fn with_last_confirmed(
        local_peer_signer: A::PS,
        last_confirmed_proposal: Manifest<A::P, A::State>,
        app: A,
        config: SolidConfig,
    ) -> Self {
        let max_height = last_confirmed_proposal.height;

        Self {
            local_peer_signer,
            proposals: ProposalCache::new(
                Proposal::new(last_confirmed_proposal),
                config.max_proposal_history,
            ),
            app,
            max_height,
            orphan_accepts: HashMap::new(),
            config,
        }
    }

    /// Height of the proposal that was last confirmed
    pub fn height(&self) -> u64 {
        self.proposals.height()
    }

    /// Hash of last confirmed
    pub fn hash(&self) -> &ProposalHash {
        self.proposals().last_confirmed_proposal().hash()
    }

    /// Checks if the proposal hash exists, only checks pending proposals
    /// as confirmed proposals can be checked via height.
    pub fn exists(&self, hash: &ProposalHash) -> bool {
        self.proposals.contains(hash)
    }

    pub fn max_height(&self) -> u64 {
        self.max_height
    }

    pub fn proposals(&self) -> &ProposalCache<A> {
        &self.proposals
    }

    /// Add a pending proposal to the storeProposalManifestContent
    pub fn receive_proposal(&mut self, manifest: Manifest<A::P, A::State>) -> Result<()> {
        let mut proposal = Proposal::<A>::new(manifest);
        let hash = proposal.hash().clone();

        // Check that the manifest is valid (basic accepts/sig check)
        proposal.validate_structure(&self.app, self.config.accept_threshold)?;

        // Proposal already exists, don't recreate
        if self.exists(&hash) {
            return Err(Error::ProposalAlreadyExists(hash));
        }

        // Check that the height of this proposal > confirmed
        if self.height() >= proposal.height() {
            return Err(Error::ProposalHeightTooLow);
        }

        // Set max height based on proposal
        if proposal.height() > self.max_height {
            self.max_height = proposal.height();
        }

        // Check if we have orphaned accepts
        if let Some(orphan) = self.orphan_accepts.remove(proposal.hash()) {
            for accept in orphan.accepts {
                proposal.add_accept(accept, self.config.accept_threshold);
            }
        }

        // Insert the proposal to be processed later
        self.proposals.insert(proposal);

        Ok(())
    }

    /// Processes the next event in the store, this will either return a commit or accept
    /// event, or None if no more events are ready
    pub fn next_event(&mut self) -> Option<SolidEvent<A::P, A::State>> {
        // Check if there is a proposal we can commit
        if let Some(proposal) = self.proposals.next_pending_proposal(0)
            && let Some(confirmed_by) = self.proposals.next_pending_proposal(1)
        {
            let manifest = proposal.manifest().clone();
            let confirmed_by = confirmed_by.manifest().clone();

            // Add proposal to confirmed list
            self.proposals.confirm(A::hash(&manifest));

            // Send commit
            return Some(SolidEvent::Commit {
                manifest,
                confirmed_by,
            });
        }

        // Check if we're out of sync
        if self.is_out_of_sync() {
            return Some(SolidEvent::OutOfSync {
                height: self.height(),
                max_seen_height: self.max_height,
            });
        }

        let current_proposal = self.validated_current_proposal();

        // If we have not already sent an accept for this proposal, then do so now
        if !current_proposal.initial_accept_sent {
            return self.get_next_accept_event(false);
        }

        None
    }

    /// Skip should be called when we have not received a proposal from the next leader
    /// within the timeout period. Skip will send an accept to the next leader.
    pub fn skip(&mut self) -> Option<SolidEvent<A::P, A::State>> {
        // Just in case we try to skip when we're still catching up
        if self.is_out_of_sync() {
            return None;
        }

        // Get the next accept/skip
        self.get_next_accept_event(true)
    }

    /// Gets the next accept to send, where no pending proposal is available,
    /// last confirmed will be used.
    fn get_next_accept_event(&mut self, skip: bool) -> Option<SolidEvent<A::P, A::State>> {
        let local_peer = self.local_peer_signer.peer();
        let threshold = self.config.accept_threshold;
        let current_proposal = self.validated_current_proposal();

        let current_proposal_hash = current_proposal.hash().clone();

        // Get the skip counter
        let skips = current_proposal.next_accept_skip(threshold, skip);

        let proposal_header = ProposalHeader {
            hash: current_proposal_hash,
            height: current_proposal.height(),
            skips: current_proposal.skips(),
        };

        let signature = ProposalAcceptSigData::new(proposal_header.clone(), skips)
            .sign(&self.local_peer_signer);

        let current_proposal = self.validated_current_proposal();

        let accept = ProposalAccept {
            proposal: proposal_header,
            leader_id: current_proposal.manifest.get_leader_for_skip(skips),
            skips,
            from: local_peer,
            signature,
        };

        current_proposal.skips_sent = skips;
        current_proposal.initial_accept_sent = true;

        // Add our own accept to the proposal
        self.add_accept(&accept)
            .or(Some(SolidEvent::Accept { accept }))
    }

    pub fn receive_accept(
        &mut self,
        accept: &ProposalAccept<A::P>,
    ) -> Result<Option<SolidEvent<A::P, A::State>>> {
        // Check if accept is valid
        accept.verify_signature()?;

        let current_proposal_hash = self.validated_current_proposal().hash().clone();

        // Update max height
        if accept.proposal.height > self.height() {
            self.max_height = accept.proposal.height;
        }

        // Check if we have the proposal, so we can exit early if we know it's invalid
        if let Some(p) = self.proposals.get(&accept.proposal.hash) {
            // Check if the accept is valid
            p.validate_accept(accept)?;
        }

        // Add accept to proposal
        Ok(self.add_accept(accept).or_else(|| {
            // Check if the current proposal has changed, due to adding the accept
            if current_proposal_hash != *self.validated_current_proposal().hash() {
                return self.get_next_accept_event(false);
            }
            None
        }))
    }

    /// Adds an accept to a proposal, we should only be receiving accepts if we are the
    /// next designated leader. Returns ProposalNextState if we have hit the majority and the
    /// accept is still valid, otherwise returns None.
    pub fn add_accept(
        &mut self,
        accept: &ProposalAccept<A::P>,
    ) -> Option<SolidEvent<A::P, A::State>> {
        let ProposalAccept {
            proposal: accepted_proposal,
            skips,
            ..
        } = accept;

        // Check if accept is out of date, accept_height must be greater than confirmed height,
        // but if there are no pending proposals we may need to accept a proposal with
        // accept height == confirmed height (e.g. during start up)
        if self.height() > accepted_proposal.height {
            return None;
        }

        let is_out_of_sync = self.is_out_of_sync();
        let local_peer = &self.local_peer_signer.peer();
        let current_proposal_hash = self.validated_current_proposal().hash().clone();

        // Add accept to proposal (or to orphaned hash map if proposal is not found/received yet).
        // We always store accepts for any future proposal, as we may need them later
        match self.proposals.get_mut(&accepted_proposal.hash) {
            Some(p) => {
                // Trigger propose if adding this accept brings us over the accept threshold AND
                // we are the leader for this accept skip AND we are not out of sync
                if p.add_accept(accept.clone(), self.config.accept_threshold)
                    && &p.manifest.get_leader_for_skip(*skips) == local_peer
                    && !is_out_of_sync
                {
                    return Some(SolidEvent::Propose {
                        last_proposal_hash: accepted_proposal.hash.clone(),
                        height: p.height() + 1,
                        skips: *skips,
                        accepts: p.accepts_for_skip(skips).unwrap_or_default(),
                    });
                }

                // This allows us to send an accept for the higher accept we just received, or produce
                // a proposal if we have >2/3 accepts for the current proposal
                if current_proposal_hash == accepted_proposal.hash && skips > &p.skips_sent {
                    // TODO: this is UNSAFE in an untrusted network, as a validator could send a high skip
                    // that would make them the leader, and all other validators would then send supporing accepts/skips.
                    // We either need to have a VDF that ensures that the skip height is valid (or use some other
                    // safe method for convergence)
                    p.skips_sent = *skips;
                    return self.get_next_accept_event(false);
                }

                None
            }
            None => {
                // Get exisiting orphaned proposal list (or create it if it doesn't exist yet)
                let first_seen =
                    if let Some(o) = self.orphan_accepts.get_mut(&accepted_proposal.hash) {
                        o.accepts.push(accept.clone());
                        o.first_seen
                    } else {
                        let now = Instant::now();
                        self.orphan_accepts.insert(
                            accepted_proposal.hash.clone(),
                            ProposalOrphan {
                                accepts: vec![accept.clone()],
                                first_seen: now,
                            },
                        );
                        now
                    };

                // We're the designated leader, and we don't have the proposal being
                // accepted. We may need to request it from the network.
                let missing_proposal_timeout = self.config.missing_proposal_timeout;
                let current_proposal = self.validated_current_proposal();
                let current_proposal_height = current_proposal.height();

                // It's normal to receive accepts for proposals +1 height ahead of our current proposal,
                // because a recently published proposal may be received by another node before me, and they
                // could then send that accept.
                let missing_more_than_one_proposal =
                    accepted_proposal.height > (current_proposal_height + 1);

                // We've been waiting a long time for a proposal and that proposal is a higher height or
                // same height but higher skip
                let proposal_first_seen_expired = first_seen.elapsed() > missing_proposal_timeout;

                let accept_is_greater = accepted_proposal.height > current_proposal.height()
                    || accepted_proposal.skips > current_proposal.skips();

                let accept_is_greater_and_timeout =
                    proposal_first_seen_expired && accept_is_greater;

                if missing_more_than_one_proposal || accept_is_greater_and_timeout {
                    // We are behind, so we need to request proposals from the network
                    return Some(SolidEvent::OutOfSync {
                        height: self.height(),
                        max_seen_height: accepted_proposal.height,
                    });
                }
                None
            }
        }
    }

    fn highest_next_pending(&self) -> Option<&Proposal<A>> {
        let last_confirmed = self.proposals.last_confirmed_proposal();

        let last_confirmed_highest_skip = last_confirmed
            .highest_skip_with_inverse(self.config.accept_threshold)
            .unwrap_or(0);

        // The proposal with the highest skip at height + 1
        self.proposals.next_pending_proposal(0).and_then(|p| {
            // Check if the skips for the next proposal are high enough:
            //  1. network has already skipped passed this proposal, then we should ignore it (it can never be valid)
            //  2. we have skipped pass this proposal ourselves
            if last_confirmed_highest_skip > p.skips() || last_confirmed.skips_sent > p.skips() {
                return None;
            }
            Some(p)
        })
    }

    /// The current proposal we are sending accepts for, this will be either:
    ///  - confirmed_proposal + 1 (with the heighest skip we have seen)
    ///  - confirmed_proposal (if we have no pending proposals)
    fn validated_current_proposal_hash(&mut self) -> ProposalHash {
        loop {
            let last_confirmed = self.proposals.last_confirmed_proposal();
            let (highest_next_pending, is_valid) = self
                .highest_next_pending()
                .map(|p| {
                    (
                        p.hash(),
                        p.is_validated || p.validate_contents(&self.app, last_confirmed).is_ok(),
                    )
                })
                .unwrap_or_else(|| (self.proposals.last_confirmed_proposal().hash(), true));

            let hash = highest_next_pending.clone();

            if is_valid {
                return hash;
            }

            // Remove the invalid proposal
            self.proposals.remove(&hash);
        }
    }

    pub fn current_proposal(&mut self) -> &Proposal<A> {
        let hash = self.validated_current_proposal_hash();
        #[allow(clippy::expect_used)]
        self.proposals
            .get(&hash)
            .expect("Proposal is current, but now missing from proposal cache")
    }

    pub fn validated_current_proposal(&mut self) -> &mut Proposal<A> {
        let hash = self.validated_current_proposal_hash();
        #[allow(clippy::expect_used)]
        let proposal = self
            .proposals
            .get_mut(&hash)
            .expect("Proposal is current, but now missing from proposal cache");

        // Cache the validation status, we can simply mark as true as we only get the hash if
        // it's valid
        proposal.is_validated = true;
        proposal
    }

    /// Do we have have proposals from the network that are beyond the normal operating
    /// limit (i.e. confirmed height + 1)
    /// TODO: should we include accepts in this too?
    pub fn is_out_of_sync(&self) -> bool {
        self.max_height > self.height() + 1
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::{
        Error, assert_none,
        config::SolidConfig,
        test::{
            app::TestApp,
            util::{
                SolidCore, accept, accept_event, commit_event, core_genesis,
                core_genesis_with_config, core_with_last_confirmed, create_manifest, hash,
                last_confirmed, leader, manifest_at, out_of_sync_event, peer, propose_event, skips,
            },
        },
    };

    // Propoasl/accept numbering:
    // - 1.0 = height 1, skip 0
    // - 1.1 = height 1, skip 1
    // - 2.0 = height 2, skip 0
    // etc

    // TODO:
    // - test .skip() - more events are possible than are tested
    // - do we detect invalid proposal chains? E.g. 1.0 -> 2.0 -> 1.1
    // - test we do not send accepts or proposals when out of sync with the network
    // - out of order accepts are stored and can be used to propose later
    // - ignores accepts where I am not the designated leader, we should just reject these completely

    /// Test nominal path for proposal confirmation from genesis -> 0.0 -> 1.0 -> 2.0
    #[test]
    fn test_next_genesis() {
        // Create store from genesis
        let mut store = core_genesis();

        // Confirmed genesis
        let p_0_0 = last_confirmed(&store);

        // Send accept for 0.0 (to 1.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_0_0, skips(0), leader(4), peer(1))
        );
        assert_none!(store.next_event());

        // Proposal 1.0 received (from 0.0)
        let p_1_0 = create_manifest(1, 0, 4, &p_0_0);
        store.receive_proposal(p_1_0.clone()).unwrap();

        // Send accept for 1.0 (to 2.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_1_0, skips(0), leader(1), peer(1))
        );
        assert_none!(store.next_event());

        // Add proposal for height 2.0 (from 1.0)
        let p_2_0 = create_manifest(2, 0, 1, &p_1_0);
        store.receive_proposal(p_2_0.clone()).unwrap();

        // Commit 1.0 (from 0.0)
        assert_eq!(store.next_event(), commit_event(&p_1_0, &p_2_0));

        // Send accept for 2.0 (from 1.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_2_0, skips(0), leader(3), peer(1))
        );

        // Noting else to do
        assert_none!(store.next_event());
    }

    // Tests nominal path for confirmation from a given starting proposal -> 10.0 -> 11.0 -> 12.0
    #[test]
    fn test_next_with_last_confirmed() {
        // Create a store with last confirmed 10.0
        let p_10_0 = create_manifest(10, 0, 2, &manifest_at(9));
        let mut store = core_with_last_confirmed(&p_10_0);

        // Send accept for 10.0 (to 11.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_10_0, skips(0), leader(4), peer(1))
        );

        // Proposal 11.0 received (from 10.0)
        let p_11_0 = create_manifest(11, 0, 4, &p_10_0);
        store.receive_proposal(p_11_0.clone()).unwrap();

        // Send accept for 11.0 (to 12.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_11_0, skips(0), leader(4), peer(1))
        );
        assert_none!(store.next_event());

        // Proposal 12.0 received (from 11.0)
        let p_12_0 = create_manifest(12, 0, 4, &p_11_0);
        store.receive_proposal(p_12_0.clone()).unwrap();

        // Commit 11.0 (from 10.0)
        assert_eq!(store.next_event(), commit_event(&p_11_0, &p_12_0));

        // Send accept for 12.0 (from 11.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_12_0, skips(0), leader(4), peer(1))
        );

        // Nothing else to do
        assert_none!(store.next_event());
    }

    /// Node skips, network skips -> 0.0 -> 1.1 -> 2.0
    ///
    ///  - 0.0 is confirmed (genesis)
    ///  - Send accept for 0.0 (to 1.0)
    ///  - Timeout expires waiting for 1.0
    ///  - Send accept for 0.0 (to 1.1)
    ///  - 1.1 proposal received
    ///  - Send accept for 1.1 (to 2.0)
    ///  - 2.0 proposal received
    ///  - Confirm 1.0 proposal
    ///
    #[test]
    fn test_node_skips_network_skips() {
        // Create store from genesis
        let mut store = core_genesis();

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        // Send accept for 0.0 (to 1.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_0_0, skips(0), leader(4), peer(1))
        );

        // Send skip for 0.0 (to 1.1) after timeout
        assert_eq!(store.skip(), accept_event(&p_0_0, 1, leader(3), peer(1)));

        // Proposal 1.1 received (from 0.0)
        let p_1_1 = create_manifest(1, 1, 3, &p_0_0);
        store.receive_proposal(p_1_1.clone()).unwrap();

        // Send accept for 1.1 (to 2.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_1_1, skips(0), leader(3), peer(1))
        );

        // Proposal 2.0 received (from 1.1)
        let p_2_0 = create_manifest(2, 0, 3, &p_1_1);
        store.receive_proposal(p_2_0.clone()).unwrap();

        // Commit
        assert_eq!(store.next_event(), commit_event(&p_1_1, &p_2_0));

        // Send accept for 2.0 (to 3.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_2_0, skips(0), leader(3), peer(1))
        );

        // Nothing else to do
        assert_none!(store.next_event());
    }

    /// Node accepts, network skips -> 0.0 -> 1.1 -> 2.0
    ///
    /// In this scenario, the network skips a proposal, but the node does not.
    /// Node will later conform to the network decision to skip.
    ///
    ///  - 0.0 is confirmed (genesis)
    ///  - Send accept for 0.0 (to 1.0)
    ///  - 1.0 proposal received
    ///  - Send accept for 1.0 (to 2.0)
    ///  - 1.1 proposal received
    ///  - Send accept for 1.1 (to 2.0)
    ///  - 2.0 proposal received
    ///  - Confirm 1.1 proposal
    ///
    #[test]
    fn test_node_accepts_network_skips() {
        // Create store from genesis
        let mut store = core_genesis();

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        // Send accept for 0.0 (to 1.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_0_0, skips(0), leader(4), peer(1))
        );
        assert_none!(store.next_event());

        // Proposal 1.0 received (from 0.0)
        let p_1_0 = create_manifest(1, 0, 4, &p_0_0);
        store.receive_proposal(p_1_0.clone()).unwrap();

        // Send accept for 1.0 (to 2.0), I can do this because I have not sent a skip for 0.0 to 1.1
        assert_eq!(
            store.next_event(),
            accept_event(&p_1_0, skips(0), leader(1), peer(1))
        );
        assert_none!(store.next_event());

        // Proposal 1.1 received (from 1.0), >2/3 must have sent a skip to 1.1
        let p_1_1 = create_manifest(1, 1, 3, &p_0_0);
        store.receive_proposal(p_1_1.clone()).unwrap();

        // Send accept for 1.1 (to 2.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_1_1, skips(0), leader(3), peer(1))
        );
        assert_none!(store.next_event());

        // TODO: if I receive a proposal 2.0 based on 1.0 then something malicious is going on, we should test this scenario
        // and impl alerting/slashing rules

        // Proposal 2.0 received (from 1.1)
        let p_2_0 = create_manifest(2, 0, 3, &p_1_1);
        store.receive_proposal(p_2_0.clone()).unwrap();

        // Commit
        assert_eq!(store.next_event(), commit_event(&p_1_1, &p_2_0));

        // Send accept for 2.0 (to 3.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_2_0, skips(0), leader(3), peer(1))
        );

        // Nothing else to do
        assert_none!(store.next_event());
    }

    /// Node skips, network accepts -> 0.0 -> 1.0 -> 2.0
    ///
    /// In this scenario, the node skips a proposal, but the network does not. Although, I may
    /// see the proposal I skipped (as the network has voted for it), I must not accept it.
    ///
    ///  - 0.0 is confirmed (last confirmed proposal, genesis)
    ///  - Send accept for 0.0 (to 1.0)
    ///  - Timeout expires waiting for 1.0
    ///  - Send accept for 0.0 (to 1.1)
    ///  - 1.0 proposal received (ignored)
    ///  - 1.1 proposal received
    ///  - 2.0 proposal received
    ///  - Confirm 1.0 proposal
    ///
    #[test]
    fn test_node_skips_network_accepts() {
        // Create store from genesis
        let mut store = core_genesis();

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        // Send accept for 0.0 (to 1.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_0_0, skips(0), leader(4), peer(1))
        );

        // Send skip for 0.0 (to 1.1) after timeout
        assert_eq!(store.skip(), accept_event(&p_0_0, 1, peer(3), peer(1)));

        // Proposal 1.0 received (from 0.0)
        let p_1_0 = create_manifest(1, 0, 1, &p_0_0);
        store.receive_proposal(p_1_0.clone()).unwrap();

        // Ignore proposal, as we skipped it
        assert_none!(store.next_event());

        // Proposal 2.0 received (from 1.0)
        let p_2_0 = create_manifest(2, 0, 1, &p_1_0);
        store.receive_proposal(p_2_0.clone()).unwrap();

        // Commit
        assert_eq!(store.next_event(), commit_event(&p_1_0, &p_2_0));

        // Send accept for 2.0 (to 3.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_2_0, skips(0), leader(3), peer(1))
        );

        // Nothing else to do
        assert_none!(store.next_event());
    }

    /// Node accepts, network skips, node reverts -> 0.0 -> 1.1 -> 2.0
    ///
    /// In this scenario, the node accepts a proposal (1.0) along with >=1/3 of the network, however >=1/3
    /// skip 1.0. As a result, those who have accepted 1.0 must now revert to skipping 1.0 and instead vote
    /// for 1.1 so the network can converge.
    ///
    ///  - 0.0 is confirmed (last confirmed proposal, genesis)
    ///  - Send accept for 0.0 (to 1.0)
    ///  - 1.0 proposal received
    ///  - Send accept for 1.0 (to 2.0)
    ///  - Receive skips from >1/3 for 0.0 (to 1.1) - 1.0 can never be confirmed now
    ///  - Send accept for 0.0 (to 1.1)
    ///  - 1.1 proposal received
    ///  - Send accept for 1.1 (to 2.0)
    ///
    #[test]
    fn test_node_accepts_network_skips_with_skip_revert() {
        // Create store from genesis
        let mut store = core_genesis();

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        // Send accept for 0.0 (to 1.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_0_0, skips(0), leader(4), peer(1))
        );
        assert_none!(store.next_event());

        // Proposal 1.0 received (from 0.0)
        let p_1_0 = create_manifest(1, 0, 4, &p_0_0);
        store.receive_proposal(p_1_0.clone()).unwrap();

        // Send accept for 1.0 (to 2.0), I can do this because I have not sent a skip for 0.0 to 1.1
        assert_eq!(
            store.next_event(),
            accept_event(&p_1_0, skips(0), leader(1), peer(1))
        );
        assert_none!(store.next_event());

        // Notified that peer 2 has skipped to 1.1
        assert_none!(
            store
                .receive_accept(&accept(&p_0_0, 1, leader(3), peer(2)))
                .unwrap()
        );

        // Notified that peer 3 has skipped to 1.1, now more than >=1/3 skips seen,
        // we should send an accept for 1.1 too
        assert_eq!(
            store.receive_accept(&accept(&p_0_0, 1, peer(3), peer(3))),
            Ok(accept_event(&p_0_0, 1, peer(3), peer(1)))
        );

        // Proposal 1.1 received (from 1.0), >2/3 must have sent a skip to 1.1
        let p_1_1 = create_manifest(1, 1, 3, &p_0_0);
        store.receive_proposal(p_1_1.clone()).unwrap();

        // Send accept for 1.1 (to 2.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_1_1, skips(0), leader(3), peer(1))
        );
        assert_none!(store.next_event());

        // Proposal 2.0 received (from 1.1)
        let p_2_0 = create_manifest(2, 0, 3, &p_1_1);
        store.receive_proposal(p_2_0.clone()).unwrap();

        // Commit
        assert_eq!(store.next_event(), commit_event(&p_1_1, &p_2_0));

        // Send accept for 2.0 (to 3.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_2_0, skips(0), leader(3), peer(1))
        );

        // Nothing else to do
        assert_none!(store.next_event());
    }

    /// Test multiple skips for a proposal, leader should rotate for each skip,
    /// no further action should be taken by .next_event()
    #[test]
    fn test_multiple_skips() {
        // Create store from genesis
        let mut store = core_genesis();

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        // Send accept for 0.0 (to 1.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_0_0, skips(0), leader(4), peer(1))
        );
        assert_none!(store.next_event());

        // Send skip for 0.0 (to 1.1) after timeout
        assert_eq!(store.skip(), accept_event(&p_0_0, 1, peer(3), peer(1)));
        assert_none!(store.next_event());

        // Send skip for 0.0 (to 1.2) after timeout
        assert_eq!(store.skip(), accept_event(&p_0_0, 2, peer(2), peer(1)));
        assert_none!(store.next_event());

        // Send skip for 0.0 (to 1.3) after timeout
        assert_eq!(store.skip(), accept_event(&p_0_0, 3, peer(1), peer(1)));
        assert_none!(store.next_event());

        // Send skip for 0.0 (to 1.4) after timeout
        assert_eq!(store.skip(), accept_event(&p_0_0, 4, peer(4), peer(1)));
        assert_none!(store.next_event());
    }

    /// Node should propose a proposal once it has received >2/3 of accepts
    #[test]
    fn test_propose_when_accept_threshold_met() {
        // Create store from genesis
        let mut store = core_genesis();

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        assert_eq!(
            store.next_event(),
            accept_event(&p_0_0, skips(0), leader(4), peer(1))
        );

        // Skip 3 times so we are the leader (so we can ensure the leader adds their own accept)
        assert_eq!(store.skip(), accept_event(&p_0_0, 1, leader(3), peer(1)));
        assert_eq!(store.skip(), accept_event(&p_0_0, 2, leader(2), peer(1)));
        assert_eq!(store.skip(), accept_event(&p_0_0, 3, leader(1), peer(1)));

        // Receive first accept, no >2/3
        assert_none!(
            store
                .receive_accept(&accept(&p_0_0, 3, leader(1), peer(2)))
                .unwrap()
        );

        // Receive duplicate first accept, no >2/3
        assert_none!(
            store
                .receive_accept(&accept(&p_0_0, 3, leader(1), peer(2)))
                .unwrap()
        );

        // Receive second accept, >2/3, so we should propose
        let propose = propose_event(
            hash(&p_0_0),
            1,
            3,
            vec![
                accept(&p_0_0, 3, leader(1), peer(1)),
                accept(&p_0_0, 3, leader(1), peer(2)),
                accept(&p_0_0, 3, leader(1), peer(3)),
            ],
        );
        assert_eq!(
            store.receive_accept(&accept(&p_0_0, skips(3), leader(1), peer(3))),
            Ok(propose)
        );

        // Technically, we'd expect to be passed a proposal (generated locally from the propose event
        // but if we haven't been sent that, then we have nothing else to do)
        assert_none!(store.next_event());

        // Final accept received, but no further action required
        assert_none!(
            store
                .receive_accept(&accept(&p_0_0, skips(0), leader(4), peer(4)))
                .unwrap()
        );

        // Still nothing to do
        assert_none!(store.next_event());
    }

    #[test]
    fn test_propose_with_higher_skip_when_threshold_met() {
        // Create store from genesis
        let mut store = core_genesis();

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        // Skip
        assert_eq!(
            store.next_event(),
            accept_event(&p_0_0, 0, leader(4), peer(1))
        );
        assert_eq!(store.skip(), accept_event(&p_0_0, 1, leader(3), peer(1)));

        // Add accept for 0.0 (to 1.11), we generate an accept here so the network
        // can converge more quickly
        assert_eq!(
            store.receive_accept(&accept(&p_0_0, 11, leader(1), peer(2))),
            Ok(accept_event(&p_0_0, 11, leader(1), peer(1)))
        );

        // Receive second accept, >2/3, so we should propose
        let propose = propose_event(
            hash(&p_0_0),
            1,
            11,
            vec![
                accept(&p_0_0, 11, peer(1), peer(1)),
                accept(&p_0_0, 11, peer(1), peer(2)),
                accept(&p_0_0, 11, peer(1), peer(3)),
            ],
        );
        assert_eq!(
            store.receive_accept(&accept(&p_0_0, skips(11), leader(1), peer(3))),
            Ok(propose)
        );

        assert_none!(store.next_event());
    }

    #[test]
    fn test_higher_skip_accept_received() {
        // Create store from genesis
        let mut store = core_genesis();

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        // Add accept for 0.0 (to 1.10)
        assert_eq!(
            store.receive_accept(&accept(&p_0_0, 10, leader(2), peer(2))),
            Ok(accept_event(&p_0_0, 10, leader(2), peer(1)))
        );

        // Skip should now start from 11
        assert_eq!(store.skip(), accept_event(&p_0_0, 11, leader(1), peer(1)));
    }

    /// Test that during start up (where we have no pending proposals), that we generate
    /// an out of sync event if we are behind the network.
    #[test]
    fn test_out_of_sync_from_proposal_at_startup() {
        // Create store from genesis
        let mut store = core_genesis();

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        // Send accept for 0.0 (to 1.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_0_0, skips(0), leader(4), peer(1))
        );
        assert_none!(store.next_event());

        // Not out of sync currently
        assert!(!store.is_out_of_sync());

        // DID NOT receive proposal 1.0
        let p_1_0 = create_manifest(1, 0, 2, &p_0_0);

        // Receive proposal 2.0
        let p_2_0 = create_manifest(2, 0, 1, &p_1_0);
        store.receive_proposal(p_2_0.clone()).unwrap();

        // We are behind, so we should send an out of sync message
        assert_eq!(store.next_event(), out_of_sync_event(0, 2));

        // Out of sync
        assert!(store.is_out_of_sync());

        // Now we receive proposal 1.0
        store.receive_proposal(p_1_0.clone()).unwrap();

        // Commit 1.0
        assert_eq!(store.next_event(), commit_event(&p_1_0, &p_2_0));

        // No longer out of sync
        assert!(!store.is_out_of_sync());
    }

    /// Test that after start up, where we should always have a pending proposal, that we
    /// generate an out of sync event if we are behind the network.
    #[test]
    fn test_out_of_sync_from_proposal_with_pending() {
        // Create store from genesis
        let mut store = core_genesis();

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        // Receive proposal 1.0
        let p_1_0 = create_manifest(1, 0, 4, &p_0_0);
        store.receive_proposal(p_1_0.clone()).unwrap();

        // Send accept for 1.0 (to 2.0)
        assert_eq!(
            store.next_event(),
            accept_event(&p_1_0, skips(0), leader(1), peer(1))
        );
        assert_none!(store.next_event());

        // Not out of sync currently
        assert!(!store.is_out_of_sync());

        // DID NOT receive proposal 2.0
        let p_2_0 = create_manifest(2, 0, 2, &p_1_0);

        // Receive proposal 3.0
        let p_3_0 = create_manifest(3, 0, 1, &p_2_0);
        store.receive_proposal(p_3_0.clone()).unwrap();

        // We are behind, so we should send an out of sync message
        assert_eq!(store.next_event(), out_of_sync_event(0, 3));

        // Out of sync
        assert!(store.is_out_of_sync());

        // Now we receive proposal 2.0
        store.receive_proposal(p_2_0.clone()).unwrap();

        // Commit 1.0
        assert_eq!(store.next_event(), commit_event(&p_1_0, &p_2_0));

        // Commit 2.0
        assert_eq!(store.next_event(), commit_event(&p_2_0, &p_3_0));

        // No longer out of sync
        assert!(!store.is_out_of_sync());
    }

    /// We can also use accepts to determine if we're out of sync with the network
    #[test]
    fn test_out_of_sync_from_accept_at_startup() {
        // Create store from genesis
        let mut store = core_genesis();

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        // Not out of sync currently
        assert!(!store.is_out_of_sync());

        // DID NOT receive proposal 1.0
        let p_1_0 = create_manifest(1, 0, 2, &p_0_0);

        // DID NOT receive proposal 2.0
        let p_2_0 = create_manifest(2, 0, 2, &p_1_0);

        // Receive accept for 1.0
        assert_none!(
            store
                .receive_accept(&accept(&p_1_0, 0, leader(1), peer(1)))
                .unwrap()
        );

        // Not out of sync currently
        assert!(!store.is_out_of_sync());

        // Receive accept for 2.0
        assert_eq!(
            store.receive_accept(&accept(&p_2_0, 0, leader(1), peer(1))),
            Ok(out_of_sync_event(0, 2))
        );

        // Receive accept for 2.0
        assert_none!(
            store
                .receive_accept(&accept(&p_0_0, 0, leader(4), peer(2)))
                .unwrap()
        );
    }

    #[test]
    fn test_out_of_sync_from_accept_at_startup_with_delay() {
        // Create store from genesis
        let mut store: SolidCore<TestApp> = core_genesis_with_config(SolidConfig {
            missing_proposal_timeout: Duration::from_millis(200),
            ..Default::default()
        });

        // Last confirmed proposal
        let p_0_0 = last_confirmed(&store);

        // Not out of sync currently
        assert!(!store.is_out_of_sync());

        // DID NOT receive proposal 1.0
        let p_1_0 = create_manifest(1, 0, 2, &p_0_0);

        // Receive accept for 1.0
        assert_none!(
            store
                .receive_accept(&accept(&p_1_0, 0, leader(1), peer(1)))
                .unwrap()
        );

        // Time passes and we receive another accept for 1.0
        std::thread::sleep(store.config.missing_proposal_timeout);

        // Now enough time has passed, we should send out of sync
        assert_eq!(
            store.receive_accept(&accept(&p_1_0, 0, leader(1), peer(1))),
            Ok(out_of_sync_event(0, 1))
        );
    }

    #[test]
    fn error_duplicate_proposal() {
        let mut core = core_genesis();

        let manifest = manifest_at(10);
        let hash = hash(&manifest);

        // Send proposal twice
        core.receive_proposal(manifest.clone()).unwrap();
        assert_eq!(
            core.receive_proposal(manifest),
            Err(Error::ProposalAlreadyExists(hash))
        );
    }
}
