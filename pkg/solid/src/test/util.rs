// lint-long-file-override allow-max-lines=300
use super::app::{InsecurePeerSigner, TestApp, TestAppTxnState, UncheckedPeerId as PeerId};
use crate::proposal::{Manifest, Proposal};
pub use crate::{
    config::SolidConfig,
    event::SolidEvent,
    proposal::{ManifestContent, ProposalAccept, ProposalHash, ProposalHeader},
    solid::core::SolidCore,
    traits::App,
};
use std::vec;

pub fn peer(id: u8) -> PeerId {
    PeerId::new(vec![id])
}

pub fn leader(id: u8) -> PeerId {
    peer(id)
}

pub fn signer(id: u8) -> InsecurePeerSigner {
    InsecurePeerSigner::new(peer(id))
}

pub fn create_peers() -> [PeerId; 4] {
    [peer(1), peer(2), peer(3), peer(4)]
}

pub fn skips(skip: u64) -> u64 {
    skip
}

pub fn hash(m: &ManifestContent<PeerId, TestAppTxnState>) -> ProposalHash {
    TestApp::hash(m)
}

pub fn proposal(manifest: Manifest<PeerId, TestAppTxnState>) -> Proposal<TestApp> {
    Proposal::new(manifest)
}

pub fn create_proposal(
    height: u64,
    skips: u64,
    leader: u8,
    last_proposal_manifest: &ManifestContent<PeerId, TestAppTxnState>,
) -> Proposal<TestApp> {
    Proposal::new(create_manifest(
        height,
        skips,
        leader,
        last_proposal_manifest,
    ))
}

pub fn create_manifest(
    height: u64,
    skips: u64,
    leader: u8,
    last_proposal_manifest: &ManifestContent<PeerId, TestAppTxnState>,
) -> Manifest<PeerId, TestAppTxnState> {
    let last_proposal_hash = &TestApp::hash(last_proposal_manifest);
    create_manifest_with_accepts(
        height,
        skips,
        leader,
        last_proposal_hash,
        vec![
            accept(last_proposal_manifest, 0, peer(leader), peer(1)),
            accept(last_proposal_manifest, 0, peer(leader), peer(2)),
            accept(last_proposal_manifest, 0, peer(leader), peer(3)),
        ],
    )
}

pub fn create_manifest_with_accepts(
    height: u64,
    skips: u64,
    leader: u8,
    last_proposal_hash: &ProposalHash,
    accepts: Vec<ProposalAccept<PeerId>>,
) -> Manifest<PeerId, TestAppTxnState> {
    Manifest::new(
        ManifestContent {
            last_proposal_hash: last_proposal_hash.clone(),
            height,
            skips,
            leader_id: peer(leader),
            state: 0.into(),
            validators: create_peers().to_vec(),
            accepts,
        },
        vec![],
    )
}

pub fn accept(
    manifest: &ManifestContent<PeerId, TestAppTxnState>,
    skips: u64,
    leader_id: PeerId,
    from: PeerId,
) -> ProposalAccept<PeerId> {
    ProposalAccept {
        proposal: ProposalHeader {
            hash: TestApp::hash(manifest),
            height: manifest.height,
            skips: manifest.skips,
        },
        leader_id,
        skips,
        from,
        signature: vec![],
    }
}

pub fn accept_event(
    proposal: &ManifestContent<PeerId, TestAppTxnState>,
    skips: u64,
    leader_id: PeerId,
    from: PeerId,
) -> Option<SolidEvent<PeerId, TestAppTxnState>> {
    Some(SolidEvent::Accept {
        accept: accept(proposal, skips, leader_id, from),
    })
}

pub fn commit_event(
    manifest: &Manifest<PeerId, TestAppTxnState>,
    confirmed_by: &Manifest<PeerId, TestAppTxnState>,
) -> Option<SolidEvent<PeerId, TestAppTxnState>> {
    Some(SolidEvent::Commit {
        manifest: manifest.clone(),
        confirmed_by: confirmed_by.clone(),
    })
}

pub fn out_of_sync_event(
    height: u64,
    max_seen_height: u64,
) -> Option<SolidEvent<PeerId, TestAppTxnState>> {
    Some(SolidEvent::OutOfSync {
        height,
        max_seen_height,
    })
}

pub fn propose_event(
    last_proposal_hash: ProposalHash,
    height: u64,
    skips: u64,
    accepts: Vec<ProposalAccept<PeerId>>,
) -> Option<SolidEvent<PeerId, TestAppTxnState>> {
    Some(SolidEvent::Propose {
        last_proposal_hash,
        height,
        skips,
        accepts,
    })
}

pub fn genesis_manifest() -> Manifest<PeerId, TestAppTxnState> {
    Manifest::genesis(create_peers().to_vec())
}

pub fn core_genesis() -> SolidCore<TestApp> {
    core_genesis_with_config(SolidConfig::default())
}

pub fn core_genesis_with_config(config: SolidConfig) -> SolidCore<TestApp> {
    SolidCore::with_last_confirmed(signer(1), genesis_manifest(), TestApp, config)
}

pub fn core_with_last_confirmed(
    proposal: &Manifest<PeerId, TestAppTxnState>,
) -> SolidCore<TestApp> {
    SolidCore::with_last_confirmed(signer(1), proposal.clone(), TestApp, SolidConfig::default())
}

pub fn gensis_hash() -> ProposalHash {
    TestApp::hash(&Manifest::<PeerId, TestAppTxnState>::genesis(
        create_peers().to_vec(),
    ))
}

pub fn manifest_at(height: u64) -> Manifest<PeerId, TestAppTxnState> {
    create_manifest(
        height,
        0,
        1,
        &create_manifest_with_accepts(0, 0, 0, &ProposalHash::genesis(), vec![]),
    )
}

pub fn last_confirmed(store: &SolidCore<TestApp>) -> Manifest<PeerId, TestAppTxnState> {
    store
        .proposals()
        .last_confirmed_proposal()
        .manifest()
        .clone()
}

#[macro_export]
macro_rules! assert_none {
    ($expr:expr $(,)?) => ({
      match $expr {
        None => (),
        ref got => panic!("assertion failed: `None` does not equal `{:?}`", got),
      }
    });
    ($expr:expr, $($arg:tt)*) => ({
      match $expr {
        None => (),
        ref got => {
            let msg = format!($($arg)*);
            panic!("{}: `None` does not equal `{}`", msg, got);
        }
      }
    });
}
