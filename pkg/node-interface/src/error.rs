// lint-long-file-override allow-max-lines=300
use element::Element;
use primitives::block_height::BlockHeight;
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-rs")]
use ts_rs::TS;

/// Result for public errors from Payy Network
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error data for a note already spent
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteAlreadySpentData {
    /// Commitment of the spent note
    pub spent_note: Element,
    /// Transaction hash that included the spent note
    pub failing_txn_hash: Element,
}

/// Error data detailed the element related to the error
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ElementData {
    /// Element related to error
    pub element: Element,
}

/// Error data detailed the element related to the error
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct ElementsVecData {
    /// Element related to error
    pub elements: Vec<Element>,
}

/// Error data for mint in contract is different
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, thiserror::Error)]
#[error(
    "[node-interface] mint in contract mismatch: contract_value={contract_value:?}, contract_note_kind={contract_note_kind:?}, proof_value={proof_value:?}, proof_note_kind={proof_note_kind:?}"
)]
pub struct MintInContractIsDifferent {
    /// Value of mint in contract
    pub contract_value: Element,
    /// Kind of note in contract
    pub contract_note_kind: Element,
    /// Proof mint/burn value message
    pub proof_value: Element,
    /// Proof mint/burn note kind
    pub proof_note_kind: Element,
}

/// Node client interface error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A rpc error
    #[error("rpc error")]
    Rpc(#[from] RpcError),

    /// A client error
    #[error("client error")]
    Client(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

/// Public errors from Payy Network node
#[derive(Debug, Clone, thiserror::Error, HTTPErrorConversion, Serialize, Deserialize)]
pub enum RpcError {
    /// Submitted proof is not valid (i.e. verify of proof failed)
    #[bad_request("invalid-proof")]
    #[error("invalid proof")]
    InvalidProof,

    /// Output note included in the transaction is already present in the tree,
    /// the transaction could be accepted but then the user would lose their funds
    #[already_exists("txn-root-not-recent-enough")]
    #[error("txn root is not recent enough (must be last 120)")]
    TxnRootNotRecentEnough(ElementData),

    /// Conflicting element in another transaction in the same block
    #[already_exists("conflicting-elements")]
    #[error("conflicting element in the same block")]
    ConflictingElementsInBlock(ElementsVecData),

    /// Output commitment included in the transaction is already present in the tree,
    /// this indicates that the client may be trying to re-submit an existing transaction
    #[already_exists("output-commitments-exists")]
    #[error("output commitments already exists")]
    TxnOutputCommitmentsExist(ElementsVecData),

    /// Input commitments are not in the tree (note may never have existed or may be spent)
    #[not_found("input-commitments-not-found")]
    #[error("input commitments are not in the tree")]
    TxnInputCommitmentsNotInTree(ElementsVecData),

    /// Output note commitment
    #[already_exists("output-commitments-existed-recetly")]
    #[error("output commitments existed recently")]
    TxnOutputCommitmentsExistedRecently(ElementsVecData),

    /// Mint is not in the contract on the base chain, it is required that
    /// all mints be registered on the base chain before being submitted to Payy
    /// Network, as we need to validate the users locked funds before releasing them
    /// on Payy Network
    #[not_found("mint-not-in-contract")]
    #[error("mint leaf is not in the contract")]
    MintIsNotInTheContract(ElementData),

    /// Mint has already been spent on the Payy Network. This error occurs when attempting
    /// to spend a mint that has already been rolled up. The smart contract has marked the mint as spent.
    /// Each mint can only be spent once to prevent double-spending attacks. The client should check
    /// their transaction history and avoid reusing spent mints.
    #[already_exists("mint-is-already-spent")]
    #[error("mint is already spent")]
    MintIsAlreadySpent(ElementsVecData),

    /// Mint is in the contract on the base chain, but the minted amount is different
    /// to the txn proof sent. A new txn should be submitted with the correct values.
    #[bad_request("mint-in-contract-is-different")]
    #[error("mint in contract is different to provided txn proof")]
    MintInContractIsDifferent(Box<MintInContractIsDifferent>),

    /// Mint references a chain that is not configured on this validator
    #[bad_request("unsupported-chain")]
    #[error("[node-interface] unsupported chain {chain_id}")]
    UnsupportedChain {
        /// Chain identifier encoded in the mint note kind.
        chain_id: u64,
    },

    /// Transaction contains duplicate input commitments
    #[bad_request("duplicate-input-commitments")]
    #[error("transaction contains duplicate input commitments")]
    TxnDuplicateInputCommitments(ElementsVecData),

    /// Transaction contains duplicate output commitments
    #[bad_request("duplicate-output-commitments")]
    #[error("transaction contains duplicate output commitments")]
    TxnDuplicateOutputCommitments(ElementsVecData),

    /// Transaction uses a commitment that is already pending in the mempool
    #[already_exists("commitment-already-pending")]
    #[error("commitment already pending in another transaction")]
    TxnCommitmentAlreadyPending(ElementsVecData),

    /// Prevent the user from accidentally burning to the zero address and therefore
    /// losing their funds
    #[bad_request("burn-to-address-cannot-be-zero")]
    #[error("burn 'to' address cannot be zero")]
    BurnToAddressCannotBeZero,

    /// Element was not found
    #[not_found("element-not-found")]
    #[error("failed to find element in tree")]
    ElementNotFound(ElementData),

    /// Transaction was not found
    #[not_found("txn-not-found")]
    #[error("failed to find transaction")]
    TxnNotFound(ElementData),

    /// Element is too large for the modulus of the zk primitive
    #[bad_request("invalid-element-size")]
    #[error("invalid element, size exceeds modulus")]
    InvalidElementSize(ElementData),

    /// Element string provided was in invalid format
    #[bad_request("failed-to-parse-element")]
    #[error("invalid element")]
    FailedToParseElement(ElementData),

    /// Mint hash already exists
    #[already_exists("mint-hash-already-exists")]
    #[error("mint hash already exists")]
    MintHashAlreadyExists(ElementData),

    /// Transaction has already been included in a block
    #[already_exists("txn-already-included")]
    #[error("[node-interface] transaction already included in block {height}")]
    TxnAlreadyIncluded {
        /// Root hash of the block that included the transaction
        root_hash: Element,
        /// Hash of the submitted transaction
        txn_hash: Element,
        /// Height of the block that already contains the transaction
        height: BlockHeight,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert() {
        let error = RpcError::InvalidProof;
        let http_error: HTTPError = error.into();
        let http_output: ErrorOutput = http_error.into();
        let orig_error: RpcError = http_output.try_into().unwrap();
        println!("orig_error: {orig_error}");
    }
}
