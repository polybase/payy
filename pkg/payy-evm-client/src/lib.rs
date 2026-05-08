#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![deny(missing_docs)]

//! Rust implementation of the Payy EVM client.

mod bridge;
mod client;
mod links;
mod local_signer;
mod operations;
mod raw_submitter;
mod signing;
mod state;
mod util;

#[cfg(test)]
mod tests;

pub use bridge::{BridgeClient, MerklePathResult};
pub use client::{
    BaseClient, EvmClient, PayyClientBuilder, PrivacyClient, PrivacyNamespace, TransactionsClient,
};
pub use links::{
    LinksClient, encode_direct_claim_link, encode_ephemeral_claim_link, parse_claim_link,
};
pub use local_signer::{LocalPrivacySigner, derive_grumpkin_private_key};
pub use operations::{
    BurnParams, ClaimClient, DirectSendParams, EphemeralSendParams, MintParams, OperationBuilder,
    Prepared, SendClient,
};
#[cfg(feature = "alloy")]
pub use payy_evm_client_alloy::{
    AlloyRawTransactionSubmitter, AlloyReadClient, AlloyTransactionOptions, AlloyWalletSubmitter,
    alloy_raw_transaction_submitter, alloy_read_client, alloy_wallet_submitter,
    alloy_wallet_submitter_with_address, to_alloy_transaction, to_alloy_transaction_with_options,
};
pub use payy_evm_client_interface::{
    Address, B256, Bytes, ClaimLink, ClaimSourceKind, ConfirmedOperationResult, DirectLinkedNote,
    DirectSendDelivery, EphemeralKeyPair, Error, EvmAccount, EvmSignerAccount, IncomingNote,
    IncomingNoteStatus, IncomingTransfer, OwnedNote, OwnedNoteState, OwnerSignature,
    ParsedClaimLink, PayyBlockTag, PayyEvmCallRequest, PayyEvmFeeData, PayyEvmLog,
    PayyEvmLogFilter, PayyEvmReadClient, PayyEvmSubmitter, PayyEvmTransactionRequest,
    PayyNetworkConfig, PayyNetworkPreset, PayyRawTransactionSubmitter, PayyTransactionCountArgs,
    PayyTransactionReceipt, PayyTransactionStatus, PayyWaitForTransactionReceiptArgs,
    PreparedOperationResult, PreparedPrivacyCall, PreparedPrivacyCallSource, PrivacyAccount,
    PrivacyAddress, PrivacyAddressPrefix, PrivacyOperationKind, PrivacySigner,
    PrivacySignerAccount, PrivacyStatePreview, PrivateBalance, PrivateBalanceState, Result,
    SignerResponseField, SourceChainPosition, SubmittedOperationResult, TxHash, TxnData,
    ValidationErrorKind,
};
pub use raw_submitter::LocalRawEvmSubmitter;
pub use state::{
    BalancesClient, IncomingClient, IncomingListParams, IncomingWatchResult, NotesClient,
    OwnedNoteGetParams,
};
