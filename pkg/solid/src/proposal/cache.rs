// lint-long-file-override allow-max-lines=500
use crate::{
    proposal::{Proposal, ProposalHash},
    traits::App,
};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Responsible for storing proposals temporarily in the cache.
/// Provides untility methods for easily traversing proposals.
#[derive(Debug)]
pub struct ProposalCache<A: App> {
    /// Hash of last confirmed proposal, we can work backwards from here to get
    /// all other confirmed proposals in the cache.
    pub(crate) last_confirmed_proposal_hash: ProposalHash,

    /// List of proposals cached in memory
    proposals: HashMap<ProposalHash, Proposal<A>>,

    /// Config for the proposal cache
    cache_size: u64,
}

impl<A: App> ProposalCache<A> {
    pub fn new(last_confirmed_proposal: Proposal<A>, cache_size: u64) -> Self {
        let proposal_hash = last_confirmed_proposal.hash().clone();
        let mut proposals = HashMap::new();

        // Add last confirmed proposal to pending proposals
        proposals.insert(proposal_hash.clone(), last_confirmed_proposal);

        ProposalCache {
            last_confirmed_proposal_hash: proposal_hash,
            proposals,
            cache_size,
        }
    }

    /// Confirmed height
    pub fn height(&self) -> u64 {
        // We can use unwrap because last_confirmed_proposal_hash must always be set
        #[allow(clippy::unwrap_used)]
        self.proposals
            .get(&self.last_confirmed_proposal_hash)
            .unwrap()
            .height()
    }

    #[cfg(test)]
    fn len(&self) -> u64 {
        self.proposals.len() as u64
    }

    /// Check if a proposal exists in the cache
    pub fn contains(&self, hash: &ProposalHash) -> bool {
        self.proposals.contains_key(hash)
    }

    /// Insert a proposal into the cache
    pub fn insert(&mut self, proposal: Proposal<A>) {
        self.proposals.insert(proposal.hash().clone(), proposal);
    }

    /// Get a proposal by hash (mutable)
    pub fn get_mut(&mut self, proposal_hash: &ProposalHash) -> Option<&mut Proposal<A>> {
        self.proposals.get_mut(proposal_hash)
    }

    /// Get a proposal
    pub fn get(&self, proposal_hash: &ProposalHash) -> Option<&Proposal<A>> {
        self.proposals.get(proposal_hash)
    }

    // Remove a proposal
    pub fn remove(&mut self, proposal_hash: &ProposalHash) -> Option<Proposal<A>> {
        self.proposals.remove(proposal_hash)
    }

    /// Get the last confirmed proposal
    pub fn last_confirmed_proposal(&self) -> &Proposal<A> {
        // We can use unwrap because last_confirmed_proposal_hash must always be set
        #[allow(clippy::unwrap_used)]
        self.proposals
            .get(&self.last_confirmed_proposal_hash)
            .unwrap()
    }

    /// Returns all confirmed proposals from height to confirmed proposal
    pub fn proposals_from(&self, from_height: u64) -> Vec<&Proposal<A>> {
        self.proposals
            .values()
            .filter(|p| p.height() >= from_height)
            .collect()
    }

    /// Returns all confirmed proposals from height to confirmed proposal
    pub fn confirmed_proposals_from(&self, from_height: u64) -> Vec<&Proposal<A>> {
        // Start with the last confirmed proposal and work backwards
        // We can use unwrap because last_confirmed_proposal_hash must always be set
        #[allow(clippy::unwrap_used)]
        let mut proposal = self
            .proposals
            .get(&self.last_confirmed_proposal_hash)
            .unwrap();

        let mut proposals = vec![proposal];

        // Loop through to get the next proposal by looking at the chain of proposals
        while proposal.height() >= from_height {
            if let Some(p) = self.proposals.get(proposal.last_hash()) {
                proposals.push(p);
                proposal = p
            } else {
                return proposals;
            }
        }

        proposals
    }

