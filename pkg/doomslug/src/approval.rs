use crate::error::Error;
use crate::types::{Balance, BlockHeight};
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::DateTime;
use primitives::peer::PeerIdSigner;
use primitives::u256::U256;
use primitives::{hash::CryptoHash, peer::Address, sig::Signature};
use serde::Serialize;
use sha3::{Digest, Keccak256};
use std::collections::HashMap;
use std::convert::TryFrom;

/// The part of the block approval that is different for endorsements and skips
#[derive(BorshSerialize, BorshDeserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApprovalInner {
    Endorsement(CryptoHash),
    Skip(BlockHeight),
}

/// Approval message to be signed
#[derive(BorshSerialize, BorshDeserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApprovalContent {
    pub inner: ApprovalInner,
    pub target_height: BlockHeight,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Approval {
    pub content: ApprovalContent,
    pub signature: Signature,
}

/// Block approval by other block producers with a signature
#[derive(BorshSerialize, BorshDeserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ApprovalValidated {
    pub content: ApprovalContent,
    pub signature: Signature,
    // This is a verified address from the signature, we cache it so we don't need to calculate it
    // every time we use it
    pub validator: Address,
}

/// Stores validator and its stake for two consecutive epochs.
/// It is necessary because the blocks on the epoch boundary need to contain approvals from both
/// epochs.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ApprovalStake {
    /// Account that stakes money.
    pub validator: Address,
    /// Public key of the proposed validator.
    // pub public_key: PublicKey,
    /// Stake / weight of the validator.
    pub stake_this_epoch: Balance,
    pub stake_next_epoch: Balance,
}

// Infromation about the approvals that we received.
#[derive(serde::Serialize, Debug, Default, Clone)]
pub struct ApprovalAtHeightStatus {
    // Map from validator id to the type of approval that they sent and timestamp.
    pub approvals: HashMap<Address, (ApprovalInner, DateTime<chrono::Utc>)>,
    // Time at which we received 2/3 approvals (doomslug threshold).
    pub ready_at: Option<DateTime<chrono::Utc>>,
}

// Information about the approval created by this node.
// Used for debug purposes only.
#[derive(serde::Serialize, Debug, Clone)]
pub struct ApprovalHistoryEntry {
    // If target_height == base_height + 1  - this is endorsement.
    // Otherwise this is a skip.
    pub parent_height: BlockHeight,
    pub target_height: BlockHeight,
    // Time when we actually created the approval and sent it out.
    pub approval_creation_time: DateTime<chrono::Utc>,
    // The moment when we were ready to send this approval (or skip)
    pub timer_started_ago_millis: u64,
    // But we had to wait at least this long before doing it.
    pub expected_delay_millis: u64,
}

impl TryFrom<Approval> for ApprovalValidated {
    type Error = Error;

    fn try_from(msg: Approval) -> Result<Self, Self::Error> {
        Ok(ApprovalValidated {
            validator: msg
                .signature
                .verify(&msg.content.hash())
                .ok_or(Error::InvalidSignature)?,
            content: msg.content,
            signature: msg.signature,
        })
    }
}

impl ApprovalContent {
    pub fn new(
        parent_hash: CryptoHash,
        parent_height: BlockHeight,
        target_height: BlockHeight,
    ) -> Self {
        let inner = ApprovalInner::new(&parent_hash, parent_height, target_height);
        ApprovalContent {
            inner,
            target_height,
        }
    }

    pub fn new_endorsement(parent_hash: &CryptoHash, target_height: u64) -> Self {
        let inner = ApprovalInner::Endorsement(*parent_hash);
        ApprovalContent {
            inner,
            target_height,
        }
    }

    fn hash(&self) -> CryptoHash {
        let mut hasher = Keccak256::new();
        let height_bytes = U256::from(self.target_height).to_big_endian();
        hasher.update(height_bytes);
        hasher.update(match &self.inner {
            ApprovalInner::Endorsement(h) => h.inner(),
            ApprovalInner::Skip(_) => todo!(),
        });
        CryptoHash(hasher.finalize().into())
    }

    pub fn serialize(&self) -> Vec<u8> {
        #[allow(clippy::unwrap_used)]
        #[allow(clippy::disallowed_methods)]
        borsh::to_vec(&self).unwrap()
    }

    pub fn to_approval(&self, peer: &PeerIdSigner) -> Approval {
        let sig = peer.sign(&self.hash());
        Approval {
            content: self.clone(),
            signature: sig,
        }
    }

    pub fn to_approval_validated(&self, peer: &PeerIdSigner) -> ApprovalValidated {
        let sig = peer.sign(&self.hash());
        ApprovalValidated {
            content: self.clone(),
            signature: sig,
            validator: peer.address(),
        }
    }
}

impl ApprovalInner {
    pub fn new(
        parent_hash: &CryptoHash,
        parent_height: BlockHeight,
        target_height: BlockHeight,
    ) -> Self {
        if target_height == parent_height + 1 {
            ApprovalInner::Endorsement(*parent_hash)
        } else {
            ApprovalInner::Skip(parent_height)
        }
    }
}
