use borsh::{BorshDeserialize, BorshSerialize};

use super::ProposalHash;

/// ProposalHeader provides the position of the proposal in the block
/// sequence and the hash of the proposal at that sequence.
#[derive(Debug, Clone, Hash, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ProposalHeader {
    pub hash: ProposalHash,
    pub height: u64,
    pub skips: u64,
}