    /// Next pending proposal to be processed (`confirmed height + 1 + offset`).
    /// If there are a no pending commits for a given offset
    /// then None is returned. Offset can be used to get
    /// proposals higher up the chain. This is usually only used with a value of `0` or `1`.
    ///   - `0`: next proposal (used to send accept)
    ///   - `1`: proposal after next proposal (used to confirm the next proposal)
    pub fn next_pending_proposal(&self, offset: u64) -> Option<&Proposal<A>> {
        // Get the first next proposal, by looking for an unconfirmed proposal with the highest height
        // and skip
        let mut proposal = self.max_continuous_proposal()?;

        // Max proposal is not higher than requested (minimum is always height + 1, as we
        // are looking for the next proposal)
        if proposal.height() < self.height() + 1 + offset {
            return None;
        }

        // Loop through to get the next proposal by looking at the chain of proposals
        while proposal.height() > self.height() + 1 + offset {
            proposal = self.proposals.get(proposal.last_hash())?;
        }

        Some(proposal)
    }

    /// Max proposal that decends to the last confirmed proposal
    fn max_continuous_proposal(&self) -> Option<&Proposal<A>> {
        // Get the last confirmed, so we can check if an unconfirmed proposal is a parent of the
        // last confirmed proposal
        let last_confirmed = &self.last_confirmed_proposal_hash;
        let from_height = self.height();

        self.proposals
            .values()
            .filter(|proposal| {
                proposal.height() > from_height
                    && self.is_decendent(last_confirmed, proposal.hash())
            })
            .max_by(|a, b| {
                (a.height(), a.skips())
                    .partial_cmp(&(b.height(), b.skips()))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Confirm a proposal, all subsequent proposals must now
    /// include this proposal in the tree.
    pub fn confirm(&mut self, proposal_hash: ProposalHash) {
        self.last_confirmed_proposal_hash = proposal_hash;
        self.purge();
    }

    /// Check if a hash is a decendent of another hash
    fn is_decendent(&self, decendent_hash: &ProposalHash, parent_hash: &ProposalHash) -> bool {
        let mut proposal = match self.proposals.get(parent_hash) {
            Some(p) => p,
            None => return false,
        };

        loop {
            if proposal.hash() == decendent_hash {
                return true;
            }

            let next_decendent = match self.proposals.get(proposal.last_hash()) {
                Some(p) => p,
                None => return false,
            };

            // Check the decendent is one level higher than the parent
            if next_decendent.height() + 1 != proposal.height() {
                return false;
            }

            proposal = next_decendent
        }
    }

    /// Get decendents from a parent to a specified decendent (not including the specified decendent).
    /// Ordered from parent to decendent
    pub fn decendents(
        &self,
        decendent_hash: &ProposalHash,
        parent_hash: &ProposalHash,
    ) -> Vec<&Proposal<A>> {
        // Get the first proposal
        let mut proposal = match self.proposals.get(parent_hash) {
            Some(p) => p,
            None => return vec![],
        };

        let mut proposals = vec![];

        // Loop through to get the next proposal by looking at the chain of proposals
        // until we reach the decendent
        loop {
            if proposal.hash() == decendent_hash {
                return proposals;
            }

            proposals.push(proposal);

            proposal = match self.proposals.get(proposal.last_hash()) {
                Some(p) => p,
                None => return vec![],
            };
        }
    }

    /// Remove redundant proposals from the cache
    fn purge(&mut self) {
        let confirmed_height = self.height();
        let confirmed_hash = self.last_confirmed_proposal_hash.clone();

        let keys_to_remove = self
            .proposals
            .iter()
            .filter(|(_, p)| match confirmed_height.partial_cmp(&p.height()) {
                Some(Ordering::Greater) => {
                    if p.height() + (self.cache_size) < confirmed_height {
                        return true;
                    }
                    false
                }
                Some(Ordering::Less) => !self.is_decendent(&confirmed_hash, p.hash()),
                Some(Ordering::Equal) => p.hash() != &self.last_confirmed_proposal_hash,
                None => true,
            })
            .map(|(k, _)| k.clone())
            .collect::<Vec<_>>();

        for key in keys_to_remove {
            self.proposals.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proposal::{Manifest, ManifestContent, ProposalHash};
    use crate::test::app::{TestApp, UncheckedPeerId as PeerId};

    fn create_peers() -> [PeerId; 3] {
        let p1 = PeerId::new(vec![1u8]);
        let p2 = PeerId::new(vec![2u8]);
        let p3 = PeerId::new(vec![3u8]);
        [p1, p2, p3]
    }

    fn create_proposal(
        height: u64,
        skips: u64,
        last_proposal_hash: ProposalHash,
    ) -> (Proposal<TestApp>, ProposalHash) {
        let m = Manifest::new(
            ManifestContent {
                last_proposal_hash,
                height,
                skips,
                leader_id: PeerId::new(vec![1u8]),
                state: 0.into(),
                validators: create_peers().to_vec(),
                accepts: vec![],
            },
            vec![],
        );
        let m_hash = TestApp::hash(&m);
        (Proposal::new(m), m_hash)
    }

    #[test]
    fn test_new_cache() {
        let (genesis, genesis_hash) = create_proposal(0, 0, ProposalHash::genesis());
        let cache = ProposalCache::new(genesis.clone(), 1000);

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.height(), 0);
        assert_eq!(cache.last_confirmed_proposal(), &genesis);
        assert!(cache.contains(&genesis_hash), "contains genesis hash");
    }

    #[test]
    fn test_insert() {
        let (genesis, genesis_hash) = create_proposal(0, 0, ProposalHash::genesis());
        let mut cache = ProposalCache::new(genesis, 1000);

        let (p1, _) = create_proposal(1, 0, genesis_hash);
        cache.insert(p1);

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.height(), 0);
    }

    #[test]
    fn test_confirm_proposal() {
        let (genesis, genesis_hash) = create_proposal(0, 0, ProposalHash::genesis());
        let mut cache = ProposalCache::new(genesis, 1000);

        let (p1, p1_hash) = create_proposal(1, 0, genesis_hash);
        cache.insert(p1.clone());
        cache.confirm(p1_hash);

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.height(), 1);
        assert_eq!(cache.last_confirmed_proposal(), &p1);
    }

    #[test]
    fn test_is_decendent() {
        let (genesis, genesis_hash) = create_proposal(0, 0, ProposalHash::genesis());
        let mut cache = ProposalCache::new(genesis, 1000);

        let (p1, p1_hash) = create_proposal(1, 0, genesis_hash.clone());
        let (p2, p2_hash) = create_proposal(2, 0, p1_hash.clone());
        let (p3, p3_hash) = create_proposal(3, 0, genesis_hash.clone());
        cache.insert(p1);
        cache.insert(p2);
        cache.insert(p3);

        assert!(
            cache.is_decendent(&genesis_hash, &p2_hash),
            "genesis is decendent of p2"
        );

        assert!(
            !cache.is_decendent(&genesis_hash, &p3_hash),
            "genesis is decendent of p3"
        );

        assert!(
            !cache.is_decendent(&p2_hash, &genesis_hash),
            "p2 is not decendent of genesis"
        );

        assert!(
            !cache.is_decendent(&p1_hash, &p3_hash),
            "p1 not is decendent of p3"
        );
    }

    #[test]
    fn test_purge_proposals() {
        let (genesis, genesis_hash) = create_proposal(0, 0, ProposalHash::genesis());
        let mut cache = ProposalCache::new(genesis, 1000);

        let (p1a, p1a_hash) = create_proposal(1, 0, genesis_hash);
        let (p1b, p1b_hash) = create_proposal(1, 1, ProposalHash::from_vec_hash(vec![1u8]));
        // let (p2a, p2a_hash) = create_proposal(2, 0, p1_hash.clone());
        // let (p2b, p2b_hash) = create_proposal(2, 1, p1_hash);

        cache.insert(p1a);
        cache.insert(p1b);

        assert_eq!(cache.len(), 3);

        cache.purge();

        assert_eq!(cache.len(), 2);
        assert!(cache.contains(&p1a_hash), "p1a should  not be purged");
        assert!(!cache.contains(&p1b_hash), "p1b should be purged");

        let mut last_hash = p1a_hash;
        for i in 2..1010 {
            let (p, h) = create_proposal(i, 0, last_hash);
            last_hash = h.clone();
            cache.insert(p);
        }

        cache.confirm(last_hash);

        assert_eq!(cache.len(), 1001);
    }

    #[test]
    fn test_next_pending_proposal() {
        let (genesis, genesis_hash) = create_proposal(0, 0, ProposalHash::genesis());
        let mut cache = ProposalCache::new(genesis, 1000);

        let (p1, p1_hash) = create_proposal(1, 0, genesis_hash);
        let (p2a, p2a_hash) = create_proposal(2, 0, p1_hash.clone());
        let (p2b, p2b_hash) = create_proposal(2, 1, p1_hash);
        let (p3a, _) = create_proposal(3, 0, p2a_hash.clone());
        let (p3b, _) = create_proposal(3, 1, p2b_hash);

        cache.insert(p1.clone());
        cache.insert(p2a);
        cache.insert(p2b);
        cache.insert(p3a.clone());
        cache.insert(p3b);

        assert_eq!(cache.len(), 6);
        assert_eq!(cache.height(), 0);
        assert_eq!(cache.next_pending_proposal(0), Some(&p1));

        cache.confirm(p2a_hash);

        assert_eq!(cache.next_pending_proposal(0), Some(&p3a));
    }

    #[test]
    fn test_next_pending_proposal_invalid_decendent() {
        let (genesis, genesis_hash) = create_proposal(0, 0, ProposalHash::genesis());
        let mut cache = ProposalCache::new(genesis, 1000);

        let (p1_0, p1_0_hash) = create_proposal(1, 0, genesis_hash);

        // Incorrect decendent, so should be ignored
        let (p1_1, _) = create_proposal(1, 1, p1_0_hash);

        cache.insert(p1_0.clone());
        cache.insert(p1_1);

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.height(), 0);
        assert_eq!(cache.next_pending_proposal(0), Some(&p1_0));
    }

    #[test]
    fn test_next_pending_proposal_no_pending() {
        let (genesis, _) = create_proposal(0, 0, ProposalHash::genesis());
        let cache = ProposalCache::new(genesis, 1000);

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.height(), 0);
        assert_eq!(cache.next_pending_proposal(0), None);
    }

    #[test]
    fn test_next_pending_proposal_with_offset() {
        let (genesis, genesis_hash) = create_proposal(0, 0, ProposalHash::genesis());
        let mut cache = ProposalCache::new(genesis, 1000);

        let (p1, p1_hash) = create_proposal(1, 0, genesis_hash);
        let (p2, p2_hash) = create_proposal(2, 0, p1_hash);
        let (p3, _) = create_proposal(3, 0, p2_hash);

        cache.insert(p1.clone());
        cache.insert(p2.clone());
        cache.insert(p3.clone());

        assert_eq!(cache.len(), 4);
        assert_eq!(cache.height(), 0);
        assert_eq!(cache.next_pending_proposal(0), Some(&p1));
        assert_eq!(cache.next_pending_proposal(1), Some(&p2));
        assert_eq!(cache.next_pending_proposal(2), Some(&p3));
        assert_eq!(cache.next_pending_proposal(3), None);
    }
}
